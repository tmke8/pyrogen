use pyrogen_source_file::newlines::UniversalNewlines;
use std::{borrow::Cow, cmp};

use crate::PythonWhitespace;

/// Removes common leading whitespace from each line.
///
/// This function will look at each non-empty line and determine the
/// maximum amount of whitespace that can be removed from all lines:
///
/// ```
/// # use pyrogen_python_trivia::textwrap::dedent;
///
/// assert_eq!(dedent("
///     1st line
///       2nd line
///     3rd line
/// "), "
/// 1st line
///   2nd line
/// 3rd line
/// ");
/// ```
pub fn dedent(text: &str) -> Cow<'_, str> {
    // Find the minimum amount of leading whitespace on each line.
    let prefix_len = text
        .universal_newlines()
        .fold(usize::MAX, |prefix_len, line| {
            let leading_whitespace_len = line.len() - line.trim_whitespace_start().len();
            if leading_whitespace_len == line.len() {
                // Skip empty lines.
                prefix_len
            } else {
                cmp::min(prefix_len, leading_whitespace_len)
            }
        });

    // If there is no common prefix, no need to dedent.
    if prefix_len == usize::MAX {
        return Cow::Borrowed(text);
    }

    // Remove the common prefix from each line.
    let mut result = String::with_capacity(text.len());
    for line in text.universal_newlines() {
        if line.trim_whitespace().is_empty() {
            if let Some(line_ending) = line.line_ending() {
                result.push_str(&line_ending);
            }
        } else {
            result.push_str(&line.as_full_str()[prefix_len..]);
        }
    }
    Cow::Owned(result)
}
