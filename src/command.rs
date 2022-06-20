use crate::Cli;
use crate::error::PxlsResult;

pub trait PxlsCommand
{
    fn run(&self, settings: &Cli) -> PxlsResult<()>;
}