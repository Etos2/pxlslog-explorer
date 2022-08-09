pub mod filter;
pub mod render;

use crate::{
    error::{ConfigResult, ParseResult},
    Cli,
};

pub trait CommandInput<T>
where
    T: Command,
{
    fn validate(&self) -> ConfigResult<T>;
}

pub trait Command {
    fn run(&self, settings: &Cli) -> ParseResult<()>;
}