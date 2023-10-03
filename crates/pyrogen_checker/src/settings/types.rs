use std::{hash::Hasher, ops::Deref, path::PathBuf, str::FromStr};

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use pep440_rs::{Version as Pep440Version, VersionSpecifiers};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use pyrogen_cache::{CacheKey, CacheKeyHasher};
use pyrogen_macros::CacheKey;

use crate::fs;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    CacheKey,
    EnumIter,
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum PythonVersion {
    #[default]
    Py310,
    Py311,
    Py312,
}

impl From<PythonVersion> for Pep440Version {
    fn from(version: PythonVersion) -> Self {
        let (major, minor) = version.as_tuple();
        Self::from_str(&format!("{major}.{minor}.100")).unwrap()
    }
}

impl PythonVersion {
    /// Return the latest supported Python version.
    pub const fn latest() -> Self {
        Self::Py312
    }

    pub const fn as_tuple(&self) -> (u8, u8) {
        match self {
            Self::Py310 => (3, 10),
            Self::Py311 => (3, 11),
            Self::Py312 => (3, 12),
        }
    }

    pub const fn major(&self) -> u8 {
        self.as_tuple().0
    }

    pub const fn minor(&self) -> u8 {
        self.as_tuple().1
    }

    pub fn get_minimum_supported_version(requires_version: &VersionSpecifiers) -> Option<Self> {
        let mut minimum_version = None;
        for python_version in PythonVersion::iter() {
            if requires_version
                .iter()
                .all(|specifier| specifier.contains(&python_version.into()))
            {
                minimum_version = Some(python_version);
                break;
            }
        }
        minimum_version
    }
}

#[derive(Debug, Clone, CacheKey, PartialEq, PartialOrd, Eq, Ord)]
pub enum FilePattern {
    Builtin(&'static str),
    User(String, PathBuf),
}

impl FilePattern {
    pub fn add_to(self, builder: &mut GlobSetBuilder) -> Result<()> {
        match self {
            FilePattern::Builtin(pattern) => {
                builder.add(Glob::from_str(pattern)?);
            }
            FilePattern::User(pattern, absolute) => {
                // Add the absolute path.
                builder.add(Glob::new(&absolute.to_string_lossy())?);

                // Add basename path.
                if !pattern.contains(std::path::MAIN_SEPARATOR) {
                    builder.add(Glob::from_str(&pattern)?);
                }
            }
        }
        Ok(())
    }
}

impl FromStr for FilePattern {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pattern = s.to_string();
        let absolute = fs::normalize_path(&pattern);
        Ok(Self::User(pattern, absolute))
    }
}

#[derive(Debug, Clone, Default)]
pub struct FilePatternSet {
    set: GlobSet,
    cache_key: u64,
}

impl FilePatternSet {
    pub fn try_from_iter<I>(patterns: I) -> Result<Self, anyhow::Error>
    where
        I: IntoIterator<Item = FilePattern>,
    {
        let mut builder = GlobSetBuilder::new();
        let mut hasher = CacheKeyHasher::new();

        for pattern in patterns {
            pattern.cache_key(&mut hasher);
            pattern.add_to(&mut builder)?;
        }

        let set = builder.build()?;

        Ok(FilePatternSet {
            set,
            cache_key: hasher.finish(),
        })
    }
}

impl Deref for FilePatternSet {
    type Target = GlobSet;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl CacheKey for FilePatternSet {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        state.write_usize(self.set.len());
        state.write_u64(self.cache_key);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum SerializationFormat {
    Text,
}

impl Default for SerializationFormat {
    fn default() -> Self {
        Self::Text
    }
}
