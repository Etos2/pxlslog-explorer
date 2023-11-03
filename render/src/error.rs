use std::fmt::Display;

use chrono::NaiveDateTime;
use common::data::DATE_FMT;
use image::ImageError;
use nom_supreme::{error::ErrorTree, final_parser::Location};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode image: {0}")]
    Image(#[from] ImageError),
    #[error("failed to parse actions: {0}")]
    Parse(#[from] ErrorTree<Location>),
    #[error("invalid action: {0}")]
    InvalidAction(#[from] ActionError),
}

#[derive(Error, Debug)]
pub struct ActionError {
    pub line: usize,
    pub kind: ActionErrorKind,
}

#[derive(Debug)]
pub enum ActionErrorKind {
    _OutOfOrder {
        time: i64,
        prev_time: i64,
    },
    _InvalidIndex {
        index: usize,
        max_index: usize,
    },
    _InvalidPosition {
        position: (u32, u32),
        bounds: (u32, u32),
    },
}

impl Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ line {}", self.kind, self.line)
    }
}

impl Display for ActionErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionErrorKind::_OutOfOrder { time, prev_time } => {
                let time_str = NaiveDateTime::from_timestamp_millis(*time)
                    .unwrap() // Safety: Fails in the year 262000, not my problem
                    .format(DATE_FMT)
                    .to_string();
                let prev_time_str = NaiveDateTime::from_timestamp_millis(*prev_time)
                    .unwrap() // Safety: Fails in the year 262000, not my problem
                    .format(DATE_FMT)
                    .to_string();
                write!(f, "out of order (expected {time_str} < {prev_time_str})")
            }
            ActionErrorKind::_InvalidIndex { index, max_index } => write!(
                f,
                "index was out of bounds (expected {index} < {max_index})"
            ),
            ActionErrorKind::_InvalidPosition { position, bounds } => write!(
                f,
                "position was out of bounds (expected {position:?} to be within {bounds:?})"
            ),
        }
    }
}
