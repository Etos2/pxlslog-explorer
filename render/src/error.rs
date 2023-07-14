use std::fmt::Display;

use chrono::NaiveDateTime;
use common::action::DATE_FMT;
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
    OutOfOrder {
        time: NaiveDateTime,
        prev_time: NaiveDateTime,
    },
    InvalidIndex {
        index: usize,
        max_index: usize,
    },
    InvalidPosition {
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
            ActionErrorKind::OutOfOrder { time, prev_time } => {
                let time_str = time.format(DATE_FMT).to_string();
                let prev_time_str = prev_time.format(DATE_FMT).to_string();
                write!(
                    f,
                    "action is out of order (expected {time_str} < {prev_time_str})"
                )
            }
            ActionErrorKind::InvalidIndex { index, max_index } => write!(
                f,
                "index was out of bounds (expected {index} < {max_index})"
            ),
            ActionErrorKind::InvalidPosition { position, bounds } => write!(
                f,
                "position was out of bounds (expected {position:?} to be within {bounds:?})"
            ),
        }
    }
}
