use anyhow::{anyhow, Result};
use glob::{glob, GlobError, Paths, PatternError};
use pyrogen_cache::cache_dir;
use pyrogen_checker::{
    fs,
    settings::{
        resolve_per_file_ignores,
        rule_table::RuleTable,
        types::{FilePattern, FilePatternSet, PerFileIgnore},
        CheckerSettings,
    },
    ErrorCodeSelector,
};
use shellexpand::LookupError;
use std::{
    borrow::Cow,
    env::VarError,
    path::{Path, PathBuf},
};

use pyrogen_checker::settings::types::PythonVersion;

use crate::options::Options;
use crate::settings::{FileResolverSettings, Settings, EXCLUDE, INCLUDE};

#[derive(Debug, Default)]
pub struct RuleSelection {
    pub select: Option<Vec<ErrorCodeSelector>>,
    pub ignore: Vec<ErrorCodeSelector>,
}

#[derive(Debug, Default)]
pub struct Configuration {
    pub rule_selections: Vec<RuleSelection>,
    pub per_file_ignores: Option<Vec<PerFileIgnore>>,
    pub cache_dir: Option<PathBuf>,
    pub exclude: Option<Vec<FilePattern>>,
    pub force_exclude: Option<bool>,
    pub include: Option<Vec<FilePattern>>,
    pub respect_gitignore: Option<bool>,
    pub target_version: Option<PythonVersion>,
    pub namespace_packages: Option<Vec<PathBuf>>,
    pub src: Option<Vec<PathBuf>>,
}

impl Configuration {
    pub fn into_settings(self, project_root: &Path) -> Result<Settings> {
        let target_version = self.target_version.unwrap_or_default();
        let rules = self.as_rule_table();

        Ok(Settings {
            cache_dir: self
                .cache_dir
                .clone()
                .unwrap_or_else(|| cache_dir(project_root)),

            file_resolver: FileResolverSettings {
                exclude: FilePatternSet::try_from_iter(
                    self.exclude.unwrap_or_else(|| EXCLUDE.to_vec()),
                )?,
                force_exclude: self.force_exclude.unwrap_or(false),
                include: FilePatternSet::try_from_iter(
                    self.include.unwrap_or_else(|| INCLUDE.to_vec()),
                )?,
                respect_gitignore: self.respect_gitignore.unwrap_or(true),
                project_root: project_root.to_path_buf(),
            },
            checker: CheckerSettings {
                project_root: project_root.to_path_buf(),
                rules: rules,
                per_file_ignores: resolve_per_file_ignores(
                    self.per_file_ignores
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                )?,
                target_version: target_version,
                namespace_packages: self.namespace_packages.unwrap_or_default(),
                src: self.src.unwrap_or_else(|| vec![project_root.to_path_buf()]),
            },
        })
    }

    pub fn from_options(options: Options, project_root: &Path) -> Result<Self> {
        Ok(Self {
            rule_selections: vec![RuleSelection {
                select: options.select,
                ignore: options.ignore.into_iter().flatten().collect(),
            }],
            per_file_ignores: options.per_file_ignores.map(|per_file_ignores| {
                per_file_ignores
                    .into_iter()
                    .map(|(pattern, prefixes)| {
                        PerFileIgnore::new(pattern, &prefixes, Some(project_root))
                    })
                    .collect()
            }),
            cache_dir: options
                .cache_dir
                .map(|dir| {
                    let dir = shellexpand::full(&dir);
                    dir.map(|dir| PathBuf::from(dir.as_ref()))
                })
                .transpose()
                .map_err(|e| anyhow!("Invalid `cache-dir` value: {e}"))?,
            exclude: options.exclude.map(|paths| {
                paths
                    .into_iter()
                    .map(|pattern| {
                        let absolute = fs::normalize_path_to(&pattern, project_root);
                        FilePattern::User(pattern, absolute)
                    })
                    .collect()
            }),
            force_exclude: options.force_exclude,
            include: options.include.map(|paths| {
                paths
                    .into_iter()
                    .map(|pattern| {
                        let absolute = fs::normalize_path_to(&pattern, project_root);
                        FilePattern::User(pattern, absolute)
                    })
                    .collect()
            }),
            namespace_packages: options
                .namespace_packages
                .map(|namespace_package| resolve_src(&namespace_package, project_root))
                .transpose()?,
            src: options
                .src
                .map(|src| resolve_src(&src, project_root))
                .transpose()?,
            respect_gitignore: options.respect_gitignore,
            target_version: options.target_version,
        })
    }

    #[must_use]
    pub fn combine(self, config: Self) -> Self {
        Self {
            rule_selections: config
                .rule_selections
                .into_iter()
                .chain(self.rule_selections)
                .collect(),
            per_file_ignores: self.per_file_ignores.or(config.per_file_ignores),
            cache_dir: self.cache_dir.or(config.cache_dir),
            exclude: self.exclude.or(config.exclude),
            force_exclude: self.force_exclude.or(config.force_exclude),
            include: self.include.or(config.include),
            namespace_packages: self.namespace_packages.or(config.namespace_packages),
            respect_gitignore: self.respect_gitignore.or(config.respect_gitignore),
            src: self.src.or(config.src),
            target_version: self.target_version.or(config.target_version),
        }
    }

    pub fn as_rule_table(&self) -> RuleTable {
        RuleTable::empty()
    }
}

/// Given a list of source paths, which could include glob patterns, resolve the
/// matching paths.
pub fn resolve_src(src: &[String], project_root: &Path) -> Result<Vec<PathBuf>> {
    let expansions = src
        .iter()
        .map(shellexpand::full)
        .collect::<Result<Vec<Cow<'_, str>>, LookupError<VarError>>>()?;
    let globs = expansions
        .iter()
        .map(|path| Path::new(path.as_ref()))
        .map(|path| fs::normalize_path_to(path, project_root))
        .map(|path| glob(&path.to_string_lossy()))
        .collect::<Result<Vec<Paths>, PatternError>>()?;
    let paths: Vec<PathBuf> = globs
        .into_iter()
        .flatten()
        .collect::<Result<Vec<PathBuf>, GlobError>>()?;
    Ok(paths)
}
