use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use crate::command::{PxlsCommand, PxlsError, PxlsInput, PxlsResult};
use crate::parser::{PaletteParser, PxlsParser};
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgEnum, ArgGroup, Args};
use image::io::Reader as ImageReader;
use image::{ImageBuffer, Rgba, RgbaImage, Pixel};

#[derive(Args)]
#[clap(
    about = "Render individual frames or output raw frame data to STDOUT.",
    long_about = "Render individual frames or output raw frame data to STDOUT.
Guaranted to produce 2 frames per render, where the first frame is the background and the last frame is the complete contents of the log.
To output only the final result, use the \"--screenshot\" arg or manually skip the first frame \"--skip\"."
)]
#[clap(group = ArgGroup::new("qol").args(&["step", "skip", "screenshot"]).required(true).multiple(true))]
#[clap(group = ArgGroup::new("qol-conflict").args(&["step", "skip"]).conflicts_with("screenshot"))]
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
    r#type: Option<RenderType>,
    #[clap(long)]
    #[clap(value_name("LONG"))]
    #[clap(help = "Time between frames (0 is max)")]
    step: Option<i64>,
    #[clap(long)]
    #[clap(value_name("INT"))]
    #[clap(help = "Skip specified frames")]
    skip: Option<usize>,
    #[clap(long)]
    #[clap(help = "Render only final frame")]
    #[clap(long_help = "Render only final frame (Alias of \"--step 0 --skip 1\")")]
    screenshot: bool,
    // #[clap(long)]
    // #[clap(value_name("FLOAT"))]
    // #[clap(help = "Opacity of render")]
    // #[clap(long_help = "Opacity of render over background")]
    // opacity: Option<f32>,
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

#[derive(Debug, Copy, Clone)]
pub struct PixelAction {
    x: u32,
    y: u32,
    i: usize,
    delta: i64,
}

pub struct Render {
    src: String,
    dst: Option<String>,
    background: RgbaImage,
    style: RenderType,
    step: i64,
    skip: usize,
    palette: Vec<[u8; 4]>,
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum RenderType {
    Normal,
    Heat,
    Virgin,
    Activity,
}

impl PxlsInput for RenderInput {
    fn parse(&self, _settings: &Cli) -> PxlsResult<Box<dyn PxlsCommand>> {
        let style = match self.r#type {
            Some(t) => t,
            None => RenderType::Normal,
        };

        let pixels = Render::get_pixels(&self.src)?;
        let background = match &self.bg {
            Some(path) => ImageReader::open(path)?.decode()?.to_rgba8(),
            None => {
                let mut size = (0, 0);
                for pixel in &pixels {
                    if pixel.x + 1 > size.0 {
                        size.0 = pixel.x + 1;
                    }
                    if pixel.y + 1 > size.1 {
                        size.1 = pixel.y + 1;
                    }
                }
                ImageBuffer::from_pixel(size.0, size.1, image::Rgba::from([0, 0, 0, 0]))
            }
        };

        let step = match self.step {
            Some(step) => {
                if step > 0 {
                    step
                } else {
                    i64::MAX
                }
            }
            None => i64::MAX,
        };

        let mut skip = self.skip.unwrap_or(0);
        if self.screenshot {
            skip = 1;
        }

        let palette = match &self.palette {
            Some(path) => PaletteParser::try_parse(&path)?,
            None => PALETTE.to_vec(),
        };

        Ok(Box::new(Render {
            src: self.src.to_owned(),
            dst: self.dst.clone(),
            background,
            style,
            step,
            skip,
            palette,
        }))
    }
}

