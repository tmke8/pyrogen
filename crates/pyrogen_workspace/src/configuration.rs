use anyhow::{anyhow, Result};
use glob::{glob, GlobError, Paths, PatternError};
use rustc_hash::FxHashMap;
use shellexpand::LookupError;
use std::{
    borrow::Cow,
    env::VarError,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;

use pyrogen_cache::cache_dir;
use pyrogen_checker::settings::types::PythonVersion;
use pyrogen_checker::{
    code_selector::Specificity,
    fs,
    registry::{ErrorCode, ErrorCodeSet},
    settings::{
        code_table::ErrorCodeTable,
        resolve_per_file_ignores,
        types::{FilePattern, FilePatternSet, PerFileIgnore},
        CheckerSettings, DEFAULT_ERRORS, DEFAULT_WARNINGS,
    },
    warn_user, ErrorCodeSelector,
};

use crate::options::Options;
use crate::settings::{FileResolverSettings, Settings, EXCLUDE, INCLUDE};

#[derive(Debug, Default)]
pub struct ErrorCodeSelection {
    pub error: Option<Vec<ErrorCodeSelector>>,
    pub extend_error: Vec<ErrorCodeSelector>,
    pub warning: Option<Vec<ErrorCodeSelector>>,
    pub extend_warning: Vec<ErrorCodeSelector>,
    pub ignore: Vec<ErrorCodeSelector>,
}

#[derive(Debug, Default)]
pub struct Configuration {
    pub rule_selections: Vec<ErrorCodeSelection>,
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
                table: rules,
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
            rule_selections: vec![ErrorCodeSelection {
                error: options.error,
                warning: options.warning,
                ignore: options.ignore.into_iter().flatten().collect(),
                extend_error: options.extend_error.unwrap_or_default(),
                extend_warning: options.extend_warning.unwrap_or_default(),
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

    pub fn as_rule_table(&self) -> ErrorCodeTable {
        // The select_set keeps track of which rules have been selected.
        let mut error_set: ErrorCodeSet = DEFAULT_ERRORS
            .iter()
            .flat_map(|selector| selector.rules())
            .collect();

        let mut warning_set: ErrorCodeSet = DEFAULT_WARNINGS
            .iter()
            .flat_map(|selector| selector.rules())
            .collect();

        // Ignores normally only subtract from the current set of selected
        // rules.  By that logic the ignore in `select = [], ignore = ["E501"]`
        // would be effectless. Instead we carry over the ignores to the next
        // selection in that case, creating a way for ignores to be reused
        // across config files (which otherwise wouldn't be possible since ruff
        // only has `extended` but no `extended-by`).
        let mut carryover_ignores: Option<&[ErrorCodeSelector]> = None;

        for selection in &self.rule_selections {
            // If a selection only specifies extend-select we cannot directly
            // apply its rule selectors to the select_set because we firstly have
            // to resolve the effectively selected rules within the current rule selection
            // (taking specificity into account since more specific selectors take
            // precedence over less specific selectors within a rule selection).
            // We do this via the following HashMap where the bool indicates
            // whether to enable or disable the given rule.
            let mut error_map_updates: FxHashMap<ErrorCode, bool> = FxHashMap::default();
            let mut warning_map_updates: FxHashMap<ErrorCode, bool> = FxHashMap::default();

            let carriedover_ignores = carryover_ignores.take();

            for spec in Specificity::iter() {
                // Iterate over rule selectors in order of specificity.
                for selector in selection
                    .error
                    .iter()
                    .flatten()
                    .chain(selection.extend_error.iter())
                    .filter(|s| s.specificity() == spec)
                {
                    for rule in selector.rules() {
                        error_map_updates.insert(rule, true);
                    }
                }
                for selector in selection
                    .ignore
                    .iter()
                    .chain(carriedover_ignores.into_iter().flatten())
                    .filter(|s| s.specificity() == spec)
                {
                    for rule in selector.rules() {
                        error_map_updates.insert(rule, false);
                    }
                }
                // Apply the same logic to `fixable` and `unfixable`.
                for selector in selection
                    .warning
                    .iter()
                    .flatten()
                    .chain(selection.extend_warning.iter())
                    .filter(|s| s.specificity() == spec)
                {
                    for rule in selector.rules() {
                        warning_map_updates.insert(rule, true);
                    }
                }
                for selector in selection
                    .ignore
                    .iter()
                    .chain(carriedover_ignores.into_iter().flatten())
                    .filter(|s| s.specificity() == spec)
                {
                    for rule in selector.rules() {
                        warning_map_updates.insert(rule, false);
                    }
                }
            }

            if let Some(error) = &selection.error {
                // If the `select` option is given we reassign the whole select_set
                // (overriding everything that has been defined previously).
                error_set = error_map_updates
                    .into_iter()
                    .filter_map(|(rule, enabled)| enabled.then_some(rule))
                    .collect();

                if error.is_empty()
                    && selection.extend_error.is_empty()
                    && !selection.ignore.is_empty()
                {
                    carryover_ignores = Some(&selection.ignore);
                }
            } else {
                // Otherwise we apply the updates on top of the existing select_set.
                for (rule, enabled) in error_map_updates {
                    if enabled {
                        error_set.insert(rule);
                    } else {
                        error_set.remove(rule);
                    }
                }
            }

            // Apply the same logic for warnings.
            if let Some(warning) = &selection.warning {
                // If the `select` option is given we reassign the whole select_set
                // (overriding everything that has been defined previously).
                warning_set = warning_map_updates
                    .into_iter()
                    .filter_map(|(rule, enabled)| enabled.then_some(rule))
                    .collect();

                if warning.is_empty()
                    && selection.extend_warning.is_empty()
                    && !selection.ignore.is_empty()
                {
                    carryover_ignores = Some(&selection.ignore);
                }
            } else {
                // Otherwise we apply the updates on top of the existing select_set.
                for (rule, enabled) in warning_map_updates {
                    if enabled {
                        warning_set.insert(rule);
                    } else {
                        warning_set.remove(rule);
                    }
                }
            }
        }

        let mut table = ErrorCodeTable::empty();

        for code in error_set {
            if warning_set.contains(code) {
                warn_user!(
                    "Code `{}` is both an error and a warning. Treating as warning.",
                    code
                )
            }
            table.enable_error(code);
        }
        for code in warning_set {
            table.enable_warning(code);
        }
        table
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
