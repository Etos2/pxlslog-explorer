use std::fs::{self, OpenOptions};
use std::io::{self, prelude::*};

use crate::error::{PxlsError, PxlsResult};
use crate::pixel::{ActionKind, PxlsParser};
use crate::util::Region;
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgGroup, Args};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

// TODO: Custom handling of specific types (e.g. region)
#[derive(Args)]
#[clap(about = "Filter logs and outputs to new file", long_about = None)]
#[clap(group(ArgGroup::new("hashes").args(&["hash", "hash-src"])))]
#[clap(group(ArgGroup::new("overwrite").args(&["dst", "modify"])))]
pub struct FilterInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(
        help = "Filepath of input log file [Defaults to STDIN]",
        display_order = 0
    )]
    src: Option<String>,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(
        help = "Filepath of output log file [Defaults to STDOUT]",
        display_order = 1
    )]
    dst: Option<String>,
    #[clap(short, long)]
    #[clap(
        help = "If input log should be modified with output",
        display_order = 2
    )]
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
    #[clap(value_name("INT"))]
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
    #[clap(value_name("ENUM"))]
    #[clap(help = "Only include entries with this action", display_order = 9999)]
    action: Vec<ActionKind>,
}

impl FilterInput {
    pub fn run(&self, settings: &Cli) -> PxlsResult<()> {
        let hashes = self.get_hash(settings.verbose);
        let region = Region::new_from_slice(&self.region);

        let dst = if self.modify && self.src.is_some() {
            self.src.clone()
        } else {
            self.dst.clone()
        };

        let mut buffer = String::new();
        let mut tokens = match &self.src {
            Some(s) => {
                PxlsParser::parse_raw(&mut OpenOptions::new().read(true).open(s)?, &mut buffer)?
            }
            None => PxlsParser::parse_raw(&mut io::stdin().lock(), &mut buffer)?,
        };

        let chunk_size = tokens.len() / settings.threads.unwrap_or(1);
        let total = tokens.len() as i32 / 6;
        let passed_tokens = tokens
            .par_chunks_mut(chunk_size)
            .flat_map(|chunk| {
                chunk.par_chunks(6).filter_map(|tokens| {
                    match self.is_filtered(tokens, &hashes, &region) {
                        Ok(true) => Some(Ok(tokens)),
                        Ok(false) => None,
                        Err(e) => Some(Err(e)),
                    }
                })
            })
            .collect::<PxlsResult<Vec<_>>>()?;

        let out = passed_tokens
            .par_iter()
            .map(|tokens| tokens.join("\t"))
            .collect::<Vec<String>>()
            .join("\n");

        match &dst {
            Some(path) => {
                OpenOptions::new()
                    .create_new(settings.noclobber)
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?
                    .write_all(out.as_bytes())?;
            }
            None => {
                print!("{}", out);
            }
        };

        if settings.verbose {
            let passed = passed_tokens.len() as i32;
            println!("Returned {} of {} entries", passed, total);
        }

        Ok(())
    }

    // TODO: Improve how tokens are inputted
    fn is_filtered(
        &self,
        tokens: &[&str],
        hashes: &[String],
        region: &Option<Region<i32>>,
    ) -> PxlsResult<bool> {
        let mut out = true;

        if let Some(time) = self.after {
            out &= time <= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f")?;
        }
        if let Some(time) = self.before {
            out &= time >= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f")?;
        }
        if let Some(region) = region {
            let x = tokens[2].parse::<i32>()?;
            let y = tokens[3].parse::<i32>()?;
            out &= region.contains(x, y);
        }
        if self.color.len() > 0 {
            let mut temp = false;
            for color in &self.color {
                temp |= tokens[4].parse::<i32>()? == *color;
            }
            out &= temp;
        }
        if self.action.len() > 0 {
            let mut temp = false;
            for action in &self.action {
                temp |= match action {
                    ActionKind::Place => tokens[5] == "user place",
                    ActionKind::Undo => tokens[5] == "user undo",
                    ActionKind::Overwrite => tokens[5] == "mod overwrite",
                    ActionKind::Rollback => tokens[5] == "rollback",
                    ActionKind::RollbackUndo => tokens[5] == "rollback undo",
                    ActionKind::Nuke => tokens[5] == "console nuke",
                };
            }
            out &= temp;
        }
        // Skip if line didn't pass (Hashing is expen$ive)
        if out == true && hashes.len() > 0 {
            let mut temp = false;
            for hash in hashes {
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
        Ok(out)
    }

    fn get_hash(&self, verbosity: bool) -> Vec<String> {
        let mut hashes = self.hash.to_owned();
        if let Some(src) = &self.hash_src {
            let input = fs::read_to_string(src).map_err(|e| PxlsError::from(e, &src, 0));
            if let Ok(input) = input {
                let lines: Vec<&str> = input
                    .split_whitespace()
                    .filter(|&x| !x.is_empty())
                    .collect();

                for (i, line) in lines.iter().enumerate() {
                    if line.len() == 512 {
                        hashes.push(line.to_string());
                    } else if verbosity {
                        eprintln!("Invalid hash at line {}! Ignoring...", i);
                    }
                }
            }
        }

        hashes
    }
}