trait Renderable {
    fn render(&mut self, actions: &[PixelAction], frame: &mut RgbaImage);
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
    fn render(&mut self, actions: &[PixelAction], frame: &mut RgbaImage) {
        for action in actions {
            if let Some(pixel) = self.palette.get(action.i) {
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

struct HeatMapRender {
    heatmap: Vec<i32>,
    max: i32,
    width: u32,
    height: u32,
}

impl HeatMapRender {
    fn new(width: u32, height: u32) -> Self {
        Self {
            heatmap: vec![0; width as usize * height as usize],
            max: i32::MIN,
            width,
            height,
        }
    }

    const fn index(&self, x: usize, y: usize) -> usize {
        x + y * self.width as usize
    }
}

impl Renderable for HeatMapRender {
    fn render(&mut self, actions: &[PixelAction], frame: &mut RgbaImage) {
        for action in actions {
            let index = self.index(action.x as usize, action.y as usize);
            self.heatmap[index] += 1;

            if self.heatmap[index] > self.max {
                self.max = self.heatmap[index];
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x as usize, y as usize);
                let val = self.heatmap[index] as f32 / self.max as f32;

                let r = f32::min(f32::max(0.0, 1.5 - f32::abs(1.0 - 4.0 * (val - 0.5))), 1.0);
                let g = f32::min(f32::max(0.0, 1.5 - f32::abs(1.0 - 4.0 * (val - 0.25))), 1.0);
                let b = f32::min(f32::max(0.0, 1.5 - f32::abs(1.0 - 4.0 * (val - 0.0))), 1.0);

                let r = (r * 255.0) as u8;
                let g = (g * 255.0) as u8;
                let b = (b * 255.0) as u8;

                frame.put_pixel(x, y, Rgba::from([r, g, b, 255]));
            }
        }
    }
}

struct VirginRender {
    virginmap: Vec<bool>,
    width: u32,
    height: u32,
}

impl VirginRender {
    fn new(width: u32, height: u32) -> Self {
        Self {
            virginmap: vec![true; width as usize * height as usize],
            width,
            height,
        }
    }

    const fn index(&self, x: usize, y: usize) -> usize {
        x + y * self.width as usize
    }
}

impl Renderable for VirginRender {
    fn render(&mut self, actions: &[PixelAction], frame: &mut RgbaImage) {
        for action in actions {
            let index = self.index(action.x as usize, action.y as usize);
            self.virginmap[index] = false;
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x as usize, y as usize);
                if self.virginmap[index] {
                    frame.put_pixel(x, y, Rgba::from([0, 255, 0, 255]));
                } else {
                    frame.put_pixel(x, y, Rgba::from([0, 0, 0, 255]));
                }
            }
        }
    }
}

impl PxlsCommand for Render {
    fn run(&self, settings: &Cli) -> PxlsResult<()> {
        let stdin = io::stdout();
        let pixels = Self::get_pixels(&self.src)?;
        let frames = Self::get_frame_slices(&pixels, self.step);
        let mut current = self.background.clone();

        if settings.verbose {
            println!("Rendering {} frames", frames.len());
        }

        let mut renderer: Box<dyn Renderable> = match self.style {
            RenderType::Normal => Box::new(NormalRender::new(&self.background, &self.palette)),
            RenderType::Heat => Box::new(HeatMapRender::new(current.width(), current.height())),
            RenderType::Virgin => Box::new(VirginRender::new(current.width(), current.height())),
            RenderType::Activity => unimplemented!(),
        };

        for (i, frame) in frames[self.skip..].iter().enumerate() {
            current = current.clone();
            renderer.render(frame, &mut current);

            match &self.dst {
                Some(path) => Self::frame_to_file(&current, &path, i)?,
                None => Self::frame_to_raw(&current, &mut stdin.lock())?,
            }
        }

        Ok(())
    }
}

impl Render {
    // TODO: Error handling
    fn frame_to_file(frame: &RgbaImage, path: &str, i: usize) -> PxlsResult<()> {
        let ext = Path::new(path)
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(PxlsError::Unsupported())?;
        let mut dst = path.to_owned();
        dst.truncate(dst.len() - ext.len() - 1);
        frame.save(format!("{}_{}.{}", dst, i, ext))?;
        Ok(())
    }

    fn frame_to_raw<R: Write>(frame: &RgbaImage, out: &mut R) -> PxlsResult<()> {
        let buf = &frame.as_raw()[..];
        out.write_all(buf)?;
        out.flush()?;
        Ok(())
    }

    // TODO: External io
    fn get_pixels(path: &str) -> PxlsResult<Vec<PixelAction>> {
        PxlsParser::parse(
            &mut OpenOptions::new().read(true).open(path)?,
            |s: &[&str]| -> PxlsResult<PixelAction> {
                Ok(PixelAction {
                    x: s[2].parse()?,
                    y: s[3].parse()?,
                    i: s[4].parse()?,
                    delta: NaiveDateTime::parse_from_str(s[0], "%Y-%m-%d %H:%M:%S,%3f")?
                        .timestamp_millis(),
                })
            },
        )
    }

    fn get_frame_slices(pixels: &[PixelAction], step: i64) -> Vec<&[PixelAction]> {
        let mut frames: Vec<&[PixelAction]> = vec![];
        let mut start = 0;

        frames.push(&[]);
        if step != 0 {
            for (end, pair) in pixels.windows(2).enumerate() {
                let diff = pair[1].delta / step - pair[0].delta / step;
                if diff > 0 {
                    frames.push(&pixels[start..=end]);
                    start = end;
                    for _ in 1..diff {
                        frames.push(&[]);
                    }
                }
            }

            frames.push(&pixels[start..]);
        } else {
            frames.push(&pixels);
        }

        frames
    }
}
