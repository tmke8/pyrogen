use std::path::PathBuf;

use pyrogen_parser;

use clap::Parser;

pub fn print_message() {
    let num = 10;
    println!(
        "Hello, world! {num} plus one is {}!",
        pyrogen_parser::add_one(num)
    );
}

/// Pyrogen: a Python type checker
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of files or directories to check.
    pub files: Vec<PathBuf>,

    /// Whether to check in strict mode.
    #[arg(short, long)]
    strict: bool,
}

pub fn parse_args() {
    let args = Args::parse();

    if args.strict {
        println!("Hello {:?}!", args.files)
    }
}
