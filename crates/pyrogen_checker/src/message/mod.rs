use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;
use std::ops::Deref;

use rustc_hash::FxHashMap;
use rustpython_parser::ast::Ranged;
use rustpython_parser::text_size::{TextRange, TextSize};

use pyrogen_source_file::{SourceFile, SourceLocation};

pub use github::GithubEmitter;
pub use json::JsonEmitter;
pub use text::TextEmitter;

use crate::registry::{Diagnostic, DiagnosticKind};
use crate::settings::code_table::MessageKind;

// mod diff;
mod github;
mod json;
mod text;

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub diagnostic: DiagnosticKind,
    pub range: TextRange,
    pub file: SourceFile,
    pub ignore_offset: TextSize,
    pub kind: MessageKind,
}

impl Message {
    pub fn from_diagnostic(
        diagnostic: Diagnostic,
        file: SourceFile,
        ignore_offset: TextSize,
        kind: MessageKind,
    ) -> Self {
        Self {
            range: diagnostic.range(),
            diagnostic: diagnostic.kind,
            file,
            ignore_offset,
            kind,
        }
    }

    pub fn filename(&self) -> &str {
        self.file.name()
    }

    pub fn compute_start_location(&self) -> SourceLocation {
        self.file.to_source_code().source_location(self.start())
    }

    pub fn compute_end_location(&self) -> SourceLocation {
        self.file.to_source_code().source_location(self.end())
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.file, self.start()).cmp(&(&other.file, other.start()))
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ranged for Message {
    fn range(&self) -> TextRange {
        self.range
    }
}

struct MessageWithLocation<'a> {
    message: &'a Message,
    start_location: SourceLocation,
}

impl Deref for MessageWithLocation<'_> {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        self.message
    }
}

fn group_messages_by_filename(messages: &[Message]) -> BTreeMap<&str, Vec<MessageWithLocation>> {
    let mut grouped_messages = BTreeMap::default();
    for message in messages {
        grouped_messages
            .entry(message.filename())
            .or_insert_with(Vec::new)
            .push(MessageWithLocation {
                message,
                start_location: message.compute_start_location(),
            });
    }
    grouped_messages
}

/// Display format for a [`Message`]s.
///
/// The emitter serializes a slice of [`Message`]'s and writes them to a [`Write`].
pub trait Emitter {
    /// Serializes the `messages` and writes the output to `writer`.
    fn emit(&mut self, writer: &mut dyn Write, messages: &[Message]) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;
    use rustpython_parser::ast::Ranged;
    use rustpython_parser::text_size::{TextRange, TextSize};

    use pyrogen_source_file::SourceFileBuilder;

    use crate::message::{Emitter, Message};
    use crate::registry::{Diagnostic, DiagnosticKind, ErrorCode};
    use crate::settings::code_table::MessageKind;

    pub(super) fn create_messages() -> Vec<Message> {
        let fib = r#"import os


def fibonacci(n):
    """Compute the nth number in the Fibonacci sequence."""
    x = 1
    if n == 0:
        return 0
    elif n == 1:
        return 1
    else:
        return fibonacci(n - 1) + fibonacci(n - 2)
"#;

        let unused_import = Diagnostic::new(
            DiagnosticKind {
                error_code: ErrorCode::UnusedImport,
                body: "`os` imported but unused".to_string(),
            },
            TextRange::new(TextSize::from(7), TextSize::from(9)),
        );

        let fib_source = SourceFileBuilder::new("fib.py", fib).finish();

        let unused_variable = Diagnostic::new(
            DiagnosticKind {
                error_code: ErrorCode::UnusedVariable,
                body: "Local variable `x` is assigned to but never used".to_string(),
            },
            TextRange::new(TextSize::from(94), TextSize::from(95)),
        );

        let file_2 = r#"if a == 1: pass"#;

        let undefined_name = Diagnostic::new(
            DiagnosticKind {
                error_code: ErrorCode::UndefinedName,
                body: "Undefined name `a`".to_string(),
            },
            TextRange::new(TextSize::from(3), TextSize::from(4)),
        );

        let file_2_source = SourceFileBuilder::new("undef.py", file_2).finish();

        let unused_import_start = unused_import.start();
        let unused_variable_start = unused_variable.start();
        let undefined_name_start = undefined_name.start();
        vec![
            Message::from_diagnostic(
                unused_import,
                fib_source.clone(),
                unused_import_start,
                MessageKind::Warning,
            ),
            Message::from_diagnostic(
                unused_variable,
                fib_source,
                unused_variable_start,
                MessageKind::Warning,
            ),
            Message::from_diagnostic(
                undefined_name,
                file_2_source,
                undefined_name_start,
                MessageKind::Error,
            ),
        ]
    }

    pub(super) fn capture_emitter_output(
        emitter: &mut dyn Emitter,
        messages: &[Message],
    ) -> String {
        let mut output: Vec<u8> = Vec::new();
        emitter.emit(&mut output, messages).unwrap();

        String::from_utf8(output).expect("Output to be valid UTF-8")
    }
}
