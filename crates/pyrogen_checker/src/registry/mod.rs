//! Remnant of the registry of all [`Rule`] implementations, now it's reexporting from codes.rs
//! with some helper symbols

use pyrogen_diagnostics::DiagnosticKind;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

pub use rule_set::{RuleSet, RuleSetIterator};

mod rule_set;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumString, EnumIter, IntoStaticStr)] // strum macros.
pub enum Rule {
    InvalidPyprojectToml,
    #[strum(serialize = "override")]
    Override,

    #[strum(serialize = "unreachable")]
    Unreachable,

    #[strum(serialize = "unused-type-ignore")]
    UnusedTypeIgnore,

    #[strum(serialize = "syntax-error")]
    SyntaxError,
}

pub trait AsRule {
    fn rule(&self) -> Rule;
}

impl Rule {
    pub fn from_code(code: &str) -> Result<Self, FromCodeError> {
        code.to_owned().parse().map_err(|x| FromCodeError::Unknown)
    }
}

impl AsRule for DiagnosticKind {
    fn rule(&self) -> Rule {
        match Rule::from_code(&self.name) {
            Ok(rule) => rule,
            Err(_) => panic!("invalid rule name: {}", self.name),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FromCodeError {
    #[error("unknown rule code")]
    Unknown,
}

pub trait RuleNamespace: Sized {
    /// Returns the prefix that every single code that ruff uses to identify
    /// rules from this linter starts with.  In the case that multiple
    /// `#[prefix]`es are configured for the variant in the `Linter` enum
    /// definition this is the empty string.
    fn common_prefix(&self) -> &'static str;

    /// Attempts to parse the given rule code. If the prefix is recognized
    /// returns the respective variant along with the code with the common
    /// prefix stripped.
    fn parse_code(code: &str) -> Option<(Self, &str)>;

    fn name(&self) -> &'static str;

    fn url(&self) -> Option<&'static str>;
}

#[derive(is_macro::Is, Copy, Clone)]
pub enum LintSource {
    Ast,
    Io,
    PhysicalLines,
    LogicalLines,
    Tokens,
    Imports,
    Noqa,
    Filesystem,
    PyprojectToml,
}

impl Rule {
    /// The source for the diagnostic (either the AST, the filesystem, or the
    /// physical lines).
    pub const fn lint_source(&self) -> LintSource {
        match self {
            Rule::InvalidPyprojectToml => LintSource::PyprojectToml,
            Rule::UnusedTypeIgnore => LintSource::Noqa,
            Rule::Override => LintSource::Tokens,
            Rule::Unreachable => LintSource::LogicalLines,
            _ => LintSource::Ast,
        }
    }

    // /// Return the URL for the rule documentation, if it exists.
    // pub fn url(&self) -> Option<String> {
    //     self.explanation()
    //         .is_some()
    //         .then(|| format!("{}/rules/{}", env!("CARGO_PKG_HOMEPAGE"), self.as_ref()))
    // }
}

#[cfg(feature = "clap")]
pub mod clap_completion {
    use clap::builder::{PossibleValue, TypedValueParser, ValueParserFactory};
    use strum::IntoEnumIterator;

    use crate::registry::Rule;

    #[derive(Clone)]
    pub struct RuleParser;

    impl ValueParserFactory for Rule {
        type Parser = RuleParser;

        fn value_parser() -> Self::Parser {
            RuleParser
        }
    }

    impl TypedValueParser for RuleParser {
        type Value = Rule;

        fn parse_ref(
            &self,
            cmd: &clap::Command,
            arg: Option<&clap::Arg>,
            value: &std::ffi::OsStr,
        ) -> Result<Self::Value, clap::Error> {
            let value = value
                .to_str()
                .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;

            Rule::from_code(value).map_err(|_| {
                let mut error =
                    clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
                if let Some(arg) = arg {
                    error.insert(
                        clap::error::ContextKind::InvalidArg,
                        clap::error::ContextValue::String(arg.to_string()),
                    );
                }
                error.insert(
                    clap::error::ContextKind::InvalidValue,
                    clap::error::ContextValue::String(value.to_string()),
                );
                error
            })
        }

        fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
            Some(Box::new(Rule::iter().map(|rule| {
                let name = rule.to_string();
                PossibleValue::new(name)
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use strum::IntoEnumIterator;

    use super::{Rule, RuleNamespace};

    #[test]
    fn check_code_serialization() {
        for rule in Rule::iter() {
            assert!(
                Rule::from_code(&format!("{}", rule.to_string())).is_ok(),
                "{rule:?} could not be round-trip serialized."
            );
        }
    }

    #[test]
    fn test_linter_parse_code() {
        for rule in Rule::iter() {
            let code = format!("{}", rule.to_string());
            let linter: Rule = code
                .parse()
                .unwrap_or_else(|err| panic!("couldn't parse {code:?}"));
        }
    }

    #[test]
    fn rule_size() {
        assert_eq!(2, size_of::<Rule>());
    }
}
