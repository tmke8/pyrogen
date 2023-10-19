use std::path::Path;

use pyrogen_diagnostics::Diagnostic;
use pyrogen_python_ast::PySourceType;
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::Suite;

use crate::{
    settings::{flags, CheckerSettings},
    type_ignore::NoqaMapping,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_ast(
    python_ast: &Suite,
    locator: &Locator,
    indexer: &Indexer,
    noqa_line_for: &NoqaMapping,
    settings: &CheckerSettings,
    noqa: flags::Noqa,
    path: &Path,
    package: Option<&Path>,
    source_type: PySourceType,
) -> Vec<Diagnostic> {
    todo!("check_ast")
}
