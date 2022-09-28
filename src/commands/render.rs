use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;

use crate::action::{ActionKind, ActionRef};
use crate::commands::{Command, CommandInput};
use crate::error::{ConfigError, ConfigResult, RuntimeError, RuntimeErrorKind, RuntimeResult};
use crate::palette::PaletteParser;
use crate::util::Region;
use crate::Cli;

use clap::{ArgEnum, ArgGroup, Args};
use image::io::Reader as ImageReader;
use image::{Pixel, Rgba, RgbaImage};
use rayon::iter::ParallelIterator;
use rayon::str::ParallelString;

#[derive(Args)]
#[clap(
    about = "Render individual frames or output raw frame data to STDOUT.",
    long_about = "Render individual frames or output raw frame data to STDOUT.
Guaranted to produce 2 frames per render, where the first frame is the background and the last frame is the complete contents of the log.
To output only the final result, use the \"--screenshot\" arg or manually skip the first frame \"--skip\"."
)]
#[clap(group = ArgGroup::new("step-qol").args(&["step", "skip", "screenshot"]).required(true).multiple(true))]
#[clap(group = ArgGroup::new("step-qol-conflict").args(&["step", "skip"]).multiple(true).conflicts_with("screenshot"))]
#[clap(group = ArgGroup::new("bg-qol").args(&["color", "size", "bg"]).required(true).multiple(true))]
#[clap(group = ArgGroup::new("bg-qol-conflict").args(&["color", "size"]).multiple(true).conflicts_with("bg"))]
pub struct RenderInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file")]
    #[clap(display_order = 0)]
    src: String,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of output frames")]
    #[clap(long_help = "Filepath of output frames [defaults to STDOUT]")]
    #[clap(display_order = 0)]
    dst: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of background image")]
    #[clap(display_order = 0)]
    bg: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of palette")]
    #[clap(long_help = "Filepath of palette [possible types: .json, .txt, .gpl, .aco, .csv]")]
    #[clap(display_order = 0)]
    palette: Option<String>,
    #[clap(long, arg_enum)]
    #[clap(value_name("ENUM"))]
    #[clap(help = "Type of render")]
    style: Option<RenderType>,
    #[clap(long)]
    #[clap(value_name("LONG"))]
    #[clap(help = "Time or pixels between frames (0 is max)")]
    step: Option<i64>,
    #[clap(long, arg_enum)]
    #[clap(value_name("ENUM"))]
    #[clap(help = "Whether step represents time or pixels")]
    step_type: Option<StepType>,
    #[clap(long)]
    #[clap(value_name("INT"))]
    #[clap(help = "Skip specified frames")]
    skip: Option<usize>,
    #[clap(long)]
    #[clap(max_values(2))]
    #[clap(min_values(2))]
    #[clap(value_name("INT"))]
    #[clap(help = "Size of render")]
    size: Option<Vec<u32>>,
    #[clap(long)]
    #[clap(help = "Render only final frame")]
    #[clap(long_help = "Render only final frame (Alias of \"--step 0 --skip 1\")")]
    screenshot: bool,
    // #[clap(long)]
    // #[clap(value_name("FLOAT"))]
    // #[clap(help = "Opacity of render")]
    // #[clap(long_help = "Opacity of render over background")]
    // opacity: Option<f32>,
    #[clap(long)]
    #[clap(max_values(4))]
    #[clap(min_values(4))]
    #[clap(value_name("INT"))]
    #[clap(help = "Color of background")]
    #[clap(long_help = "Color of background (RGBA value)")]
    color: Option<Vec<u8>>,
    #[clap(long)]
    #[clap(max_values(4))]
    #[clap(value_name("INT"))]
    #[clap(help = "Region to save")]
    #[clap(long_help = "Region to save (x1, y1, x2, y2)")]
    crop: Vec<u32>,
}

