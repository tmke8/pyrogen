use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

use pyrogen_cli::args::Args;
use pyrogen_cli::{run, ExitStatus};

pub fn main() -> ExitCode {
    let args = wild::args_os();
    let args = argfile::expand_args_from(args, argfile::parse_fromfile, argfile::PREFIX).unwrap();

    let args = Args::parse_from(args);
    match run(args) {
        Ok(code) => code.into(),
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                // This communicates that this isn't a linter error but pyrogen itself hard-errored for
                // some reason (e.g. failed to resolve the configuration)
                eprintln!("{}", "pyrogen failed".red().bold());
                // Currently we generally only see one error, but e.g. with io errors when resolving
                // the configuration it is help to chain errors ("resolving configuration failed" ->
                // "failed to read file: subdir/pyproject.toml")
                for cause in err.chain() {
                    eprintln!("  {} {cause}", "Cause:".bold());
                }
            }
            ExitStatus::Error.into()
        }
    }
}
