mod action;
mod commands;
mod error;
mod palette;
mod util;

use commands::filter::FilterInput;
use commands::render::RenderInput;
use commands::stats::StatisticInput;
use commands::{Command, CommandInput};

use clap::{Parser, Subcommand};

use crate::error::Terminate;

#[derive(Parser)]
#[clap(arg_required_else_help(true))]
#[clap(name = "PxlsLog-Explorer")]
#[clap(author = " - Etos2 <github.com/Etos2>")]
#[clap(version = "1.0")]
#[clap(about = "Filter pxls.space logs and generate timelapses.\nA simple program for pxls.space users to explore or adapt for their own uses.\n", long_about = None)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long)]
    #[clap(help = "Enable verbosity")]
    pub verbose: bool,
    #[clap(short, long)]
    #[clap(help = "Prevent files from being overwritten")]
    pub noclobber: bool,
    // #[clap(short, long)]
    // #[clap(help = "Forcibly exit rather than ignoring errors")]
    // pub strict: bool,
    #[clap(long)]
    #[clap(value_name("INT"))]
    #[clap(help = "Number of threads utilised [Defaults to all available threads]")]
    pub threads: Option<usize>,
    #[clap(subcommand)]
    pub input: Input,
}

#[derive(Subcommand)]
pub enum Input {
    Filter(FilterInput),
    Render(RenderInput),
    Stats(StatisticInput),
}

fn main() {
    let cli = Cli::parse();
    let num_threads = match cli.threads {
        Some(threads) => threads,
        None => num_cpus::get(),
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    if cli.verbose {
        eprintln!("Running with {} threads", num_threads);
        if cli.noclobber {
            eprintln!("Preserving output files");
        }
    }

    match &cli.input {
        Input::Filter(filter_input) => execute_command(filter_input, &cli),
        Input::Render(render_input) => execute_command(render_input, &cli),
        Input::Stats(stats_input) => execute_command(stats_input, &cli),
    };
}

fn execute_command<I, C>(input: &I, cli: &Cli)
where
    I: CommandInput<C>,
    C: Command,
{
    match input.validate() {
        Ok(data) => data.run(&cli).unwrap_or_else(|e| e.terminate()),
        Err(e) => e.terminate(),
    };
}
