use std::{path::{PathBuf, Path}, str::FromStr};

const SOURCE_ALIAS: [&str; 2] = ["pipe:0", "stdin"];
const DESTINATION_ALIAS: [&str; 2] = ["pipe:1", "stdout"];

#[derive(Default, Debug, Clone)]
pub enum Source {
    #[default]
    Stdin,
    File(PathBuf)
}

#[derive(Default, Debug, Clone)]
pub enum Destination {
    #[default]
    Stdout,
    File(PathBuf)
}

impl FromStr for Source {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if SOURCE_ALIAS.contains(&s) {
            Ok(Source::Stdin)
        } else {
            Ok(Source::File(PathBuf::from(s)))
        }
    }
}

impl<T: ?Sized + AsRef<str>> From<&T> for Source {
    fn from(value: &T) -> Self {
        Source::from_str(value.as_ref()).unwrap()
    }
}

impl FromStr for Destination {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if DESTINATION_ALIAS.contains(&s) {
            Ok(Destination::Stdout)
        } else {
            Ok(Destination::File(PathBuf::from(s)))
        }
    }
}

impl<T: ?Sized + AsRef<str>> From<&T> for Destination {
    fn from(value: &T) -> Self {
        Destination::from_str(value.as_ref()).unwrap()
    }
}

pub fn is_file(path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    let meta = std::fs::metadata(path)?;

    if !meta.is_file() {
        // TODO: change to std::io::ErrorKind::IsADirectory when stabilised
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "expected file",
        ))?
    }

    Ok(())
}

pub fn _is_dir(path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    let meta = std::fs::metadata(path)?;

    if !meta.is_dir() {
        // TODO: change to std::io::ErrorKind::IsADirectory when stabilised
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "expected directory",
        ))?
    }

    Ok(())
}

pub fn is_file_or_dir(path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    path.try_exists()?;
    Ok(())
}
