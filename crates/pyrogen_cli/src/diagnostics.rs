#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::fs::{write, File};
use std::io;
use std::io::{BufWriter, Write};
use std::ops::AddAssign;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use filetime::FileTime;
use log::{debug, error, warn};
use rustc_hash::FxHashMap;
use rustpython_parser::ast::Ranged;
use rustpython_parser::text_size::{TextRange, TextSize};
use similar::TextDiff;
use thiserror::Error;

use pyrogen_checker::checker::{lint_only, CheckerResult, FixTable, FixerResult};
use pyrogen_checker::fs;
use pyrogen_checker::logging::DisplayParseError;
use pyrogen_checker::message::Message;
use pyrogen_checker::pyproject_toml::lint_pyproject_toml;
use pyrogen_checker::registry::{AsErrorCode, Diagnostic, DiagnosticKind, ErrorCode};
use pyrogen_checker::settings::{flags, CheckerSettings};
use pyrogen_checker::source_kind::SourceKind;
use pyrogen_macros::CacheKey;
use pyrogen_python_ast::imports::ImportMap;
use pyrogen_python_ast::{PySourceType, SourceType, TomlSourceType};
use pyrogen_source_file::{LineIndex, SourceCode, SourceFileBuilder};
use pyrogen_workspace::Settings;

use crate::cache::Cache;

#[derive(CacheKey)]
pub(crate) struct FileCacheKey {
    /// Timestamp when the file was last modified before the (cached) check.
    file_last_modified: FileTime,
    /// Permissions of the file before the (cached) check.
    file_permissions_mode: u32,
}

