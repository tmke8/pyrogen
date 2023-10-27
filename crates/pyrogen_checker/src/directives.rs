//! Extract `# noqa`, `# isort: skip`, and `# TODO` directives from tokenized source.

use rustpython_parser::lexer::LexResult;
use rustpython_parser::text_size::TextRange;
use rustpython_parser::Tok;

use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;

use crate::type_ignore::NoqaMapping;

/// Extract a mapping from logical line to noqa line.
pub fn extract_noqa_line_for(
    lxr: &[LexResult],
    locator: &Locator,
    indexer: &Indexer,
) -> NoqaMapping {
    let mut string_mappings = Vec::new();

    for (tok, range) in lxr.iter().flatten() {
        match tok {
            Tok::EndOfFile => {
                break;
            }

            // For multi-line strings, we expect `noqa` directives on the last line of the
            // string.
            Tok::String {
                triple_quoted: true,
                ..
            } => {
                if locator.contains_line_break(*range) {
                    string_mappings.push(TextRange::new(
                        locator.line_start(range.start()),
                        range.end(),
                    ));
                }
            }

            _ => {}
        }
    }

    let mut continuation_mappings = Vec::new();

    // For continuations, we expect `noqa` directives on the last line of the
    // continuation.
    let mut last: Option<TextRange> = None;
    for continuation_line in indexer.continuation_line_starts() {
        let line_end = locator.full_line_end(*continuation_line);
        if let Some(last_range) = last.take() {
            if last_range.end() == *continuation_line {
                last = Some(TextRange::new(last_range.start(), line_end));
                continue;
            }
            // new continuation
            continuation_mappings.push(last_range);
        }

        last = Some(TextRange::new(*continuation_line, line_end));
    }

    if let Some(last_range) = last.take() {
        continuation_mappings.push(last_range);
    }

    // Merge the mappings in sorted order
    let mut mappings =
        NoqaMapping::with_capacity(continuation_mappings.len() + string_mappings.len());

    let mut continuation_mappings = continuation_mappings.into_iter().peekable();
    let mut string_mappings = string_mappings.into_iter().peekable();

    while let (Some(continuation), Some(string)) =
        (continuation_mappings.peek(), string_mappings.peek())
    {
        if continuation.start() <= string.start() {
            mappings.push_mapping(continuation_mappings.next().unwrap());
        } else {
            mappings.push_mapping(string_mappings.next().unwrap());
        }
    }

    for mapping in continuation_mappings {
        mappings.push_mapping(mapping);
    }

    for mapping in string_mappings {
        mappings.push_mapping(mapping);
    }

    mappings
}

#[cfg(test)]
mod tests {
    use rustpython_parser::lexer::LexResult;
    use rustpython_parser::text_size::{TextLen, TextRange, TextSize};
    use rustpython_parser::{lexer, Mode};

    use pyrogen_python_index::Indexer;
    use pyrogen_source_file::Locator;

    use crate::directives::extract_noqa_line_for;
    use crate::type_ignore::NoqaMapping;

    fn noqa_mappings(contents: &str) -> NoqaMapping {
        let lxr: Vec<LexResult> = lexer::lex(contents, Mode::Module).collect();
        let locator = Locator::new(contents);
        let indexer = Indexer::from_tokens(&lxr, &locator);

        extract_noqa_line_for(&lxr, &locator, &indexer)
    }

    #[test]
    fn noqa_extraction() {
        let contents = "x = 1
y = 2 \
    + 1
z = x + 1";

        assert_eq!(noqa_mappings(contents), NoqaMapping::default());

        let contents = "
x = 1
y = 2
z = x + 1";
        assert_eq!(noqa_mappings(contents), NoqaMapping::default());

        let contents = "x = 1
y = 2
z = x + 1
        ";
        assert_eq!(noqa_mappings(contents), NoqaMapping::default());

        let contents = "x = 1

y = 2
z = x + 1
        ";
        assert_eq!(noqa_mappings(contents), NoqaMapping::default());

        let contents = "x = '''abc
def
ghi
'''
y = 2
z = x + 1";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(0), TextSize::from(22))])
        );

        let contents = "x = 1
y = '''abc
def
ghi
'''
z = 2";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(6), TextSize::from(28))])
        );

        let contents = "x = 1
y = '''abc
def
ghi
'''";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(6), TextSize::from(28))])
        );

        let contents = r"x = \
    1";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(0), TextSize::from(6))])
        );

        let contents = r"from foo import \
    bar as baz, \
    qux as quux";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(0), TextSize::from(36))])
        );

        let contents = r"
# Foo
from foo import \
    bar as baz, \
    qux as quux # Baz
x = \
    1
y = \
    2";
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([
                TextRange::new(TextSize::from(7), TextSize::from(43)),
                TextRange::new(TextSize::from(65), TextSize::from(71)),
                TextRange::new(TextSize::from(77), TextSize::from(83)),
            ])
        );

        // https://github.com/astral-sh/ruff/issues/7530
        let contents = r"
assert foo, \
    '''triple-quoted
    string'''
"
        .trim();
        assert_eq!(
            noqa_mappings(contents),
            NoqaMapping::from_iter([TextRange::new(TextSize::from(0), TextSize::from(48))])
        );
    }
}
