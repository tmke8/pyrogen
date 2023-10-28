use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::registry::{ErrorCode, ErrorCodeIter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCodeSelector {
    /// Select all error codes.
    All,
    /// Select an individual error code.
    ErrorCode(ErrorCode),
}

impl FromStr for ErrorCodeSelector {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ALL" => Ok(Self::All),
            _ => {
                // Does the selector select a single error code?
                let error_code =
                    ErrorCode::from_str(s).map_err(|_| ParseError::Unknown(s.to_string()))?;
                Ok(Self::ErrorCode(error_code))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unknown rule selector: `{0}`")]
    Unknown(String),
}

impl ErrorCodeSelector {
    pub fn code(&self) -> &'static str {
        match self {
            ErrorCodeSelector::All => "ALL",
            ErrorCodeSelector::ErrorCode(rule) => rule.to_str(),
        }
    }
}

impl Serialize for ErrorCodeSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for ErrorCodeSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // We are not simply doing:
        // let s: &str = Deserialize::deserialize(deserializer)?;
        // FromStr::from_str(s).map_err(de::Error::custom)
        // here because the toml crate apparently doesn't support that
        // (as of toml v0.6.0 running `cargo test` failed with the above two lines)
        deserializer.deserialize_str(SelectorVisitor)
    }
}

struct SelectorVisitor;

impl Visitor<'_> for SelectorVisitor {
    type Value = ErrorCodeSelector;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "expected a string code identifying a linter or specific rule, or a partial rule code or ALL to refer to all rules",
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        FromStr::from_str(v).map_err(de::Error::custom)
    }
}

impl ErrorCodeSelector {
    /// Return all matching rules, regardless of whether they're in preview.
    pub fn all_rules(&self) -> impl Iterator<Item = ErrorCode> + '_ {
        match self {
            ErrorCodeSelector::All => ErrorCodeSelectorIter::All(ErrorCode::iter()),

            ErrorCodeSelector::ErrorCode(rule) => {
                ErrorCodeSelectorIter::Vec(vec![*rule].into_iter())
            }
        }
    }

    /// Returns rules matching the selector, taking into account preview options enabled.
    pub fn rules(&self) -> impl Iterator<Item = ErrorCode> + '_ {
        self.all_rules()
    }
}

pub enum ErrorCodeSelectorIter {
    All(ErrorCodeIter),
    Vec(std::vec::IntoIter<ErrorCode>),
}

impl Iterator for ErrorCodeSelectorIter {
    type Item = ErrorCode;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ErrorCodeSelectorIter::All(iter) => iter.next(),
            ErrorCodeSelectorIter::Vec(iter) => iter.next(),
        }
    }
}

#[cfg(feature = "schemars")]
mod schema {
    use itertools::Itertools;
    use schemars::JsonSchema;
    use schemars::_serde_json::Value;
    use schemars::schema::{InstanceType, Schema, SchemaObject};
    use strum::IntoEnumIterator;

    use crate::code_selector::{Linter, RuleCodePrefix};
    use crate::ErrorCodeSelector;

    impl JsonSchema for ErrorCodeSelector {
        fn schema_name() -> String {
            "RuleSelector".to_string()
        }

        fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
            Schema::Object(SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                enum_values: Some(
                    [
                        // Include the non-standard "ALL" and "NURSERY" selectors.
                        "ALL".to_string(),
                        "NURSERY".to_string(),
                        // Include the legacy "C" and "T" selectors.
                        "C".to_string(),
                        "T".to_string(),
                        // Include some common redirect targets for those legacy selectors.
                        "C9".to_string(),
                        "T1".to_string(),
                        "T2".to_string(),
                    ]
                    .into_iter()
                    .chain(
                        RuleCodePrefix::iter()
                            .map(|p| {
                                let prefix = p.linter().common_prefix();
                                let code = p.short_code();
                                format!("{prefix}{code}")
                            })
                            .chain(Linter::iter().filter_map(|l| {
                                let prefix = l.common_prefix();
                                (!prefix.is_empty()).then(|| prefix.to_string())
                            })),
                    )
                    // Filter out rule gated behind `#[cfg(feature = "unreachable-code")]`, which is
                    // off-by-default
                    .filter(|prefix| prefix != "RUF014")
                    .sorted()
                    .map(Value::String)
                    .collect(),
                ),
                ..SchemaObject::default()
            })
        }
    }
}

impl ErrorCodeSelector {
    pub fn specificity(&self) -> Specificity {
        match self {
            ErrorCodeSelector::All => Specificity::All,
            ErrorCodeSelector::ErrorCode { .. } => Specificity::Rule,
        }
    }
}

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum Specificity {
    /// The specificity when selecting all rules (e.g., `--select ALL`).
    All,
    // /// The specificity when selecting a legacy linter group (e.g., `--select C` or `--select T`).
    // LinterGroup,
    /// The specificity when selecting an individual rule (e.g., `--select PLE1205`).
    Rule,
}

#[cfg(feature = "clap")]
pub mod clap_completion {
    use clap::builder::{PossibleValue, TypedValueParser, ValueParserFactory};
    use strum::IntoEnumIterator;

    use crate::{registry::ErrorCode, ErrorCodeSelector};

    #[derive(Clone)]
    pub struct ErrorCodeSelectorParser;

    impl ValueParserFactory for ErrorCodeSelector {
        type Parser = ErrorCodeSelectorParser;

        fn value_parser() -> Self::Parser {
            ErrorCodeSelectorParser
        }
    }

    impl TypedValueParser for ErrorCodeSelectorParser {
        type Value = ErrorCodeSelector;

        fn parse_ref(
            &self,
            cmd: &clap::Command,
            arg: Option<&clap::Arg>,
            value: &std::ffi::OsStr,
        ) -> Result<Self::Value, clap::Error> {
            let value = value
                .to_str()
                .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;

            value.parse().map_err(|_| {
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
            Some(Box::new(
                std::iter::once(PossibleValue::new("ALL").help("all rules")).chain(
                    ErrorCode::iter().map(|rule| {
                        let name = rule.to_string();
                        PossibleValue::new(name)
                    }),
                ),
            ))
        }
    }
}
