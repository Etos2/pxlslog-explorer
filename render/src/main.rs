mod config;
mod error;
mod palette;
mod render;
mod util;

use std::{fs::File, io::BufReader};

use clap::Parser;
use common::parse::pxlslog::PxlsLogParser;
use common::parse::ActionsParser;
use config::{
    builder::BuilderOverride, source::cli::CliData, source::toml::read_toml, source::ConfigSource,
};
use error::RuntimeError;
use rayon::ThreadPoolBuilder;
use toml::Table;

use crate::render::RenderCommand;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliData::parse();
    let (config, render_configs) = if let Some(config_path) = &args.config {
        let toml = read_toml(config_path)?;
        let toml_config = Table::get_config(toml)?;
        let cli_config = CliData::get_config(args)?;
        cli_config.or(&toml_config).build()?
    } else {
        CliData::get_config(args)?.build()?
    };

    ThreadPoolBuilder::new()
        .num_threads(config.threads)
        .build_global()?;

    eprintln!("Parsing actions...");

    // TODO: Get flags from render styles
    let mut parser = PxlsLogParser;
    let actions = match &config.log_source {
        util::io::Source::Stdin => parser.parse(BufReader::new(std::io::stdin()))?,
        util::io::Source::File(path) => {
            let file = File::open(path).map_err(RuntimeError::from)?;
            parser.parse(BufReader::new(file))?
        }
    };

    let actions = match actions {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{:?}", e);
            Err(e)?
        }
    };

    eprintln!("Parsed actions");

    eprintln!("Time: {:?}", actions.time.len());
    eprintln!("User: {:?}", actions.user.as_ref().map(|v| v.len()));
    eprintln!("Pos:  {:?}", actions.coord.len());
    eprintln!("Index:{:?}", actions.index.as_ref().map(|v| v.len()));
    eprintln!("Kind: {:?}", actions.kind.as_ref().map(|v| v.len()));
    eprintln!("Bound:{:?}", actions.bounds);

    // let (actions, bounds) = match &config.log_source {
    //     util::io::Source::Stdin => get_actions(std::io::stdin())?,
    //     util::io::Source::File(path) => get_actions(File::open(path).map_err(RuntimeError::from)?)?,
    // };

    eprintln!("Rendering actions...");
    for render_config in render_configs {
        let command = RenderCommand::new(render_config, actions.bounds)?;
        command.run(actions.iter())?;
    }
    eprintln!("Rendered action");

    Ok(())
}
