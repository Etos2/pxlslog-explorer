use chrono::NaiveDateTime;
use clap::{ArgEnum, Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(arg_required_else_help(true))]
#[clap(name = "PxlsLog-Explorer")]
#[clap(author = " - Etos2 <your@mother.com>")]
#[clap(version = "1.0")]
#[clap(about = "Filter pxls.space logs and generate timelapses.\nA simple program for pxls.space users to explore or adapt for their own uses.\n", long_about = None)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    //#[clap(short, long)]
    //#[clap(help = "Enable verbosity")]
    //pub verbose: bool,
    #[clap(short, long)]
    #[clap(help = "Prevent files from being overwritten")]
    pub noclobber: bool,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Filter(FilterState),
    Render(RenderState),
}

#[derive(Args)]
#[clap(about = "Filter logs and output", long_about = None)]
pub struct FilterState {
    #[clap(help = "Filepath of input log file")]
    pub input: String,
    #[clap(help = "Filepath of output log file [Defaults to STDOUT]")]
    pub output: Option<String>,
    #[clap(long, parse(try_from_str))]
    #[clap(help = "Only include entries after this date [%Y-%m-%dT%H:%M:%S%.f]")]
    pub after: Option<NaiveDateTime>,
    #[clap(long, parse(try_from_str))]
    #[clap(help = "Only include entries before this date [%Y-%m-%dT%H:%M:%S%.f]")]
    pub before: Option<NaiveDateTime>,
    #[clap(long, parse(try_from_str))]
    #[clap(help = "Only include entries with this color")]
    pub color: Option<i32>,
    #[clap(long, parse(try_from_str))]
    #[clap(help = "Only include entries within a region [\"x1,y1,x2,y2\"]")]
    pub region: Option<Region>,
    #[clap(long)]
    #[clap(help = "*Only include entries with the respective hash")]
    pub user: Option<String>,
    #[clap(long, arg_enum)]
    #[clap(help = "Only include entries with this action", display_order = 9999)]
    pub action: Option<Action>,
}

// TODO
#[derive(Args)]
#[clap(about = "[TODO] Render timelapses and other imagery", long_about = None)]
pub struct RenderState {}

#[derive(Debug, Copy, Clone)]
pub struct Region {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl std::str::FromStr for Region {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s.split_terminator(&[',', '.'][..]).collect();

        if coords.len() != 4 {
            panic!("Not enough tokens ({} of 4 provided)", coords.len());
        }

        let mut x1 = coords[0].parse::<i32>()?;
        let mut y1 = coords[1].parse::<i32>()?;
        let mut x2 = coords[2].parse::<i32>()?;
        let mut y2 = coords[3].parse::<i32>()?;

        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }

        Ok(Region { x1, y1, x2, y2 })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum Action {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}