use std::error::Error;
use std::fmt::Display;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use log::warn;
use rustpython_parser::ast::Ranged;
use rustpython_parser::text_size::{TextLen, TextRange, TextSize};

use pyrogen_python_trivia::CommentRanges;
use pyrogen_source_file::Locator;

use crate::fs::relativize_path;
use crate::registry::ErrorCode;

/// A directive to ignore a set of rules for a given line of Python source code (e.g.,
/// `# type: ignore[call-arg]`).
#[derive(Debug)]
pub(crate) enum Directive<'a> {
    /// The `type: ignore` directive ignores all rules (e.g., `# type: ignore`).
    All(All),
    /// The `type: ignore` directive ignores specific rules (e.g., `# type: ignore[call-arg]`).
    Codes(Codes<'a>),
}

impl<'a> Directive<'a> {
    /// Extract the type-ignore `Directive` from a line of Python source code.
    pub(crate) fn try_extract(text: &'a str, offset: TextSize) -> Result<Option<Self>, ParseError> {
        for (char_index, char) in text.char_indices() {
            // Only bother checking for the `noqa` literal if the character is `n` or `N`.
            if !matches!(char, 't' | 'T') {
                continue;
            }

            // Determine the start of the `type:` literal.
            if !matches!(
                text[char_index..].as_bytes(),
                [b't' | b'T', b'y' | b'Y', b'p' | b'P', b'e' | b'E', b':', ..]
            ) {
                continue;
            }

            let ignore_literal_start = char_index;

            // try to find the start of the "ignore"
            let mut ignore_start = ignore_literal_start + "type:".len();

            // Skip any whitespace between the `:` and the "ignore".
            ignore_start += skip_whitespace(&text[ignore_start..]);

            // Check whether the next characters are "ignore".
            if !matches!(
                text[ignore_start..].as_bytes(),
                [
                    b'i' | b'I',
                    b'g' | b'G',
                    b'n' | b'N',
                    b'o' | b'O',
                    b'r' | b'R',
                    b'e' | b'E',
                    ..
                ]
            ) {
                continue;
            }

            let ignore_literal_end = ignore_start + "ignore".len();

            // Determine the start of the comment.
            let mut comment_start = ignore_literal_start;

            // Trim any whitespace between the `#` character and the `noqa` literal.
            comment_start = text[..comment_start].trim_end().len();

            // The next character has to be the `#` character.
            if text[..comment_start]
                .chars()
                .last()
                .map_or(true, |c| c != '#')
            {
                continue;
            }
            comment_start -= '#'.len_utf8();

            // If the next character is `[`, then it's a list of codes. Otherwise, it's a directive
            // to ignore all rules.
            let directive = match text[ignore_literal_end..].chars().next() {
                Some('[') => {
                    // E.g., `# type: ignore[call-arg,attr-defined]`.
                    let mut codes_start = ignore_literal_end;

                    // Skip the `[` character.
                    codes_start += '['.len_utf8();

                    // Find the closing bracket.
                    let bracket_end = codes_start
                        + text[codes_start..]
                            .find(|c: char| c == ']')
                            .ok_or(ParseError::NoClosingBracket)?;

                    // Skip any whitespace between the `[` and the codes.
                    codes_start += skip_whitespace(&text[codes_start..]);
                    if codes_start >= bracket_end {
                        return Err(ParseError::MissingCodes);
                    }

                    // Extract the comma-separated list of codes.
                    let mut codes = vec![];
                    let mut codes_end = codes_start;

                    while codes_end < bracket_end {
                        // Find next comma, whitespace, or end of bracket.
                        let code_end = text[codes_end..bracket_end]
                            .find(|c: char| c == ',' || c.is_whitespace())
                            .unwrap_or(bracket_end - codes_end);

                        codes.push(&text[codes_end..codes_end + code_end]);
                        codes_end += code_end;

                        // Skip any whitespace.
                        codes_end += skip_whitespace(&text[codes_end..]);

                        if codes_end >= bracket_end {
                            break; // We've reached the closing bracket.
                        }

                        // Verify that the next character is a comma.
                        if text[codes_end..].chars().next().map_or(true, |c| c != ',') {
                            return Err(ParseError::MissingComma);
                        }
                        codes_end += ','.len_utf8();

                        // Skip any whitespace.
                        codes_end += skip_whitespace(&text[codes_end..]);
                    }

                    // If we didn't identify any codes, warn.
                    if codes.is_empty() {
                        return Err(ParseError::MissingCodes);
                    }

                    let range = TextRange::new(
                        TextSize::try_from(comment_start).unwrap(),
                        TextSize::try_from(codes_end).unwrap(),
                    );

                    Self::Codes(Codes {
                        range: range.add(offset),
                        codes,
                    })
                }
                None | Some('#') => {
                    // E.g., `# type: ignore` or `# type:ignore# some comment`.
                    let range = TextRange::new(
                        TextSize::try_from(comment_start).unwrap(),
                        TextSize::try_from(ignore_literal_end).unwrap(),
                    );
                    Self::All(All {
                        range: range.add(offset),
                    })
                }
                Some(c) if c.is_whitespace() => {
                    // Skip any whitespace.
                    let next_char = skip_whitespace(&text[ignore_literal_end..]);
                    if next_char != 0
                        && text[ignore_literal_end + next_char..]
                            .chars()
                            .next()
                            .map_or(true, |c| c != '#')
                    {
                        return Err(ParseError::InvalidSuffix);
                    } else {
                        // E.g., `# type: ignore # some comment`.
                        let range = TextRange::new(
                            TextSize::try_from(comment_start).unwrap(),
                            TextSize::try_from(ignore_literal_end).unwrap(),
                        );
                        Self::All(All {
                            range: range.add(offset),
                        })
                    }
                }
                _ => continue, // There is something weird after "ignore" which makes this invalid
            };

            return Ok(Some(directive));
        }

        Ok(None)
    }
}

