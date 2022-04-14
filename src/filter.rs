use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, prelude::*};

use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgEnum, ArgGroup, Args};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

#[derive(Args)]
#[clap(about = "Filter logs and outputs to new file", long_about = None)]
#[clap(group(ArgGroup::new("hashes").args(&["hash", "hash-src"])))]
#[clap(group(ArgGroup::new("overwrite").args(&["dst", "modify"])))]
pub struct FilterInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file [Defaults to STDIN]", display_order = 0)]
    src: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(
        help = "Filepath of output log file [Defaults to STDOUT]",
        display_order = 1
    )]
    dst: Option<String>,
    #[clap(short, long)]
    #[clap(help = "If input log should be modified with output", display_order = 2)]
    modify: bool,
    #[clap(long, parse(try_from_str))]
    #[clap(value_name("TIMESTAMP"))]
    #[clap(help = "Only include entries after this date [%Y-%m-%dT%H:%M:%S%.f]")]
    after: Option<NaiveDateTime>,
    #[clap(long, parse(try_from_str))]
    #[clap(value_name("TIMESTAMP"))]
    #[clap(help = "Only include entries before this date [%Y-%m-%dT%H:%M:%S%.f]")]
    before: Option<NaiveDateTime>,
    #[clap(long)]
    #[clap(multiple_values(true))]
    #[clap(value_name("INDEX"))]
    #[clap(help = "Only include entries with provided colors")]
    color: Vec<i32>,
    #[clap(long, parse(try_from_str))]
    #[clap(max_values(4))]
    #[clap(help = "Only include entries within a region [\"x1 y1 x2 y2\"]")]
    region: Vec<i32>,
    #[clap(long)]
    #[clap(multiple_values(true))]
    #[clap(value_name("HASH"))]
    #[clap(help = "Only include entries that belong to this hash")]
    hash: Vec<String>,
    #[clap(long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Only include entries that belong to these hashes")]
    hash_src: Option<String>,
    #[clap(long, arg_enum)]
    #[clap(multiple_values(true))]
    #[clap(help = "Only include entries with this action", display_order = 9999)]
    action: Vec<Action>,
}

pub struct Filter {
    pub src: Option<String>,
    pub dst: Option<String>,
    pub after: Option<NaiveDateTime>,
    pub before: Option<NaiveDateTime>,
    pub colors: Vec<i32>,
    pub region: Option<[i32; 4]>,
    pub hashes: Vec<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Copy, Clone, ArgEnum)]
pub enum Action {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}

impl FilterInput {
    // TODO: Actually verify some inputs + custom validation errors?
    pub fn validate(&self) -> Result<Filter, std::io::Error> {
        let mut hash = self.hash.to_owned();
        if let Some(src) = &self.hash_src {
            let input = fs::read_to_string(src)?;
            let lines: Vec<&str> = input
                .split_whitespace()
                .filter(|&x| !x.is_empty())
                .collect();
            // TODO: Warn when hash ignored
            for line in lines {
                if line.len() == 512 {
                    hash.push(line.to_string());
                }
            }
        }

        let dst = if self.modify && self.src.is_some() {
            self.src.clone()
        } else {
            self.dst.clone()
        };

        let region = match self.region.len() {
            0 => None,
            1 => Some([self.region[0], 0, i32::MAX, i32::MAX]),
            2 => Some([self.region[0], self.region[1], i32::MAX, i32::MAX]),
            3 => Some([self.region[0], self.region[1], self.region[2], i32::MAX]),
            4 => Some([
                self.region[0],
                self.region[1],
                self.region[2],
                self.region[3],
            ]),
            _ => unreachable!(),
        };
        if let Some(mut region) = region {
            if region[0] > region[2] {
                region.swap(0, 2);
            }
            if region[1] > region[3] {
                region.swap(1, 3);
            }
        }

        Ok(Filter {
            src: self.src.to_owned(),
            dst: dst,
            after: self.after,
            before: self.before,
            colors: self.color.to_owned(),
            region: region,
            hashes: hash,
            actions: self.action.to_owned(),
        })
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.has_filter() {
            write!(f, "Performing FILTER command with following arguments:")?;
        } else {
            write!(
                f,
                "Performing FILTER command with NO filters (Why would you do this?):"
            )?;
        }

        if let Some(src) = &self.src {
            write!(f, "\n  --src:    {}", src)?;
        } else {
            write!(f, "\n  --src:    STDIN")?;
        }
        if let Some(dst) = &self.dst {
            write!(f, "\n  --dst:    {}", dst)?;
        } else {
            write!(f, "\n  --dst:    STDOUT")?;
        }

        if let Some(time) = &self.after {
            write!(f, "\n  --after:  {}", time)?;
        }
        if let Some(time) = &self.before {
            write!(f, "\n  --before: {}", time)?;
        }
        if self.colors.len() > 0 {
            write!(f, "\n  --color:  {:?}", &self.colors)?;
        }
        if let Some(region) = &self.region {
            write!(f, "\n  --region: {:?}", region)?;
        }
        if self.actions.len() > 0 {
            write!(f, "\n  --action: {:?}", &self.actions)?;
        }
        if self.hashes.len() > 0 {
            write!(f, "\n  --user:   {:.32}...", self.hashes[0])?;
            for hash in &self.hashes[1..] {
                write!(f, "\n            {:.32}...", hash)?;
            }
        }
        Ok(())
    }
}

