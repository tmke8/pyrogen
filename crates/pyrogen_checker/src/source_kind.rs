#[derive(Clone, Debug, PartialEq)]
pub struct SourceKind(
    /// The source contains Python source code.
    String,
);

impl SourceKind {
    pub fn new(source: String) -> Self {
        Self(source)
    }
    #[must_use]
    pub(crate) fn updated(&self, new_source: String) -> Self {
        Self(new_source)
    }

    pub fn source_code(&self) -> &str {
        &self.0
    }
}
