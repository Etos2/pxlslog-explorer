pub mod builder;
pub mod error;
pub mod source;

use std::{num::NonZeroI64, path::PathBuf};

use clap::ValueEnum;

use crate::{
    render::{pixel::Rgba, Step},
    util::io::{Destination, Source},
};

use self::builder::ConfigBuilder;

#[derive(Debug, Clone)]
pub enum DestinationCommand {
    _Ffmpeg,
    // Gstream,
    // Gmagic,
    _Other(String, Option<Vec<String>>),
}

#[derive(Debug, Clone)]
pub enum DestinationKind {
    File(PathBuf),
    _Dir(PathBuf),
    // NamedPipe(),
    Stdout,
    _Process(Destination, DestinationCommand),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PixelFormat {
    Rgba,
    Rgb,
    Yuv420p,
}

#[derive(Debug, Clone)]
pub enum PaletteSource {
    File(PathBuf),
    _Array(Vec<Rgba>),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum MethodKind {
    #[default]
    Normal,
    Heatmap(NonZeroI64),
    Virgin,
    Activity,
    Action,
    Milliseconds,
    Seconds,
    Minutes,
    Combined,
    Age,
}

#[derive(Debug, Clone)]
pub struct MethodConfig {
    pub palette: Option<PaletteSource>,
    pub kind: MethodKind,
}

#[derive(Debug, Clone)]
pub struct DestinationConfig {
    pub format: PixelFormat,
    pub kind: DestinationKind,
}

#[derive(Debug, Clone)]
pub struct CanvasConfig {
    pub source: Option<PathBuf>,
    pub size: Option<(u32, u32, u32, u32)>,
    pub background: Option<Rgba>,
    pub transparency: bool,
}

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub destination: DestinationConfig,
    pub method: MethodConfig,
    pub canvas: CanvasConfig,
    pub step: Step,
}

#[derive(Debug, Clone)]
pub struct ProgramConfig {
    pub log_source: Source,
    pub quiet: bool,
    pub threads: usize,
    pub dry_run: bool,
}
