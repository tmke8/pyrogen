use std::path::Path;

use anyhow::Result;

use pyrogen_checker::packaging;
use pyrogen_checker::settings::flags;
use pyrogen_workspace::resolver::{python_file_at_path, PyprojectConfig};

use crate::args::CliOverrides;
use crate::diagnostics::{type_check_stdin, Messages};
use crate::stdin::read_from_stdin;

/// Run the linter over a single file, read from `stdin`.
pub(crate) fn check_stdin(
    filename: Option<&Path>,
    pyproject_config: &PyprojectConfig,
    overrides: &CliOverrides,
    respect_type_ignore: flags::TypeIgnore,
) -> Result<Messages> {
    if let Some(filename) = filename {
        if !python_file_at_path(filename, pyproject_config, overrides)? {
            return Ok(Messages::default());
        }
    }
    let package_root = filename.and_then(Path::parent).and_then(|path| {
        packaging::detect_package_root(path, &pyproject_config.settings.checker.namespace_packages)
    });
    let stdin = read_from_stdin()?;
    let mut diagnostics = type_check_stdin(
        filename,
        package_root,
        stdin,
        &pyproject_config.settings,
        respect_type_ignore,
    )?;
    diagnostics.messages.sort_unstable();
    Ok(diagnostics)
}
