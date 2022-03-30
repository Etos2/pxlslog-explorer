mod cli;

use std::fs::{self, File};
use std::io::{prelude::*, BufReader};
use std::path::Path;

use chrono::NaiveDateTime;
use clap::Parser;
use sha2::{Digest, Sha256};

use cli::{Action, Cli, Command, FilterState};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Filter(filter_state) => {
            // Only filter if required
            if !is_empty(&filter_state) {
                let input = BufReader::new(File::open(&filter_state.input).unwrap());
                match &filter_state.output {
                    Some(path) => {
                        if cli.noclobber && Path::new(path).exists() {
                           return;
                        }

                        let mut output = File::create(path).unwrap();
                        for line in input.lines() {
                            if is_accepted(&filter_state, &line.as_ref().unwrap()) {
                                output.write(line.unwrap().as_ref()).unwrap();
                            }
                        }
                    }

                    None => {
                        for line in input.lines() {
                            if is_accepted(&filter_state, &line.as_ref().unwrap()) {
                                println!("{}", line.unwrap());
                            }
                        }
                    }
                };
            // No filter, thus simplify
            } else {
                let input = fs::read_to_string(&filter_state.input).unwrap();
                match filter_state.output {
                    Some(path) => {
                        File::create(path)
                            .unwrap()
                            .write_all(input.as_ref())
                            .unwrap();
                    }
                    None => {
                        print!("{}", input);
                    }
                };
            }
        }
        // Todo!
        Command::Render(_render) => {
            unimplemented!("soon:tm:")
        }
    }
}

fn is_accepted(filter: &FilterState, line: &str) -> bool {
    let tokens: Vec<&str> = line.split_terminator(&['\t', '\n'][..]).collect();
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
        let digest_format = format!(
            "{},{},{},{},{}",
            tokens[0], tokens[2], tokens[3], tokens[4], user
        );
        let digest = hex::encode(Sha256::digest(&digest_format));
        out &= &digest[..] == tokens[1];
    }
    out
}

fn is_empty(filter: &FilterState) -> bool {
    filter.after.is_none()
        && filter.before.is_none()
        && filter.color.is_none()
        && filter.region.is_none()
        && filter.user.is_none()
        && filter.action.is_none()
}
