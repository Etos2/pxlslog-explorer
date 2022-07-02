use std::error;
use std::io;

pub type PxlsResult<T> = Result<T, PxlsError>;

#[derive(Debug)]
pub struct PxlsError {
    file: Option<String>,
    kind: PxlsErrorKind,
}

impl PxlsError {
    pub fn new(kind: PxlsErrorKind) -> PxlsError {
        PxlsError { file: None, kind }
    }

    pub fn new_with_file(kind: PxlsErrorKind, file: &str) -> PxlsError {
        PxlsError {
            file: Some(file.to_owned()),
            kind,
        }
    }

    pub fn from<T>(err: T, file: &str) -> PxlsError
    where
        T: Into<PxlsError>,
    {
        let mut err = err.into();
        err.file = Some(file.to_owned());
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
        match &self.kind {
            PxlsErrorKind::Io(err) => write!(f, "{} ({})", err, file),
            PxlsErrorKind::Unsupported() => {
                write!(f, "Unsupported file or file format: ({})", file)
            }
            PxlsErrorKind::Eof() => write!(f, "Unexpected eof: ({})", file),
            PxlsErrorKind::BadToken(s) => write!(f, "Invalid token: {} ({})", s, file),
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
