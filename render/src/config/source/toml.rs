use std::path::Path;

use toml::{map::Map, Table, Value};

use super::{super::{
    builder::ConfigBuilder,
    error::{ConfigError, ConfigValue, InvalidPathKind},
}, ConfigSource};

pub fn read_toml(path: &Path) -> Result<Table, ConfigError> {
    if !path.exists() {
        Err(ConfigError::new_invalid_path(
            ConfigValue::ConfigSource,
            path.to_path_buf(),
            InvalidPathKind::NotFound,
        ))
    } else if path.is_dir() {
        Err(ConfigError::new_invalid_path(
            ConfigValue::ConfigSource,
            path.to_path_buf(),
            InvalidPathKind::NotFile,
        ))
    } else {
        let raw_toml = std::fs::read_to_string(path)?;
        Ok(raw_toml.parse()?)
    }
}

impl ConfigSource for Map<String, Value> {
    fn get_config(source: Self) -> Result<ConfigBuilder, ConfigError> {
        todo!()
    }
}
