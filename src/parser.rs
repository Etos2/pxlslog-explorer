use std::io::{self, Read};

use rayon::prelude::*;

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