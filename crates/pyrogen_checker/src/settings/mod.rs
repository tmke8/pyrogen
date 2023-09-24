use std::path::{Path, PathBuf};

use crate::settings::types::PythonVersion;
use path_absolutize::path_dedot;
use pyrogen_macros::CacheKey;

pub mod types;

#[derive(Debug, CacheKey)]
pub struct CheckerSettings {
    pub project_root: PathBuf,

    pub target_version: PythonVersion,
    pub namespace_packages: Vec<PathBuf>,
    pub src: Vec<PathBuf>,
}

impl CheckerSettings {
    pub fn new(project_root: &Path) -> Self {
        Self {
            target_version: PythonVersion::default(),
            project_root: project_root.to_path_buf(),
            namespace_packages: vec![],

            src: vec![path_dedot::CWD.clone()],
        }
    }

    #[must_use]
    pub fn with_target_version(mut self, target_version: PythonVersion) -> Self {
        self.target_version = target_version;
        self
    }
}