#[inline]
fn skip_whitespace(line: &str) -> usize {
    line.find(|c: char| !c.is_whitespace()).unwrap_or(0)
}

#[derive(Debug)]
pub(crate) struct All {
    range: TextRange,
}

impl Ranged for All {
    /// The range of the `noqa` directive.
    fn range(&self) -> TextRange {
        self.range
    }
}

#[derive(Debug)]
pub(crate) struct Codes<'a> {
    range: TextRange,
    codes: Vec<&'a str>,
}

impl Codes<'_> {
    /// The codes that are ignored by the `type: ignore` directive.
    pub(crate) fn codes(&self) -> &[&str] {
        &self.codes
    }
}

impl Ranged for Codes<'_> {
    /// The range of the `type: ignore` directive.
    fn range(&self) -> TextRange {
        self.range
    }
}

/// Returns `true` if the string list of `codes` includes `code`.
pub(crate) fn includes(needle: ErrorCode, haystack: &[&str]) -> bool {
    let needle = needle.to_str();
    haystack.iter().any(|&candidate| needle == candidate)
}

/// Returns `true` if the given [`Rule`] is ignored at the specified `lineno`.
pub(crate) fn rule_is_ignored(
    code: ErrorCode,
    offset: TextSize,
    noqa_line_for: &TypeIgnoreMapping,
    locator: &Locator,
) -> bool {
    let offset = noqa_line_for.resolve(offset);
    let line_range = locator.line_range(offset);
    match Directive::try_extract(locator.slice(line_range), line_range.start()) {
        Ok(Some(Directive::All(_))) => true,
        Ok(Some(Directive::Codes(Codes { codes, range: _ }))) => includes(code, &codes),
        _ => false,
    }
}

