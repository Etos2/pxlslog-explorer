use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use crate::error::{ParseResult, ParseError, ParseErrorKind};

use hex::FromHex;
use serde_json::Value;

pub struct PaletteParser {}

impl PaletteParser {
    pub fn try_parse(path: &str) -> ParseResult<Vec<[u8; 4]>> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| ParseError::from_err(e, path, 0))?;

        match Path::new(path).extension().and_then(OsStr::to_str) {
            Some("json") => Ok(Self::parse_json(&mut file)?),
            Some("aco") => Ok(Self::parse_aco(&mut file)?),
            Some("csv") => Ok(Self::parse_csv(&mut file)?),
            Some("gpl") => Ok(Self::parse_gpl(&mut file)?),
            Some("txt") => Ok(Self::parse_txt(&mut file)?),
            _ => Err(ParseError::new(ParseErrorKind::Unsupported)),
        }.map_err(|e| ParseError::from_err(e, path, 0))
    }

    // TODO: Improve (?)
    pub fn parse_json<R>(input: &mut R) -> ParseResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let v: Value = serde_json::from_str(&buffer)?;
        v["palette"]
            .as_array()
            .ok_or(ParseError::new(ParseErrorKind::BadToken(String::from(
                "cannot find \"palette\" token",
            ))))?
            .iter()
            .map(|v| {
                let rgb = <[u8; 3]>::from_hex(
                    v.as_object()
                        .ok_or(ParseError::new(ParseErrorKind::BadToken(String::from(
                            "invalid \"palette entry\" token",
                        ))))?["value"]
                        .as_str()
                        .ok_or(ParseError::new(ParseErrorKind::BadToken(String::from(
                            "invalid \"value\" token",
                        ))))?,
                )?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<ParseResult<Vec<[u8; 4]>>>()
    }

    // Todo: Better parsing(?)
    pub fn parse_csv<R>(input: &mut R) -> ParseResult<Vec<[u8; 4]>>
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
                    .collect::<ParseResult<Vec<u8>>>()?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<ParseResult<Vec<[u8; 4]>>>()
    }

    // Todo: Better parsing
    pub fn parse_txt<R>(input: &mut R) -> ParseResult<Vec<[u8; 4]>>
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
                rgba.push([vals[1], vals[2], vals[3], vals[0]]);
                temp.clear();
            }
        }

        Ok(rgba)
    }

    pub fn parse_gpl<R>(input: &mut R) -> ParseResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut rgba = vec![];
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;
        let mut data = buffer.lines();

        // Header
        let magic = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
        if magic != "GIMP Palette" {
            return Err(ParseError::new(ParseErrorKind::BadToken(magic.to_string())));
        }

        // TODO: Better comments handling
        while let Some(line) = data.next() {
            if line == "#" {
                break;
            }
        }

        // Data
        while let Some(line) = data.next() {
            let mut values = line.split_whitespace();
            let r = values.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
            let g = values.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
            let b = values.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
            // Ignore name, etc...

            rgba.push([r.parse::<u8>()?, g.parse::<u8>()?, b.parse::<u8>()?, 255]);
        }

        Ok(rgba)
    }

    // Todo: Version 2 + Additional colour spaces
    pub fn parse_aco<R>(input: &mut R) -> ParseResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut buffer = vec![];
        input.read_to_end(&mut buffer)?;

        let mut data = buffer
            .chunks_exact(2)
            .into_iter()
            .map(|a| u16::from_be_bytes([a[0], a[1]]));

        let version = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
        let len = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))? as usize;
        let mut rgba = Vec::with_capacity(len);
        match version {
            1 => {
                for _ in 1..=len {
                    let color_space = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
                    match color_space {
                        0 => {
                            let r = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
                            let g = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
                            let b = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?;
                            let _ = data.next().ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?; // Skip

                            // Safe unwrap
                            rgba.push([
                                u8::try_from(r / 257).unwrap(),
                                u8::try_from(g / 257).unwrap(),
                                u8::try_from(b / 257).unwrap(),
                                255,
                            ]);
                        }
                        _ => return Err(ParseError::new(ParseErrorKind::Unsupported)),
                    }
                }
            }
            _ => return Err(ParseError::new(ParseErrorKind::Unsupported)),
        }
        Ok(rgba)
    }
}
