use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use hex::FromHex;
use serde_json::Value;

use crate::render::pixel::Rgba;

pub const DEFAULT_PALETTE: [Rgba; 32] = [
    Rgba([0, 0, 0, 255]),       // Black
    Rgba([34, 34, 34, 255]),    // Dark Grey
    Rgba([85, 85, 85, 255]),    // Deep Grey
    Rgba([136, 136, 136, 255]), // Medium Grey
    Rgba([205, 205, 205, 255]), // Light Grey
    Rgba([255, 255, 255, 255]), // White
    Rgba([255, 213, 188, 255]), // Beige
    Rgba([255, 183, 131, 255]), // Peach
    Rgba([182, 109, 61, 255]),  // Brown
    Rgba([119, 67, 31, 255]),   // Chocolate
    Rgba([252, 117, 16, 255]),  // Rust
    Rgba([252, 168, 14, 255]),  // Orange
    Rgba([253, 232, 23, 255]),  // Yellow
    Rgba([255, 244, 145, 255]), // Pastel Yellow
    Rgba([190, 255, 64, 255]),  // Lime
    Rgba([112, 221, 19, 255]),  // Green
    Rgba([49, 161, 23, 255]),   // Dark Green
    Rgba([11, 95, 53, 255]),    // Forest
    Rgba([39, 126, 108, 255]),  // Dark Teal
    Rgba([50, 182, 159, 255]),  // Light Teal
    Rgba([136, 255, 243, 255]), // Aqua
    Rgba([36, 181, 254, 255]),  // Azure
    Rgba([18, 92, 199, 255]),   // Blue
    Rgba([38, 41, 96, 255]),    // Navy
    Rgba([139, 47, 168, 255]),  // Purple
    Rgba([210, 76, 233, 255]),  // Mauve
    Rgba([255, 89, 239, 255]),  // Magenta
    Rgba([255, 169, 217, 255]), // Pink
    Rgba([255, 100, 116, 255]), // Watermelon
    Rgba([240, 37, 35, 255]),   // Red
    Rgba([177, 18, 6, 255]),    // Rose
    Rgba([116, 12, 0, 255]),    // Maroon
];

pub type Palette = Vec<Rgba>;

pub struct PaletteParser {}

impl PaletteParser {
    pub fn try_parse(path: &Path) -> Result<Palette> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .with_context(|| format!("Failed to open file {}", path.display()))?;

        match Path::new(path).extension().and_then(OsStr::to_str) {
            Some("json") => Ok(Self::parse_json(&mut file)?),
            Some("aco") => Ok(Self::parse_aco(&mut file)?),
            Some("csv") => Ok(Self::parse_csv(&mut file)?),
            Some("gpl") => Ok(Self::parse_gpl(&mut file)?),
            Some("txt") => Ok(Self::parse_txt(&mut file)?),
            _ => Err(anyhow!("Unsupported file: {}", path.display())),
        }
    }

    // TODO: Improve (?)
    pub fn parse_json<R>(input: &mut R) -> Result<Palette>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let v: Value = serde_json::from_str(&buffer)?;
        v["palette"]
            .as_array()
            .context("Cannot find \"palette\" token")?
            .iter()
            .map(|v| {
                let rgb = <[u8; 3]>::from_hex(
                    v.as_object()
                        .context("Invalid \"palette entry\" token")?["value"]
                        .as_str()
                        .context("Invalid \"value\" token")?,
                )?;
                Ok(Rgba::from([rgb[0], rgb[1], rgb[2], 255]))
            })
            .collect::<Result<Palette>>()
    }

    // Todo: Better parsing(?)
    pub fn parse_csv<R>(input: &mut R) -> Result<Palette>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer
            .split_terminator(&['\n'][..])
            .skip(1) // Skip 'Name,#hexadecimal,R,G,B'
            .map(|line| {
                let rgb = line
                    .split_terminator(&[','][..])
                    .skip(2)
                    .map(|s| Ok(s.parse::<u8>()?))
                    .collect::<Result<Vec<u8>>>()?;
                Ok(Rgba::from([rgb[0], rgb[1], rgb[2], 255]))
            })
            .collect::<Result<Palette>>()
    }

    // Todo: Better parsing
    pub fn parse_txt<R>(input: &mut R) -> Result<Palette>
    where
        R: Read,
    {
        let mut rgba = vec![];
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;
        let data = buffer.lines();

        let mut temp = String::with_capacity(8);
        for line in data {
            for c in line.chars() {
                if c == ';' || c == ' ' || c == '\t' {
                    break;
                } else {
                    temp.push(c);
                }
            }

            if !temp.is_empty() {
                let vals = <[u8; 4]>::from_hex(&temp)?;
                rgba.push(Rgba::from([vals[1], vals[2], vals[3], vals[0]]));
                temp.clear();
            }
        }

        Ok(rgba)
    }

    pub fn parse_gpl<R>(input: &mut R) -> Result<Palette>
    where
        R: Read,
    {
        let mut rgba = vec![];
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;
        let mut data = buffer.lines();

        // Header
        let magic = data
            .next()
            .context("Unexpected end of file")?;

        if magic != "GIMP Palette" {
            bail!("Invalid magic header");
        }

        // TODO: Better comments handling
        for line in data.by_ref() {
            if line == "#" {
                break;
            }
        }

        // Data
        for line in data {
            let mut values = line.split_whitespace();
            let r = values
                .next()
                .context("Unexpected end of file")?;
            let g = values
                .next()
                .context("Unexpected end of file")?;
            let b = values
                .next()
                .context("Unexpected end of file")?;
            // Ignore name, etc...

            rgba.push(Rgba::from([
                r.parse::<u8>()?,
                g.parse::<u8>()?,
                b.parse::<u8>()?,
                255,
            ]));
        }

        Ok(rgba)
    }

    // Todo: Version 2 + Additional colour spaces
    pub fn parse_aco<R>(input: &mut R) -> Result<Palette>
    where
        R: Read,
    {
        let mut buffer = vec![];
        input.read_to_end(&mut buffer)?;

        let mut data = buffer
            .chunks_exact(2)
            .map(|a| u16::from_be_bytes([a[0], a[1]]));

        let version = data
            .next()
            .context("Unexpected end of file")?;
        let len = data
            .next()
            .context("Unexpected end of file")? as usize;
        let mut rgba = Vec::with_capacity(len);

        match version {
            1 => {
                for _ in 1..=len {
                    let color_space = data
                        .next()
                        .context("Unexpected end of file")?;
                    match color_space {
                        0 => {
                            let r = data
                                .next()
                                .context("Unexpected end of file")?;
                            let g = data
                                .next()
                                .context("Unexpected end of file")?;
                            let b = data
                                .next()
                                .context("Unexpected end of file")?;
                            let _ = data
                                .next()
                                .context("Unexpected end of file")?; // Skip

                            // Safe unwrap
                            rgba.push(Rgba::from([
                                u8::try_from(r / 257).unwrap(),
                                u8::try_from(g / 257).unwrap(),
                                u8::try_from(b / 257).unwrap(),
                                255,
                            ]));
                        }
                        _ => bail!("Unsupported file"),
                    }
                }
            }
            _ => bail!("Unsupported file"),
        }

        Ok(rgba)
    }
}
