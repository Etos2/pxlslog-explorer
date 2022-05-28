mod command;
mod filter;
mod parser;
mod render;

use command::PxlsInput;
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
        println!("Running with {} threads", num_threads);
        if cli.noclobber {
            println!("Preserving output files");
        }
    }

    let command = match &cli.input {
        Input::Filter(filter_input) => filter_input.parse(&cli),
        Input::Render(render_input) => render_input.parse(&cli),
    };

    match command {
        Ok(c) => match c.run(&cli) {
            Ok(_) => eprintln!("Executed successfully!"),
            Err(e) => eprintln!("{}", e),
        },
        Err(e) => eprintln!("{}", e),
    };
}
