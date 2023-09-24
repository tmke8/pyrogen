use std::path::PathBuf;

use anyhow::Result;
use pyrogen_parser::{self, parser_test};

use clap::Parser;
use pyrogen_workspace::pyproject::{find_settings_toml, parse_pyproject_toml};
use pyrogen_workspace::resolver::python_files_in_path;

pub fn print_message() {
    let num = 10;
    println!(
        "Hello, world! {num} plus one is {}!",
        pyrogen_parser::add_one(num)
    );
    parser_test();
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

pub fn parse_args() -> Result<()> {
    let args = Args::parse();

    if args.strict {
        println!("Hello {:?}!", args.files)
    }
    let path = find_settings_toml(&args.files[0])?;
    if let Some(pyp_path) = path {
        let pyproject = parse_pyproject_toml(pyp_path)?;
        python_files_in_path(&args.files, pyproject);
    }
}
