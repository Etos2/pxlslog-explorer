use std::error;
use std::io;

pub type PxlsResult<T> = Result<T, PxlsError>;

#[derive(Debug)]
pub struct PxlsError {
    file: Option<String>,
    line: Option<usize>,
    kind: PxlsErrorKind,
}

impl PxlsError {
    pub fn new(kind: PxlsErrorKind) -> PxlsError {
        PxlsError { file: None, line: None, kind }
    }

    pub fn new_with_file(kind: PxlsErrorKind, file: &str) -> PxlsError {
        PxlsError {
            file: Some(file.to_owned()),
            line: None,
            kind,
        }
    }

    pub fn new_with_line(kind: PxlsErrorKind, file: &str, line: usize) -> PxlsError {
        PxlsError {
            file: Some(file.to_owned()),
            line: Some(line),
            kind,
        }
    }

    pub fn from<T>(err: T, file: &str, line: usize) -> PxlsError
    where
        T: Into<PxlsError>,
    {
        let mut err = err.into();
        err.file = Some(file.to_owned());
        err.line = Some(line);
        err
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum PxlsErrorKind {
    Io(io::Error),
    Unsupported(),
    Eof(),
    BadToken(String),
    InvalidState(String),
}

impl error::Error for PxlsError {}

impl std::fmt::Display for PxlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let file = self.file.as_deref().unwrap_or("");
        let line = self.line.unwrap_or(0);
        match &self.kind {
            PxlsErrorKind::Io(err) => write!(f, "{} ({})", err, file),
            PxlsErrorKind::Unsupported() => {
                write!(f, "Unsupported file or file format: ({})", file)
            }
            PxlsErrorKind::Eof() => write!(f, "Unexpected eof: ({}, {})", file, line),
            PxlsErrorKind::BadToken(s) => write!(f, "Invalid token: {} ({}, {})", s, file, line),
            PxlsErrorKind::InvalidState(s) => write!(f, "Invalid state: {}", s),
        }
    }
}

impl From<std::io::Error> for PxlsError {
    fn from(value: std::io::Error) -> Self {
        PxlsError::new(PxlsErrorKind::Io(value))
    }
}

impl From<serde_json::Error> for PxlsError {
    fn from(value: serde_json::Error) -> Self {
        PxlsError::new(PxlsErrorKind::BadToken(value.to_string()))
    }
}

impl From<hex::FromHexError> for PxlsError {
    fn from(value: hex::FromHexError) -> Self {
        PxlsError::new(PxlsErrorKind::BadToken(value.to_string()))
    }
}

impl From<image::ImageError> for PxlsError {
    fn from(_: image::ImageError) -> Self {
        PxlsError::new(PxlsErrorKind::Unsupported())
    }
}

impl From<chrono::ParseError> for PxlsError {
    fn from(value: chrono::ParseError) -> Self {
        PxlsError::new(PxlsErrorKind::BadToken(value.to_string()))
    }
}

impl From<std::num::ParseIntError> for PxlsError {
    fn from(value: std::num::ParseIntError) -> Self {
        PxlsError::new(PxlsErrorKind::BadToken(value.to_string()))
    }
}
