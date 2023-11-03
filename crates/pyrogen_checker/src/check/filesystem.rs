use std::path::Path;

use crate::{registry::Diagnostic, settings::CheckerSettings};

pub(crate) fn check_file_path(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
) -> Vec<Diagnostic> {
    let diagnostics: Vec<Diagnostic> = vec![];

    diagnostics
}
