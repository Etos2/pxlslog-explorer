use std::io::Read;
use std::str::FromStr;

use crate::error::{PxlsError, PxlsErrorKind, PxlsResult};

use clap::ArgEnum;
use rayon::prelude::*;

// TODO: Hash(?)
pub struct Action {
    pub x: u32,
    pub y: u32,
    pub timestamp: i64,
    pub index: usize,
    pub kind: ActionKind,
}

// TODO: Move ArgEnum into filter.rs
#[derive(Debug, Copy, Clone, ArgEnum)]
pub enum ActionKind {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}

impl FromStr for ActionKind {
    type Err = PxlsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user place" => Ok(ActionKind::Place),
            "user undo" => Ok(ActionKind::Undo),
            "mod overwrite" => Ok(ActionKind::Overwrite),
            "rollback" => Ok(ActionKind::Rollback),
            "rollback undo" => Ok(ActionKind::RollbackUndo),
            "console nuke" => Ok(ActionKind::Nuke),
            _ => Err(PxlsError::new(PxlsErrorKind::BadToken(s.to_string()))),
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

    // pub fn parse<R, T, F>(input: &mut R, parser: F) -> PxlsResult<Vec<T>>
    // where
    //     R: Read,
    //     T: Send,
    //     F: Fn(&[&str], usize) -> PxlsResult<Option<T>> + Sync + Send,
    // {
    //     let mut buffer = String::new();
    //     input.read_to_string(&mut buffer)?;

    //     let temp = buffer
    //         .as_parallel_string()
    //         .par_split_terminator(|c| c == '\n' || c == '\r' || c == '\t')
    //         .filter(|t| !t.is_empty())
    //         .collect::<Vec<_>>();
            
    //     temp.par_chunks_exact(6)
    //         .enumerate()
    //         .filter_map(|(i, s)| {
    //             parser(s, i).transpose()
    //         })
    //         .collect()
    // }
}
