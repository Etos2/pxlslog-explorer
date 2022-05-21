use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};

use crate::parser::{PaletteParser, PxlsParser};
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgEnum, ArgGroup, Args};
use image::io::Reader as ImageReader;
use image::{ImageBuffer, RgbaImage};

#[derive(Args)]
#[clap(
    about = "Render individual frames or output raw frame data to STDOUT.",
    long_about = "Render individual frames or output raw frame data to STDOUT.
Guaranted to produce 2 frames per render, where the first frame is the background and the last frame is the complete contents of the log.
To output only the final result, use the \"--screenshot\" arg."
)]
#[clap(group = ArgGroup::new("qol").args(&["step", "skip", "screenshot"]).required(true).multiple(true))]
#[clap(group = ArgGroup::new("qol-conflict").args(&["step", "skip"]).conflicts_with("screenshot"))]
pub struct RenderInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file", display_order = 0)]
    src: String,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of output frames", display_order = 1)]
    dst: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of background image")]
    bg: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of palette (.json)")]
    palette: Option<String>,
    #[clap(long)]
    #[clap(max_values(2))]
    #[clap(min_values(2))]
    #[clap(value_name("INT"))]
    #[clap(help = "Size of canvas")]
    size: Option<Vec<u32>>,
    #[clap(long, arg_enum)]
    #[clap(value_name("ENUM"))]
    #[clap(help = "Type of render")]
    r#type: Option<RenderType>,
    #[clap(long)]
    #[clap(value_name("LONG"))]
    #[clap(help = "Time between frames (0 is equilavent to i64::MAX)")]
    step: Option<i64>,
    #[clap(long)]
    #[clap(value_name("INT"))]
    #[clap(help = "Skip n frames")]
    skip: Option<usize>,
    #[clap(long)]
    #[clap(help = "Only return final frame (Alias for \"--step 0 --skip 1\")")]
    screenshot: bool,
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

// TODO: Clean
// const PALETTE: [[u8; 3]; 15] = [
//     [5, 22, 32],
//     [49, 105, 80],
//     [134, 192, 108],
//     [223, 248, 209],
//     [0, 0, 0],
//     [34, 34, 34],
//     [85, 85, 85],
//     [136, 136, 136],
//     [205, 205, 205],
//     [255, 255, 255],
//     [36, 181, 254],
//     [19, 92, 199],
//     [240, 37, 35],
//     [177, 18, 6],
// ];

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
    src_bg: Option<String>,
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
            skip += 1;
        }

        let palette = match &self.palette {
            Some(path) => {
                PaletteParser::parse_json(&mut OpenOptions::new().read(true).open(path).unwrap()).unwrap()
            },
            None => PALETTE.to_vec(),
        };

        Ok(Render {
            src: self.src.to_owned(),
            dst: self.dst.clone(),
            src_bg: self.bg.to_owned(),
            background,
            style,
            step,
            skip,
            palette,
        })
    }
}

impl fmt::Display for Render {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Performing RENDER command with following arguments:")?;

        write!(f, "\n  --src:    {}", self.src)?;
        if let Some(path) = &self.dst {
            write!(f, "\n  --dst:    {}", path)?;
        } else {
            write!(f, "\n  --dst:    STDOUT")?;
        }
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
        let stdin = io::stdout();
        let pixels = Self::get_pixels(&self.src)?;
        let frames = Self::get_frame_slices(&pixels, self.step);
        let mut current_frame = self.background.clone();

        if settings.verbose {
            println!("Rendering {} frames", frames.len());
        }

        for (i, frame) in frames[self.skip..].iter().enumerate() {
            if !frame.is_empty() {
                current_frame = match self.style {
                    RenderType::Normal => {
                        Self::get_frame(&self.background, &current_frame, frame, &self.palette)
                    }
                    RenderType::Heat => unimplemented!(),
                    RenderType::Virgin => unimplemented!(),
                };
            }

            match &self.dst {
                Some(path) => Self::frame_to_file(&current_frame, &path, i),
                None => Self::frame_to_raw(&current_frame, &mut stdin.lock()),
            }
        }

        Ok(frames.len())
    }

    // TODO: Error handling
    fn frame_to_file(frame: &RgbaImage, path: &str, i: usize) {
        let dst = path.rsplit_once('.').unwrap();
        frame.save(format!("{}_{}.{}", dst.0, i, dst.1)).unwrap();
    }

    fn frame_to_raw<R: Write>(frame: &RgbaImage, out: &mut R) {
        let buf = &frame.as_raw()[..];
        out.write_all(buf).unwrap();
        out.flush().unwrap();
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

    fn get_frame(
        background: &RgbaImage,
        previous: &RgbaImage,
        actions: &[PixelAction],
        palette: &[[u8; 4]],
    ) -> RgbaImage {
        let mut frame = previous.clone();
        for action in actions {
            if let Some(pixel) = palette.get(action.i) {
                frame.put_pixel(action.x, action.y, image::Rgba::from(*pixel));
            } else {
                frame.put_pixel(
                    action.x,
                    action.y,
                    *background.get_pixel(action.x, action.y),
                );
            }
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
