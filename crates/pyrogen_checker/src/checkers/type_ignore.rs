//! `NoQA` enforcement and validation.

use std::path::Path;
use std::str::FromStr;

use itertools::Itertools;
use rustpython_parser::ast::Ranged;

use pyrogen_python_trivia::CommentRanges;
use pyrogen_source_file::Locator;

use crate::registry::{AsErrorCode, Diagnostic, DiagnosticKind, ErrorCode};
use crate::settings::CheckerSettings;
use crate::type_ignore;
use crate::type_ignore::{Directive, FileExemption, TypeIgnoreMapping, TypeIgnores};

#[derive(Debug, PartialEq, Eq)]
pub struct UnusedCodes {
    pub unknown: Vec<String>,
    pub disabled: Vec<String>,
    pub unmatched: Vec<String>,
}

fn unused_type_ignore(codes: Option<UnusedCodes>) -> DiagnosticKind {
    DiagnosticKind {
        body: format!("Unused type ignore directive, with codes: {:?}", codes),
        error_code: ErrorCode::UnusedTypeIgnore,
    }
}

pub(crate) fn check_type_ignore(
    diagnostics: &mut Vec<Diagnostic>,
    path: &Path,
    locator: &Locator,
    comment_ranges: &CommentRanges,
    noqa_line_for: &TypeIgnoreMapping,
    analyze_directives: bool,
    settings: &CheckerSettings,
) -> Vec<usize> {
    // Identify any codes that are globally exempted (within the current file).
    let exemption = FileExemption::try_extract(locator.contents(), comment_ranges, path, locator);

    // Extract all `noqa` directives.
    let mut noqa_directives = TypeIgnores::from_commented_ranges(comment_ranges, path, locator);

    // Indices of diagnostics that were ignored by a `noqa` directive.
    let mut ignored_diagnostics = vec![];

    // Remove any ignored diagnostics.
    'outer: for (index, diagnostic) in diagnostics.iter().enumerate() {
        match &exemption {
            Some(FileExemption::All) => {
                // If the file is exempted, ignore all diagnostics.
                ignored_diagnostics.push(index);
                continue;
            }
            Some(FileExemption::Codes(codes)) => {
                // If the diagnostic is ignored by a global exemption, ignore it.
                if codes.contains(&diagnostic.kind.error_code().to_str()) {
                    ignored_diagnostics.push(index);
                    continue;
                }
            }
            None => {}
        }

        let noqa_offsets = diagnostic
            .parent
            .into_iter()
            .chain(std::iter::once(diagnostic.start()))
            .map(|position| noqa_line_for.resolve(position))
            .unique();

        for noqa_offset in noqa_offsets {
            if let Some(directive_line) = noqa_directives.find_line_with_directive_mut(noqa_offset)
            {
                let suppressed = match &directive_line.directive {
                    Directive::All(_) => {
                        directive_line
                            .matches
                            .push(diagnostic.kind.error_code().to_str());
                        ignored_diagnostics.push(index);
                        true
                    }
                    Directive::Codes(directive) => {
                        if type_ignore::includes(diagnostic.kind.error_code(), directive.codes()) {
                            directive_line
                                .matches
                                .push(diagnostic.kind.error_code().to_str());
                            ignored_diagnostics.push(index);
                            true
                        } else {
                            false
                        }
                    }
                };

                if suppressed {
                    continue 'outer;
                }
            }
        }
    }

    // Enforce that the noqa directive was actually used (RUF100), unless RUF100 was itself
    // suppressed.
    if settings.table.enabled(ErrorCode::UnusedTypeIgnore)
        && analyze_directives
        && !exemption.is_some_and(|exemption| match exemption {
            FileExemption::All => true,
            FileExemption::Codes(codes) => codes.contains(&ErrorCode::UnusedTypeIgnore.to_str()),
        })
    {
        for line in noqa_directives.lines() {
            match &line.directive {
                Directive::All(directive) => {
                    if line.matches.is_empty() {
                        let diagnostic =
                            Diagnostic::new(unused_type_ignore(None), directive.range());
                        diagnostics.push(diagnostic);
                    }
                }
                Directive::Codes(directive) => {
                    let mut disabled_codes = vec![];
                    let mut unknown_codes = vec![];
                    let mut unmatched_codes = vec![];
                    let mut valid_codes = vec![];
                    let mut self_ignore = false;
                    for &code in directive.codes() {
                        if ErrorCode::UnusedTypeIgnore.to_str() == code {
                            self_ignore = true;
                            break;
                        }

                        if line.matches.iter().any(|match_| *match_ == code) {
                            valid_codes.push(code);
                        } else if let Ok(rule) = ErrorCode::from_str(code) {
                            if settings.table.enabled(rule) {
                                unmatched_codes.push(code);
                            } else {
                                disabled_codes.push(code);
                            }
                        } else {
                            unknown_codes.push(code);
                        }
                    }

                    if self_ignore {
                        continue;
                    }

                    if !(disabled_codes.is_empty()
                        && unknown_codes.is_empty()
                        && unmatched_codes.is_empty())
                    {
                        let diagnostic = Diagnostic::new(
                            unused_type_ignore(Some(UnusedCodes {
                                disabled: disabled_codes
                                    .iter()
                                    .map(|code| (*code).to_string())
                                    .collect(),
                                unknown: unknown_codes
                                    .iter()
                                    .map(|code| (*code).to_string())
                                    .collect(),
                                unmatched: unmatched_codes
                                    .iter()
                                    .map(|code| (*code).to_string())
                                    .collect(),
                            })),
                            directive.range(),
                        );
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    ignored_diagnostics.sort_unstable();
    ignored_diagnostics
}
