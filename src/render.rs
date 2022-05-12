use std::fmt;
use std::fs::OpenOptions;
use std::io;

use crate::parser::PxlsParser;
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgEnum, Args};
use image::io::Reader as ImageReader;
use image::{ImageBuffer, RgbaImage};

// TODO
#[derive(Args)]
#[clap(about = "Render timelapses and other imagery", long_about = None)]
pub struct RenderInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file", display_order = 0)]
    src: String,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of output image file", display_order = 1)]
    dst: String,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(conflicts_with("src_bg"))]
    #[clap(help = "Filepath of background image")]
    bg: Option<String>,
    #[clap(short, long)]
    #[clap(max_values(2))]
    #[clap(min_values(2))]
    #[clap(value_name("SIZE"))]
    #[clap(help = "Size of canvas")]
    size: Option<Vec<u32>>,
    #[clap(long, arg_enum)]
    #[clap(help = "Type of render")]
    r#type: Option<RenderType>,
    #[clap(long)]
    #[clap(help = "Time between frames")]
    step: Option<i64>,
}

// TODO: Clean
const PALETTE: [[u8; 4]; 33] = [
    [0, 0, 0, 0],         // Transparent
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
    dst: String,
    src_bg: Option<String>,
    background: RgbaImage,
    style: RenderType,
    step: i64,
    palette: [[u8; 4]; 33],
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum RenderType {
    Normal,
    Heat,
    Virgin,
}

impl RenderInput {
    // TODO: Custom errors
    pub fn validate(&self) -> Result<Render, std::io::Error> {
        let style = match self.r#type {
            Some(t) => t,
            None => RenderType::Normal,
        };

        let pixels = Render::get_pixels(&self.src).unwrap();
        let background = match &self.bg {
            Some(path) => ImageReader::open(path)?.decode().unwrap().to_rgba8(),
            None => match &self.size {
                Some(size) => {
                    ImageBuffer::from_pixel(size[0], size[1], image::Rgba::from([0, 0, 0, 0]))
                }
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
            },
        };

        // TODO: Move to filter
        let mut offset = (u32::MAX, u32::MAX);
        for pixel in &pixels {
            if pixel.x < offset.0 {
                offset.0 = pixel.x;
            }
            if pixel.y < offset.1 {
                offset.1 = pixel.y;
            }
        }

        Ok(Render {
            src: self.src.to_owned(),
            dst: self.dst.to_owned(),
            src_bg: self.bg.to_owned(),
            background,
            style,
            step: self.step.unwrap_or(0),
            palette: PALETTE, // TODO: Allow user input
        })
    }
}

impl fmt::Display for Render {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Performing RENDER command with following arguments:")?;

        write!(f, "\n  --src:    {}", self.src)?;
        write!(f, "\n  --dst:    {}", self.dst)?;
        if let Some(path) = &self.src_bg {
            write!(f, "\n  --bg:     {}", path)?;
        } else {
            write!(
                f,
                "\n  --size:   {} x {}",
                self.background.width(),
                self.background.height()
            )?;
        }
        write!(f, "\n  --step:   {}", self.step)?;
        write!(f, "\n  --type:   {:?}", self.style)?;

        Ok(())
    }
}

impl Render {
    pub fn execute(self, settings: &Cli) -> io::Result<usize> {
        let dst = self.dst.rsplit_once('.').unwrap();
        let pixels = Self::get_pixels(&self.src)?;
        let frames = Self::get_frame_slices(&pixels, self.step);
        let mut current_frame = self.background;

        current_frame
            .save(format!("{}_{}.{}", dst.0, 0, dst.1))
            .unwrap();

        if settings.verbose {
            println!("Rendering {} frames", frames.len());
        }

        for (i, frame) in frames.iter().enumerate() {
            if !frame.is_empty() {
                current_frame = match self.style {
                    RenderType::Normal => Self::get_frame(&current_frame, frame, &self.palette),
                    RenderType::Heat => unimplemented!(),
                    RenderType::Virgin => unimplemented!(),
                };
            }

            current_frame
                .save(format!("{}_{}.{}", dst.0, i + 1, dst.1))
                .unwrap();

            if settings.verbose && (i + 1) % 250 == 0 {
                println!("Rendered {} frames", i + 1);
            }
        }

        Ok(frames.len())
    }

    // TODO: Better error handling
    // TODO: External io
    fn get_pixels(path: &str) -> io::Result<Vec<PixelAction>> {
        PxlsParser::parse(
            &mut OpenOptions::new().read(true).open(path)?,
            |s: &[&str]| -> PixelAction {
                PixelAction {
                    x: s[2].parse().unwrap(),
                    y: s[3].parse().unwrap(),
                    i: match s[4].parse::<usize>().unwrap() {
                        255 => 0,
                        i => i,
                    },
                    delta: NaiveDateTime::parse_from_str(s[0], "%Y-%m-%d %H:%M:%S,%3f")
                        .unwrap()
                        .timestamp_millis(),
                }
            },
        )
    }

    fn get_frame_slices(pixels: &[PixelAction], step: i64) -> Vec<&[PixelAction]> {
        let mut frames: Vec<&[PixelAction]> = vec![];
        let mut start = 0;

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

        frames
    }

    fn no_get_frame_slices(pixels: &[PixelAction], step: i64) -> Vec<&[PixelAction]> {
        let mut frames = vec![];
        let mut start = 0;
        let mut cutoff = pixels[0].delta + step;

        for (end, pixel) in pixels.iter().enumerate() {
            if pixel.delta >= cutoff {
                cutoff += step;
                start = end;
                frames.push(&pixels[start..=end]);
            }
        }
        if start != pixels.len() - 1 {
            frames.push(&pixels[start..]);
        }

        frames
    }

    fn get_frame(background: &RgbaImage, pixels: &[PixelAction], palette: &[[u8; 4]]) -> RgbaImage {
        let mut frame = background.clone();
        for pixel in pixels {
            frame.put_pixel(pixel.x, pixel.y, image::Rgba::from(palette[pixel.i + 1]));
        }
        frame
    }

    // TODO
    fn _get_heat_frame(
        _background: &RgbaImage,
        _pixels: &[PixelAction],
        _palette: &[[u8; 4]],
    ) -> RgbaImage {
        unimplemented!()
    }

    // TODO
    fn _get_virgin_images(
        _background: &RgbaImage,
        _pixels: &[PixelAction],
        _palette: &[[u8; 4]],
    ) -> RgbaImage {
        unimplemented!()
    }
}