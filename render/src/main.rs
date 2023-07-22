mod config;
mod error;
mod palette;
mod render;
mod util;

use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
};

use chrono::NaiveDateTime;
use clap::Parser;
use common::data::{
    action::Action,
    actions::{ActionsParseFlags, ActionsParser},
};
use config::{
    builder::BuilderOverride, source::cli::CliData, source::toml::read_toml, source::ConfigSource,
};
use error::{ActionError, ActionErrorKind, RuntimeError};
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
    let mut parser = ActionsParser::new(ActionsParseFlags::INDEX);
    match &config.log_source {
        util::io::Source::Stdin => parser.read(std::io::stdin())?,
        util::io::Source::File(path) => {
            parser.read(File::open(path).map_err(RuntimeError::from)?)?
        }
    }
    let actions = parser.build();

    eprintln!("Parsed actions");

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

// TODO: OsStr support?
fn get_actions(reader: impl Read) -> Result<(Vec<Action>, (u32, u32, u32, u32)), RuntimeError> {
    let mut reader = BufReader::with_capacity(64000000, reader);
    let mut buffer = String::new();
    let mut out = Vec::new();
    let mut prev_time = NaiveDateTime::MIN.timestamp_millis();
    let mut line = 0;
    let mut bounds = (u32::MAX, u32::MAX, u32::MIN, u32::MIN);

    while reader.read_line(&mut buffer)? != 0 {
        match Action::try_from(buffer.trim_end_matches(char::is_whitespace)) {
            Ok(action) => {
                if action.time < prev_time {
                    let err = RuntimeError::InvalidAction(ActionError {
                        line,
                        kind: ActionErrorKind::OutOfOrder {
                            time: action.time,
                            prev_time,
                        },
                    });
                    eprintln!("ignored {err}");
                }

                bounds.0 = bounds.0.min(action.x);
                bounds.1 = bounds.1.min(action.y);
                bounds.2 = bounds.2.max(action.x);
                bounds.3 = bounds.3.max(action.y);
                prev_time = action.time;
                out.push(action);
                buffer.clear();
            }
            Err(err) => eprintln!("{err}"),
        }

        line += 1;
    }

    bounds = (bounds.0, bounds.1, bounds.2 + 1, bounds.3 + 1);
    Ok((out, bounds))
}
