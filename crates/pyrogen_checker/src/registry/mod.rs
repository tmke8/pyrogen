use rustpython_ast::{text_size::TextRange, Ranged, TextSize};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

pub use rule_set::{ErrorCodeSet, ErrorCodeSetIterator};

mod rule_set;

#[repr(u16)]
#[derive(
    Eq,
    Hash,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Display,
    EnumString,
    EnumIter,
    IntoStaticStr,
    Serialize,
    Deserialize,
)]
pub enum ErrorCode {
    InvalidPyprojectToml,

    #[strum(serialize = "override")]
    Override,

    #[strum(serialize = "unreachable")]
    Unreachable,

    #[strum(serialize = "unused-type-ignore")]
    UnusedTypeIgnore,

    #[strum(serialize = "syntax-error")]
    SyntaxError,

    #[strum(serialize = "general")]
    GeneralTypeError,

    #[strum(serialize = "unused-import")]
    UnusedImport,

    #[strum(serialize = "unused-variable")]
    UnusedVariable,

    #[strum(serialize = "undefined-name")]
    UndefinedName,

    #[strum(serialize = "io-error")]
    IOError,
}

pub trait AsErrorCode {
    fn error_code(&self) -> ErrorCode;
}

impl ErrorCode {
    // pub fn from_str(code: &str) -> Result<Self, FromCodeError> {
    //     code.to_owned().parse().map_err(|x| FromCodeError::Unknown)
    // }

    pub fn to_str(&self) -> &'static str {
        self.into()
    }
}

impl AsErrorCode for DiagnosticKind {
    #[inline]
    fn error_code(&self) -> ErrorCode {
        self.error_code
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FromCodeError {
    #[error("unknown error code")]
    Unknown,
}

#[derive(is_macro::Is, Copy, Clone)]
pub enum CheckerSource {
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

impl ErrorCode {
    /// The source for the diagnostic (either the AST, the filesystem, or the
    /// physical lines).
    pub const fn lint_source(&self) -> CheckerSource {
        match self {
            ErrorCode::InvalidPyprojectToml => CheckerSource::PyprojectToml,
            ErrorCode::UnusedTypeIgnore => CheckerSource::Noqa,
            ErrorCode::Override => CheckerSource::Tokens,
            ErrorCode::Unreachable => CheckerSource::LogicalLines,
            _ => CheckerSource::Ast,
        }
    }

    // /// Return the URL for the rule documentation, if it exists.
    // pub fn url(&self) -> Option<String> {
    //     self.explanation()
    //         .is_some()
    //         .then(|| format!("{}/rules/{}", env!("CARGO_PKG_HOMEPAGE"), self.as_ref()))
    // }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct DiagnosticKind {
    /// The error code that this diagnostic is associated with.
    pub error_code: ErrorCode,
    /// The message body to display to the user, to explain the diagnostic.
    pub body: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub range: TextRange,
    pub parent: Option<TextSize>,
}

impl Diagnostic {
    pub fn new<T: Into<DiagnosticKind>>(kind: T, range: TextRange) -> Self {
        Self {
            kind: kind.into(),
            range,
            parent: None,
        }
    }

    /// Set the location of the diagnostic's parent node.
    #[inline]
    pub fn set_parent(&mut self, parent: TextSize) {
        self.parent = Some(parent);
    }
}

impl Ranged for Diagnostic {
    fn range(&self) -> TextRange {
        self.range
    }
}

#[cfg(feature = "clap")]
pub mod clap_completion {
    use clap::builder::{PossibleValue, TypedValueParser, ValueParserFactory};
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    use crate::registry::ErrorCode;

    #[derive(Clone)]
    pub struct RuleParser;

    impl ValueParserFactory for ErrorCode {
        type Parser = RuleParser;

        fn value_parser() -> Self::Parser {
            RuleParser
        }
    }

    impl TypedValueParser for RuleParser {
        type Value = ErrorCode;

        fn parse_ref(
            &self,
            cmd: &clap::Command,
            arg: Option<&clap::Arg>,
            value: &std::ffi::OsStr,
        ) -> Result<Self::Value, clap::Error> {
            let value = value
                .to_str()
                .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;

            ErrorCode::from_str(value).map_err(|_| {
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
            Some(Box::new(ErrorCode::iter().map(|error_code| {
                let name = error_code.to_string();
                PossibleValue::new(name)
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::str::FromStr;

    use strum::IntoEnumIterator;

    use super::ErrorCode;

    #[test]
    fn check_code_serialization() {
        for error_code in ErrorCode::iter() {
            assert!(
                ErrorCode::from_str(&format!("{}", error_code)).is_ok(),
                "{error_code:?} could not be round-trip serialized."
            );
        }
    }

    #[test]
    fn test_linter_parse_code() {
        for error_code in ErrorCode::iter() {
            let code = format!("{}", error_code);
            let linter: ErrorCode = code
                .parse()
                .unwrap_or_else(|err| panic!("couldn't parse {code:?}"));
        }
    }

    #[test]
    fn rule_size() {
        assert_eq!(2, size_of::<ErrorCode>());
    }
}
