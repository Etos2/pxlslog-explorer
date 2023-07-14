mod error;
mod filter;
mod interface;

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
};

use clap::Parser;

use common::data::action::Action;
use error::{Error, ProgramResult};
use filter::FilterPredicates;
use interface::ProgramArgs;
use log::{info, warn, SetLoggerError};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};

fn main() -> ProgramResult<()> {
    let args = ProgramArgs::parse();
    let settings = args
        .settings
        .ok_or(Error::Config("no filters specified".to_string()))?;

    if !args.quiet {
        config_logger(args.verbose)?;
    }

    let src_handle = get_reader(args.log.as_deref())?;
    let mut dst_handle = get_writer(args.output.as_deref())?;

    let filters = FilterPredicates::try_from(settings)?;
    let mut lines_read = 0;
    let mut lines_written = 0;
    let mut lines_removed = 0;
    let mut lines_errored = 0;

    for line in src_handle.lines() {
        let line = line?;
        match Action::try_from(line.as_str()) {
            Ok(action) => {
                if filters.eval(&action) {
                    let action_str = action.to_string() + "\n";
                    dst_handle.write_all(action_str.as_bytes())?;
                    lines_written += 1;
                } else {
                    lines_removed += 1;
                }
            }
            Err(e) => {
                warn!("{e} @ line {}", lines_read + 1);
                warn!("Str: {line:?}");
                lines_errored += 1;
            }
        }

        lines_read += 1;
    }

    info!("Read:    {lines_read}");
    info!("Wrote:   {lines_written}");
    info!("Removed: {lines_removed}");
    info!("Invalid: {lines_errored}");

    Ok(())
}

fn get_reader(path: Option<&Path>) -> ProgramResult<BufReader<Box<dyn Read>>> {
    Ok(BufReader::new(match path {
        Some(path) => {
            info!("Set source to: {}", path.display());
            Box::new(File::open(path)?) as Box<dyn Read>
        }
        None => {
            info!("Set source to: STDIN");
            Box::new(std::io::stdin()) as Box<dyn Read>
        }
    }))
}

fn get_writer(path: Option<&Path>) -> ProgramResult<BufWriter<Box<dyn Write>>> {
    Ok(BufWriter::new(match path {
        Some(path) => {
            info!("Set destination to: {}", path.display());
            Box::new(File::create(path)?) as Box<dyn Write>
        }
        None => {
            info!("Set destination to: STDOUT");
            Box::new(std::io::stdout()) as Box<dyn Write>
        }
    }))
}

fn config_logger(verbosity: u8) -> Result<(), SetLoggerError> {
    let filter = match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    let config = ConfigBuilder::default()
        .set_time_level(LevelFilter::Debug)
        .build();
    WriteLogger::init(filter, config, std::io::stderr())
}
