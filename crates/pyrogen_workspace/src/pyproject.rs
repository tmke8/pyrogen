//! Utilities for locating (and extracting configuration from) a pyproject.toml.

use std::path::{Path, PathBuf};

use anyhow::Result;
use pep440_rs::VersionSpecifiers;
use serde::{Deserialize, Serialize};

use pyrogen_checker::settings::types::PythonVersion;

use crate::options::Options;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Tools {
    pyrogen: Option<Options>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
struct Project {
    #[serde(alias = "requires-python", alias = "requires_python")]
    requires_python: Option<VersionSpecifiers>,
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Pyproject {
    tool: Option<Tools>,
    project: Option<Project>,
}

impl Pyproject {
    pub const fn new(options: Options) -> Self {
        Self {
            tool: Some(Tools {
                pyrogen: Some(options),
            }),
            project: None,
        }
    }
}

// /// Parse a `pyrogen.toml` file.
// fn parse_pyrogen_toml<P: AsRef<Path>>(path: P) -> Result<Options> {
//     let contents = std::fs::read_to_string(path)?;
//     toml::from_str(&contents).map_err(Into::into)
// }

/// Parse a `pyproject.toml` file.
fn parse_pyproject_toml<P: AsRef<Path>>(path: P) -> Result<Pyproject> {
    let contents = std::fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(Into::into)
}

/// Return `true` if a `pyproject.toml` contains a `[tool.pyrogen]` section.
pub fn pyrogen_enabled<P: AsRef<Path>>(path: P) -> Result<bool> {
    let pyproject = parse_pyproject_toml(path)?;
    Ok(pyproject.tool.and_then(|tool| tool.pyrogen).is_some())
}

/// Return the path to the `pyproject.toml` file in a given
/// directory.
pub fn settings_toml<P: AsRef<Path>>(path: P) -> Result<Option<PathBuf>> {
    // // Check for `.pyrogen.toml`.
    // let pyrogen_toml = path.as_ref().join(".pyrogen.toml");
    // if pyrogen_toml.is_file() {
    //     return Ok(Some(pyrogen_toml));
    // }

    // // Check for `pyrogen.toml`.
    // let pyrogen_toml = path.as_ref().join("pyrogen.toml");
    // if pyrogen_toml.is_file() {
    //     return Ok(Some(pyrogen_toml));
    // }

    // Check for `pyproject.toml`.
    let pyproject_toml = path.as_ref().join("pyproject.toml");
    if pyproject_toml.is_file() && pyrogen_enabled(&pyproject_toml)? {
        return Ok(Some(pyproject_toml));
    }

    Ok(None)
}

/// Find the path to the `pyproject.toml` file, if such a file
/// exists.
pub fn find_settings_toml<P: AsRef<Path>>(path: P) -> Result<Option<PathBuf>> {
    for directory in path.as_ref().ancestors() {
        if let Some(pyproject) = settings_toml(directory)? {
            return Ok(Some(pyproject));
        }
    }
    Ok(None)
}

/// Find the path to the user-specific `pyproject.toml`, if it
/// exists.
pub fn find_user_settings_toml() -> Option<PathBuf> {
    // // Search for a user-specific `.pyrogen.toml`.
    // let mut path = dirs::config_dir()?;
    // path.push("pyrogen");
    // path.push(".pyrogen.toml");
    // if path.is_file() {
    //     return Some(path);
    // }

    // // Search for a user-specific `pyrogen.toml`.
    // let mut path = dirs::config_dir()?;
    // path.push("pyrogen");
    // path.push("pyrogen.toml");
    // if path.is_file() {
    //     return Some(path);
    // }

    // Search for a user-specific `pyproject.toml`.
    let mut path = dirs::config_dir()?;
    path.push("pyrogen");
    path.push("pyproject.toml");
    if path.is_file() {
        return Some(path);
    }

    None
}

/// Load `Options` from a `pyproject.toml` file.
pub fn load_options<P: AsRef<Path>>(path: P) -> Result<Options> {
    let pyproject = parse_pyproject_toml(&path)?;
    let mut pyrogen = pyproject
        .tool
        .and_then(|tool| tool.pyrogen)
        .unwrap_or_default();
    if pyrogen.target_version.is_none() {
        if let Some(project) = pyproject.project {
            if let Some(requires_python) = project.requires_python {
                pyrogen.target_version =
                    PythonVersion::get_minimum_supported_version(&requires_python);
            }
        }
    }
    Ok(pyrogen)
    // else {
    //     let pyrogen = parse_pyrogen_toml(path);
    //     if let Ok(pyrogen) = &pyrogen {
    //         if pyrogen.target_version.is_none() {
    //             debug!("`project.requires_python` in `pyproject.toml` will not be used to set `target_version` when using `pyrogen.toml`.");
    //         }
    //     }
    //     pyrogen
    // }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::options::Options;
    use crate::pyproject::{find_settings_toml, parse_pyproject_toml, Pyproject, Tools};
    use crate::tests::test_resource_path;

    #[test]

    fn deserialize() -> Result<()> {
        let pyproject: Pyproject = toml::from_str(r#""#)?;
        assert_eq!(pyproject.tool, None);

        let pyproject: Pyproject = toml::from_str(
            r#"
[tool.black]
"#,
        )?;
        assert_eq!(pyproject.tool, Some(Tools { pyrogen: None }));

        let pyproject: Pyproject = toml::from_str(
            r#"
[tool.black]
[tool.pyrogen]
"#,
        )?;
        assert_eq!(
            pyproject.tool,
            Some(Tools {
                pyrogen: Some(Options::default())
            })
        );

        let pyproject: Pyproject = toml::from_str(
            r#"
[tool.black]
[tool.pyrogen]
exclude = ["foo.py"]
"#,
        )?;
        assert_eq!(
            pyproject.tool,
            Some(Tools {
                pyrogen: Some(Options {
                    exclude: Some(vec!["foo.py".to_string()]),
                    ..Options::default()
                })
            })
        );

        assert!(toml::from_str::<Pyproject>(
            r#"
[tool.black]
[tool.pyrogen]
line_length = 79
"#,
        )
        .is_err());

        assert!(toml::from_str::<Pyproject>(
            r#"
[tool.black]
[tool.pyrogen]
select = ["E123"]
"#,
        )
        .is_err());

        assert!(toml::from_str::<Pyproject>(
            r#"
[tool.black]
[tool.pyrogen]
line-length = 79
other-attribute = 1
"#,
        )
        .is_err());

        Ok(())
    }

    #[test]
    fn find_and_parse_pyproject_toml() -> Result<()> {
        let pyproject = find_settings_toml(test_resource_path("fixtures/__init__.py"))?.unwrap();
        assert_eq!(pyproject, test_resource_path("fixtures/pyproject.toml"));

        let pyproject = parse_pyproject_toml(&pyproject)?;
        let config = pyproject.tool.unwrap().pyrogen.unwrap();
        // let exlude: Option<Vec<String>> = Some(vec!["examples/excluded".to_string()]);
        assert_eq!(
            config,
            Options {
                exclude: Some(vec!["examples/excluded".to_string()]),
                cache_dir: Some(".checker_cache".to_string()),
                ..Options::default()
            }
        );

        Ok(())
    }
}
