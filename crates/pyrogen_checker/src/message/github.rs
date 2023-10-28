use std::io::Write;

use crate::fs::relativize_path;
use crate::message::{Emitter, Message};
use crate::settings::code_table::MessageKind;

/// Generate error workflow command in GitHub Actions format.
/// See: [GitHub documentation](https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions#setting-an-error-message)
#[derive(Default)]
pub struct GithubEmitter;

impl Emitter for GithubEmitter {
    fn emit(&mut self, writer: &mut dyn Write, messages: &[Message]) -> anyhow::Result<()> {
        for message in messages {
            let source_location = message.compute_start_location();
            let location = source_location.clone();

            let end_location = message.compute_end_location();
            let kind: &str = match message.kind {
                MessageKind::Error => "error",
                MessageKind::Warning => "warning",
            };

            write!(
                writer,
                "::{kind} title=Pyrogen \
                         ({code}),file={file},line={row},col={column},endLine={end_row},endColumn={end_column}::",
                kind = kind,
                code = message.diagnostic.error_code,
                file = message.filename(),
                row = source_location.row,
                column = source_location.column,
                end_row = end_location.row,
                end_column = end_location.column,
            )?;

            writeln!(
                writer,
                "{path}:{row}:{column}: {code} {body}",
                path = relativize_path(message.filename()),
                row = location.row,
                column = location.column,
                code = message.diagnostic.error_code,
                body = message.diagnostic.body,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::message::tests::{capture_emitter_output, create_messages};
    use crate::message::GithubEmitter;

    #[test]
    fn output() {
        let mut emitter = GithubEmitter;
        let content = capture_emitter_output(&mut emitter, &create_messages());

        assert_snapshot!(content);
    }
}
