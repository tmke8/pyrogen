use rustpython_parser::ast::Ranged;
use rustpython_parser::text_size::{TextRange, TextSize};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiagnosticKind {
    /// The identifier of the diagnostic, used to align the diagnostic with a rule.
    pub name: String,
    /// The message body to display to the user, to explain the diagnostic.
    pub body: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub range: TextRange,
    pub parent: Option<TextSize>,
}

impl Diagnostic {
    pub fn new<T: Into<DiagnosticKind>>(kind: T, range: TextRange) -> Self {
        Self {
            kind: kind.into(),
            range,
            parent: None,
        }
    }

    /// Set the location of the diagnostic's parent node.
    #[inline]
    pub fn set_parent(&mut self, parent: TextSize) {
        self.parent = Some(parent);
    }
}

impl Ranged for Diagnostic {
    fn range(&self) -> TextRange {
        self.range
    }
}
