use std::error;
use std::fmt::Display;
use std::io;
use std::path::PathBuf;

pub type ConfigResult<T> = Result<T, ConfigError>;
pub type ParseResult<T> = Result<T, ParseError>;

pub trait Terminate
where
    Self: Display,
{
    fn exitcode(&self) -> i32;
    fn terminate(&self) -> ! {
        eprintln!("{}", self);
        std::process::exit(self.exitcode())
    }
}

#[derive(Debug)]
pub struct ConfigError {
    arg: String,
    reason: String,
}

impl error::Error for ConfigError {}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid argument \'{}\': {}", self.arg, self.reason)
    }
}

impl ConfigError {
    pub fn new(arg: &str, reason: &str) -> ConfigError {
        ConfigError {
            arg: arg.to_owned(),
            reason: reason.to_owned(),
        }
    }
}

impl Terminate for ConfigError {
    fn exitcode(&self) -> i32 {
        exitcode::USAGE
    }
}

#[derive(Debug)]
pub struct ParseError {
    file: PathBuf,
    line: usize,
    kind: ParseErrorKind,
}

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    Io(io::ErrorKind),
    BadToken(String),
    UnexpectedEof,
    Unsupported,
    InvalidFile,
}

impl error::Error for ParseError {}
impl ParseError {
    pub fn new(kind: ParseErrorKind) -> ParseError {
        ParseError {
            file: PathBuf::new(),
            line: 0,
            kind,
        }
    }

    pub fn new_with_file(kind: ParseErrorKind, file: &str, line: usize) -> ParseError {
        ParseError {
            file: PathBuf::from(file),
            line,
            kind,
        }
    }

    pub fn from_err<E>(err: E, file: &str, line: usize) -> ParseError
    where
        E: Into<ParseError>,
    {
        let mut e = err.into();
        e.file = PathBuf::from(file);
        e.line = line;
        e
    }
}

impl Terminate for ParseError {
    fn exitcode(&self) -> i32 {
        match self.kind {
            ParseErrorKind::Io(e) => match e {
                io::ErrorKind::NotFound => exitcode::NOINPUT,
                io::ErrorKind::AlreadyExists => exitcode::CANTCREAT,
                io::ErrorKind::PermissionDenied => exitcode::NOINPUT,
                _ => exitcode::IOERR,
            },
            ParseErrorKind::UnexpectedEof => exitcode::DATAERR,
            ParseErrorKind::BadToken(_) => exitcode::DATAERR,
            ParseErrorKind::Unsupported => exitcode::DATAERR,
            ParseErrorKind::InvalidFile => exitcode::DATAERR,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.kind {
            ParseErrorKind::InvalidFile => write!(
                f,
                "{}, {} contains no valid data",
                self.kind.to_string(),
                self.file.display(),
            ),
            ParseErrorKind::Io(_) => write!(
                f,
                "{} while reading {}",
                self.kind.to_string(),
                self.file.display(),
            ),
            _ => write!(
                f,
                "{} while reading {} at line {}",
                self.kind.to_string(),
                self.file.display(),
                self.line.to_string(),
            ),
        }
    }
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseErrorKind::Io(kind) => write!(f, "IO error ({})", kind.to_string()),
            ParseErrorKind::BadToken(t) => write!(f, "Token \'{}\' is invalid", t),
            ParseErrorKind::UnexpectedEof => write!(f, "Unexpected EOF"),
            ParseErrorKind::Unsupported => write!(f, "Unsupported file"),
            ParseErrorKind::InvalidFile => write!(f, "Invalid log"),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::new(ParseErrorKind::Io(e.kind()))
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(e: serde_json::Error) -> Self {
        ParseError::new(ParseErrorKind::BadToken(e.to_string()))
    }
}

impl From<hex::FromHexError> for ParseError {
    fn from(e: hex::FromHexError) -> Self {
        ParseError::new(ParseErrorKind::BadToken(e.to_string()))
    }
}

impl From<chrono::ParseError> for ParseError {
    fn from(e: chrono::ParseError) -> Self {
        ParseError::new(ParseErrorKind::BadToken(e.to_string()))
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(e: std::num::ParseIntError) -> Self {
        ParseError::new(ParseErrorKind::BadToken(e.to_string()))
    }
}

impl From<image::ImageError> for ParseError {
    fn from(e: image::ImageError) -> Self {
        ParseError::new(match e {
            image::ImageError::IoError(e) => ParseErrorKind::Io(e.kind()),
            _ => ParseErrorKind::InvalidFile,
        })
    }
}
