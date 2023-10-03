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
use similar::TextDiff;
use thiserror::Error;

use pyrogen_checker::checker::{lint_fix, lint_only, FixTable, FixerResult, LinterResult};
use pyrogen_checker::logging::DisplayParseError;
use pyrogen_checker::message::Message;
use pyrogen_checker::pyproject_toml::lint_pyproject_toml;
use pyrogen_checker::registry::AsRule;
use pyrogen_checker::settings::{flags, CheckerSettings};
use pyrogen_checker::source_kind::SourceKind;
use pyrogen_checker::{fs, IOError, SyntaxError};
use pyrogen_diagnostics::Diagnostic;
use pyrogen_macros::CacheKey;
use pyrogen_python_ast::imports::ImportMap;
use pyrogen_python_ast::{PySourceType, SourceType, TomlSourceType};
use pyrogen_source_file::{LineIndex, SourceCode, SourceFileBuilder};
use pyrogen_workspace::Settings;
use rustpython_parser::ast::Ranged;
use rustpython_parser::text_size::TextRange;

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
    pub(crate) fixed: FxHashMap<String, FixTable>,
    pub(crate) imports: ImportMap,
}

impl Diagnostics {
    pub(crate) fn new(messages: Vec<Message>, imports: ImportMap) -> Self {
        Self {
            messages,
            fixed: FxHashMap::default(),
            imports,
        }
    }

