use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::action::{ActionKind, ActionRef};
use crate::error::{ConfigError, ConfigResult, ParseError, ParseErrorKind, ParseResult};
use crate::util::Region;
use crate::Cli;

use chrono::NaiveDateTime;
use clap::{ArgGroup, Args};
use rayon::iter::ParallelIterator;
use rayon::str::ParallelString;
use sha2::{Digest, Sha256};

// TODO: Custom handling of specific types (e.g. region)
#[derive(Args)]
#[clap(about = "Filter logs and outputs to new file", long_about = None)]
#[clap(group(ArgGroup::new("user-conflict").args(&["hash", "hash-src", "username"])))]
#[clap(group(ArgGroup::new("overwrite").args(&["dst", "modify"])))]
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
    color: Vec<usize>,
    #[clap(long, parse(try_from_str))]
    #[clap(max_values(4))]
    #[clap(value_name("INT"))]
    #[clap(help = "Only include entries within a region [\"x1 y1 x2 y2\"]")]
    region: Vec<u32>,
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
    region: Option<Region<u32>>,
    after: Option<NaiveDateTime>,
    before: Option<NaiveDateTime>,
    color: Vec<usize>,
    kind: Vec<ActionKind>,
}

enum Identifier {
    Hash(Vec<String>),
    Username(Vec<String>),
    None,
}

impl FilterInput {
    pub fn validate(&self) -> ConfigResult<FilterData> {
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
            Identifier::Hash(
                self.get_hashes(&src)
                    .map_err(|e| ConfigError::new("hash_src", &e.to_string()))?,
            )
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
            kind: self.action.clone(),
        })
    }

    fn get_hashes(&self, src: &str) -> ParseResult<Vec<String>> {
        let mut hashes = Vec::new();
        let input = fs::read_to_string(src).map_err(|e| ParseError::from_err(e, &src, 0))?;

        for (i, line) in input.lines().enumerate() {
            match Self::verify_hash(line) {
                Some(hash) => hashes.push(hash.to_string()),
                None => Err(ParseError::new_with_file(
                    ParseErrorKind::BadToken(line.to_owned()),
                    src,
                    i,
                ))?,
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
    pub fn run(&self, settings: &Cli) -> ParseResult<()> {
        // TODO: No atomics?
        let passed = AtomicI32::new(0);
        let total = AtomicI32::new(0);

        let mut data = String::new();
        match &self.src {
            Some(path) => File::open(path)?.read_to_string(&mut data)?,
            None => std::io::stdin().lock().read_to_string(&mut data)?,
        };

        let filename = match &self.src {
            Some(path) => Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            None => "STDIN".to_string(),
        };

        let out: String = data
            .as_parallel_string()
            .par_lines()
            .inspect(|_| {
                total.fetch_add(1, Ordering::SeqCst);
            })
            .filter_map(|s| match ActionRef::try_from(s) {
                Ok(a) => {
                    if self.is_filtered(&a) {
                        Some(a.to_string() + "\n")
                    } else {
                        None
                    }
                }
                Err(e) => {
                    if settings.verbose {
                        eprintln!("{}", ParseError::from_err(e, &filename, 0));
                    }
                    None
                } // TODO
            })
            .inspect(|_| {
                passed.fetch_add(1, Ordering::SeqCst);
            })
            .collect();

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
                //print!("{}", out);
            }
        };

        if settings.verbose {
            println!(
                "Returned {} of {} entries",
                passed.load(Ordering::Acquire),
                total.load(Ordering::Acquire)
            );
        }

        Ok(())
    }

    // TODO: Improve how tokens are inputted
    // TODO: Split into individual functions
    fn is_filtered(&self, action: &ActionRef) -> bool {
        let mut out = true;

        if let Some(time) = self.after {
            out &= time <= action.time;
        }
        if let Some(time) = self.before {
            out &= time >= action.time;
        }
        if let Some(region) = self.region {
            out &= region.contains(action.x, action.y);
        }
        if self.color.len() > 0 {
            let mut temp = false;
            for color in &self.color {
                temp |= *color == action.index;
            }
            out &= temp;
        }
        if self.kind.len() > 0 {
            let mut temp = false;
            for kind in &self.kind {
                temp |= *kind == action.kind;
            }
            out &= temp;
        }
        // Skip if line didn't pass (Hashing is expen$ive)
        if out == true {
            match &self.users {
                Identifier::Hash(hashes) => {
                    let mut temp = false;
                    let time = action.time.format("%Y-%m-%d %H:%M:%S,%3f").to_string();
                    for hash in hashes {
                        let mut hasher = Sha256::new();
                        hasher.update(time.as_bytes());
                        hasher.update(",");
                        hasher.update(action.x.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(action.y.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(action.index.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(hash.as_bytes());
                        let digest = hex::encode(hasher.finalize());
                        temp |= &digest[..] == hash;
                    }
                    out &= temp;
                }
                Identifier::Username(_) => {
                    todo!()
                }
                Identifier::None => (),
            }
        }
        out
    }
}
