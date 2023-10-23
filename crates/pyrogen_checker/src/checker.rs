use std::borrow::Cow;
use std::ops::Deref;
use std::path::Path;

use anyhow::{anyhow, Result};
use colored::Colorize;
use itertools::Itertools;
use log::error;
use rustc_hash::FxHashMap;
use rustpython_ast::text_size::{TextLen, TextRange};
use rustpython_ast::TextSize;
use rustpython_parser::ast::Ranged;
use rustpython_parser::lexer::LexResult;
use rustpython_parser::{Parse, ParseError};

use pyrogen_python_ast::imports::ImportMap;
use pyrogen_python_ast::{AsMode, PySourceType};
use pyrogen_python_index::Indexer;
use pyrogen_source_file::{Locator, SourceFileBuilder};

use crate::checkers::filesystem::check_file_path;
use crate::checkers::imports::check_imports;
use crate::checkers::type_ignore::check_type_ignore;
use crate::checkers::typecheck::check_ast;
use crate::directives::Directives;
use crate::logging::DisplayParseError;
use crate::message::Message;
use crate::registry::{AsErrorCode, Diagnostic, DiagnosticKind, ErrorCode};
use crate::settings::{flags, CheckerSettings};
use crate::source_kind::SourceKind;
use crate::{directives, fs};

/// A [`Result`]-like type that returns both data and an error. Used to return
/// diagnostics even in the face of parse errors, since many diagnostics can be
/// generated without a full AST.
pub struct CheckerResult<T> {
    pub data: T,
    pub error: Option<ParseError>,
}

impl<T> CheckerResult<T> {
    const fn new(data: T, error: Option<ParseError>) -> Self {
        Self { data, error }
    }

    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> CheckerResult<U> {
        CheckerResult::new(f(self.data), self.error)
    }
}

pub type FixTable = FxHashMap<ErrorCode, usize>;

pub struct FixerResult<'a> {
    /// The result returned by the linter, after applying any fixes.
    pub result: CheckerResult<(Vec<Message>, Option<ImportMap>)>,
    /// The resulting source code, after applying any fixes.
    pub transformed: Cow<'a, SourceKind>,
    /// The number of fixes applied for each [`Rule`].
    pub fixed: FixTable,
}

/// Generate `Diagnostic`s from the source code contents at the
/// given `Path`.
#[allow(clippy::too_many_arguments)]
pub fn check_path(
    path: &Path,
    package: Option<&Path>,
    tokens: impl IntoIterator<Item = LexResult>,
    locator: &Locator,
    indexer: &Indexer,
    directives: &Directives,
    settings: &CheckerSettings,
    noqa: flags::Noqa,
    source_kind: &SourceKind,
    source_type: PySourceType,
) -> CheckerResult<(Vec<Diagnostic>, Option<ImportMap>)> {
    // Aggregate all diagnostics.
    let mut diagnostics = vec![];
    let mut imports = None;
    let mut error = None;

    // Run the filesystem-based rules.
    if settings
        .rules
        .iter_enabled()
        .any(|rule_code| rule_code.lint_source().is_filesystem())
    {
        diagnostics.extend(check_file_path(path, package, settings));
    }

    // Run the AST-based rules.
    match rustpython_parser::parse_tokens(tokens, source_type.as_mode(), &path.to_string_lossy()) {
        Ok(python_ast) => {
            diagnostics.extend(check_ast(
                &python_ast.expect_module().body,
                locator,
                indexer,
                &directives.noqa_line_for,
                settings,
                noqa,
                path,
                package,
                source_type,
            ));
            // let (import_diagnostics, module_imports) = check_imports(
            //     &python_ast,
            //     locator,
            //     indexer,
            //     settings,
            //     path,
            //     package,
            //     source_kind,
            //     source_type,
            // );
            // imports = module_imports;
            // diagnostics.extend(import_diagnostics);
        }
        Err(parse_error) => {
            // Always add a diagnostic for the syntax error, regardless of whether
            // `ErrorCode::SyntaxError` is enabled. We avoid propagating the syntax error
            // if it's disabled via any of the usual mechanisms (e.g., `noqa`,
            // `per-file-ignores`), and the easiest way to detect that suppression is
            // to see if the diagnostic persists to the end of the function.

            let rest = locator.after(parse_error.offset);
            // Try to create a non-empty range so that the diagnostic can print a caret at the
            // right position. This requires that we retrieve the next character, if any, and take its length
            // to maintain char-boundaries.
            let len = rest
                .chars()
                .next()
                .map_or(TextSize::new(0), TextLen::text_len);
            diagnostics.push(Diagnostic::new(
                DiagnosticKind {
                    body: format!("Syntax error: {}", parse_error.error),
                    error_code: ErrorCode::SyntaxError,
                },
                TextRange::at(parse_error.offset, len),
            ));
            error = Some(parse_error);
        }
    }

    // Ignore diagnostics based on per-file-ignores.
    if !diagnostics.is_empty() && !settings.per_file_ignores.is_empty() {
        let ignores = fs::ignores_from_path(path, &settings.per_file_ignores);
        if !ignores.is_empty() {
            diagnostics.retain(|diagnostic| !ignores.contains(diagnostic.kind.error_code()));
        }
    };

    // Enforce `noqa` directives.
    if (noqa.into() && !diagnostics.is_empty())
        || settings
            .rules
            .iter_enabled()
            .any(|rule_code| rule_code.lint_source().is_noqa())
    {
        let ignored = check_type_ignore(
            &mut diagnostics,
            path,
            locator,
            indexer.comment_ranges(),
            &directives.noqa_line_for,
            error.is_none(),
            settings,
        );
        if noqa.into() {
            for index in ignored.iter().rev() {
                diagnostics.swap_remove(*index);
            }
        }
    }

    // If there was a syntax error, check if it should be discarded.
    if error.is_some() {
        // If the syntax error was removed by _any_ of the above disablement methods (e.g., a
        // `noqa` directive, or a `per-file-ignore`), discard it.
        if !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind.error_code() == ErrorCode::SyntaxError)
        {
            error = None;
        }

        // If the syntax error _diagnostic_ is disabled, discard the _diagnostic_.
        if !settings.rules.enabled(ErrorCode::SyntaxError) {
            diagnostics.retain(|diagnostic| diagnostic.kind.error_code() != ErrorCode::SyntaxError);
        }
    }

    CheckerResult::new((diagnostics, imports), error)
}

