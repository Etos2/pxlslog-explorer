use std::error;
use std::fmt::Display;
use std::io;
use std::path::PathBuf;

pub type ConfigResult<T> = Result<T, ConfigError>;
pub type RuntimeResult<T> = Result<T, RuntimeError>;

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
pub struct RuntimeError {
    file: PathBuf,
    line: usize,
    kind: RuntimeErrorKind,
}

#[derive(Debug, Clone)]
pub enum RuntimeErrorKind {
    Io(io::ErrorKind),
    BadToken(String),
    UnexpectedEof,
    Unsupported,
    InvalidFile,
}

impl error::Error for RuntimeError {}
impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind) -> RuntimeError {
        RuntimeError {
            file: PathBuf::new(),
            line: 0,
            kind,
        }
    }

    pub fn new_with_file(kind: RuntimeErrorKind, file: &str, line: usize) -> RuntimeError {
        RuntimeError {
            file: PathBuf::from(file),
            line,
            kind,
        }
    }

    pub fn from_err<E>(err: E, file: &str, line: usize) -> RuntimeError
    where
        E: Into<RuntimeError>,
    {
        let mut e = err.into();
        e.file = PathBuf::from(file);
        e.line = line;
        e
    }
}

impl Terminate for RuntimeError {
    fn exitcode(&self) -> i32 {
        match self.kind {
            RuntimeErrorKind::Io(e) => match e {
                io::ErrorKind::NotFound => exitcode::NOINPUT,
                io::ErrorKind::AlreadyExists => exitcode::CANTCREAT,
                io::ErrorKind::PermissionDenied => exitcode::NOINPUT,
                _ => exitcode::IOERR,
            },
            RuntimeErrorKind::UnexpectedEof => exitcode::DATAERR,
            RuntimeErrorKind::BadToken(_) => exitcode::DATAERR,
            RuntimeErrorKind::Unsupported => exitcode::DATAERR,
            RuntimeErrorKind::InvalidFile => exitcode::DATAERR,
        }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.kind {
            RuntimeErrorKind::InvalidFile => write!(
                f,
                "{}, {} contains no valid data",
                self.kind.to_string(),
                self.file.display(),
            ),
            RuntimeErrorKind::Io(_) => write!(
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

impl std::fmt::Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::Io(kind) => write!(f, "IO error ({})", kind.to_string()),
            RuntimeErrorKind::BadToken(t) => write!(f, "Token \'{}\' is invalid", t),
            RuntimeErrorKind::UnexpectedEof => write!(f, "Unexpected EOF"),
            RuntimeErrorKind::Unsupported => write!(f, "Unsupported file"),
            RuntimeErrorKind::InvalidFile => write!(f, "Invalid log"),
        }
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(e: std::io::Error) -> Self {
        RuntimeError::new(RuntimeErrorKind::Io(e.kind()))
    }
}

impl From<serde_json::Error> for RuntimeError {
    fn from(e: serde_json::Error) -> Self {
        RuntimeError::new(RuntimeErrorKind::BadToken(e.to_string()))
    }
}

impl From<hex::FromHexError> for RuntimeError {
    fn from(e: hex::FromHexError) -> Self {
        RuntimeError::new(RuntimeErrorKind::BadToken(e.to_string()))
    }
}

impl From<chrono::ParseError> for RuntimeError {
    fn from(e: chrono::ParseError) -> Self {
        RuntimeError::new(RuntimeErrorKind::BadToken(e.to_string()))
    }
}

impl From<std::num::ParseIntError> for RuntimeError {
    fn from(e: std::num::ParseIntError) -> Self {
        RuntimeError::new(RuntimeErrorKind::BadToken(e.to_string()))
    }
}

impl From<image::ImageError> for RuntimeError {
    fn from(e: image::ImageError) -> Self {
        RuntimeError::new(match e {
            image::ImageError::IoError(e) => RuntimeErrorKind::Io(e.kind()),
            _ => RuntimeErrorKind::InvalidFile,
        })
    }
}
