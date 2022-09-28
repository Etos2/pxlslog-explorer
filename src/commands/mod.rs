pub mod filter;
pub mod render;
pub mod stats;

use crate::{
    error::{ConfigResult, RuntimeResult},
    Cli,
};

pub trait CommandInput<T>
where
    T: Command,
{
    fn validate(&self) -> ConfigResult<T>;
}

pub trait Command {
    fn run(&self, settings: &Cli) -> RuntimeResult<()>;
}