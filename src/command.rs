use std::error;
use std::io;

use crate::Cli;

pub trait PxlsInput
{
    fn parse(&self, settings: &Cli) -> Result<Box<dyn PxlsCommand>, PxlsError>;
}

pub trait PxlsCommand
{
    fn run(&self, settings: &Cli) -> Result<(), PxlsError>;
}

// TODO: Line numbers for errors
#[non_exhaustive]
#[derive(Debug)]
pub enum PxlsError {
    Io(io::Error),
    Eof(),
    BadToken(String),
    BadByte(u8),
    Unsupported(),
}

impl error::Error for PxlsError {}

impl std::fmt::Display for PxlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PxlsError::Io(err) => write!(f, "{}", err),
            PxlsError::Eof() => write!(f, "unexpected eof"),
            PxlsError::BadToken(s) => write!(f, "invalid token ({})", s),
            PxlsError::BadByte(b) => write!(f, "invalid byte ({})", b),
            PxlsError::Unsupported() => write!(f, "unsupported format"),
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