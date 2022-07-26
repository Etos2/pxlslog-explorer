use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::error::{PxlsError, PxlsErrorKind, PxlsResult};
use crate::palette::PaletteParser;
use crate::pixel::{Action, ActionKind}; // TODO: PxlsPixel -> Pixel
use crate::util::Region;
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgEnum, ArgGroup, Args};
use image::io::Reader as ImageReader;
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use rayon::prelude::*;

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
    #[clap(help = "Time between frames (0 is max)")]
    step: Option<i64>,
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
const PALETTE: [[u8; 4]; 32] = [
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
    dst: Option<String>,
    pixels: Vec<Action>,
    background: RgbaImage,
    style: RenderType,
    step: i64,
    skip: usize,
    palette: Vec<[u8; 4]>,
}

impl RenderInput {
    pub fn validate(&self) -> PxlsResult<RenderData> {
        let palette = match &self.palette {
            Some(path) => PaletteParser::try_parse(&path)?,
            None => PALETTE.to_vec(),
        };

        let mut step = self.step.unwrap_or(i64::MAX);
        if step == 0 {
            step = i64::MAX;
        }

        let mut skip = self.skip.unwrap_or(0);
        if self.screenshot {
            skip = 1;
        }

        let crop = Region::from_slice(&self.crop).unwrap_or(Region::all());
        let pixels = Self::get_pixels(&self.src, &crop)?;
        let background = match &self.bg {
            Some(path) => Self::get_background(path, &crop)?,
            None => match &self.size {
                Some(size) => Self::generate_background(size[0], size[1], self.color.as_deref()),
                None => {
                    return Err(PxlsError::new(PxlsErrorKind::InvalidState(
                        "cannot infer size".to_string(),
                    )))
                }
            },
        };

        if pixels.len() == 0 {
            return Err(PxlsError::new(PxlsErrorKind::InvalidState(
                "No pixels found in region!".to_string(),
            )));
        }

        Ok(RenderData {
            dst: self.dst.to_owned(),
            pixels,
            background,
            style: self.style.unwrap_or(RenderType::Normal),
            step,
            skip,
            palette,
        })
    }

    fn get_pixels<P>(path: P, region: &Region<u32>) -> PxlsResult<Vec<Action>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        Ok(buffer
            .par_lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                let tokens: Vec<_> = l.split_terminator(|c| c == '\t').collect();

                if tokens.len() == 6 {
                    let x: u32 = tokens[2].parse().ok()?;
                    let y: u32 = tokens[3].parse().ok()?;

                    if region.contains(x, y) {
                        let offset = region.start();
                        let timestamp =
                            NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f")
                                .ok()?
                                .timestamp_millis();
                        let index = tokens[4].parse().ok()?;
                        let kind = tokens[5].parse().ok()?;

                        Some(Action {
                            x: x - offset.0,
                            y: y - offset.1,
                            index,
                            timestamp,
                            kind,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect())
    }

    fn get_background(path: &str, crop: &Region<u32>) -> PxlsResult<RgbaImage> {
        let x = crop.start().0;
        let y = crop.start().1;
        let width = crop.width();
        let height = crop.height();
        Ok(ImageReader::open(path)?
            .decode()?
            .crop(x, y, width, height)
            .to_rgba8())
    }

    fn generate_background(x: u32, y: u32, color: Option<&[u8]>) -> RgbaImage {
        let pixel = match &color {
            Some(color) => image::Rgba::from_slice(&color).to_owned(),
            None => image::Rgba::from([0, 0, 0, 0]),
        };

        ImageBuffer::from_pixel(x, y, pixel)
    }
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

trait Renderable {
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage);
}

impl RenderData {
    pub fn run(&self, settings: &Cli) -> PxlsResult<()> {
        let stdout = io::stdout();

        // TODO: Clobber
        if settings.noclobber {
            return Err(PxlsError::new(PxlsErrorKind::InvalidState(
                "No clobber is NOT implemented for RENDER! Yet...".to_string(),
            )));
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
                let val = Rgba::from([255, 0, 0, 255]);
                Box::new(PlacementRender::new(val, 1000))
            }
            RenderType::Seconds => {
                let val = Rgba::from([0, 255, 0, 255]);
                Box::new(PlacementRender::new(val, 60000))
            }
            RenderType::Minutes => {
                let val = Rgba::from([0, 0, 255, 255]);
                Box::new(PlacementRender::new(val, 3600000))
            }
            RenderType::Age => {
                // Safe unwrap (pixels.len > 0)
                let min = self.pixels.first().unwrap().timestamp;
                let max = self.pixels.last().unwrap().timestamp;
                Box::new(AgeRender::new(min, max))
            }
        };

        let frames = Self::get_frame_slices(&self.pixels, self.step);
        let mut current = self.background.clone();

        if settings.verbose {
            eprintln!("Rendering {} frames", frames.len());
        }

        for (i, frame) in frames[self.skip..].iter().enumerate() {
            if let Some(frame) = frame {
                current = current.clone();
                renderer.render(frame, &mut current);
            }

            match &self.dst {
                Some(path) => Self::frame_to_file(&current, &path, i)?,
                None => Self::frame_to_raw(&current, &mut stdout.lock())?,
            }
        }

        Ok(())
    }

    // TODO: Error handling
    fn frame_to_file(frame: &RgbaImage, path: &str, i: usize) -> PxlsResult<()> {
        let ext = Path::new(path)
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(PxlsError::new_with_file(PxlsErrorKind::Unsupported(), path))?;

        let mut dst = path.to_owned();
        dst.truncate(dst.len() - ext.len() - 1);

        frame
            .save(format!("{}_{}.{}", dst, i, ext))
            .map_err(|e| PxlsError::from(e, &path, 0))?;

        Ok(())
    }

    fn frame_to_raw<R: Write>(frame: &RgbaImage, out: &mut R) -> PxlsResult<()> {
        let buf = &frame.as_raw()[..];
        out.write_all(buf)?;
        out.flush()?;
        Ok(())
    }

    fn get_frame_slices(pixels: &[Action], step: i64) -> Vec<Option<&[Action]>> {
        let mut frames: Vec<Option<&[Action]>> = vec![];
        let mut start = 0;

        frames.push(None);
        if step != 0 {
            for (end, pair) in pixels.windows(2).enumerate() {
                // TODO: Diff could be negative
                let diff = pair[1].timestamp / step - pair[0].timestamp / step;
                if diff > 0 {
                    frames.push(Some(&pixels[start..=end]));
                    start = end;
                    for _ in 1..diff {
                        frames.push(None);
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
        for action in actions {
            let index = action.x + action.y * self.width;
            self.activity_map[index as usize] = action.timestamp;

            if action.timestamp > self.step * self.i {
                self.i = action.timestamp as i64 / self.step + 1;
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
        for action in actions {
            let val = ((action.timestamp - 1) % self.step) as f32 / self.step as f32;
            let color = color_lerp(self.color.channels(), val);
            frame.put_pixel(action.x, action.y, color);
        }
    }
}

struct CombinedRender {}

impl Renderable for CombinedRender {
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
        for action in actions {
            let r = (((action.timestamp - 1) % 1000) as f32 / 1000.0 * 255.0) as u8;
            let g = (((action.timestamp - 1) % 60000) as f32 / 60000.0 * 255.0) as u8;
            let b = (((action.timestamp - 1) % 3600000) as f32 / 3600000.0 * 255.0) as u8;

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
    fn render(&mut self, actions: &[Action], frame: &mut RgbaImage) {
        for action in actions {
            let mut val = (action.timestamp as f32 - self.min) / (self.max - self.min);
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
