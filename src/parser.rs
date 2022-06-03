use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use crate::command::{PxlsError, PxlsResult};

use hex::FromHex;
use rayon::prelude::*;
use serde_json::Value;

pub struct PxlsParser {}

impl PxlsParser {
    pub fn parse_raw<'a, R>(input: &mut R, buffer: &'a mut String) -> PxlsResult<Vec<&'a str>>
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

    pub fn parse<R, T>(input: &mut R, parser: fn(&[&str]) -> PxlsResult<T>) -> PxlsResult<Vec<T>>
    where
        R: Read,
        T: Send,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let temp = buffer
            .as_parallel_string()
            .par_split_terminator(|c| c == '\n' || c == '\r' || c == '\t')
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();

        temp.par_chunks_exact(6)
            .map(|s| parser(s))
            .collect::<PxlsResult<Vec<T>>>()
    }
}

pub struct PaletteParser {}

impl PaletteParser {
    pub fn try_parse(path: &str) -> PxlsResult<Vec<[u8; 4]>> {
        let mut file = OpenOptions::new().read(true).open(path)?;

        match Path::new(path).extension().and_then(OsStr::to_str) {
            Some("json") => Ok(Self::parse_json(&mut file)?),
            Some("aco") => Ok(Self::parse_aco(&mut file)?),
            Some("csv") => Ok(Self::parse_csv(&mut file)?),
            Some("gpl") => Ok(Self::parse_gpl(&mut file)?),
            Some("txt") => Ok(Self::parse_txt(&mut file)?),
            _ => Err(PxlsError::Unsupported()),
        }
    }

    // TODO: Improve (?)
    pub fn parse_json<R>(input: &mut R) -> PxlsResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let v: Value = serde_json::from_str(&buffer)?;
        v["palette"]
            .as_array()
            .ok_or(PxlsError::BadToken(String::from("cannot find \"palette\"")))?
            .iter()
            .map(|v| {
                let rgb = <[u8; 3]>::from_hex(
                    v.as_object().ok_or(PxlsError::BadToken(String::from(
                        "invalid \"palette entry\"",
                    )))?["value"]
                        .as_str()
                        .ok_or(PxlsError::BadToken(String::from("invalid \"value\"")))?,
                )?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<PxlsResult<Vec<[u8; 4]>>>()
    }

    // Todo: Better parsing(?)
    pub fn parse_csv<R>(input: &mut R) -> PxlsResult<Vec<[u8; 4]>>
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
                    .map(|s| Ok(s.parse::<u8>()?))
                    .collect::<PxlsResult<Vec<u8>>>()?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<PxlsResult<Vec<[u8; 4]>>>()
    }

    // Todo: Better parsing
    pub fn parse_txt<R>(input: &mut R) -> PxlsResult<Vec<[u8; 4]>>
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

    pub fn parse_gpl<R>(input: &mut R) -> PxlsResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut rgba = vec![];
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;
        let mut data = buffer.lines();

        // Header
        let magic = data.next().ok_or(PxlsError::Eof())?;
        if magic != "GIMP Palette" {
            return Err(PxlsError::BadToken(magic.to_string()));
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
            let r = values.next().ok_or(PxlsError::Eof())?;
            let g = values.next().ok_or(PxlsError::Eof())?;
            let b = values.next().ok_or(PxlsError::Eof())?;
            // Ignore name, etc...

            rgba.push([r.parse::<u8>()?, g.parse::<u8>()?, b.parse::<u8>()?, 255]);
        }

        Ok(rgba)
    }

    // Todo: Version 2 + Additional colour spaces
    pub fn parse_aco<R>(input: &mut R) -> PxlsResult<Vec<[u8; 4]>>
    where
        R: Read,
    {
        let mut buffer = vec![];
        input.read_to_end(&mut buffer)?;

        let mut data = buffer
            .chunks_exact(2)
            .into_iter()
            .map(|a| u16::from_be_bytes([a[0], a[1]]));

        let version = data.next().ok_or(PxlsError::Eof())?;
        let len = data.next().ok_or(PxlsError::Eof())? as usize;
        let mut rgba = Vec::with_capacity(len);
        match version {
            1 => {
                for _ in 1..=len {
                    let color_space = data.next().ok_or(PxlsError::Eof())?;
                    match color_space {
                        0 => {
                            let r = data.next().ok_or(PxlsError::Eof())?;
                            let g = data.next().ok_or(PxlsError::Eof())?;
                            let b = data.next().ok_or(PxlsError::Eof())?;
                            let _ = data.next().ok_or(PxlsError::Eof())?; // Skip

                            // Safe unwrap
                            rgba.push([
                                u8::try_from(r / 257).unwrap(),
                                u8::try_from(g / 257).unwrap(),
                                u8::try_from(b / 257).unwrap(),
                                255,
                            ]);
                        }
                        _ => return Err(PxlsError::Unsupported()),
                    }
                }
            }
            _ => return Err(PxlsError::Unsupported()),
        }
        Ok(rgba)
    }
}
