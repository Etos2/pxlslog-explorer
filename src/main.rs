mod error;
mod filter;
mod palette;
mod action;
mod render;
mod util;

use exitcode;
use filter::FilterInput;
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

    let err = match &cli.input {
        Input::Filter(filter_input) => {
            match filter_input.validate() {
                Ok(data) => data.run(&cli).err(),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(e.exitcode())
                },
            }
        },
        Input::Render(render_input) => {
            // TODO: Fix
            if cli.noclobber {
                eprintln!("Invalid argument \'noclobber\': Unsupported currently");
                std::process::exit(exitcode::USAGE)
            }

            match render_input.validate() {
                Ok(data) => data.run(&cli).err(),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(e.exitcode())
                },
            }
        },
    };

    if let Some(e) = err {
        eprintln!("{}", e);
        std::process::exit(e.exitcode())
    }
}
