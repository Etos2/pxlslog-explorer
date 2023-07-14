use super::{builder::ConfigBuilder, error::ConfigError};

pub mod cli;
pub mod toml;

pub trait ConfigSource {
    fn get_config(source: Self) -> Result<ConfigBuilder, ConfigError>;
}
