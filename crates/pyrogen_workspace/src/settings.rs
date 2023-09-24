use std::path::{Path, PathBuf};

use path_absolutize::path_dedot;
use pyrogen_cache::cache_dir;
use pyrogen_checker::settings::{
    types::{FilePattern, FilePatternSet},
    CheckerSettings,
};
use pyrogen_macros::CacheKey;

#[derive(Debug, CacheKey)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    #[cache_key(ignore)]
    pub cache_dir: PathBuf,
    pub file_resolver: FileResolverSettings,

    pub checker: CheckerSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let project_root = path_dedot::CWD.as_path();
        Self {
            cache_dir: cache_dir(project_root),
            checker: CheckerSettings::new(project_root),
            file_resolver: FileResolverSettings::new(project_root),
        }
    }
}

pub(crate) static EXCLUDE: &[FilePattern] = &[
    FilePattern::Builtin(".bzr"),
    FilePattern::Builtin(".direnv"),
    FilePattern::Builtin(".eggs"),
    FilePattern::Builtin(".git"),
    FilePattern::Builtin(".git-rewrite"),
    FilePattern::Builtin(".hg"),
    FilePattern::Builtin(".ipynb_checkpoints"),
    FilePattern::Builtin(".mypy_cache"),
    FilePattern::Builtin(".nox"),
    FilePattern::Builtin(".pants.d"),
    FilePattern::Builtin(".pyenv"),
    FilePattern::Builtin(".pytest_cache"),
    FilePattern::Builtin(".pytype"),
    FilePattern::Builtin(".pyrogen_cache"),
    FilePattern::Builtin(".svn"),
    FilePattern::Builtin(".tox"),
    FilePattern::Builtin(".venv"),
    FilePattern::Builtin(".vscode"),
    FilePattern::Builtin("__pypackages__"),
    FilePattern::Builtin("_build"),
    FilePattern::Builtin("buck-out"),
    FilePattern::Builtin("build"),
    FilePattern::Builtin("dist"),
    FilePattern::Builtin("node_modules"),
    FilePattern::Builtin("venv"),
];

pub(crate) static INCLUDE: &[FilePattern] = &[
    FilePattern::Builtin("*.py"),
    FilePattern::Builtin("*.pyi"),
    FilePattern::Builtin("**/pyproject.toml"),
];

#[derive(Debug, CacheKey)]
pub struct FileResolverSettings {
    pub exclude: FilePatternSet,
    pub force_exclude: bool,
    pub include: FilePatternSet,
    pub respect_gitignore: bool,
    pub project_root: PathBuf,
}

impl FileResolverSettings {
    fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            exclude: FilePatternSet::try_from_iter(EXCLUDE.iter().cloned()).unwrap(),
            force_exclude: false,
            respect_gitignore: true,
            include: FilePatternSet::try_from_iter(INCLUDE.iter().cloned()).unwrap(),
        }
    }
}
