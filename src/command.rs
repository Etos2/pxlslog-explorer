use std::error;
use std::io;

use crate::Cli;

pub trait PxlsCommand
{
    fn run(&self, settings: &Cli) -> PxlsResult<()>;
}

pub type PxlsResult<T> = Result<T, PxlsError>;

// TODO: Specific errors for Parsing, Rendering, Filtering, etc (?)
#[non_exhaustive]
#[derive(Debug)]
pub enum PxlsError {
    Io(io::Error),
    Unsupported(),
    Eof(),
    BadToken(String),
}

impl error::Error for PxlsError {}

impl std::fmt::Display for PxlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PxlsError::Io(err) => write!(f, "{}", err),
            PxlsError::Unsupported() => write!(f, "unsupported file or file format"),
            PxlsError::Eof() => write!(f, "unexpected eof"),
            PxlsError::BadToken(s) => write!(f, "invalid token ({})", s),
        }
    }
}

impl From<std::io::Error> for PxlsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PxlsError {
    fn from(value: serde_json::Error) -> Self {
        Self::BadToken(value.to_string())
    }
}

impl From<hex::FromHexError> for PxlsError {
    fn from(value: hex::FromHexError) -> Self {
        Self::BadToken(value.to_string())
    }
}

impl From<image::ImageError> for PxlsError {
    fn from(_: image::ImageError) -> Self {
        Self::Unsupported()
    }
}

impl From<chrono::ParseError> for PxlsError {
    fn from(value: chrono::ParseError) -> Self {
        Self::BadToken(value.to_string())
    }
}
impl From<std::num::ParseIntError> for PxlsError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::BadToken(value.to_string())
    }
}