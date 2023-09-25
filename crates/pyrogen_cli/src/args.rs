use std::path::PathBuf;

use clap::{command, Parser};

use pyrogen_checker::logging::LogLevel;
use pyrogen_checker::settings::types::{FilePattern, PythonVersion};
use pyrogen_workspace::configuration::Configuration;
use pyrogen_workspace::resolver::ConfigurationTransformer;

#[derive(Debug, Parser)]
#[command(
    author,
    name = "pyrogen",
    about = "Pyrogen, an extremely strict Python type checker."
)]
#[command(version)]
pub struct Args {
    #[clap(flatten)]
    pub checker_args: CheckCommand,
    #[clap(flatten)]
    pub log_level_args: LogLevelArgs,
}

// The `Parser` derive is for pyrogen_dev, for pyrogen_cli `Args` would be sufficient
#[derive(Clone, Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct CheckCommand {
    /// List of files or directories to check.
    pub files: Vec<PathBuf>,
    /// The minimum Python version that should be supported.
    #[arg(long, value_enum)]
    pub target_version: Option<PythonVersion>,
    /// Path to the `pyproject.toml` or `pyrogen.toml` file to use for
    /// configuration.
    #[arg(long, conflicts_with = "isolated")]
    pub config: Option<PathBuf>,
    /// List of paths, used to omit files and/or directories from analysis.
    #[arg(
        long,
        value_delimiter = ',',
        value_name = "FILE_PATTERN",
        help_heading = "File selection"
    )]
    pub exclude: Option<Vec<FilePattern>>,
    /// Respect file exclusions via `.gitignore` and other standard ignore files.
    /// Use `--no-respect-gitignore` to disable.
    #[arg(
        long,
        overrides_with("no_respect_gitignore"),
        help_heading = "File selection"
    )]
    respect_gitignore: bool,
    #[clap(long, overrides_with("respect_gitignore"), hide = true)]
    no_respect_gitignore: bool,
    /// Enforce exclusions, even for paths passed to Ruff directly on the command-line.
    /// Use `--no-force-exclude` to disable.
    #[arg(
        long,
        overrides_with("no_force_exclude"),
        help_heading = "File selection"
    )]
    force_exclude: bool,
    #[clap(long, overrides_with("force_exclude"), hide = true)]
    no_force_exclude: bool,
    /// Disable cache reads.
    #[arg(short, long, help_heading = "Miscellaneous")]
    pub no_cache: bool,
    /// Ignore all configuration files.
    #[arg(long, conflicts_with = "config", help_heading = "Miscellaneous")]
    pub isolated: bool,
    /// Path to the cache directory.
    #[arg(long, env = "PYROGEN_CACHE_DIR", help_heading = "Miscellaneous")]
    pub cache_dir: Option<PathBuf>,
    /// The name of the file when passing it through stdin.
    #[arg(long, help_heading = "Miscellaneous")]
    pub stdin_filename: Option<PathBuf>,
    /// Exit with status code "0", even upon detecting lint violations.
    #[arg(short, long, help_heading = "Miscellaneous")]
    pub exit_zero: bool,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, clap::Args)]
pub struct LogLevelArgs {
    /// Enable verbose logging.
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub verbose: bool,
    /// Print diagnostics, but nothing else.
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub quiet: bool,
    /// Disable all logging (but still exit with status code "1" upon detecting diagnostics).
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub silent: bool,
}

impl From<&LogLevelArgs> for LogLevel {
    fn from(args: &LogLevelArgs) -> Self {
        if args.silent {
            Self::Silent
        } else if args.quiet {
            Self::Quiet
        } else if args.verbose {
            Self::Verbose
        } else {
            Self::Default
        }
    }
}

impl CheckCommand {
    /// Partition the CLI into command-line arguments and configuration
    /// overrides.
    pub fn partition(self) -> (CheckArguments, CliOverrides) {
        (
            CheckArguments {
                config: self.config,
                exit_zero: self.exit_zero,
                files: self.files,
                no_cache: self.no_cache,
                isolated: self.isolated,
                stdin_filename: self.stdin_filename,
            },
            CliOverrides {
                exclude: self.exclude,
                respect_gitignore: resolve_bool_arg(
                    self.respect_gitignore,
                    self.no_respect_gitignore,
                ),
                target_version: self.target_version,
                // TODO(charlie): Included in `pyproject.toml`, but not inherited.
                cache_dir: self.cache_dir,
                force_exclude: resolve_bool_arg(self.force_exclude, self.no_force_exclude),
            },
        )
    }
}

fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
        (..) => unreachable!("Clap should make this impossible"),
    }
}

/// CLI settings that are distinct from configuration (commands, lists of files,
/// etc.).
#[allow(clippy::struct_excessive_bools)]
pub struct CheckArguments {
    pub config: Option<PathBuf>,
    pub exit_zero: bool,
    pub files: Vec<PathBuf>,
    pub isolated: bool,
    pub no_cache: bool,
    pub stdin_filename: Option<PathBuf>,
}

/// CLI settings that function as configuration overrides.
#[derive(Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CliOverrides {
    pub exclude: Option<Vec<FilePattern>>,
    pub respect_gitignore: Option<bool>,
    pub target_version: Option<PythonVersion>,
    // TODO(charlie): Captured in pyproject.toml as a default, but not part of `Settings`.
    pub cache_dir: Option<PathBuf>,
    pub force_exclude: Option<bool>,
}

impl ConfigurationTransformer for CliOverrides {
    fn transform(&self, mut config: Configuration) -> Configuration {
        if let Some(cache_dir) = &self.cache_dir {
            config.cache_dir = Some(cache_dir.clone());
        }
        if let Some(exclude) = &self.exclude {
            config.exclude = Some(exclude.clone());
        }
        if let Some(force_exclude) = &self.force_exclude {
            config.force_exclude = Some(*force_exclude);
        }
        if let Some(respect_gitignore) = &self.respect_gitignore {
            config.respect_gitignore = Some(*respect_gitignore);
        }
        if let Some(target_version) = &self.target_version {
            config.target_version = Some(*target_version);
        }

        config
    }
}
