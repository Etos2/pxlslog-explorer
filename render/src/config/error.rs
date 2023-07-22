use std::{fmt::Display, path::PathBuf};

use thiserror::Error;

#[derive(Debug)]
pub enum ConfigValue {
    ConfigSource,
    ProgramLogSource,
    ProgramQuiet,
    ProgramThreads,
    ProgramDryRun,
    MethodPalette,
    MethodKind,
    CanvasSource,
    CanvasSize,
    CanvasBackgroundSource,
    CanvasTransparency,
    DestinationKind,
    DestinationFormat,
    Step,
}

#[derive(Debug)]
pub enum ConfigAlias {
    Screenshot,
}

impl ConfigValue {
    fn to_str(&self) -> &'static str {
        match self {
            ConfigValue::ConfigSource => "config source",
            ConfigValue::ProgramLogSource => "program actions",
            ConfigValue::ProgramQuiet => "program quiet",
            ConfigValue::ProgramThreads => "program threads",
            ConfigValue::ProgramDryRun => "program dry run",
            ConfigValue::MethodPalette => "method palette",
            ConfigValue::MethodKind => "method palette",
            ConfigValue::CanvasSource => "canvas source",
            ConfigValue::CanvasSize => "canvas source",
            ConfigValue::CanvasBackgroundSource => "canvas background",
            ConfigValue::CanvasTransparency => "canvas transparency",
            ConfigValue::DestinationKind => "destination kind",
            ConfigValue::DestinationFormat => "destination format",
            ConfigValue::Step => "step",
        }
    }

    fn stringify_vec(values: &Vec<ConfigValue>) -> String {
        let mut iter = values.iter().map(ConfigValue::to_str);
        let mut out = "\"".to_string();

        // SAFETY: Empty vec is a dev error
        out.push_str(iter.next().unwrap());
        out.push('\"');

        for str in iter {
            out.push_str(" \"");
            out.push_str(str);
            out.push('\"');
        }

        out
    }
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.to_str())
    }
}

impl ConfigAlias {
    fn to_str(&self) -> &'static str {
        match self {
            ConfigAlias::Screenshot => "screenshot",
        }
    }

    fn stringify_vec(values: &Vec<ConfigAlias>) -> String {
        let mut iter = values.iter().map(ConfigAlias::to_str);
        let mut out = "\"".to_string();

        // SAFETY: Empty vec is a dev error
        out.push_str(iter.next().unwrap());
        out.push('\"');

        for str in iter {
            out.push_str(" \"");
            out.push_str(str);
            out.push('\"');
        }

        out
    }
}

impl Display for ConfigAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

// #[derive(Error, Debug)]
// #[error(transparent)]
// pub struct ConfigError(#[from] ConfigErrorKind);

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("io error with {0}: {2} @ {1}")]
    Io(ConfigValue, PathBuf, std::io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    #[error("required value {} not provided", ConfigValue::stringify_vec(.0))]
    MissingValue(Vec<ConfigValue>),
    #[error("value for \"{0}\" is invalid")]
    InvalidValue(ConfigValue),
    // #[error("the path for \"{1}\" does not exist or is not a file ({0})")]
    // InvalidPath(ConfigValue, PathBuf, InvalidPathKind),
    #[error("\"{0}\" could not be infered with current values")]
    CannotInfer(ConfigValue),
    #[error("alias {0} overrides values that have already been declared {}", ConfigValue::stringify_vec(.1))]
    AliasConflict(ConfigAlias, Vec<ConfigValue>),
}

impl ConfigError {
    pub fn new_missing(values: Vec<ConfigValue>) -> ConfigError {
        ConfigError::MissingValue(values)
    }

    pub fn new_infer(value: ConfigValue) -> ConfigError {
        ConfigError::CannotInfer(value)
    }
}