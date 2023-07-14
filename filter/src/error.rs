use log::SetLoggerError;
use nom_supreme::{error::ErrorTree, final_parser::Location};
use thiserror::Error;

pub type ProgramResult<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("logger error")]
    Log(#[from] SetLoggerError),
    #[error("invalid configuration ({0})")]
    Config(String),
    #[error("parser error")]
    Parse(#[from] ErrorTree<Location>),
}