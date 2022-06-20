use std::io::Read;
use std::str::FromStr;

use crate::error::{PxlsError, PxlsResult};

use clap::ArgEnum;
use rayon::prelude::*;

// TODO: Hash(?)
pub struct Pixel {
    pub x: u32,
    pub y: u32,
    pub timestamp: i64,
    pub index: usize,
    pub kind: PixelKind,
}

// TODO: Move ArgEnum into filter.rs
#[derive(Debug, Copy, Clone, ArgEnum)]
pub enum PixelKind {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}

impl FromStr for PixelKind {
    type Err = PxlsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user place" => Ok(PixelKind::Place),
            "user undo" => Ok(PixelKind::Undo),
            "mod overwrite" => Ok(PixelKind::Overwrite),
            "rollback" => Ok(PixelKind::Rollback),
            "rollback undo" => Ok(PixelKind::RollbackUndo),
            "console nuke" => Ok(PixelKind::Nuke),
            _ => Err(PxlsError::BadToken(s.to_string())),
        }
    }
}

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