use std::fs::{self, OpenOptions};
use std::io::prelude::*;

use crate::error::{PxlsError, PxlsErrorKind, PxlsResult};
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
#[clap(group(ArgGroup::new("username-hash-conflict").args(&["hashes", "username"])))]
pub struct FilterInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file", display_order = 0)]
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
    #[clap(value_name("INT"))]
    #[clap(help = "Only include entries with provided colors")]
    color: Vec<i32>,
    #[clap(long, parse(try_from_str))]
    #[clap(max_values(4))]
    #[clap(value_name("INT"))]
    #[clap(help = "Only include entries within a region [\"x1 y1 x2 y2\"]")]
    region: Vec<i32>,
    #[clap(long)]
    #[clap(multiple_values(true))]
    #[clap(value_name("STRING"))]
    #[clap(help = "Only include entries that belong to this username")]
    username: Vec<String>,
    #[clap(long)]
    #[clap(multiple_values(true))]
    #[clap(value_name("STRING"))]
    #[clap(help = "Only include entries that belong to this hash")]
    hash: Option<Vec<String>>,
    #[clap(long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Only include entries that belong to hashes from a file")]
    hash_src: Option<String>,
    #[clap(long, arg_enum)]
    #[clap(multiple_values(true))]
    #[clap(value_name("ENUM"))]
    #[clap(help = "Only include entries with this action", display_order = 9999)]
    action: Vec<ActionKind>,
}

pub struct FilterData {
    src: Option<String>,
    dst: Option<String>,
    users: Identifier,
    region: Option<Region<i32>>,
    after: Option<NaiveDateTime>,
    before: Option<NaiveDateTime>,
    color: Vec<i32>,
    action: Vec<ActionKind>,
}

enum Identifier {
    Hash(Vec<String>),
    Username(Vec<String>),
    None,
}

impl FilterInput {
    pub fn validate(&self) -> PxlsResult<FilterData> {
        let dst = if self.modify && self.src.is_some() {
            self.src.clone()
        } else {
            self.dst.clone()
        };

        let users = if self.username.len() > 0 {
            Identifier::Username(self.username.clone())
        } else if let Some(hash) = &self.hash {
            Identifier::Hash(hash.to_owned())
        } else if let Some(src) = &self.hash_src {
            Identifier::Hash(self.get_hashes(&src)?)
        } else {
            Identifier::None
        };

        Ok(FilterData {
            src: self.src.clone(),
            dst,
            users,
            region: Region::from_slice(&self.region),
            after: self.after,
            before: self.before,
            color: self.color.clone(),
            action: self.action.clone(),
        })
    }

    fn get_hashes(&self, src: &str) -> PxlsResult<Vec<String>> {
        let mut hashes = Vec::new();
        let input = fs::read_to_string(src).map_err(|e| PxlsError::from(e, &src, 0))?;

        for (i, line) in input.lines().enumerate() {
            match Self::verify_hash(line) {
                Some(hash) => hashes.push(hash.to_string()),
                None => {
                    return Err(PxlsError::new_with_line(
                        PxlsErrorKind::BadToken("Invalid hash".to_string()),
                        src,
                        i,
                    ))
                }
            }
        }

        Ok(hashes)
    }

    fn verify_hash(hash: &str) -> Option<&str> {
        if hash.len() == 512 {
            None
        } else {
            Some(hash)
        }
    }
}

impl FilterData {
    pub fn run(&self, settings: &Cli) -> PxlsResult<()> {
        let mut buffer = String::new();
        let mut tokens = match &self.src {
            Some(s) => {
                let mut file = OpenOptions::new().read(true).open(s)?;
                PxlsParser::parse_raw(&mut file, &mut buffer)?
            }
            None => {
                let mut stdin = std::io::stdin().lock();
                PxlsParser::parse_raw(&mut stdin, &mut buffer)?
            }
        };

        let chunk_size = tokens.len() / settings.threads.unwrap_or(1);
        let total = tokens.len() as i32 / 6;
        let passed_tokens = tokens
            .par_chunks_mut(chunk_size)
            .flat_map(|chunk| {
                chunk
                    .par_chunks(6)
                    .filter_map(|tokens| match self.is_filtered(tokens) {
                        Ok(true) => Some(Ok(tokens)),
                        Ok(false) => None,
                        Err(e) => Some(Err(e)),
                    })
            })
            .collect::<PxlsResult<Vec<_>>>()?;

        let out = passed_tokens
            .par_iter()
            .map(|tokens| tokens.join("\t"))
            .collect::<Vec<String>>()
            .join("\n");

        match &self.dst {
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
    // TODO: Split into individual functions
    fn is_filtered(&self, tokens: &[&str]) -> PxlsResult<bool> {
        let mut out = true;

        if let Some(time) = self.after {
            out &= time <= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f")?;
        }
        if let Some(time) = self.before {
            out &= time >= NaiveDateTime::parse_from_str(tokens[0], "%Y-%m-%d %H:%M:%S,%3f")?;
        }
        if let Some(region) = self.region {
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
        if out == true {
            match &self.users {
                Identifier::Hash(hashes) => {
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
                Identifier::Username(_) => {
                    unimplemented!()
                }
                Identifier::None => (),
            }
        }
        Ok(out)
    }
}
