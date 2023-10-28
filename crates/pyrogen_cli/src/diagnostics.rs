#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::io;
use std::ops::AddAssign;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use filetime::FileTime;
use log::{debug, error, warn};
use rustpython_parser::text_size::{TextRange, TextSize};
use thiserror::Error;

use pyrogen_checker::checker::{lint_only, CheckerResult};
use pyrogen_checker::fs;
use pyrogen_checker::logging::DisplayParseError;
use pyrogen_checker::message::Message;
use pyrogen_checker::pyproject_toml::lint_pyproject_toml;
use pyrogen_checker::registry::{AsErrorCode, Diagnostic, DiagnosticKind, ErrorCode};
use pyrogen_checker::settings::{flags, CheckerSettings};
use pyrogen_checker::source_kind::SourceKind;
use pyrogen_macros::CacheKey;
use pyrogen_python_ast::imports::ImportMap;
use pyrogen_python_ast::{SourceType, TomlSourceType};
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
pub(crate) struct Messages {
    pub(crate) messages: Vec<Message>,
    pub(crate) imports: ImportMap,
}

impl Messages {
    pub(crate) fn new(messages: Vec<Message>, imports: ImportMap) -> Self {
        Self { messages, imports }
    }

    /// Generate [`Messages`] based on a [`SourceExtractionError`].
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

impl AddAssign for Messages {
    fn add_assign(&mut self, other: Self) {
        self.messages.extend(other.messages);
        self.imports.extend(other.imports);
    }
}

/// Type-check the source code at the given `Path`.
pub(crate) fn type_check_path(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
    cache: Option<&Cache>,
    respect_type_ignore: flags::TypeIgnore,
) -> Result<Messages> {
    // Check the cache.
    let caching = match cache {
        Some(cache) if respect_type_ignore.into() => {
            let relative_path = cache
                .relative_path(path)
                .expect("wrong package cache for file");

            let cache_key = FileCacheKey::from_path(path).context("Failed to create cache key")?;

            if let Some(cache) = cache.get(relative_path, &cache_key) {
                return Ok(cache.as_diagnostics(path));
            }

            // Stash the file metadata for later so when we update the cache it reflects the prerun
            // information
            Some((cache, relative_path, cache_key))
        }
        _ => None,
    };

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
                            return Ok(Messages::from_source_error(&err, Some(path), settings));
                        }
                    };
                let source_file = SourceFileBuilder::new(path.to_string_lossy(), contents).finish();
                lint_pyproject_toml(source_file, settings)
            } else {
                vec![]
            };
            return Ok(Messages {
                messages,
                ..Messages::default()
            });
        }
        SourceType::Toml(_) => return Ok(Messages::default()),
        SourceType::Python(source_type) => source_type,
    };

    // Extract the sources from the file.
    let LintSource(source_kind) = match LintSource::try_from_path(path) {
        Ok(Some(sources)) => sources,
        Ok(None) => return Ok(Messages::default()),
        Err(err) => {
            return Ok(Messages::from_source_error(&err, Some(path), settings));
        }
    };
    let source_kind = SourceKind::new(source_kind);

    // Lint the file.
    let CheckerResult {
        data: (messages, imports),
        error: parse_error,
    } = lint_only(
        path,
        package,
        settings,
        respect_type_ignore,
        &source_kind,
        source_type,
    );

    let imports = imports.unwrap_or_default();

    if let Some((cache, relative_path, key)) = caching {
        // We don't cache parsing errors.
        if parse_error.is_none() {
            cache.update(relative_path.to_owned(), key, &messages, &imports);
        }
    }

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

    Ok(Messages { messages, imports })
}

/// Generate `Diagnostic`s from source code content derived from
/// stdin.
pub(crate) fn type_check_stdin(
    path: Option<&Path>,
    package: Option<&Path>,
    contents: String,
    settings: &Settings,
    noqa: flags::TypeIgnore,
) -> Result<Messages> {
    let SourceType::Python(source_type) = path.map(SourceType::from).unwrap_or_default() else {
        return Ok(Messages::default());
    };

    let source_kind = SourceKind::new(contents);

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

    Ok(Messages { messages, imports })
}

#[derive(Debug)]
pub(crate) struct LintSource(String);

impl LintSource {
    /// Extract the lint [`LintSource`] from the given file path.
    pub(crate) fn try_from_path(path: &Path) -> Result<Option<LintSource>, SourceExtractionError> {
        // This is tested by ruff_cli integration test `unreadable_file`
        let contents = std::fs::read_to_string(path)?;
        Ok(Some(LintSource(contents)))
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
