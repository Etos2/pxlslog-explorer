use std::error;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;

use hex::FromHex;
use rayon::prelude::*;
use serde_json::Value;

// TODO: Line numbers for errors
#[non_exhaustive]
#[derive(Debug)]
pub enum ParserError {
    Io(io::Error),
    Eof(),
    BadToken(String),
    BadByte(u8),
    Unsupported(),
}

impl error::Error for ParserError {}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::Io(err) => write!(f, "{}", err),
            ParserError::Eof() => write!(f, "unexpected eof"),
            ParserError::BadToken(c) => write!(f, "invalid token ({})", c),
            ParserError::BadByte(b) => write!(f, "invalid byte ({})", b),
            ParserError::Unsupported() => write!(f, "unsupported format"),
        }
    }
}

impl From<std::io::Error> for ParserError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ParserError {
    fn from(value: serde_json::Error) -> Self {
        Self::BadToken(value.to_string())
    }
}
impl From<hex::FromHexError> for ParserError {
    fn from(value: hex::FromHexError) -> Self {
        Self::BadToken(value.to_string())
    }
}

pub struct PxlsParser {}

impl PxlsParser {
    // TODO: Error detection
    pub fn parse_raw<'a, R>(
        input: &mut R,
        buffer: &'a mut String,
    ) -> Result<Vec<&'a str>, ParserError>
    where
        R: Read,
    {
        input.read_to_string(buffer)?;
        Ok(buffer
            .as_parallel_string()
            .par_split_terminator(|c| c == '\n' || c == '\r' || c == '\t')
            .filter(|t| !t.is_empty())
            .collect())
    }

    // TODO: Error detection
    pub fn parse<R, T>(input: &mut R, parser: fn(&[&str]) -> T) -> Result<Vec<T>, ParserError>
    where
        R: Read,
        T: Send,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let temp: Vec<&str> = buffer
            .as_parallel_string()
            .par_split_terminator(|c| c == '\n' || c == '\r' || c == '\t')
            .filter(|t| !t.is_empty())
            .collect();
        Ok(temp.par_chunks_exact(6).map(|s| parser(s)).collect())
    }
}

pub struct PaletteParser {}

impl PaletteParser {
    pub fn try_parse(path: &str) -> Result<Vec<[u8; 4]>, ParserError> {
        let mut file = OpenOptions::new().read(true).open(path)?;

        match Path::new(path).extension().and_then(OsStr::to_str) {
            Some("json") => Ok(Self::parse_json(&mut file)?),
            Some("aco") => Ok(Self::parse_aco(&mut file)?),
            Some("csv") => Ok(Self::parse_csv(&mut file)?),
            Some("gpl") => Ok(Self::parse_gpl(&mut file)?),
            Some("txt") => Ok(Self::parse_txt(&mut file)?),
            _ => Err(ParserError::Unsupported()),
        }
    }

    // TODO: Json error
    pub fn parse_json<R>(input: &mut R) -> Result<Vec<[u8; 4]>, ParserError>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let v: Value = serde_json::from_str(&buffer)?;
        v["palette"]
            .as_array()
            .ok_or(ParserError::BadToken(String::from(
                "cannot find \"palette\"",
            )))?
            .iter()
            .map(|v| {
                let rgb = <[u8; 3]>::from_hex(
                    v.as_object().ok_or(ParserError::BadToken(String::from(
                        "invalid \"palette entry\"",
                    )))?["value"]
                        .as_str()
                        .ok_or(ParserError::BadToken(String::from("invalid \"value\"")))?,
                )?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<Result<Vec<[u8; 4]>, _>>()
    }

    // Todo: Better parsing
    pub fn parse_csv<R>(input: &mut R) -> Result<Vec<[u8; 4]>, ParserError>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer
            .split_terminator(&['\n'][..])
            .skip(1)
            .map(|line| {
                let rgb = line
                    .split_terminator(&[','][..])
                    .skip(2)
                    .map(|s| s.parse::<u8>().unwrap())
                    .collect::<Vec<u8>>();
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<Result<Vec<[u8; 4]>, _>>()
    }

    // Todo: Better parsing
    pub fn parse_txt<R>(input: &mut R) -> Result<Vec<[u8; 4]>, ParserError>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer
            .split_terminator(&['\n'][..])
            .skip(1)
            .map(|line| {
                let rgba = <[u8; 4]>::from_hex(line.split_terminator(&[' '][..]).next().unwrap())?;
                Ok([rgba[1], rgba[2], rgba[3], rgba[0]])
            })
            .collect::<Result<Vec<[u8; 4]>, _>>()
    }

    // Todo: Better parsing
    pub fn parse_gpl<R>(input: &mut R) -> Result<Vec<[u8; 4]>, ParserError>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer
            .rsplit_once(&['#'][..])
            .unwrap()
            .1
            .split_terminator(&['\n'][..])
            .filter(|s| !s.is_empty())
            .map(|line| {
                let rgb = line
                    .split_terminator(&[' '][..])
                    .filter_map(|s| s.parse::<u8>().ok())
                    .collect::<Vec<u8>>();
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<Result<Vec<[u8; 4]>, _>>()
    }

    // Todo: Version 2 + Additional colour spaces
    pub fn parse_aco<R>(input: &mut R) -> Result<Vec<[u8; 4]>, ParserError>
    where
        R: Read,
    {
        let mut rgba = vec![];
        let mut buffer = vec![];
        input.read_to_end(&mut buffer)?;

        let buffer: Vec<u16> = buffer
            .chunks_exact(2)
            .into_iter()
            .map(|a| u16::from_be_bytes([a[0], a[1]]))
            .collect();
        let mut data = buffer.iter();

        let version = data.next().ok_or(ParserError::Eof())?;
        let len = *data.next().ok_or(ParserError::Eof())?;
        match version {
            1 => {
                for _ in 1..=len {
                    let color_space = data.next().ok_or(ParserError::Eof())?;
                    match color_space {
                        0 => {
                            let r = data.next().ok_or(ParserError::Eof())?;
                            let g = data.next().ok_or(ParserError::Eof())?;
                            let b = data.next().ok_or(ParserError::Eof())?;
                            let _ = data.next().ok_or(ParserError::Eof())?; // Skip

                            // Safe unwrap
                            rgba.push([
                                u8::try_from(r / 257).unwrap(),
                                u8::try_from(g / 257).unwrap(),
                                u8::try_from(b / 257).unwrap(),
                                255,
                            ]);
                        }
                        _ => return Err(ParserError::Unsupported()),
                    }
                }
            }
            _ => return Err(ParserError::Unsupported()),
        }
        Ok(rgba)
    }
}
