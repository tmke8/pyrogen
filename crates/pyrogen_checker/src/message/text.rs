use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io::Write;

use annotate_snippets::display_list::{DisplayList, FormatOptions};
use annotate_snippets::snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation};
use bitflags::bitflags;
use colored::Colorize;

use pyrogen_source_file::OneIndexed;
use rustpython_parser::text_size::{TextRange, TextSize};

use crate::fs::relativize_path;
use crate::line_width::{LineWidthBuilder, TabSize};
use crate::message::{Emitter, Message};
use crate::registry::AsErrorCode;
use crate::settings::code_table::MessageKind;

bitflags! {
    #[derive(Default)]
    struct EmitterFlags: u8 {
        /// Whether to show the fix status of a diagnostic.
        const SHOW_FIX_STATUS = 0b0000_0001;
        /// Whether to show the diff of a fix, for diagnostics that have a fix.
        const SHOW_FIX_DIFF   = 0b0000_0010;
        /// Whether to show the source code of a diagnostic.
        const SHOW_SOURCE     = 0b0000_0100;
    }
}

#[derive(Default)]
pub struct TextEmitter {
    flags: EmitterFlags,
}

impl TextEmitter {
    #[must_use]
    pub fn with_show_fix_status(mut self, show_fix_status: bool) -> Self {
        self.flags
            .set(EmitterFlags::SHOW_FIX_STATUS, show_fix_status);
        self
    }

    #[must_use]
    pub fn with_show_fix_diff(mut self, show_fix_diff: bool) -> Self {
        self.flags.set(EmitterFlags::SHOW_FIX_DIFF, show_fix_diff);
        self
    }

    #[must_use]
    pub fn with_show_source(mut self, show_source: bool) -> Self {
        self.flags.set(EmitterFlags::SHOW_SOURCE, show_source);
        self
    }
}

impl Emitter for TextEmitter {
    fn emit(&mut self, writer: &mut dyn Write, messages: &[Message]) -> anyhow::Result<()> {
        for message in messages {
            write!(
                writer,
                "{path}{sep}",
                path = relativize_path(message.filename()).bold(),
                sep = ":".cyan(),
            )?;

            let start_location = message.compute_start_location();

            let diagnostic_location = start_location;

            writeln!(
                writer,
                "{row}{sep}{col}{sep} {code_and_body}",
                row = diagnostic_location.row,
                col = diagnostic_location.column,
                sep = ":".cyan(),
                code_and_body = RuleCodeAndBody { message }
            )?;

            if self.flags.intersects(EmitterFlags::SHOW_SOURCE) {
                writeln!(writer, "{}", MessageCodeFrame { message })?;
            }
        }

        Ok(())
    }
}

pub(super) struct RuleCodeAndBody<'a> {
    pub(crate) message: &'a Message,
}

impl Display for RuleCodeAndBody<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let diagnostic = &self.message.diagnostic;

        match &self.message.kind {
            MessageKind::Error => {
                write!(
                    f,
                    "error: {body} [{code}]",
                    code = diagnostic.error_code().to_string().red().bold(),
                    body = diagnostic.body,
                )
            }
            MessageKind::Warning => {
                write!(
                    f,
                    "warn: {body} [{code}]",
                    code = diagnostic.error_code().to_string().yellow().bold(),
                    body = diagnostic.body,
                )
            }
        }
    }
}

pub(super) struct MessageCodeFrame<'a> {
    pub(crate) message: &'a Message,
}

impl Display for MessageCodeFrame<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Message {
            diagnostic: kind,
            file,
            range,
            ..
        } = self.message;

        let footer = Vec::new();

        let source_code = file.to_source_code();

        let content_start_index = source_code.line_index(range.start());
        let mut start_index = content_start_index.saturating_sub(2);

        // Trim leading empty lines.
        while start_index < content_start_index {
            if !source_code.line_text(start_index).trim().is_empty() {
                break;
            }
            start_index = start_index.saturating_add(1);
        }

        let content_end_index = source_code.line_index(range.end());
        let mut end_index = content_end_index
            .saturating_add(2)
            .min(OneIndexed::from_zero_indexed(source_code.line_count()));

        // Trim trailing empty lines.
        while end_index > content_end_index {
            if !source_code.line_text(end_index).trim().is_empty() {
                break;
            }

            end_index = end_index.saturating_sub(1);
        }

        let start_offset = source_code.line_start(start_index);
        let end_offset = source_code.line_end(end_index);

        let source = replace_whitespace(
            source_code.slice(TextRange::new(start_offset, end_offset)),
            range - start_offset,
        );

        let start_char = source.text[TextRange::up_to(source.annotation_range.start())]
            .chars()
            .count();

        let char_length = source.text[source.annotation_range].chars().count();

        let label = kind.error_code().to_string();

        let snippet = Snippet {
            title: None,
            slices: vec![Slice {
                source: &source.text,
                line_start: start_index.get(),
                annotations: vec![SourceAnnotation {
                    label: &label,
                    annotation_type: AnnotationType::Error,
                    range: (start_char, start_char + char_length),
                }],
                // The origin (file name, line number, and column number) is already encoded
                // in the `label`.
                origin: None,
                fold: false,
            }],
            footer,
            opt: FormatOptions {
                #[cfg(test)]
                color: false,
                #[cfg(not(test))]
                color: colored::control::SHOULD_COLORIZE.should_colorize(),
                ..FormatOptions::default()
            },
        };

        writeln!(f, "{message}", message = DisplayList::from(snippet))
    }
}

fn replace_whitespace(source: &str, annotation_range: TextRange) -> SourceCode {
    let mut result = String::new();
    let mut last_end = 0;
    let mut range = annotation_range;
    let mut line_width = LineWidthBuilder::new(TabSize::default());

    for (index, c) in source.char_indices() {
        let old_width = line_width.get();
        line_width = line_width.add_char(c);

        if matches!(c, '\t') {
            // SAFETY: The difference is a value in the range [1..TAB_SIZE] which is guaranteed to be less than `u32`.
            #[allow(clippy::cast_possible_truncation)]
            let tab_width = (line_width.get() - old_width) as u32;

            if index < usize::from(annotation_range.start()) {
                range += TextSize::new(tab_width - 1);
            } else if index < usize::from(annotation_range.end()) {
                range = range.add_end(TextSize::new(tab_width - 1));
            }

            result.push_str(&source[last_end..index]);

            for _ in 0..tab_width {
                result.push(' ');
            }

            last_end = index + 1;
        }
    }

    // No tabs
    if result.is_empty() {
        SourceCode {
            annotation_range,
            text: Cow::Borrowed(source),
        }
    } else {
        result.push_str(&source[last_end..]);
        SourceCode {
            annotation_range: range,
            text: Cow::Owned(result),
        }
    }
}

struct SourceCode<'a> {
    text: Cow<'a, str>,
    annotation_range: TextRange,
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::message::tests::{capture_emitter_output, create_messages};
    use crate::message::TextEmitter;

    #[test]
    fn default() {
        let mut emitter = TextEmitter::default().with_show_source(true);
        let content = capture_emitter_output(&mut emitter, &create_messages());

        assert_snapshot!(content);
    }

    #[test]
    fn fix_status() {
        let mut emitter = TextEmitter::default()
            .with_show_fix_status(true)
            .with_show_source(true);
        let content = capture_emitter_output(&mut emitter, &create_messages());

        assert_snapshot!(content);
    }
}
