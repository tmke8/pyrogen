#![cfg(any(test, fuzzing))]
//! Helper functions for the tests of rule implementations.

use std::path::Path;

// use anyhow::Result;

#[cfg(not(fuzzing))]
pub(crate) fn test_resource_path(path: impl AsRef<Path>) -> std::path::PathBuf {
    Path::new("./resources/test/").join(path)
}

// /// Run [`check_path`] on a file in the `resources/test/fixtures` directory.
// #[cfg(not(fuzzing))]
// pub(crate) fn test_path(path: impl AsRef<Path>, settings: &LinterSettings) -> Result<Vec<Message>> {
//     let path = test_resource_path("fixtures").join(path);
//     let contents = std::fs::read_to_string(&path)?;
//     Ok(test_contents(&SourceKind::Python(contents), &path, settings).0)
// }
