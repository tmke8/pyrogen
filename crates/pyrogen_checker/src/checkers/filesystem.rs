use std::path::Path;

use pyrogen_diagnostics::Diagnostic;

use crate::settings::CheckerSettings;

pub(crate) fn check_file_path(
    path: &Path,
    package: Option<&Path>,
    settings: &CheckerSettings,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];

    diagnostics
}