/// The file-level exemptions extracted from a given Python file.
#[derive(Debug)]
pub(crate) enum FileExemption {
    /// The file is exempt from all rules.
    All,
    /// The file is exempt from the given rules.
    Codes(Vec<&'static str>),
}

impl FileExemption {
    /// Extract the [`FileExemption`] for a given Python source file, enumerating any rules that are
    /// globally ignored within the file.
    pub(crate) fn try_extract(
        contents: &str,
        comment_ranges: &CommentRanges,
        path: &Path,
        locator: &Locator,
    ) -> Option<Self> {
        let mut exempt_codes: Vec<&'static str> = vec![];

        for range in comment_ranges {
            match ParsedFileExemption::try_extract(&contents[*range]) {
                Err(err) => {
                    #[allow(deprecated)]
                    let line = locator.compute_line_index(range.start());
                    let path_display = relativize_path(path);
                    warn!("Invalid `# type: ignore` directive at {path_display}:{line}: {err}");
                }
                Ok(Some(exemption)) => match exemption {
                    ParsedFileExemption::All => {
                        return Some(Self::All);
                    }
                    ParsedFileExemption::Codes(codes) => {
                        exempt_codes.extend(codes.into_iter().filter_map(|code| {
                                if let Ok(error_code) = ErrorCode::from_str(code)
                                {
                                    Some(error_code.to_str())
                                } else {
                                    #[allow(deprecated)]
                                    let line = locator.compute_line_index(range.start());
                                    let path_display = relativize_path(path);
                                    warn!("Invalid rule code provided to `# ruff: noqa` at {path_display}:{line}: {code}");
                                    None
                                }
                            }));
                    }
                },
                Ok(None) => {}
            }
        }

        if exempt_codes.is_empty() {
            None
        } else {
            Some(Self::Codes(exempt_codes))
        }
    }
}

/// An individual file-level exemption (e.g., `# ruff: noqa` or `# ruff: noqa: F401, F841`). Like
/// [`FileExemption`], but only for a single line, as opposed to an aggregated set of exemptions
/// across a source file.
#[derive(Debug)]
enum ParsedFileExemption<'a> {
    /// The file-level exemption ignores all rules (e.g., `# type: ignore`).
    All,
    /// The file-level exemption ignores specific rules (e.g., `# type: ignore[override]`).
    Codes(Vec<&'a str>),
}

impl<'a> ParsedFileExemption<'a> {
    /// Return a [`ParsedFileExemption`] for a given comment line.
    fn try_extract(line: &'a str) -> Result<Option<Self>, ParseError> {
        Directive::try_extract(line, TextSize::new(0)).map(|directive| {
            directive.map(|directive| match directive {
                Directive::All(_) => Self::All,
                Directive::Codes(Codes { codes, range: _ }) => Self::Codes(codes),
            })
        })
    }
}

/// The result of an [`Importer::get_or_import_symbol`] call.
#[derive(Debug)]
pub(crate) enum ParseError {
    /// The `noqa` directive was missing valid codes (e.g., `# noqa: unused-import` instead of `# noqa: F401`).
    MissingCodes,
    /// The `noqa` directive used an invalid suffix (e.g., `# noqa; F401` instead of `# noqa: F401`).
    InvalidSuffix,
    NoClosingBracket,
    MissingComma,
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingCodes => {
                fmt.write_str("expected a comma-separated list of codes (e.g., `# type: ignore[override,unreachable]`).")
            },
            ParseError::InvalidSuffix => {
                fmt.write_str("after `# type: ignore` the line should continue with brackets or start a new comment with `#`.")
            }
            ParseError::MissingComma => fmt.write_str("expected a comma-separated list of codes (e.g., `# type: ignore[override,unreachable]`)."),
            ParseError::NoClosingBracket => fmt.write_str("bracket after `ignore` directive is not closed.")

        }
    }
}

impl Error for ParseError {}

#[derive(Debug)]
pub(crate) struct TypeIgnoreLine<'a> {
    /// The range of the text line for which the noqa directive applies.
    pub(crate) range: TextRange,
    /// The noqa directive.
    pub(crate) directive: Directive<'a>,
    /// The codes that are ignored by the directive.
    pub(crate) matches: Vec<&'static str>,
}

impl Ranged for TypeIgnoreLine<'_> {
    /// The range of the `noqa` directive.
    fn range(&self) -> TextRange {
        self.range
    }
}

#[derive(Debug, Default)]
pub(crate) struct TypeIgnores<'a> {
    inner: Vec<TypeIgnoreLine<'a>>,
}

impl<'a> TypeIgnores<'a> {
    pub(crate) fn from_commented_ranges(
        comment_ranges: &CommentRanges,
        path: &Path,
        locator: &'a Locator<'a>,
    ) -> Self {
        let mut directives = Vec::new();

        for range in comment_ranges {
            match Directive::try_extract(locator.slice(*range), range.start()) {
                Err(err) => {
                    #[allow(deprecated)]
                    let line = locator.compute_line_index(range.start());
                    let path_display = relativize_path(path);
                    warn!("Invalid `# noqa` directive on {path_display}:{line}: {err}");
                }
                Ok(Some(directive)) => {
                    // noqa comments are guaranteed to be single line.
                    directives.push(TypeIgnoreLine {
                        range: locator.line_range(range.start()),
                        directive,
                        matches: Vec::new(),
                    });
                }
                Ok(None) => {}
            }
        }

        // Extend a mapping at the end of the file to also include the EOF token.
        if let Some(last) = directives.last_mut() {
            if last.range.end() == locator.contents().text_len() {
                last.range = last.range.add_end(TextSize::from(1));
            }
        }

        Self { inner: directives }
    }

    pub(crate) fn find_line_with_directive(&self, offset: TextSize) -> Option<&TypeIgnoreLine> {
        self.find_line_index(offset).map(|index| &self.inner[index])
    }

