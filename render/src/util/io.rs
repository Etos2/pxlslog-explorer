use std::{path::PathBuf, str::FromStr};

const SOURCE_ALIAS: [&str; 2] = ["pipe:0", "stdin"];
const DESTINATION_ALIAS: [&str; 2] = ["pipe:1", "stdout"];

#[derive(Debug, Clone)]
pub enum Source {
    Stdin,
    File(PathBuf)
}

#[derive(Debug, Clone)]
pub enum Destination {
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

impl Default for Source {
    fn default() -> Self {
        Source::Stdin
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

impl Default for Destination {
    fn default() -> Self {
        Destination::Stdout
    }
}