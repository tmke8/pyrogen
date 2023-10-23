use std::path::Path;

use pyrogen_python_ast::{imports::ImportMap, PySourceType};
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::Suite;

use crate::{registry::Diagnostic, settings::CheckerSettings, source_kind::SourceKind};

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_imports(
    python_ast: &Suite,
    locator: &Locator,
    indexer: &Indexer,
    settings: &CheckerSettings,
    path: &Path,
    package: Option<&Path>,
    source_kind: &SourceKind,
    source_type: PySourceType,
) -> (Vec<Diagnostic>, Option<ImportMap>) {
    todo!("check_imports")
}