// TODO: Clean
pub const DEFAULT_PALETTE: [[u8; 4]; 32] = [
    [0, 0, 0, 255],       // Black
    [34, 34, 34, 255],    // Dark Grey
    [85, 85, 85, 255],    // Deep Grey
    [136, 136, 136, 255], // Medium Grey
    [205, 205, 205, 255], // Light Grey
    [255, 255, 255, 255], // White
    [255, 213, 188, 255], // Beige
    [255, 183, 131, 255], // Peach
    [182, 109, 61, 255],  // Brown
    [119, 67, 31, 255],   // Chocolate
    [252, 117, 16, 255],  // Rust
    [252, 168, 14, 255],  // Orange
    [253, 232, 23, 255],  // Yellow
    [255, 244, 145, 255], // Pastel Yellow
    [190, 255, 64, 255],  // Lime
    [112, 221, 19, 255],  // Green
    [49, 161, 23, 255],   // Dark Green
    [11, 95, 53, 255],    // Forest
    [39, 126, 108, 255],  // Dark Teal
    [50, 182, 159, 255],  // Light Teal
    [136, 255, 243, 255], // Aqua
    [36, 181, 254, 255],  // Azure
    [18, 92, 199, 255],   // Blue
    [38, 41, 96, 255],    // Navy
    [139, 47, 168, 255],  // Purple
    [210, 76, 233, 255],  // Mauve
    [255, 89, 239, 255],  // Magenta
    [255, 169, 217, 255], // Pink
    [255, 100, 116, 255], // Watermelon
    [240, 37, 35, 255],   // Red
    [177, 18, 6, 255],    // Rose
    [116, 12, 0, 255],    // Maroon
];

pub struct RenderData {
    src: String,
    dst: Option<String>,
    crop: Region<u32>,
    background: RgbaImage,
    style: RenderType,
    step: i64,
    step_type: StepType,
    skip: usize,
    palette: Vec<[u8; 4]>,
}

impl CommandInput<RenderData> for RenderInput {
    fn validate(&self) -> ConfigResult<RenderData> {
        let palette = match &self.palette {
            Some(path) => PaletteParser::try_parse(&path)
                .map_err(|e| ConfigError::new("palette", &e.to_string()))?,
            None => DEFAULT_PALETTE.to_vec(),
        };

        let mut step = self.step.unwrap_or(i64::MAX);
        if step == 0 {
            step = i64::MAX;
        }

        let step_type = self.step_type.unwrap_or_default();

        let mut skip = self.skip.unwrap_or(0);
        if self.screenshot {
            skip = 1;
        }

        let color = match &self.color {
            Some(color) => Rgba::from_slice(color).clone(),
            None => match self.dst {
                Some(_) => Rgba::from([0, 0, 0, 255]),
                None => Rgba::from([0, 0, 0, 0]),
            },
        };

        let crop = Region::from_slice(&self.crop).unwrap_or(Region::all());
        let background = match &self.bg {
            Some(path) => get_background(path, &crop, self.dst.is_none())
                .map_err(|e| RuntimeError::from_err(e, path, 0))
                .map_err(|e| ConfigError::new("bg", &e.to_string()))?, // TODO: Mapping but better?
            None => match &self.size {
                Some(size) => RgbaImage::from_pixel(size[0], size[1], color),
                None => Err(ConfigError::new("bg", "cannot infer size"))?,
            },
        };

        Ok(RenderData {
            src: self.src.to_owned(),
            dst: self.dst.to_owned(),
            crop,
            background,
            style: self.style.unwrap_or(RenderType::Normal),
            step,
            step_type,
            skip,
            palette,
        })
    }
}