/// Generate a [`Message`] for each [`Diagnostic`] triggered by the given source
/// code.
pub fn lint_only(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
    noqa: flags::Noqa,
    source_kind: &SourceKind,
    source_type: PySourceType,
) -> CheckerResult<(Vec<Message>, Option<ImportMap>)> {
    // Tokenize once.
    // type Tokens = impl Iterator<Item = LexResult>;
    let tokens = rustpython_parser::lexer::lex(source_kind.source_code(), source_type.as_mode())
        .collect::<Vec<_>>();

    // Map row and column locations to byte slices (lazily).
    let locator = Locator::new(source_kind.source_code());

    // Extra indices from the code.
    let indexer = Indexer::from_tokens(&tokens, &locator);

    // Extract the `# noqa` and `# isort: skip` directives from the source.
    let directives = directives::extract_directives(&tokens, &locator, &indexer);

    // Generate diagnostics.
    let result = check_path(
        path,
        package,
        tokens,
        &locator,
        &indexer,
        &directives,
        settings,
        noqa,
        source_kind,
        source_type,
    );

    result.map(|(diagnostics, imports)| {
        (
            diagnostics_to_messages(diagnostics, path, &locator, &directives),
            imports,
        )
    })
}

/// Convert from diagnostics to messages.
fn diagnostics_to_messages(
    diagnostics: Vec<Diagnostic>,
    path: &Path,
    locator: &Locator,
    directives: &Directives,
) -> Vec<Message> {
    let file = once_cell::unsync::Lazy::new(|| {
        let mut builder =
            SourceFileBuilder::new(path.to_string_lossy().as_ref(), locator.contents());

        if let Some(line_index) = locator.line_index() {
            builder.set_line_index(line_index.clone());
        }

        builder.finish()
    });

    diagnostics
        .into_iter()
        .map(|diagnostic| {
            let noqa_offset = directives.noqa_line_for.resolve(diagnostic.start());
            Message::from_diagnostic(diagnostic, file.deref().clone(), noqa_offset)
        })
        .collect()
}

fn collect_rule_codes(rules: impl IntoIterator<Item = ErrorCode>) -> String {
    rules
        .into_iter()
        .map(|rule| rule.to_string())
        .sorted_unstable()
        .dedup()
        .join(", ")
}