impl FileCacheKey {
    fn from_path(path: &Path) -> io::Result<FileCacheKey> {
        // Construct a cache key for the file
        let metadata = path.metadata()?;

        #[cfg(unix)]
        let permissions = metadata.permissions().mode();
        #[cfg(windows)]
        let permissions: u32 = metadata.permissions().readonly().into();

        Ok(FileCacheKey {
            file_last_modified: FileTime::from_last_modification_time(&metadata),
            file_permissions_mode: permissions,
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Diagnostics {
    pub(crate) messages: Vec<Message>,
    pub(crate) imports: ImportMap,
}

impl Diagnostics {
    pub(crate) fn new(messages: Vec<Message>, imports: ImportMap) -> Self {
        Self { messages, imports }
    }

    /// Generate [`Diagnostics`] based on a [`SourceExtractionError`].
    pub(crate) fn from_source_error(
        err: &SourceExtractionError,
        path: Option<&Path>,
        settings: &CheckerSettings,
    ) -> Self {
        let diagnostic = Diagnostic::from(err);
        if let Some(kind) = settings.table.entry(diagnostic.kind.error_code()) {
            let name = path.map_or_else(|| "-".into(), std::path::Path::to_string_lossy);
            let dummy = SourceFileBuilder::new(name, "").finish();
            Self::new(
                vec![Message::from_diagnostic(
                    diagnostic,
                    dummy,
                    TextSize::default(),
                    kind,
                )],
                ImportMap::default(),
            )
        } else {
            match path {
                Some(path) => {
                    warn!(
                        "{}{}{} {err}",
                        "Failed to check ".bold(),
                        fs::relativize_path(path).bold(),
                        ":".bold()
                    );
                }
                None => {
                    warn!("{}{} {err}", "Failed to check".bold(), ":".bold());
                }
            }

            Self::default()
        }
    }
}

impl AddAssign for Diagnostics {
    fn add_assign(&mut self, other: Self) {
        self.messages.extend(other.messages);
        self.imports.extend(other.imports);
    }
}

/// Lint the source code at the given `Path`.
pub(crate) fn lint_path(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
    cache: Option<&Cache>,
    noqa: flags::Noqa,
) -> Result<Diagnostics> {
    // Check the cache.

    debug!("Checking: {}", path.display());

    let source_type = match SourceType::from(path) {
        SourceType::Toml(TomlSourceType::Pyproject) => {
            let messages = if settings
                .table
                .iter_enabled()
                .any(|rule_code| rule_code.lint_source().is_pyproject_toml())
            {
                let contents =
                    match std::fs::read_to_string(path).map_err(SourceExtractionError::Io) {
                        Ok(contents) => contents,
                        Err(err) => {
                            return Ok(Diagnostics::from_source_error(&err, Some(path), settings));
                        }
                    };
                let source_file = SourceFileBuilder::new(path.to_string_lossy(), contents).finish();
                lint_pyproject_toml(source_file, settings)
            } else {
                vec![]
            };
            return Ok(Diagnostics {
                messages,
                ..Diagnostics::default()
            });
        }
        SourceType::Toml(_) => return Ok(Diagnostics::default()),
        SourceType::Python(source_type) => source_type,
    };

    // Extract the sources from the file.
    let LintSource(source_kind) = match LintSource::try_from_path(path, source_type) {
        Ok(Some(sources)) => sources,
        Ok(None) => return Ok(Diagnostics::default()),
        Err(err) => {
            return Ok(Diagnostics::from_source_error(&err, Some(path), settings));
        }
    };
    let source_kind = SourceKind::new(source_kind);

    // Lint the file.
    let CheckerResult {
        data: (messages, imports),
        error: parse_error,
    } = lint_only(path, package, settings, noqa, &source_kind, source_type);

    let imports = imports.unwrap_or_default();

    if let Some(err) = parse_error {
        error!(
            "{}",
            DisplayParseError::new(
                err,
                SourceCode::new(
                    source_kind.source_code(),
                    &LineIndex::from_source_text(source_kind.source_code())
                ),
                &source_kind,
            )
        );
    }

    Ok(Diagnostics { messages, imports })
}

/// Generate `Diagnostic`s from source code content derived from
/// stdin.
pub(crate) fn lint_stdin(
    path: Option<&Path>,
    package: Option<&Path>,
    contents: String,
    settings: &Settings,
    noqa: flags::Noqa,
) -> Result<Diagnostics> {
    let SourceType::Python(source_type) = path.map(SourceType::from).unwrap_or_default() else {
        return Ok(Diagnostics::default());
    };

    // Extract the sources from the file.
    let LintSource(source_kind) = match LintSource::try_from_source_code(contents, source_type) {
        Ok(Some(sources)) => sources,
        Ok(None) => return Ok(Diagnostics::default()),
        Err(err) => {
            return Ok(Diagnostics::from_source_error(
                &err,
                path,
                &settings.checker,
            ));
        }
    };
    let source_kind = SourceKind::new(source_kind);

    // Lint the inputs.
    let CheckerResult {
        data: (messages, imports),
        error: parse_error,
    } = lint_only(
        path.unwrap_or_else(|| Path::new("-")),
        package,
        &settings.checker,
        noqa,
        &source_kind,
        source_type,
    );

    let imports = imports.unwrap_or_default();

    if let Some(err) = parse_error {
        error!(
            "Failed to parse {}: {err}",
            path.map_or_else(|| "-".into(), fs::relativize_path).bold()
        );
    }

    Ok(Diagnostics { messages, imports })
}

#[derive(Debug)]
pub(crate) struct LintSource(String);

impl LintSource {
    /// Extract the lint [`LintSource`] from the given file path.
    pub(crate) fn try_from_path(
        path: &Path,
        source_type: PySourceType,
    ) -> Result<Option<LintSource>, SourceExtractionError> {
        // This is tested by ruff_cli integration test `unreadable_file`
        let contents = std::fs::read_to_string(path)?;
        Ok(Some(LintSource(contents)))
    }

    /// Extract the lint [`LintSource`] from the raw string contents, optionally accompanied by a
    /// file path indicating the path to the file from which the contents were read. If provided,
    /// the file path should be used for diagnostics, but not for reading the file from disk.
    pub(crate) fn try_from_source_code(
        source_code: String,
        source_type: PySourceType,
    ) -> Result<Option<LintSource>, SourceExtractionError> {
        Ok(Some(LintSource(source_code)))
    }
}

#[derive(Error, Debug)]
pub(crate) enum SourceExtractionError {
    /// The extraction failed due to an [`io::Error`].
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<&SourceExtractionError> for Diagnostic {
    fn from(err: &SourceExtractionError) -> Self {
        match err {
            // IO errors.
            SourceExtractionError::Io(_) => Diagnostic::new(
                DiagnosticKind {
                    error_code: ErrorCode::IOError,
                    body: err.to_string(),
                },
                TextRange::default(),
            ),
        }
    }
}
