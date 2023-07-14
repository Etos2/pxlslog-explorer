use std::{ffi::OsStr, path::PathBuf};

use itertools::{izip, Itertools};

use super::{
    error::{ConfigError, ConfigValue, InvalidPathKind},
    CanvasConfig, DestinationConfig, DestinationKind, MethodConfig, MethodKind, PaletteSource,
    PixelFormat, ProgramConfig, RenderConfig,
};
use crate::{
    render::{pixel::Rgba, Step},
    util::io::Source,
};

// TODO: Verify if true + verify transparency support
const SUPPORTED_IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "pbm", "pam", "ppm", "pgm", "tiff", "tif", "tga", "bmp",
    "ico", "exr", "ff", "avif", "qoi",
];

pub trait BuilderOverride {
    fn or(self, rhs: &Self) -> Self;
}

pub struct ConfigBuilder {
    pub program: ProgramConfigBuilder,
    pub render_base: RenderConfigBuilder,
    pub render: Vec<RenderConfigBuilder>,
}

impl ConfigBuilder {
    pub fn build(self) -> Result<(ProgramConfig, Vec<RenderConfig>), ConfigError> {
        Ok((
            self.program.build()?,
            self.render
                .into_iter()
                .map(|r| r.or(&self.render_base).build())
                .try_collect()?,
        ))
    }
}

impl BuilderOverride for ConfigBuilder {
    fn or(self, rhs: &Self) -> Self {
        Self {
            program: self.program.or(&rhs.program),
            render_base: self.render_base.or(&rhs.render_base),
            render: izip!(self.render, &rhs.render)
                .map(|(lhs, rhs)| lhs.or(rhs))
                .collect_vec(),
        }
    }
}

pub struct ProgramConfigBuilder {
    pub log_source: Option<Source>,
    pub quiet: Option<bool>,
    pub threads: Option<usize>,
    pub dry_run: Option<bool>,
}

impl ProgramConfigBuilder {
    fn build(self) -> Result<ProgramConfig, ConfigError> {
        if let Some(Source::File(path)) = &self.log_source {
            if !path.exists() {
                Err(ConfigError::new_invalid_path(
                    ConfigValue::ProgramLogSource,
                    path.clone(),
                    InvalidPathKind::NotFound,
                ))?
            } else if path.is_dir() {
                Err(ConfigError::new_invalid_path(
                    ConfigValue::ProgramLogSource,
                    path.clone(),
                    InvalidPathKind::NotFile,
                ))?
            }
        }

        Ok(ProgramConfig {
            quiet: self.quiet.unwrap_or_default(),
            threads: self.threads.unwrap_or_default(),
            dry_run: self.dry_run.unwrap_or_default(),
            log_source: self.log_source.ok_or(ConfigError::new_missing(vec![
                ConfigValue::ProgramLogSource,
            ]))?,
        })
    }
}

impl BuilderOverride for ProgramConfigBuilder {
    fn or(self, rhs: &Self) -> Self {
        Self {
            log_source: self.log_source.or(rhs.log_source.clone()),
            quiet: self.quiet.or(rhs.quiet),
            threads: self.threads.or(rhs.threads),
            dry_run: self.dry_run.or(rhs.dry_run),
        }
    }
}

pub struct RenderConfigBuilder {
    pub method_palette_source: Option<PaletteSource>,
    pub method_kind: Option<MethodKind>,
    pub canvas_source: Option<PathBuf>,
    pub canvas_size: Option<(u32, u32, u32, u32)>,
    pub canvas_background: Option<Rgba>,
    pub canvas_transparency: Option<bool>,
    pub destination_format: Option<PixelFormat>,
    pub destination_kind: Option<DestinationKind>,
    pub step: Option<Step>,
}

impl RenderConfigBuilder {
    pub fn new() -> Self {
        RenderConfigBuilder {
            method_palette_source: None,
            method_kind: None,
            canvas_source: None,
            canvas_size: None,
            canvas_background: None,
            canvas_transparency: None,
            destination_format: None,
            destination_kind: None,
            step: None,
        }
    }

