use pyrogen_macros::OptionsMetadata;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use pyrogen_checker::{settings::types::PythonVersion, ErrorCodeSelector};

#[derive(Debug, PartialEq, Eq, Default, OptionsMetadata, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Options {
    /// A path to the cache directory.
    ///
    /// By default, Pyrogen stores cache results in a `.pyrogen_cache` directory in
    /// the current project root.
    ///
    /// However, Pyrogen will also respect the `PYROGEN_CACHE_DIR` environment
    /// variable, which takes precedence over that default.
    ///
    /// This setting will override even the `PYROGEN_CACHE_DIR` environment
    /// variable, if set.
    #[option(
        default = ".pyrogen_cache",
        value_type = "str",
        example = r#"cache-dir = "~/.cache/pyrogen""#
    )]
    pub cache_dir: Option<String>,

    /// A list of rule codes or prefixes to ignore. Prefixes can specify exact
    /// rules (like `F841`), entire categories (like `F`), or anything in
    /// between.
    ///
    /// When breaking ties between enabled and disabled rules (via `select` and
    /// `ignore`, respectively), more specific prefixes override less
    /// specific prefixes.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Skip unused variable rules (`F841`).
            ignore = ["F841"]
        "#
    )]
    pub ignore: Option<Vec<ErrorCodeSelector>>,

    /// A list of rule codes or prefixes to enable. Prefixes can specify exact
    /// rules (like `F841`), entire categories (like `F`), or anything in
    /// between.
    ///
    /// When breaking ties between enabled and disabled rules (via `select` and
    /// `ignore`, respectively), more specific prefixes override less
    /// specific prefixes.
    #[option(
        default = r#"["E", "F"]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of the defaults (`E`, `F`), enable flake8-bugbear (`B`) and flake8-quotes (`Q`).
            select = ["E", "F", "B", "Q"]
        "#
    )]
    pub warning: Option<Vec<ErrorCodeSelector>>,

    /// A list of rule codes or prefixes to enable, in addition to those
    /// specified by `error`.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of the default `select` (`E`, `F`), enable flake8-bugbear (`B`) and flake8-quotes (`Q`).
            extend-select = ["B", "Q"]
        "#
    )]
    pub extend_warning: Option<Vec<ErrorCodeSelector>>,

    /// A list of rule codes or prefixes to enable. Prefixes can specify exact
    /// rules (like `F841`), entire categories (like `F`), or anything in
    /// between.
    ///
    /// When breaking ties between enabled and disabled rules (via `select` and
    /// `ignore`, respectively), more specific prefixes override less
    /// specific prefixes.
    #[option(
        default = r#"["E", "F"]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of the defaults (`E`, `F`), enable flake8-bugbear (`B`) and flake8-quotes (`Q`).
            select = ["E", "F", "B", "Q"]
        "#
    )]
    pub error: Option<Vec<ErrorCodeSelector>>,

    /// A list of rule codes or prefixes to enable, in addition to those
    /// specified by `error`.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of the default `select` (`E`, `F`), enable flake8-bugbear (`B`) and flake8-quotes (`Q`).
            extend-select = ["B", "Q"]
        "#
    )]
    pub extend_error: Option<Vec<ErrorCodeSelector>>,

    // Tables are required to go last.
    /// A list of mappings from file pattern to rule codes or prefixes to
    /// exclude, when considering any matching files.
    #[option(
        default = "{}",
        value_type = "dict[str, list[RuleSelector]]",
        example = r#"
            # Ignore `E402` (import violations) in all `__init__.py` files, and in `path/to/file.py`.
            [tool.ruff.per-file-ignores]
            "__init__.py" = ["E402"]
            "path/to/file.py" = ["E402"]
        "#
    )]
    pub per_file_ignores: Option<FxHashMap<String, Vec<ErrorCodeSelector>>>,

    /// A list of file patterns to exclude from linting.
    ///
    /// Exclusions are based on globs, and can be either:
    ///
    /// - Single-path patterns, like `.mypy_cache` (to exclude any directory
    ///   named `.mypy_cache` in the tree), `foo.py` (to exclude any file named
    ///   `foo.py`), or `foo_*.py` (to exclude any file matching `foo_*.py` ).
    /// - Relative patterns, like `directory/foo.py` (to exclude that specific
    ///   file) or `directory/*.py` (to exclude any Python files in
    ///   `directory`). Note that these paths are relative to the project root
    ///   (e.g., the directory containing your `pyproject.toml`).
    ///
    /// For more information on the glob syntax, refer to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    ///
    /// Note that you'll typically want to use
    /// [`extend-exclude`](#extend-exclude) to modify the excluded paths.
    #[option(
        default = r#"[".bzr", ".direnv", ".eggs", ".git", ".git-rewrite", ".hg", ".mypy_cache", ".nox", ".pants.d", ".pytype", ".pyrogen_cache", ".svn", ".tox", ".venv", "__pypackages__", "_build", "buck-out", "build", "dist", "node_modules", "venv"]"#,
        value_type = "list[str]",
        example = r#"
            exclude = [".venv"]
        "#
    )]
    pub exclude: Option<Vec<String>>,

    /// Whether to enforce `exclude` and `extend-exclude` patterns, even for
    /// paths that are passed to Pyrogen explicitly. Typically, Pyrogen will lint
    /// any paths passed in directly, even if they would typically be
    /// excluded. Setting `force-exclude = true` will cause Pyrogen to
    /// respect these exclusions unequivocally.
    ///
    /// This is useful for [`pre-commit`](https://pre-commit.com/), which explicitly passes all
    /// changed files to the [`pyrogen-pre-commit`](https://github.com/astral-sh/pyrogen-pre-commit)
    /// plugin, regardless of whether they're marked as excluded by Pyrogen's own
    /// settings.
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"
            force-exclude = true
        "#
    )]
    pub force_exclude: Option<bool>,

    /// A list of file patterns to include when linting.
    ///
    /// Inclusion are based on globs, and should be single-path patterns, like
    /// `*.pyw`, to include any file with the `.pyw` extension. `pyproject.toml` is
    /// included here not for configuration but because we lint whether e.g. the
    /// `[project]` matches the schema.
    ///
    /// For more information on the glob syntax, refer to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    #[option(
        default = r#"["*.py", "*.pyi", "**/pyproject.toml"]"#,
        value_type = "list[str]",
        example = r#"
            include = ["*.py"]
        "#
    )]
    pub include: Option<Vec<String>>,

    /// The directories to consider when resolving first- vs. third-party
    /// imports.
    ///
    /// As an example: given a Python package structure like:
    ///
    /// ```text
    /// my_project
    /// ├── pyproject.toml
    /// └── src
    ///     └── my_package
    ///         ├── __init__.py
    ///         ├── foo.py
    ///         └── bar.py
    /// ```
    ///
    /// The `./src` directory should be included in the `src` option
    /// (e.g., `src = ["src"]`), such that when resolving imports,
    /// `my_package.foo` is considered a first-party import.
    ///
    /// When omitted, the `src` directory will typically default to the
    /// directory containing the nearest `pyproject.toml`, `pyrogen.toml`, or
    /// `.pyrogen.toml` file (the "project root"), unless a configuration file
    /// is explicitly provided (e.g., via the `--config` command-line flag).
    ///
    /// This field supports globs. For example, if you have a series of Python
    /// packages in a `python_modules` directory, `src = ["python_modules/*"]`
    /// would expand to incorporate all of the packages in that directory. User
    /// home directory and environment variables will also be expanded.
    #[option(
        default = r#"["."]"#,
        value_type = "list[str]",
        example = r#"
            # Allow imports relative to the "src" and "test" directories.
            src = ["src", "test"]
        "#
    )]
    pub src: Option<Vec<String>>,

    /// Mark the specified directories as namespace packages. For the purpose of
    /// module resolution, Pyrogen will treat those directories as if they
    /// contained an `__init__.py` file.
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            namespace-packages = ["airflow/providers"]
        "#
    )]
    pub namespace_packages: Option<Vec<String>>,

    /// Whether to automatically exclude files that are ignored by `.ignore`,
    /// `.gitignore`, `.git/info/exclude`, and global `gitignore` files.
    /// Enabled by default.
    #[option(
        default = "true",
        value_type = "bool",
        example = r#"
            respect-gitignore = false
        "#
    )]
    pub respect_gitignore: Option<bool>,

    /// The minimum Python version to target, e.g., when considering automatic
    /// code upgrades, like rewriting type annotations. Pyrogen will not propose
    /// changes using features that are not available in the given version.
    ///
    /// For example, to represent supporting Python >=3.10 or ==3.10
    /// specify `target-version = "py310"`.
    ///
    /// If omitted, and Pyrogen is configured via a `pyproject.toml` file, the
    /// target version will be inferred from its `project.requires-python`
    /// field (e.g., `requires-python = ">=3.8"`). If Pyrogen is configured via
    /// `pyrogen.toml` or `.pyrogen.toml`, no such inference will be performed.
    #[option(
        default = r#""py310""#,
        value_type = r#""py310" | "py311" | "py312""#,
        example = r#"
            # Always generate Python 3.10-compatible code.
            target-version = "py310"
        "#
    )]
    pub target_version: Option<PythonVersion>,
}
