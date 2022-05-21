use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;

use hex::FromHex;
use rayon::prelude::*;
use serde_json::Value;

pub struct PxlsParser {}

impl PxlsParser {
    pub fn parse_raw<'a, R>(input: &mut R, buffer: &'a mut String) -> io::Result<Vec<&'a str>>
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

    pub fn parse<R, T>(input: &mut R, parser: fn(&[&str]) -> T) -> io::Result<Vec<T>>
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
        Ok(temp.par_chunks(6).map(|s| parser(s)).collect())
    }
}

pub struct PaletteParser {}

impl PaletteParser {
    pub fn try_parse(path: &str) -> Result<Vec<[u8; 4]>, Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new().read(true).open(path)?;

        match Path::new(path).extension().and_then(OsStr::to_str) {
            Some("json") => Ok(Self::parse_json(&mut file)?),
            Some("csv") => Ok(Self::parse_csv(&mut file)?),
            Some("gpl") => unimplemented!(),
            Some("txt") => Ok(Self::parse_txt(&mut file)?),
            Some(_) => panic!("Palette not supported!"), // TODO: Custom error
            None => unreachable!(),
        }
    }

    // TODO: Json error
    pub fn parse_json<R>(input: &mut R) -> Result<Vec<[u8; 4]>, Box<dyn std::error::Error>>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let v: Value = serde_json::from_str(&buffer)?;
        v["palette"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let rgb =
                    <[u8; 3]>::from_hex(v.as_object().unwrap()["value"].as_str().unwrap())?;
                Ok([rgb[0], rgb[1], rgb[2], 255])
            })
            .collect::<Result<Vec<[u8; 4]>,_>>()
    }

    // TODO: Smarter?
    pub fn parse_csv<R>(input: &mut R) -> Result<Vec<[u8; 4]>, Box<dyn std::error::Error>>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer.split_terminator(&['\n'][..]).skip(1).map(|line| {
            let rgb = line.split_terminator(&[','][..]).skip(2).map(|s| s.parse::<u8>().unwrap()).collect::<Vec<u8>>();
            Ok([rgb[0], rgb[1], rgb[2], 255])
        }).collect::<Result<Vec<[u8; 4]>,_>>()
    }

    pub fn parse_txt<R>(input: &mut R) -> Result<Vec<[u8; 4]>, Box<dyn std::error::Error>>
    where
        R: Read,
    {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        buffer.split_terminator(&['\n'][..]).skip(1).map(|line| {
            let rgba = <[u8; 4]>::from_hex(line.split_terminator(&[' '][..]).next().unwrap())?;
            Ok([rgba[1], rgba[2], rgba[3], rgba[0]])
        }).collect::<Result<Vec<[u8; 4]>,_>>()
    }
}
