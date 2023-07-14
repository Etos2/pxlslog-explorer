use std::{num::NonZeroI64, path::PathBuf};

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use nonzero_ext::nonzero;

use crate::{
    render::{
        pixel::{Pixel, Rgba},
        Step,
    },
    util::io::{Destination, Source},
};

use super::{
    super::{
        builder::{ProgramConfigBuilder, RenderConfigBuilder},
        error::ConfigError,
        ConfigBuilder, DestinationKind, MethodKind, PaletteSource, PixelFormat,
    },
    ConfigSource,
};

#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
#[command(
    name = "PxlsLog-Explorer",
    author = " - Etos2 <github.com/Etos2>",
    version,
    about = "Render individual frames or output raw frame data to STDOUT.",
    long_about = "Render individual frames or output raw frame data to STDOUT.
Guaranted to produce 2 frames per render, where the first frame is the background and the last frame is the complete contents of the log.
To output only the final result, use the \"--screenshot\" arg or manually skip the first frame \"--skip\"."
)]
pub struct CliData {
    #[arg(short, long, value_name("PATH"))]
    #[arg(help = "Filepath of config")]
    pub config: Option<PathBuf>,
    #[command(flatten)]
    pub program: ProgramSettings,
    #[command(flatten)]
    pub render: RenderSettings,
}

#[derive(Args, Debug)]
pub struct ProgramSettings {
    #[arg(short, long)]
    #[arg(value_name("PATH"))]
    #[arg(help = "Filepath of input log file")]
    #[arg(display_order = 0)]
    pub log: Option<Source>,
    #[arg(short, long)]
    #[arg(help = "Silence all logging")]
    pub quiet: Option<bool>,
    // #[arg(short, long, action = clap::ArgAction::Count)]
    // #[arg(help = "Enable verbosity")]
    // pub verbose: Option<u8>,
    // #[clap(long)]
    // #[clap(help = "Forcibly exit rather than ignoring soft errors")]
    // pub strict: Option<bool>,
    #[arg(long)]
    #[arg(value_name("INT"))]
    #[arg(help = "Number of threads utilised [Defaults to all available threads]")]
    pub threads: Option<usize>,
    #[arg(global = true)]
    #[arg(long, value_name("BOOL"))]
    #[arg(help = "Simulate a command and return details about the result")]
    pub dry_run: Option<bool>,
}

