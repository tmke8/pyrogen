use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::registry::{Rule, RuleIter, RuleNamespace};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleSelector {
    /// Select all rules (includes rules in preview if enabled)
    All,
    /// Select an individual rule with a given prefix.
    Rule(Rule),
}

impl FromStr for RuleSelector {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ALL" => Ok(Self::All),
            _ => {
                // Does the selector select a single rule?
                let prefix = Rule::from_code(&s).map_err(|_| ParseError::Unknown(s.to_string()))?;
                Ok(Self::Rule(prefix))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unknown rule selector: `{0}`")]
    // TODO(martin): tell the user how to discover rule codes via the CLI once such a command is
    // implemented (but that should of course be done only in ruff_cli and not here)
    Unknown(String),
}

impl RuleSelector {
    pub fn code(&self) -> &'static str {
        match self {
            RuleSelector::All => "ALL",
            RuleSelector::Rule(rule) => rule.code(),
        }
    }
}

impl Serialize for RuleSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let code = self.code();
        serializer.serialize_str(&format!("{code}"))
    }
}

impl<'de> Deserialize<'de> for RuleSelector {
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
    type Value = RuleSelector;

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

impl RuleSelector {
    /// Return all matching rules, regardless of whether they're in preview.
    pub fn all_rules(&self) -> impl Iterator<Item = Rule> + '_ {
        match self {
            RuleSelector::All => RuleSelectorIter::All(Rule::iter()),

            RuleSelector::Rule(rule) => RuleSelectorIter::Vec(vec![*rule].into_iter()),
        }
    }

    /// Returns rules matching the selector, taking into account preview options enabled.
    pub fn rules<'a>(&'a self) -> impl Iterator<Item = Rule> + 'a {
        self.all_rules()
    }
}

pub enum RuleSelectorIter {
    All(RuleIter),
    Vec(std::vec::IntoIter<Rule>),
}

impl Iterator for RuleSelectorIter {
    type Item = Rule;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RuleSelectorIter::All(iter) => iter.next(),
            RuleSelectorIter::Vec(iter) => iter.next(),
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

    use crate::registry::RuleNamespace;
    use crate::rule_selector::{Linter, RuleCodePrefix};
    use crate::RuleSelector;

    impl JsonSchema for RuleSelector {
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

impl RuleSelector {
    pub fn specificity(&self) -> Specificity {
        match self {
            RuleSelector::All => Specificity::All,
            RuleSelector::Rule { .. } => Specificity::Rule,
        }
    }
}

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum Specificity {
    /// The specificity when selecting all rules (e.g., `--select ALL`).
    All,
    /// The specificity when selecting a legacy linter group (e.g., `--select C` or `--select T`).
    LinterGroup,
    /// The specificity when selecting a linter (e.g., `--select PLE` or `--select UP`).
    Linter,
    /// The specificity when selecting via a rule prefix with a one-character code (e.g., `--select PLE1`).
    Prefix1Char,
    /// The specificity when selecting via a rule prefix with a two-character code (e.g., `--select PLE12`).
    Prefix2Chars,
    /// The specificity when selecting via a rule prefix with a three-character code (e.g., `--select PLE123`).
    Prefix3Chars,
    /// The specificity when selecting via a rule prefix with a four-character code (e.g., `--select PLE1234`).
    Prefix4Chars,
    /// The specificity when selecting an individual rule (e.g., `--select PLE1205`).
    Rule,
}