impl Filter {
    pub fn execute(self, settings: &Cli) -> Result<(i32, i32), std::io::Error> {
        let mut passed = 0;
        let mut total = 0;
        let input = match &self.src {
            Some(s) => fs::read_to_string(s)?,
            None => {
                let mut buf = String::new();
                io::stdin().lock().read_to_string(&mut buf)?;
                buf
            },
        };

        let output = match self.has_filter() {
            true => {
                let mut lines: Vec<&str> = input.split_terminator(&['\n', '\r'][..]).collect();
                println!("{}", lines[0]);
                let chunk_size = lines.len() / settings.threads.unwrap_or(1);
                let passed_lines = lines
                    .par_chunks_mut(chunk_size)
                    .flat_map_iter(|chunk| {
                        chunk.iter().filter(|line| self.is_filtered(line)).copied()
                    })
                    .collect::<Vec<&str>>();

                total = lines.len() as i32;
                passed = passed_lines.len() as i32;

                passed_lines.join("\n")
            }
            // No filter, thus simplified output
            // TODO: Determine if program should exit when no filters specified, because this is a glorified 'cp'/'echo' function
            false => input,
        };
        match &self.dst {
            Some(path) => {
                OpenOptions::new()
                    .create_new(settings.noclobber)
                    .create(true)
                    .write(true)
                    .open(path)?
                    .write_all(output.as_bytes())?;
            }
            None => {
                print!("{}", output);
            }
        };
        Ok((passed, total))
    }

    fn is_filtered(&self, line: &str) -> bool {
        let tokens: Vec<&str> = line.split_terminator('\t').collect();
        let mut out = true;
        if let Some(time) = self.after {
            out &=
                time <= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f").unwrap();
        }
        if let Some(time) = self.before {
            out &=
                time >= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f").unwrap();
        }
        if let Some(region) = self.region {
            let x = tokens[2].parse::<i32>().unwrap();
            let y = tokens[3].parse::<i32>().unwrap();
            out &= x >= region[0] && y >= region[1] && x <= region[2] && y <= region[3];
        }
        if self.colors.len() > 0 {
            let mut temp = false;
            for color in &self.colors {
                temp |= tokens[4].parse::<i32>().unwrap() == *color;
            }
            out &= temp;
        }
        if self.actions.len() > 0 {
            let mut temp = false;
            for action in &self.actions {
                temp |= match action {
                    Action::Place => tokens[5] == "user place",
                    Action::Undo => tokens[5] == "user undo",
                    Action::Overwrite => tokens[5] == "mod overwrite",
                    Action::Rollback => tokens[5] == "rollback",
                    Action::RollbackUndo => tokens[5] == "rollback undo",
                    Action::Nuke => tokens[5] == "console nuke",
                };
            }
            out &= temp;
        }
        // Skip if line didn't pass (Hashing is expen$ive)
        if out == true && self.hashes.len() > 0 {
            let mut temp = false;
            for hash in &self.hashes {
                let mut hasher = Sha256::new();
                hasher.update(tokens[0].as_bytes());
                hasher.update(",");
                hasher.update(tokens[2].as_bytes());
                hasher.update(",");
                hasher.update(tokens[3].as_bytes());
                hasher.update(",");
                hasher.update(tokens[4].as_bytes());
                hasher.update(",");
                hasher.update(hash.as_bytes());
                let digest = hex::encode(hasher.finalize());
                temp |= &digest[..] == tokens[1];
            }
            out &= temp;
        }
        out
    }

    fn has_filter(&self) -> bool {
        self.after.is_some()
            || self.before.is_some()
            || !self.colors.is_empty()
            || self.region.is_some()
            || !self.hashes.is_empty()
            || !self.actions.is_empty()
    }
}
