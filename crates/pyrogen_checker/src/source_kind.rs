use pyrogen_diagnostics::SourceMap;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceKind(
    /// The source contains Python source code.
    String,
);

impl SourceKind {
    #[must_use]
    pub(crate) fn updated(&self, new_source: String, source_map: &SourceMap) -> Self {
        Self(new_source)
    }

    pub fn source_code(&self) -> &str {
        &self.0
    }
}