    /// Generate [`Diagnostics`] based on a [`SourceExtractionError`].
    pub(crate) fn from_source_error(
        err: &SourceExtractionError,
        path: Option<&Path>,
        settings: &CheckerSettings,
    ) -> Self {
        let diagnostic = Diagnostic::from(err);
        if settings.rules.enabled(diagnostic.kind.rule()) {
            let name = path.map_or_else(|| "-".into(), std::path::Path::to_string_lossy);
            let dummy = SourceFileBuilder::new(name, "").finish();
            Self::new(
                vec![Message::from_diagnostic(
                    diagnostic,
                    dummy,
                    TextSize::default(),
                )],
                ImportMap::default(),
            )
        } else {
            match path {
                Some(path) => {
                    warn!(
                        "{}{}{} {err}",
                        "Failed to lint ".bold(),
                        fs::relativize_path(path).bold(),
                        ":".bold()
                    );
                }
                None => {
                    warn!("{}{} {err}", "Failed to lint".bold(), ":".bold());
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
        for (filename, fixed) in other.fixed {
            if fixed.is_empty() {
                continue;
            }
            let fixed_in_file = self.fixed.entry(filename).or_default();
            for (rule, count) in fixed {
                if count > 0 {
                    *fixed_in_file.entry(rule).or_default() += count;
                }
            }
        }
    }
}

/// Lint the source code at the given `Path`.
pub(crate) fn lint_path(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
    cache: Option<&Cache>,
    noqa: flags::Noqa,
    autofix: flags::FixMode,
) -> Result<Diagnostics> {
    // Check the cache.
    // TODO(charlie): `fixer::Mode::Apply` and `fixer::Mode::Diff` both have
    // side-effects that aren't captured in the cache. (In practice, it's fine
    // to cache `fixer::Mode::Apply`, since a file either has no fixes, or we'll
    // write the fixes to disk, thus invalidating the cache. But it's a bit hard
    // to reason about. We need to come up with a better solution here.)
    let caching = match cache {
        Some(cache) if noqa.into() && autofix.is_generate() => {
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
                .rules
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

    // Lint the file.
    let (
        LinterResult {
            data: (messages, imports),
            error: parse_error,
        },
        fixed,
    ) = if matches!(autofix, flags::FixMode::Apply | flags::FixMode::Diff) {
        if let Ok(FixerResult {
            result,
            transformed,
            fixed,
        }) = lint_fix(path, package, noqa, settings, &source_kind, source_type)
        {
            if !fixed.is_empty() {
                match autofix {
                    flags::FixMode::Apply => {
                        let SourceKind(source_code) = transformed.as_ref();
                        write(path, source_code.as_bytes())?;
                    }
                    flags::FixMode::Diff => {
                        let SourceKind(source_code) = transformed.as_ref();
                        let mut stdout = io::stdout().lock();
                        TextDiff::from_lines(source_kind.source_code(), source_code)
                            .unified_diff()
                            .header(&fs::relativize_path(path), &fs::relativize_path(path))
                            .to_writer(&mut stdout)?;
                        stdout.write_all(b"\n")?;
                        stdout.flush()?;
                    }
                    flags::FixMode::Generate => {}
                }
            }
            (result, fixed)
        } else {
            // If we fail to autofix, lint the original source code.
            let result = lint_only(path, package, settings, noqa, &source_kind, source_type);
            let fixed = FxHashMap::default();
            (result, fixed)
        }
    } else {
        let result = lint_only(path, package, settings, noqa, &source_kind, source_type);
        let fixed = FxHashMap::default();
        (result, fixed)
    };

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

    Ok(Diagnostics {
        messages,
        fixed: FxHashMap::from_iter([(fs::relativize_path(path), fixed)]),
        imports,
    })
}

/// Generate `Diagnostic`s from source code content derived from
/// stdin.
pub(crate) fn lint_stdin(
    path: Option<&Path>,
    package: Option<&Path>,
    contents: String,
    settings: &Settings,
    noqa: flags::Noqa,
    autofix: flags::FixMode,
) -> Result<Diagnostics> {
    // TODO(charlie): Support `pyproject.toml`.
    let SourceType::Python(source_type) = path.map(SourceType::from).unwrap_or_default() else {
        return Ok(Diagnostics::default());
    };

    // Extract the sources from the file.
    let LintSource(source_kind) = match LintSource::try_from_source_code(contents, source_type) {
        Ok(Some(sources)) => sources,
        Ok(None) => return Ok(Diagnostics::default()),
        Err(err) => {
            return Ok(Diagnostics::from_source_error(&err, path, &settings.linter));
        }
    };

    // Lint the inputs.
    let (
        LinterResult {
            data: (messages, imports),
            error: parse_error,
        },
        fixed,
    ) = if matches!(autofix, flags::FixMode::Apply | flags::FixMode::Diff) {
        if let Ok(FixerResult {
            result,
            transformed,
            fixed,
        }) = lint_fix(
            path.unwrap_or_else(|| Path::new("-")),
            package,
            noqa,
            &settings.linter,
            &source_kind,
            source_type,
        ) {
            match autofix {
                flags::FixMode::Apply => {
                    // Write the contents to stdout, regardless of whether any errors were fixed.
                    io::stdout().write_all(transformed.source_code().as_bytes())?;
                }
                flags::FixMode::Diff => {
                    // But only write a diff if it's non-empty.
                    if !fixed.is_empty() {
                        let text_diff = TextDiff::from_lines(
                            source_kind.source_code(),
                            transformed.source_code(),
                        );
                        let mut unified_diff = text_diff.unified_diff();
                        if let Some(path) = path {
                            unified_diff
                                .header(&fs::relativize_path(path), &fs::relativize_path(path));
                        }

                        let mut stdout = io::stdout().lock();
                        unified_diff.to_writer(&mut stdout)?;
                        stdout.write_all(b"\n")?;
                        stdout.flush()?;
                    }
                }
                flags::FixMode::Generate => {}
            }

            (result, fixed)
        } else {
            // If we fail to autofix, lint the original source code.
            let result = lint_only(
                path.unwrap_or_else(|| Path::new("-")),
                package,
                &settings.linter,
                noqa,
                &source_kind,
                source_type,
            );
            let fixed = FxHashMap::default();

            // Write the contents to stdout anyway.
            if autofix.is_apply() {
                io::stdout().write_all(source_kind.source_code().as_bytes())?;
            }

            (result, fixed)
        }
    } else {
        let result = lint_only(
            path.unwrap_or_else(|| Path::new("-")),
            package,
            &settings.linter,
            noqa,
            &source_kind,
            source_type,
        );
        let fixed = FxHashMap::default();
        (result, fixed)
    };

    let imports = imports.unwrap_or_default();

    if let Some(err) = parse_error {
        error!(
            "Failed to parse {}: {err}",
            path.map_or_else(|| "-".into(), fs::relativize_path).bold()
        );
    }

    Ok(Diagnostics {
        messages,
        fixed: FxHashMap::from_iter([(
            fs::relativize_path(path.unwrap_or_else(|| Path::new("-"))),
            fixed,
        )]),
        imports,
    })
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
                IOError {
                    message: err.to_string(),
                },
                TextRange::default(),
            ),
        }
    }
}