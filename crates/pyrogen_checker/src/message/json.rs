use std::io::Write;

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use serde_json::{json, Value};

use pyrogen_source_file::SourceCode;
use rustpython_ast::Ranged;

use crate::message::{Emitter, Message};

#[derive(Default)]
pub struct JsonEmitter;

impl Emitter for JsonEmitter {
    fn emit(&mut self, writer: &mut dyn Write, messages: &[Message]) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(writer, &ExpandedMessages { messages })?;

        Ok(())
    }
}

struct ExpandedMessages<'a> {
    messages: &'a [Message],
}

impl Serialize for ExpandedMessages<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_seq(Some(self.messages.len()))?;

        for message in self.messages {
            let value = message_to_json_value(message);
            s.serialize_element(&value)?;
        }

        s.end()
    }
}

pub(crate) fn message_to_json_value(message: &Message) -> Value {
    let source_code = message.file.to_source_code();

    let start_location = source_code.source_location(message.start());
    let end_location = source_code.source_location(message.end());
    let type_ignore_location = source_code.source_location(message.ignore_offset);

    json!({
        "code": message.diagnostic.error_code.to_string(),
        "message": message.diagnostic.body,
        "location": start_location,
        "end_location": end_location,
        "filename": message.filename(),
        "type_ignore_row": type_ignore_location.row,
        "kind": message.kind.to_string()
    })
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::message::tests::{capture_emitter_output, create_messages};
    use crate::message::JsonEmitter;

    #[test]
    fn output() {
        let mut emitter = JsonEmitter;
        let content = capture_emitter_output(&mut emitter, &create_messages());

        assert_snapshot!(content);
    }
}