fn get_background(path: &str, crop: &Region<u32>, transparent: bool) -> RuntimeResult<RgbaImage> {
    let x = crop.start().0;
    let y = crop.start().1;
    let width = crop.width();
    let height = crop.height();
    let mut out = ImageReader::open(path)?
        .decode()?
        .crop(x, y, width, height)
        .to_rgba8();
    // Remove transparency
    if !transparent {
        for pixel in out.pixels_mut().filter(|p| p.0[3] == 0) {
            *pixel = Rgba::from([0, 0, 0, 255]);
        }
    }

    Ok(out)
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum RenderType {
    Normal,
    Heat,
    Virgin,
    Activity,
    Action,
    Milliseconds,
    Seconds,
    Minutes,
    Combined,
    Age,
}

impl Default for RenderType {
    fn default() -> Self {
        RenderType::Normal
    }
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum StepType {
    Time,
    Pixels,
}

impl Default for StepType {
    fn default() -> Self {
        StepType::Time
    }
}

trait Renderable {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage);
}

impl Command for RenderData {
    fn run(&self, settings: &Cli) -> RuntimeResult<()> {
        let stdout = io::stdout();

        // TODO: Clobber
        assert!(!settings.noclobber);

        let data = std::fs::read_to_string(&self.src)
            .map_err(|e| RuntimeError::from_err(e, &self.src, 0))?;
        let pixels: Vec<ActionRef> = data
            .as_parallel_string()
            .par_lines()
            .filter_map(|s| match ActionRef::try_from(s) {
                Ok(a) => {
                    if self.crop.contains(a.x, a.y) {
                        Some(a)
                    } else {
                        None
                    }
                }
                Err(_) => None, // TODO
            })
            .collect();

        if pixels.len() == 0 {
            Err(RuntimeError::new_with_file(
                RuntimeErrorKind::UnexpectedEof,
                &self.src,
                0,
            ))?;
        }

        let width = self.background.width();
        let height = self.background.height();
        let mut renderer: Box<dyn Renderable> = match self.style {
            RenderType::Normal => Box::new(NormalRender::new(&self.background, &self.palette)),
            RenderType::Activity => Box::new(ActivityRender::new(width, height)),
            RenderType::Heat => Box::new(HeatRender::new(width, height, self.step)),
            RenderType::Virgin => Box::new(VirginRender {}),
            RenderType::Action => Box::new(ActionRender {}),
            RenderType::Combined => Box::new(CombinedRender {}),
            RenderType::Milliseconds => {
                let bg_color = Rgba::from([255, 0, 0, 255]);
                Box::new(PlacementRender::new(bg_color, 1000))
            }
            RenderType::Seconds => {
                let bg_color = Rgba::from([0, 255, 0, 255]);
                Box::new(PlacementRender::new(bg_color, 60000))
            }
            RenderType::Minutes => {
                let bg_color = Rgba::from([0, 0, 255, 255]);
                Box::new(PlacementRender::new(bg_color, 3600000))
            }
            RenderType::Age => {
                // Safe unwrap (pixels.len > 0)
                let min = pixels.first().unwrap().time.timestamp_millis();
                let max = pixels.last().unwrap().time.timestamp_millis();
                Box::new(AgeRender::new(min, max))
            }
        };

        let frames = Self::get_frame_slices(&pixels, self.step, self.step_type);
        let mut current = self.background.clone();

        if settings.verbose {
            eprintln!("Rendering {} frames", frames.len());
        }

        // Render frames
        for (i, frame) in frames[self.skip..].iter().enumerate() {
            if let Some(frame) = frame {
                current = current.clone();
                renderer.render(frame, &mut current);
            }

            match &self.dst {
                Some(path) => Self::frame_to_file(&current, &path, i)
                    .map_err(|e| RuntimeError::from_err(e, &path, 0))?,
                None => Self::frame_to_raw(&current, &mut stdout.lock())
                    .map_err(|e| RuntimeError::from_err(e, "STDOUT", 0))?,
            }
        }

        Ok(())
    }
}

impl RenderData {
    // TODO: Error handling
    fn frame_to_file(frame: &RgbaImage, path: &str, i: usize) -> RuntimeResult<()> {
        let ext = Path::new(path)
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(RuntimeError::new(RuntimeErrorKind::Unsupported))?;

        let mut dst = path.to_owned();
        dst.truncate(dst.len() - ext.len() - 1);

        frame.save(format!("{}_{}.{}", dst, i, ext))?;

        Ok(())
    }

    fn frame_to_raw<R: Write>(frame: &RgbaImage, out: &mut R) -> RuntimeResult<()> {
        let buf = &frame.as_raw()[..];
        out.write_all(buf)?;
        out.flush()?;
        Ok(())
    }

    fn get_frame_slices<'a>(
        pixels: &'a [ActionRef],
        step: i64,
        step_type: StepType,
    ) -> Vec<Option<&'a [ActionRef<'a>]>> {
        let mut frames: Vec<Option<&[ActionRef]>> = vec![];
        let mut start = 0;

        frames.push(None);
        if step != 0 {
            match step_type {
                StepType::Time => {
                    for (end, pair) in pixels.windows(2).enumerate() {
                        let start_time = pair[0].time.timestamp_millis() / step;
                        let end_time = pair[1].time.timestamp_millis() / step;
                        // TODO: Diff could be negative
                        let diff = end_time - start_time;
                        if diff > 0 {
                            frames.push(Some(&pixels[start..=end]));
                            start = end;
                            for _ in 1..diff {
                                frames.push(None);
                            }
                        }
                    }
                },
                StepType::Pixels => {
                    let step = usize::try_from(step).unwrap();
                    for (end, _pair) in pixels.windows(2).enumerate() {
                        if end - start >= step {
                            frames.push(Some(&pixels[start..=end]));
                            start = end;
                        }
                    }
                }
            }

            frames.push(Some(&pixels[start..]));
        } else {
            frames.push(Some(&pixels));
        }

        frames
    }
}