    fn build(mut self) -> Result<RenderConfig, ConfigError> {
        self.verify()?;
        self.check_paths()?;

        Ok(RenderConfig {
            destination: DestinationConfig {
                format: self.destination_format.unwrap(),
                kind: self.destination_kind.unwrap(),
            },
            method: MethodConfig {
                palette: self.method_palette_source,
                kind: self.method_kind.unwrap_or_default(),
            },
            canvas: CanvasConfig {
                source: self.canvas_source,
                size: self.canvas_size,
                background: self.canvas_background,
                transparency: self.canvas_transparency.unwrap_or_default(),
            },
            step: self.step.unwrap_or_default(),
        })
    }

    fn verify(&mut self) -> Result<(), ConfigError> {
        let mut err_values = Vec::new();
        if let Some(kind) = &self.destination_kind {
            if let DestinationKind::File(path) = kind {
                if let Some(extension) = path.extension().and_then(OsStr::to_str) {
                    if SUPPORTED_IMAGE_EXTENSIONS.contains(&extension) {
                        match self.canvas_transparency {
                            Some(transparent) => {
                                if transparent {
                                    eprintln!("Infered output format as RGBA");
                                    self.destination_format = Some(PixelFormat::Rgba);
                                } else {
                                    eprintln!("Infered output format as RGB");
                                    self.destination_format = Some(PixelFormat::Rgb);
                                }
                            }
                            None => {
                                eprintln!("Infered canvas as transparent");
                                eprintln!("Infered output format as RGBA");
                                self.canvas_transparency = Some(true);
                                self.destination_format = Some(PixelFormat::Rgba);
                            }
                        }
                    }
                }
            } else {
                Err(ConfigError::new_infer(ConfigValue::DestinationFormat))?
            }
        } else {
            err_values.push(ConfigValue::DestinationKind)
        }

        if self.destination_format.is_none() {
            err_values.push(ConfigValue::DestinationFormat)
        }

        if !err_values.is_empty() {
            Err(ConfigError::new_missing(err_values))
        } else {
            Ok(())
        }
    }

    fn check_paths(&self) -> Result<(), ConfigError> {
            if let Some(DestinationKind::Dir(path)) = &self.destination_kind {
                    if !path.exists() {
                        Err(ConfigError::new_invalid_path(
                            ConfigValue::DestinationKind,
                            path.clone(),
                            InvalidPathKind::NotFound,
                        ))?
                    }
            }

        if let Some(path) = &self.canvas_source {
            if !path.exists() {
                Err(ConfigError::new_invalid_path(
                    ConfigValue::CanvasBackgroundSource,
                    path.clone(),
                    InvalidPathKind::NotFound,
                ))?
            } else if path.is_dir() {
                Err(ConfigError::new_invalid_path(
                    ConfigValue::CanvasBackgroundSource,
                    path.clone(),
                    InvalidPathKind::NotFile,
                ))?
            }
        }

        Ok(())
    }
}

impl BuilderOverride for RenderConfigBuilder {
    fn or(self, rhs: &Self) -> Self {
        Self {
            method_palette_source: self
                .method_palette_source
                .or(rhs.method_palette_source.clone()),
            method_kind: self.method_kind.or(rhs.method_kind),
            canvas_source: self.canvas_source.or(rhs.canvas_source.clone()),
            canvas_size: self.canvas_size.or(rhs.canvas_size),
            canvas_background: self.canvas_background.or(rhs.canvas_background),
            canvas_transparency: self.canvas_transparency.or(rhs.canvas_transparency),
            destination_format: self.destination_format.or(rhs.destination_format),
            destination_kind: self.destination_kind.or(rhs.destination_kind.clone()),
            step: self.step.or(rhs.step),
        }
    }
}
