use std::path::{Path, PathBuf};

use crate::{
    registry::{ErrorCode, ErrorCodeSet},
    settings::types::PythonVersion,
};
use anyhow::Result;
use globset::{Glob, GlobMatcher};
use path_absolutize::path_dedot;
use pyrogen_macros::CacheKey;

use self::{rule_table::RuleTable, types::PerFileIgnore};

pub mod flags;
pub mod rule_table;
pub mod types;

#[derive(Debug, CacheKey)]
pub struct CheckerSettings {
    pub project_root: PathBuf,
    pub rules: RuleTable,
    pub per_file_ignores: Vec<(GlobMatcher, GlobMatcher, ErrorCodeSet)>,

    pub target_version: PythonVersion,
    pub namespace_packages: Vec<PathBuf>,
    pub src: Vec<PathBuf>,
}

impl CheckerSettings {
    pub fn new(project_root: &Path) -> Self {
        Self {
            target_version: PythonVersion::default(),
            project_root: project_root.to_path_buf(),
            rules: RuleTable::from_iter(vec![ErrorCode::SyntaxError].into_iter()),
            namespace_packages: vec![],
            per_file_ignores: vec![],

            src: vec![path_dedot::CWD.clone()],
        }
    }

    #[must_use]
    pub fn with_target_version(mut self, target_version: PythonVersion) -> Self {
        self.target_version = target_version;
        self
    }

    pub fn for_rule(rule_code: ErrorCode) -> Self {
        Self {
            rules: RuleTable::from_iter([rule_code]),
            target_version: PythonVersion::latest(),
            ..Self::default()
        }
    }

    pub fn for_rules(rules: impl IntoIterator<Item = ErrorCode>) -> Self {
        Self {
            rules: RuleTable::from_iter(rules),
            target_version: PythonVersion::latest(),
            ..Self::default()
        }
    }
}

impl Default for CheckerSettings {
    fn default() -> Self {
        Self::new(path_dedot::CWD.as_path())
    }
}

/// Given a list of patterns, create a `GlobSet`.
pub fn resolve_per_file_ignores(
    per_file_ignores: Vec<PerFileIgnore>,
) -> Result<Vec<(GlobMatcher, GlobMatcher, ErrorCodeSet)>> {
    per_file_ignores
        .into_iter()
        .map(|per_file_ignore| {
            // Construct absolute path matcher.
            let absolute =
                Glob::new(&per_file_ignore.absolute.to_string_lossy())?.compile_matcher();

            // Construct basename matcher.
            let basename = Glob::new(&per_file_ignore.basename)?.compile_matcher();

            Ok((absolute, basename, per_file_ignore.rules))
        })
        .collect()
}