struct NormalRender<'a> {
    background: &'a RgbaImage,
    palette: &'a [[u8; 4]],
}

impl<'a> NormalRender<'a> {
    fn new(background: &'a RgbaImage, palette: &'a [[u8; 4]]) -> Self {
        Self {
            background,
            palette,
        }
    }
}

impl<'a> Renderable for NormalRender<'a> {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            if let Some(pixel) = self.palette.get(action.index) {
                frame.put_pixel(action.x, action.y, Rgba::from(*pixel));
            } else {
                frame.put_pixel(
                    action.x,
                    action.y,
                    *self.background.get_pixel(action.x, action.y),
                );
            }
        }
    }
}

// TODO: Remove map
struct ActivityRender {
    heat_map: Vec<i32>,
    max: i32,
    width: u32,
    height: u32,
}

impl ActivityRender {
    fn new(width: u32, height: u32) -> Self {
        ActivityRender {
            heat_map: vec![0; width as usize * height as usize],
            max: i32::MIN,
            width,
            height,
        }
    }
}

impl Renderable for ActivityRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let index = action.x + action.y * self.width;
            self.heat_map[index as usize] += 1;

            if self.heat_map[index as usize] > self.max {
                self.max = self.heat_map[index as usize];
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let index = x + y * self.width;
                let val = self.heat_map[index as usize] as f32 / self.max as f32;

                let r = f32::min(f32::max(0.0, 1.5 - f32::abs(1.5 - 4.0 * (val - 0.5))), 1.0);
                let g = f32::min(f32::max(0.0, 1.5 - f32::abs(1.5 - 4.0 * (val - 0.25))), 1.0);
                let b = f32::min(f32::max(0.0, 1.5 - f32::abs(1.5 - 4.0 * (val - 0.0))), 1.0);

                let r = (r * 255.0) as u8;
                let g = (g * 255.0) as u8;
                let b = (b * 255.0) as u8;

                frame.put_pixel(x, y, Rgba::from([r, g, b, 255]));
            }
        }
    }
}

struct VirginRender {}

impl Renderable for VirginRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            frame.put_pixel(action.x, action.y, Rgba::from([0, 0, 0, 255]));
        }
    }
}

struct HeatRender {
    activity_map: Vec<i64>,
    width: u32,
    height: u32,
    step: i64,
    i: i64,
}

impl HeatRender {
    fn new(width: u32, height: u32, step: i64) -> Self {
        HeatRender {
            activity_map: vec![0; width as usize * height as usize],
            width,
            height,
            step,
            i: 1,
        }
    }
}

