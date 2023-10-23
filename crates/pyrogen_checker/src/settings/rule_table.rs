use std::fmt::Debug;

use pyrogen_macros::CacheKey;

use crate::registry::{ErrorCode, ErrorCodeSet, ErrorCodeSetIterator};

/// A table to keep track of which error codes are enabled.
#[derive(Debug, CacheKey, Default)]
pub struct RuleTable {
    /// Maps rule codes to a boolean indicating if the rule should be autofixed.
    enabled: ErrorCodeSet,
}

impl RuleTable {
    /// Creates a new empty rule table.
    pub const fn empty() -> Self {
        Self {
            enabled: ErrorCodeSet::empty(),
        }
    }

    /// Returns whether the given rule should be checked.
    #[inline]
    pub const fn enabled(&self, rule: ErrorCode) -> bool {
        self.enabled.contains(rule)
    }

    /// Returns whether any of the given rules should be checked.
    #[inline]
    pub const fn any_enabled(&self, rules: &[ErrorCode]) -> bool {
        self.enabled
            .intersects(&ErrorCodeSet::from_error_codes(rules))
    }

    /// Returns an iterator over all enabled rules.
    pub fn iter_enabled(&self) -> ErrorCodeSetIterator {
        self.enabled.iter()
    }

    /// Enables the given rule.
    #[inline]
    pub fn enable(&mut self, rule: ErrorCode, should_fix: bool) {
        self.enabled.insert(rule);
    }

    /// Disables the given rule.
    #[inline]
    pub fn disable(&mut self, rule: ErrorCode) {
        self.enabled.remove(rule);
    }
}

impl FromIterator<ErrorCode> for RuleTable {
    fn from_iter<T: IntoIterator<Item = ErrorCode>>(iter: T) -> Self {
        let rules = ErrorCodeSet::from_iter(iter);
        Self {
            enabled: rules.clone(),
        }
    }
}
