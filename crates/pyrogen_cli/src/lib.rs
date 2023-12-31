use std::{
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::Result;
use itertools::Itertools;
use pyrogen_checker::{
    fs,
    logging::{set_up_logging, LogLevel},
    settings::code_table::MessageKind,
    warn_user_once,
};
use pyrogen_workspace::resolver::python_files_in_path;

use crate::args::{Args, CheckCommand};
use crate::printer::{Flags as PrinterFlags, Printer};

pub mod args;
mod cache;
mod commands;
mod diagnostics;
mod panic;
mod printer;
pub mod resolve;
mod stdin;

#[derive(Copy, Clone)]
pub enum ExitStatus {
    /// Linting was successful and there were no linting errors.
    Success,
    /// Linting was successful but there were linting errors.
    Failure,
    /// Linting failed.
    Error,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => ExitCode::from(0),
            ExitStatus::Failure => ExitCode::from(1),
            ExitStatus::Error => ExitCode::from(2),
        }
    }
}

/// Returns true if the command should read from standard input.
fn is_stdin(files: &[PathBuf], stdin_filename: Option<&Path>) -> bool {
    // If the user provided a `--stdin-filename`, always read from standard input.
    if stdin_filename.is_some() {
        if let Some(file) = files.iter().find(|file| file.as_path() != Path::new("-")) {
            warn_user_once!(
                "Ignoring file {} in favor of standard input.",
                file.display()
            );
        }
        return true;
    }

    let [file] = files else {
        return false;
    };
    // If the user provided exactly `-`, read from standard input.
    file == Path::new("-")
}

pub fn run(
    Args {
        checker_args,
        log_level_args,
    }: Args,
) -> Result<ExitStatus> {
    {
        use colored::Colorize;

        let default_panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            #[allow(clippy::print_stderr)]
            {
                eprintln!(
                    r#"
{}{} {} If you could open an issue at:

    https://github.com/tmke8/pyrogen

...quoting the executed command, along with the relevant file contents and `pyproject.toml` settings, we'd be very appreciative!
"#,
                    "error".red().bold(),
                    ":".bold(),
                    "Pyrogen crashed.".bold(),
                );
            }
            default_panic_hook(info);
        }));
    }

    // Enabled ANSI colors on Windows 10.
    #[cfg(windows)]
    assert!(colored::control::set_virtual_terminal(true).is_ok());

    let log_level = LogLevel::from(&log_level_args);
    set_up_logging(&log_level)?;

    check(checker_args, log_level)
}

pub fn check(args: CheckCommand, log_level: LogLevel) -> Result<ExitStatus> {
    let (cli, overrides) = args.partition();

    // Construct the "default" settings. These are used when no `pyproject.toml`
    // files are present, or files are injected from outside of the hierarchy.
    let pyproject_config = resolve::resolve(
        cli.isolated,
        cli.config.as_deref(),
        &overrides,
        cli.stdin_filename.as_deref(),
    )?;

    let mut writer: Box<dyn Write> = Box::new(BufWriter::new(io::stdout()));

    // Collect all files in the hierarchy.
    let (paths, _resolver) = python_files_in_path(&cli.files, &pyproject_config, &overrides)?;

    if paths.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
        return Ok(ExitStatus::Success);
    }

    // Print the list of files.
    for entry in paths
        .iter()
        .flatten()
        .sorted_by(|a, b| a.path().cmp(b.path()))
    {
        writeln!(writer, "{}", entry.path().to_string_lossy())?;
    }

    let printer_flags = PrinterFlags::SHOW_VIOLATIONS;
    let printer = Printer::new(
        pyproject_config.settings.output_format,
        log_level,
        printer_flags,
    );

    let is_stdin = is_stdin(&cli.files, cli.stdin_filename.as_deref());
    let cache = !cli.no_cache;
    // TODO: make this configurable.
    let respect_type_ignore = true;

    // Generate lint violations.
    let diagnostics = if is_stdin {
        commands::check_stdin::check_stdin(
            cli.stdin_filename.map(fs::normalize_path).as_deref(),
            &pyproject_config,
            &overrides,
            respect_type_ignore.into(),
        )?
    } else {
        commands::check::check(
            &cli.files,
            &pyproject_config,
            &overrides,
            cache.into(),
            respect_type_ignore.into(),
        )?
    };
    printer.write_once(&diagnostics, &mut writer)?;

    if !cli.exit_zero
        && diagnostics
            .messages
            .into_iter()
            .any(|message| message.kind == MessageKind::Error)
    {
        return Ok(ExitStatus::Failure);
    }

    Ok(ExitStatus::Success)
}
