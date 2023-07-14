use std::path::PathBuf;

use chrono::NaiveDateTime;
use clap::builder::PossibleValue;
use clap::{Args, Parser, ValueEnum};
use common::action::ActionKind;
use common::util::region::Region;

// TODO: Custom handling of specific types (e.g. region)
// TODO: Negating filters (e.g. --action !placed)
#[derive(Parser, Debug, Clone)]
#[clap(about = "Filter logs and outputs to new file", long_about = None)]
#[clap(arg_required_else_help(true))]
pub struct ProgramArgs {
    #[arg(long, short, value_name("PATH"))]
    #[arg(help = "Source log file")]
    pub input: Option<PathBuf>,
    #[arg(long, short, value_name("PATH"))]
    #[arg(help = "Destination log file")]
    pub output: Option<PathBuf>,
    // #[arg(long, value_name("PATH"))]
    // #[arg(help = "Source command file")]
    // pub command_src: Option<PathBuf>,
    #[arg(short, long)]
    #[arg(help = "Silence all output")]
    pub quiet: bool,
    #[arg(short, long, action = clap::ArgAction::Count)]
    #[arg(help = "Enable verbosity")]
    pub verbose: u8,
    #[command(flatten)]
    pub settings: Option<FilterArgs>,
}

#[derive(Args, Debug, Clone)]
pub struct FilterArgs {
    #[arg(long, value_name("TIMESTAMP"))]
    #[arg(value_parser = clap::value_parser!(NaiveDateTime))]
    #[arg(help = "Only include entries after this date [%Y-%m-%dT%H:%M:%S%.f]")]
    pub after: Option<NaiveDateTime>,
    #[arg(long, value_name("TIMESTAMP"))]
    #[arg(value_parser = clap::value_parser!(NaiveDateTime))]
    #[arg(help = "Only include entries before this date [%Y-%m-%dT%H:%M:%S%.f]")]
    pub before: Option<NaiveDateTime>,
    #[arg(long("color"), value_name("INT"))]
    #[arg(help = "Only include entries with provided colors")]
    pub colors: Vec<usize>,
    #[arg(long("region"), value_name("INT"), num_args(4))]
    #[arg(help = "Region to save")]
    #[arg(value_parser = into_region)]
    #[arg(help = "Only include entries within a region [\"x1 y1 x2 y2\"]")]
    pub regions: Vec<Region<u32>>,
    #[arg(long("user"), value_name("STRING"))]
    #[arg(value_parser = into_identifier)]
    #[arg(help = "Only include entries that belong to this hash")]
    pub users: Vec<UserIdentifier>,
    #[arg(long("action"), value_name("ENUM"), value_enum)]
    #[arg(help = "Only include entries with this action", display_order = 9999)]
    pub action_kinds: Vec<ArgActionKind>,
}

#[derive(Clone, Debug)]
pub enum UserIdentifier {
    Username(String),
    Key(String),
}

#[derive(Copy, Clone, Debug)]
pub struct ArgActionKind(pub ActionKind);

impl ValueEnum for ArgActionKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ArgActionKind(ActionKind::Place),
            ArgActionKind(ActionKind::Undo),
            ArgActionKind(ActionKind::Overwrite),
            ArgActionKind(ActionKind::Rollback),
            ArgActionKind(ActionKind::Rollback),
            ArgActionKind(ActionKind::Nuke),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self.0 {
            ActionKind::Place => PossibleValue::new("place"),
            ActionKind::Undo => PossibleValue::new("undo"),
            ActionKind::Overwrite => PossibleValue::new("overwrite"),
            ActionKind::Rollback => PossibleValue::new("rollback"),
            ActionKind::RollbackUndo => PossibleValue::new("rollbackundo"),
            ActionKind::Nuke => PossibleValue::new("nuke"),
        })
    }
}

// TODO (Etos2): PixelIdentifier and UserIdentifier in common lib
//               UserIdentifier::try_from(input).map_err(|e| e.to_string())
fn into_identifier(input: &str) -> Result<UserIdentifier, String> {
    if input.len() == 512 {
        Ok(UserIdentifier::Key(input.to_owned()))
    } else if input.chars().count() < 32 {
        Ok(UserIdentifier::Username(input.to_owned()))
    } else {
        Err(format!("invalid length {}", input.chars().count()))
    }
}

// TODO (Etos2): PixelIdentifier and UserIdentifier in common lib
//               UserIdentifier::try_from(input).map_err(|e| e.to_string())
fn into_region(input: &str) -> Result<Region<u32>, String> {
    let tokens_res: Result<Vec<_>, _> = input.split(',').map(str::parse).collect();
    match tokens_res {
        Ok(tokens) => {
            if tokens.len() > 4 {
                Err(format!("found {} expected 1 to 4", tokens.len()))
            } else if tokens.is_empty() {
                Err("no values found".to_string())
            } else {
                // SAFETY: len is 1 >= n >= 4
                Ok(Region::from_slice(&tokens).unwrap())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
