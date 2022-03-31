mod cli;

use std::fs::{self, OpenOptions};
use std::io::prelude::*;

use chrono::NaiveDateTime;
use clap::Parser;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use cli::{Action, Cli, Command, FilterState};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Filter(filter_state) => {
            let input = fs::read_to_string(&filter_state.input).unwrap();

            let log = match has_filter(&filter_state) {
                true => {
                    let mut lines: Vec<&str> = input.split_terminator("\n").collect();
                    let chunk_size = lines.len() / num_cpus::get();
                    lines
                        .par_chunks_mut(chunk_size)
                        .flat_map_iter(|chunk| {
                            chunk
                                .iter()
                                .filter(|line| is_filtered(&filter_state, line))
                                .copied()
                        })
                        .collect::<Vec<&str>>()
                        .join("\n")
                }
                // No filter, thus simplified output
                false => input,
            };
            match filter_state.output {
                Some(path) => {
                    let mut output = OpenOptions::new()
                        .create_new(cli.noclobber)
                        .write(true)
                        .open(path)
                        .unwrap();
                    output.write_all(log.as_bytes()).unwrap();
                }
                None => {
                    print!("{}", log);
                }
            };
        }
        // Todo!
        Command::Render(_render) => {
            unimplemented!("soon:tm:")
        }
    }
}

fn is_filtered(filter: &FilterState, line: &str) -> bool {
    let tokens: Vec<&str> = line.split_terminator('\t').collect();
    let mut out = true;

    if let Some(time) = filter.after {
        out &= time <= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f").unwrap();
    }
    if let Some(time) = filter.before {
        out &= time >= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f").unwrap();
    }
    if let Some(region) = filter.region {
        let x = tokens[2].parse::<i32>().unwrap();
        let y = tokens[3].parse::<i32>().unwrap();
        out &= x >= region.x1 && y >= region.y1 && x <= region.x2 && y <= region.y2;
    }
    if let Some(color) = &filter.color {
        out &= tokens[4].parse::<i32>().unwrap() == *color;
    }
    if let Some(action) = &filter.action {
        out &= match action {
            Action::Place => tokens[5] == "user place",
            Action::Undo => tokens[5] == "user undo",
            Action::Overwrite => tokens[5] == "mod overwrite",
            Action::Rollback => tokens[5] == "rollback",
            Action::RollbackUndo => tokens[5] == "rollback undo",
            Action::Nuke => tokens[5] == "console nuke",
        }
    }
    if let Some(user) = &filter.user {
        let mut hasher = Sha256::new();
        hasher.update(tokens[0].as_bytes());
        hasher.update(",");
        hasher.update(tokens[2].as_bytes());
        hasher.update(",");
        hasher.update(tokens[3].as_bytes());
        hasher.update(",");
        hasher.update(tokens[4].as_bytes());
        hasher.update(",");
        hasher.update(user.as_bytes());

        let digest = hex::encode(hasher.finalize());
        out &= &digest[..] == tokens[1];
    }
    out
}

fn has_filter(filter: &FilterState) -> bool {
    filter.after.is_some()
        || filter.before.is_some()
        || filter.color.is_some()
        || filter.region.is_some()
        || filter.user.is_some()
        || filter.action.is_some()
}