#[derive(Args, Debug)]
#[command(group = ArgGroup::new("step-qol-conflict").args(&["step", "skip"]).multiple(true).conflicts_with("screenshot"))]
pub struct RenderSettings {
    #[arg(short, long)]
    #[arg(value_name("PATH"))]
    #[arg(help = "Filepath of output file")]
    #[arg(display_order = 1)]
    pub output: Option<Destination>,
    #[arg(short, long, value_name("PATH"), display_order = 1)]
    #[arg(help = "Filepath of background image")]
    pub bg: Option<PathBuf>,
    #[arg(short, long, value_name("PATH"), display_order = 2)]
    #[arg(help = "Filepath of palette")]
    #[arg(long_help = "Filepath of palette [possible types: .json, .txt, .gpl, .aco, .csv]")]
    pub palette: Option<PathBuf>,
    #[command(subcommand)]
    pub style: Option<MethodKindArg>,
    #[arg(long, value_name("LONG"))]
    #[arg(help = "Time or pixels between frames (0 is max)")]
    #[arg(value_parser = duration_to_num)]
    pub step: Option<NonZeroI64>,
    #[arg(long, value_name("ENUM"), value_enum)]
    #[arg(help = "Whether step represents time or pixels")]
    #[arg(default_value_t = StepTypeArg::Time)]
    pub step_type: StepTypeArg,
    #[arg(long, value_name("INT"))]
    #[arg(help = "Skip specified frames")]
    pub skip: Option<usize>,
    #[arg(long, value_name("BOOL"))]
    #[arg(help = "Render only final frame")]
    #[arg(long_help = "Render only final frame (Alias of \"--step 0 --skip 1\")")]
    pub screenshot: bool,
    // #[clap(long)]
    // #[clap(value_name("FLOAT"))]
    // #[clap(help = "Opacity of render")]
    // #[clap(long_help = "Opacity of render over background")]
    // opacity: Option<f32>,
    #[arg(long, value_name("INT"), num_args(4))]
    #[arg(help = "Color of background")]
    #[arg(long_help = "Color of background (RGBA value)")]
    pub color: Option<Vec<u8>>,
    #[arg(long, value_name("ENUM"))]
    #[arg(help = "Type of raw output used by STDOUT")]
    #[arg(long_help = "Type of raw output used by STDOUT (if provided)")]
    pub output_format: Option<PixelFormat>,
    #[arg(long, value_name("INT"), num_args(4))]
    #[arg(help = "Region to save")]
    #[arg(long_help = "Region to save (x1, y1, x2, y2)")]
    pub region: Option<Vec<u32>>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum StepTypeArg {
    Time,
    Pixels,
}

#[derive(Subcommand, Debug, Clone)]
pub enum MethodKindArg {
    Normal,
    Heat {
        #[arg(default_value_t = nonzero!(900000_i64))] // 15 minutes
        duration: NonZeroI64,
    },
    Virgin,
    Activity,
    Action,
    Milliseconds,
    Seconds,
    Minutes,
    Combined,
    Age,
}

impl From<MethodKindArg> for MethodKind {
    fn from(value: MethodKindArg) -> Self {
        match value {
            MethodKindArg::Normal => MethodKind::Normal,
            MethodKindArg::Heat { duration } => MethodKind::Heatmap(duration),
            MethodKindArg::Virgin => MethodKind::Virgin,
            MethodKindArg::Activity => MethodKind::Activity,
            MethodKindArg::Action => MethodKind::Action,
            MethodKindArg::Milliseconds => MethodKind::Milliseconds,
            MethodKindArg::Seconds => MethodKind::Seconds,
            MethodKindArg::Minutes => MethodKind::Minutes,
            MethodKindArg::Combined => MethodKind::Combined,
            MethodKindArg::Age => MethodKind::Age,
        }
    }
}

// TODO (Etos2): Verify correctness with tests
pub fn duration_to_num(arg: &str) -> Result<NonZeroI64, String> {
    let mut chars = arg.chars();
    let discrim = chars.next_back().ok_or("empty string")?;
    if discrim.is_alphabetic() {
        let num = chars.as_str().parse::<i64>().map_err(|e| e.to_string())?;
        if num != 0 {
            match discrim {
                's' | 'S' => Ok(num.checked_mul(1000).ok_or("time too large (overflow)")?),
                'm' | 'M' => Ok(num.checked_mul(60000).ok_or("time too large (overflow)")?),
                'h' | 'H' => Ok(num
                    .checked_mul(3600000)
                    .ok_or("time too large (overflow)")?),
                'd' | 'D' => Ok(num
                    .checked_mul(86400000)
                    .ok_or("time too large (overflow)")?),
                _ => Err(format!("invalid time unit ({discrim})")),
            }
        } else {
            match discrim {
                's' | 'S' | 'm' | 'M' | 'h' | 'H' | 'd' | 'D' => Ok(i64::MAX),
                _ => Err(format!("invalid time unit ({discrim})")),
            }
        }
    } else {
        arg.parse::<i64>().map_err(|e| e.to_string())
    }
    .map(|milli| NonZeroI64::new(milli).ok_or("time was zero!".to_owned()))?
}

impl From<ProgramSettings> for ProgramConfigBuilder {
    fn from(value: ProgramSettings) -> Self {
        ProgramConfigBuilder {
            log_source: value.log,
            quiet: value.quiet,
            threads: value.threads,
            dry_run: value.dry_run,
        }
    }
}

impl From<RenderSettings> for RenderConfigBuilder {
    fn from(value: RenderSettings) -> Self {
        RenderConfigBuilder {
            method_palette_source: value.palette.map(PaletteSource::File),
            method_kind: value.style.map(|arg| arg.into()),
            canvas_source: value.bg,
            canvas_size: value.region.map(|r| (r[0], r[1], r[2], r[3])),
            canvas_background: value.color.map(|v| *Rgba::from_slice(&v[0..=4])),
            canvas_transparency: None,
            destination_format: value.output_format,
            destination_kind: value.output.map(|dst| match dst {
                Destination::Stdout => DestinationKind::Stdout,
                Destination::File(path) => DestinationKind::File(path),
            }),
            step: value.step.map(|t| match value.step_type {
                StepTypeArg::Time => Step::Time(t),
                StepTypeArg::Pixels => Step::Pixels(t),
            }),
        }
    }
}

impl ConfigSource for CliData {
    fn get_config(source: Self) -> Result<ConfigBuilder, ConfigError> {
        if source.config.is_some() {
            Ok(ConfigBuilder {
                program: source.program.into(),
                render_base: source.render.into(),
                render: vec![],
            })
        } else {
            Ok(ConfigBuilder {
                program: source.program.into(),
                render_base: RenderConfigBuilder::new(),
                render: vec![source.render.into()],
            })
        }
    }
}
