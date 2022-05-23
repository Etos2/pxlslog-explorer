use std::io::{self, Read};

use rayon::prelude::*;
<<<<<<< Updated upstream
=======
use serde_json::Value;

// TODO: Line numbers for errors
#[non_exhaustive]
#[derive(Debug)]
pub enum ParserError {
    Io(io::Error),
    Eof(),
    BadToken(String),
    BadByte(u8),
    Unsupported(),
}

impl error::Error for ParserError {}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::Io(err) => write!(f, "{}", err),
            ParserError::Eof() => write!(f, "unexpected eof"),
            ParserError::BadToken(s) => write!(f, "invalid token ({})", s),
            ParserError::BadByte(b) => write!(f, "invalid byte ({})", b),
            ParserError::Unsupported() => write!(f, "unsupported format"),
        }
    }
}

impl From<std::io::Error> for ParserError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ParserError {
    fn from(value: serde_json::Error) -> Self {
        Self::BadToken(value.to_string())
    }
}
impl From<hex::FromHexError> for ParserError {
    fn from(value: hex::FromHexError) -> Self {
        Self::BadToken(value.to_string())
    }
}
>>>>>>> Stashed changes

// TODO: impl From
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