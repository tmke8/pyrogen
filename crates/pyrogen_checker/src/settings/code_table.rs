use std::{
    f32::consts::E,
    fmt::{Debug, Display},
};

use pyrogen_macros::CacheKey;
use serde::{Deserialize, Serialize};

use crate::registry::{ErrorCode, ErrorCodeSet, ErrorCodeSetIterator};

/// A table to keep track of which error codes are enabled.
#[derive(Debug, CacheKey, Default)]
pub struct ErrorCodeTable {
    /// Maps rule codes to a boolean indicating if the rule should be autofixed.
    enabled: ErrorCodeSet,
    warning: ErrorCodeSet,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MessageKind {
    Error,
    Warning,
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageKind::Error => write!(f, "error"),
            MessageKind::Warning => write!(f, "warning"),
        }
    }
}

impl ErrorCodeTable {
    /// Creates a new empty error code table.
    pub const fn empty() -> Self {
        Self {
            enabled: ErrorCodeSet::empty(),
            warning: ErrorCodeSet::empty(),
        }
    }

    /// Returns whether the given rule should be checked.
    #[inline]
    pub const fn enabled(&self, rule: ErrorCode) -> bool {
        self.enabled.contains(rule)
    }

    #[inline]
    pub const fn entry(&self, rule: ErrorCode) -> Option<MessageKind> {
        if self.enabled(rule) {
            if self.is_warning(rule) {
                Some(MessageKind::Warning)
            } else {
                Some(MessageKind::Error)
            }
        } else {
            None
        }
    }

    /// Returns whether any of the given rules should be checked.
    #[inline]
    pub const fn any_enabled(&self, rules: &[ErrorCode]) -> bool {
        self.enabled
            .intersects(&ErrorCodeSet::from_error_codes(rules))
    }

    /// Returns whether violations of the given rule should be a warning.
    #[inline]
    pub const fn is_warning(&self, rule: ErrorCode) -> bool {
        self.warning.contains(rule)
    }

    /// Returns an iterator over all enabled rules.
    pub fn iter_enabled(&self) -> ErrorCodeSetIterator {
        self.enabled.iter()
    }

    /// Returns an iterator over all warnings.
    pub fn iter_warnings(&self) -> ErrorCodeSetIterator {
        self.warning.iter()
    }

    /// Enables the given rule.
    #[inline]
    pub fn enable_error(&mut self, rule: ErrorCode) {
        self.enabled.insert(rule);
    }

    /// Enables the given rule.
    #[inline]
    pub fn enable_warning(&mut self, rule: ErrorCode) {
        self.enabled.insert(rule);
        self.warning.insert(rule);
    }

    /// Disables the given rule.
    #[inline]
    pub fn disable(&mut self, rule: ErrorCode) {
        self.enabled.remove(rule);
        self.warning.remove(rule);
    }
}

impl FromIterator<ErrorCode> for ErrorCodeTable {
    fn from_iter<T: IntoIterator<Item = ErrorCode>>(iter: T) -> Self {
        let rules = ErrorCodeSet::from_iter(iter);
        Self {
            enabled: rules,
            warning: ErrorCodeSet::empty(),
        }
    }
}