    pub(crate) fn find_line_with_directive_mut(
        &mut self,
        offset: TextSize,
    ) -> Option<&mut TypeIgnoreLine<'a>> {
        if let Some(index) = self.find_line_index(offset) {
            Some(&mut self.inner[index])
        } else {
            None
        }
    }

    fn find_line_index(&self, offset: TextSize) -> Option<usize> {
        self.inner
            .binary_search_by(|directive| {
                if directive.range.end() < offset {
                    std::cmp::Ordering::Less
                } else if directive.range.contains(offset) {
                    std::cmp::Ordering::Equal
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .ok()
    }

    pub(crate) fn lines(&self) -> &[TypeIgnoreLine] {
        &self.inner
    }
}

/// Remaps offsets falling into one of the ranges to instead check for a "type: ignore" comment on
/// the line specified by the offset.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TypeIgnoreMapping {
    ranges: Vec<TextRange>,
}

impl TypeIgnoreMapping {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            ranges: Vec::with_capacity(capacity),
        }
    }

    /// Returns the re-mapped position or `position` if no mapping exists.
    pub(crate) fn resolve(&self, offset: TextSize) -> TextSize {
        let index = self.ranges.binary_search_by(|range| {
            if range.end() < offset {
                std::cmp::Ordering::Less
            } else if range.contains(offset) {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Greater
            }
        });

        if let Ok(index) = index {
            self.ranges[index].end()
        } else {
            offset
        }
    }

    pub(crate) fn push_mapping(&mut self, range: TextRange) {
        if let Some(last_range) = self.ranges.last_mut() {
            // Strictly sorted insertion
            if last_range.end() < range.start() {
                // OK
            } else if range.end() < last_range.start() {
                // Incoming range is strictly before the last range which violates
                // the function's contract.
                panic!("Ranges must be inserted in sorted order")
            } else {
                // Here, it's guaranteed that `last_range` and `range` overlap
                // in some way. We want to merge them into a single range.
                *last_range = last_range.cover(range);
                return;
            }
        }

        self.ranges.push(range);
    }
}

impl FromIterator<TextRange> for TypeIgnoreMapping {
    fn from_iter<T: IntoIterator<Item = TextRange>>(iter: T) -> Self {
        let mut mappings = TypeIgnoreMapping::default();

        for range in iter {
            mappings.push_mapping(range);
        }

        mappings
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use rustpython_parser::text_size::TextSize;

    use crate::type_ignore::{Directive, ParsedFileExemption};

    #[test]
    fn ignore_all() {
        let source = "# type: ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code() {
        let source = "# type: ignore[unreachable]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes() {
        let source = "# type: ignore[unreachable,override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_all_case_insensitive() {
        let source = "# TYPE: IGNORE";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code_case_insensitive() {
        let source = "# TYPE: IGNORE[override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes_case_insensitive() {
        let source = "# TYPE: IGNORE[override , unreachable]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_leading_space() {
        let source = "#   # type: ignore[override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_trailing_space() {
        let source = "# type: ignore[override]   ";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_all_no_space() {
        let source = "#type:ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code_no_space() {
        let source = "#type:ignore[override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes_no_space() {
        let source = "#type:ignore[override,unreachable]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_all_multi_space() {
        let source = "#  type:  ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code_multi_space() {
        let source = "#  type:  ignore[override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes_multi_space() {
        let source = "#  type:  ignore[ override  , unreachable]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_all_leading_comment() {
        let source = "# Comment describing the ignore # type: ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code_leading_comment() {
        let source = "# Comment describing the ignore # type: ignore[override]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes_leading_comment() {
        let source = "# Comment describing the ignore # type: ignore[override,unreachable]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_all_trailing_comment() {
        let source = "# type: ignore # Comment describing the ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_code_trailing_comment() {
        let source = "# type: ignore[override] # Comment describing the ignore";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_codes_trailing_comment() {
        let source = "# type: ignore[override,unreachable] # Comment describing the noqa";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_invalid_codes() {
        let source = "# type: ignore[code with spaces]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_open_bracket() {
        let source = "# type: ignore[override";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_empty_open_bracket() {
        let source = "# type: ignore[  ";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_empty_bracket() {
        let source = "# type: ignore[  ]";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_invalid_suffix() {
        let source = "# type: ignorea";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_typo() {
        let source = "# type: ignoe";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn noqa_invalid_suffix_with_space() {
        let source = "# type: ignore a";
        assert_debug_snapshot!(Directive::try_extract(source, TextSize::default()));
    }

    #[test]
    fn pyrogen_exemption_all() {
        let source = "# type: ignore";
        assert_debug_snapshot!(ParsedFileExemption::try_extract(source));
    }

    #[test]
    fn pyrogen_exemption_all_no_space() {
        let source = "#type:ignore";
        assert_debug_snapshot!(ParsedFileExemption::try_extract(source));
    }

    #[test]
    fn pyrogen_exemption_codes() {
        let source = "# type: ignore[override,unreachable]";
        assert_debug_snapshot!(ParsedFileExemption::try_extract(source));
    }

    #[test]
    fn pyrogen_exemption_all_case_insensitive() {
        let source = "# type: IgNoRe";
        assert_debug_snapshot!(ParsedFileExemption::try_extract(source));
    }
}
