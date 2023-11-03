use std::path::Path;

use toml::{map::Map, Table, Value};

use crate::util::io;

use super::{
    super::{
        builder::ConfigBuilder,
        error::{ConfigError, ConfigValue},
    },
    ConfigSource,
};

pub fn read_toml(path: &Path) -> Result<Table, ConfigError> {
    // TODO: simplify when read_to_string(path) returns IsDirectory error kind when `io_error_more` is stabilised
    match io::is_file(path) {
        Ok(_) => {
            let raw_toml = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::Io(ConfigValue::ConfigSource, path.to_path_buf(), e))?;
            Ok(raw_toml.parse()?)
        }
        Err(e) => Err(ConfigError::Io(
            ConfigValue::ConfigSource,
            path.to_path_buf(),
            e,
        )),
    }
}

impl ConfigSource for Map<String, Value> {
    fn get_config(_source: Self) -> Result<ConfigBuilder, ConfigError> {
        todo!()
    }
}