impl Renderable for HeatRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let index = action.x + action.y * self.width;
            self.activity_map[index as usize] = action.time.timestamp_millis();

            if action.time.timestamp_millis() > self.step * self.i {
                self.i = action.time.timestamp_millis() as i64 / self.step + 1;
            }
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let index = x + y * self.width;
                let delta = self.activity_map[index as usize];

                // If less than 15 minutes
                // TODO: Customisable
                let diff = (self.step * self.i - delta) as f32 / 900000.0;
                if diff < 1.0 {
                    let val = 1.0 - diff;
                    let r = (val * 205.0) as u8;
                    let g = (val * 92.0) as u8;
                    let b = (val * 92.0) as u8;
                    frame.put_pixel(x, y, Rgba::from([r, g, b, 255]));
                } else {
                    frame.put_pixel(x, y, Rgba::from([0, 0, 0, 255]));
                }
            }
        }
    }
}

struct ActionRender {}

impl Renderable for ActionRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let val = match action.kind {
                ActionKind::Undo => Rgba::from([255, 0, 255, 255]),
                ActionKind::Place => Rgba::from([0, 0, 255, 255]),
                ActionKind::Overwrite => Rgba::from([0, 255, 255, 255]),
                ActionKind::Rollback => Rgba::from([0, 255, 0, 255]),
                ActionKind::RollbackUndo => Rgba::from([255, 255, 0, 255]),
                ActionKind::Nuke => Rgba::from([255, 0, 0, 255]),
            };
            frame.put_pixel(action.x, action.y, val);
        }
    }
}

#[derive(Clone)]
struct PlacementRender {
    step: i64,
    color: Rgba<u8>,
}

impl PlacementRender {
    fn new(color: Rgba<u8>, step: i64) -> Self {
        Self { step, color }
    }
}

impl Renderable for PlacementRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let val = ((action.time.timestamp_millis() - 1) % self.step) as f32 / self.step as f32;
            let color = color_lerp(self.color.channels(), val);
            frame.put_pixel(action.x, action.y, color);
        }
    }
}

struct CombinedRender {}

impl Renderable for CombinedRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let r = (((action.time.timestamp_millis() - 1) % 1000) as f32 / 1000.0 * 255.0) as u8;
            let g = (((action.time.timestamp_millis() - 1) % 60000) as f32 / 60000.0 * 255.0) as u8;
            let b =
                (((action.time.timestamp_millis() - 1) % 3600000) as f32 / 3600000.0 * 255.0) as u8;

            frame.put_pixel(action.x, action.y, Rgba::from([r, g, b, 255]));
        }
    }
}

struct AgeRender {
    min: f32,
    max: f32,
}

impl AgeRender {
    fn new(min: i64, max: i64) -> Self {
        Self {
            min: min as f32,
            max: max as f32,
        }
    }
}

impl Renderable for AgeRender {
    fn render(&mut self, actions: &[ActionRef], frame: &mut RgbaImage) {
        for action in actions {
            let mut val =
                (action.time.timestamp_millis() as f32 - self.min) / (self.max - self.min);
            if self.max == self.min {
                val = 1.0;
            }

            let color = color_lerp(&[0, 0, 255, 255], val);
            frame.put_pixel(action.x, action.y, color);
        }
    }
}

fn color_lerp(color: &[u8], val: f32) -> Rgba<u8> {
    if val < 0.5 {
        let val = val * 2.0;
        let r = (color[0] as f32 * val) as u8;
        let g = (color[1] as f32 * val) as u8;
        let b = (color[2] as f32 * val) as u8;
        Rgba::from([r, g, b, 255])
    } else {
        let val = (val - 0.5) * 2.0;
        let r = (color[0] as f32 + (255 - color[0]) as f32 * val) as u8;
        let g = (color[1] as f32 + (255 - color[1]) as f32 * val) as u8;
        let b = (color[2] as f32 + (255 - color[2]) as f32 * val) as u8;
        Rgba::from([r, g, b, 255])
    }
}
