mod filter;
mod render;

use filter::{FilterInput};
use render::RenderInput;

use clap::{Parser, Subcommand};

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
    #[clap(long)]
    #[clap(help = "Number of threads utilised [Defaults to all available threads]")]
    pub threads: Option<usize>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Filter(FilterInput),
    Render(RenderInput),
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
        println!("Running with {} threads", num_threads);
        if cli.noclobber {
            println!("Preserving output files");
        }
    }

    match &cli.command {
        Command::Filter(filter_input) => {
            // TODO: Graceful error handling
            let filter = filter_input.validate().unwrap();
            if cli.verbose {
                println!("{}", filter);
            }
            let result = filter.execute(&cli).unwrap();
            if cli.verbose {
                println!("Returned {} of {} entries", result.0, result.1);
            }
        }
        // TODO: Implement lol
        Command::Render(_render) => {
            unimplemented!("soon:tm:")
        }
    }
}