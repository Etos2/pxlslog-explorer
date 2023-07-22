use std::{fmt::Display, path::PathBuf};

use chrono::ParseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct ActionParseError {
    location: Option<(u32, u32)>,
    path: Option<PathBuf>,
    #[source]
    kind: ActionParseErrorKind,
}

#[derive(Error, Debug)]
pub enum ActionParseErrorKind {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("time could not be parsed {1} ({0})")]
    InvalidTime(String, ParseError),
    #[error("identifier could not be parsed ({0})")]
    InvalidIdentifier(String),
    #[error("coordinates could not be parsed ({0})")]
    InvalidCoord(String),
    #[error("index could not be parsed ({0})")]
    InvalidIndex(String),
    #[error("kind could not be parsed ({0})")]
    InvalidKind(String),
    #[error("expected end of line")]
    ExpectedEndOfLine,
    #[error("expected end of file")]
    ExpectedEof,
    #[error("unexpected end of file")]
    UnexpectedEof,
}

impl ActionParseError {
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        self.path = Some(path);
        self
    }

    pub fn with_position(mut self, line: u32, column: u32) -> Self {
        self.location = Some((line, column));
        self
    }
}

impl From<std::io::Error> for ActionParseError {
    fn from(value: std::io::Error) -> Self {
        Self {
            location: None,
            path: None,
            kind: ActionParseErrorKind::from(value),
        }
    }
}

impl From<ActionParseErrorKind> for ActionParseError {
    fn from(value: ActionParseErrorKind) -> Self {
        Self {
            location: None,
            path: None,
            kind: value,
        }
    }
}

impl Display for ActionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ActionParseErrorKind::Io(_) => match &self.path {
                Some(path) => write!(
                    f,
                    "io error while parsing: {} @ {}",
                    self.kind,
                    path.display()
                ),
                None => write!(f, "io error while parsing: {}", self.kind),
            },
            _ => {
                write!(f, "error while parsing: {}", self.kind)?;
                match (&self.path, self.location) {
                    (None, None) => Ok(()),
                    (None, Some((l, c))) => write!(f, "@ ln {l}, col {c}"),
                    (Some(path), None) => write!(f, "@ {}", path.display()),
                    (Some(path), Some((l, c))) => {
                        write!(f, "@ ln {l}, col {c}, in {}", path.display())
                    }
                }
            }
        }
    }
}
