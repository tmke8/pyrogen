use crate::registry::ErrorCode;
use pyrogen_macros::CacheKey;
use std::fmt::{Debug, Formatter};
use std::iter::FusedIterator;

const ERRORCODESET_SIZE: usize = 11;

/// A set of [`ErrorCodes`]s.
///
/// Uses a bitset where a bit of one signals that the error code with that [u16] is in this set.
#[derive(Clone, Default, CacheKey, PartialEq, Eq)]
pub struct ErrorCodeSet([u64; ERRORCODESET_SIZE]);

impl ErrorCodeSet {
    const EMPTY: [u64; ERRORCODESET_SIZE] = [0; ERRORCODESET_SIZE];
    // 64 fits into a u16 without truncation
    #[allow(clippy::cast_possible_truncation)]
    const SLICE_BITS: u16 = u64::BITS as u16;

    /// Returns an empty error code set.
    pub const fn empty() -> Self {
        Self(Self::EMPTY)
    }

    pub fn clear(&mut self) {
        self.0 = Self::EMPTY;
    }

    #[inline]
    pub const fn from_error_code(error_code: ErrorCode) -> Self {
        let error_code = error_code as u16;

        let index = (error_code / Self::SLICE_BITS) as usize;

        debug_assert!(
            index < Self::EMPTY.len(),
            "Error code index out of bounds. Increase the size of the bitset array."
        );

        // The bit-position of this specific error code in the slice
        let shift = error_code % Self::SLICE_BITS;
        // Set the index for that error code to 1
        let mask = 1 << shift;

        let mut bits = Self::EMPTY;
        bits[index] = mask;

        Self(bits)
    }

    #[inline]
    pub const fn from_error_codes(error_codes: &[ErrorCode]) -> Self {
        let mut set = ErrorCodeSet::empty();

        let mut i = 0;

        // Uses a while because for loops are not allowed in const functions.
        while i < error_codes.len() {
            set = set.union(&ErrorCodeSet::from_error_code(error_codes[i]));
            i += 1;
        }

        set
    }

    /// Returns the union of the two rule sets `self` and `other`
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let set_1 = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    /// let set_2 = ErrorCodeSet::from_error_codes(&[
    ///     ErrorCode::GeneralTypeError,
    ///     ErrorCode::UnusedImport,
    /// ]);
    ///
    /// let union = set_1.union(&set_2);
    ///
    /// assert!(union.contains(ErrorCode::SyntaxError));
    /// assert!(union.contains(ErrorCode::UnusedTypeIgnore));
    /// assert!(union.contains(ErrorCode::GeneralTypeError));
    /// assert!(union.contains(ErrorCode::UnusedImport));
    /// ```
    #[must_use]
    pub const fn union(mut self, other: &Self) -> Self {
        let mut i = 0;

        while i < self.0.len() {
            self.0[i] |= other.0[i];
            i += 1;
        }

        self
    }

    /// Returns `self` without any of the rules contained in `other`.
    ///
    /// ## Examples
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let set_1 = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    /// let set_2 = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::Unreachable]);
    ///
    /// let subtract = set_1.subtract(&set_2);
    ///
    /// assert!(subtract.contains(ErrorCode::UnusedTypeIgnore));
    /// assert!(!subtract.contains(ErrorCode::SyntaxError));
    /// ```
    #[must_use]
    pub const fn subtract(mut self, other: &Self) -> Self {
        let mut i = 0;

        while i < self.0.len() {
            self.0[i] &= !other.0[i];
            i += 1;
        }

        self
    }

    /// Returns true if `self` and `other` contain at least one common rule.
    ///
    /// ## Examples
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let set_1 = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    ///
    /// assert!(set_1.intersects(&ErrorCodeSet::from_error_codes(&[
    ///     ErrorCode::UnusedTypeIgnore,
    ///     ErrorCode::GeneralTypeError
    /// ])));
    ///
    /// assert!(!set_1.intersects(&ErrorCodeSet::from_error_codes(&[
    ///     ErrorCode::UnusedImport,
    ///     ErrorCode::GeneralTypeError
    /// ])));
    /// ```
    pub const fn intersects(&self, other: &Self) -> bool {
        let mut i = 0;

        while i < self.0.len() {
            if self.0[i] & other.0[i] != 0 {
                return true;
            }
            i += 1;
        }

        false
    }

    /// Returns `true` if this set contains no rules, `false` otherwise.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// assert!(ErrorCodeSet::empty().is_empty());
    ///         assert!(
    ///             !ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::GeneralTypeError])
    ///                 .is_empty()
    ///         );
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of rules in this set.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// assert_eq!(ErrorCodeSet::empty().len(), 0);
    /// assert_eq!(
    ///     ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::GeneralTypeError]).len(),
    ///     2
    /// );
    pub const fn len(&self) -> usize {
        let mut len: u32 = 0;

        let mut i = 0;

        while i < self.0.len() {
            len += self.0[i].count_ones();
            i += 1;
        }

        len as usize
    }

    /// Inserts `rule` into the set.
    ///
    /// ## Examples
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let mut set = ErrorCodeSet::empty();
    ///
    /// assert!(!set.contains(ErrorCode::UnusedTypeIgnore));
    ///
    /// set.insert(ErrorCode::UnusedTypeIgnore);
    ///
    /// assert!(set.contains(ErrorCode::UnusedTypeIgnore));
    /// ```
    pub fn insert(&mut self, rule: ErrorCode) {
        let set = std::mem::take(self);
        *self = set.union(&ErrorCodeSet::from_error_code(rule));
    }

    /// Removes `rule` from the set.
    ///
    /// ## Examples
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let mut set = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    ///
    /// set.remove(ErrorCode::SyntaxError);
    ///
    /// assert!(set.contains(ErrorCode::UnusedTypeIgnore));
    /// assert!(!set.contains(ErrorCode::SyntaxError));
    /// ```
    pub fn remove(&mut self, rule: ErrorCode) {
        let set = std::mem::take(self);
        *self = set.subtract(&ErrorCodeSet::from_error_code(rule));
    }

    /// Returns `true` if `rule` is in this set.
    ///
    /// ## Examples
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let set = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    ///
    /// assert!(set.contains(ErrorCode::SyntaxError));
    /// assert!(!set.contains(ErrorCode::UndefinedName));
    /// ```
    pub const fn contains(&self, rule: ErrorCode) -> bool {
        let rule = rule as u16;
        let index = rule as usize / Self::SLICE_BITS as usize;
        let shift = rule % Self::SLICE_BITS;
        let mask = 1 << shift;

        self.0[index] & mask != 0
    }

    /// Returns an iterator over the rules in this set.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use pyrogen_checker::registry::{ErrorCode, ErrorCodeSet};
    /// let set = ErrorCodeSet::from_error_codes(&[ErrorCode::SyntaxError, ErrorCode::UnusedTypeIgnore]);
    ///
    /// let iter: Vec<_> = set.iter().collect();
    ///
    /// assert_eq!(iter, vec![ErrorCode::UnusedTypeIgnore, ErrorCode::SyntaxError]);
    /// ```
    pub fn iter(&self) -> ErrorCodeSetIterator {
        ErrorCodeSetIterator {
            set: self.clone(),
            index: 0,
        }
    }
}

impl Debug for ErrorCodeSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<ErrorCode> for ErrorCodeSet {
    fn from_iter<T: IntoIterator<Item = ErrorCode>>(iter: T) -> Self {
        let mut set = ErrorCodeSet::empty();

        for rule in iter {
            set.insert(rule);
        }

        set
    }
}

impl Extend<ErrorCode> for ErrorCodeSet {
    fn extend<T: IntoIterator<Item = ErrorCode>>(&mut self, iter: T) {
        let set = std::mem::take(self);
        *self = set.union(&ErrorCodeSet::from_iter(iter));
    }
}

impl IntoIterator for ErrorCodeSet {
    type IntoIter = ErrorCodeSetIterator;
    type Item = ErrorCode;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &ErrorCodeSet {
    type IntoIter = ErrorCodeSetIterator;
    type Item = ErrorCode;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ErrorCodeSetIterator {
    set: ErrorCodeSet,
    index: u16,
}

impl Iterator for ErrorCodeSetIterator {
    type Item = ErrorCode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slice = self.set.0.get_mut(self.index as usize)?;
            // `trailing_zeros` is guaranteed to return a value in [0;64]
            #[allow(clippy::cast_possible_truncation)]
            let bit = slice.trailing_zeros() as u16;

            if bit < ErrorCodeSet::SLICE_BITS {
                *slice ^= 1 << bit;
                let rule_value = self.index * ErrorCodeSet::SLICE_BITS + bit;
                // SAFETY: ErrorCodeSet guarantees that only valid rules are stored in the set.
                #[allow(unsafe_code)]
                return Some(unsafe { std::mem::transmute(rule_value) });
            }

            self.index += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.set.len();

        (len, Some(len))
    }
}

impl ExactSizeIterator for ErrorCodeSetIterator {}

impl FusedIterator for ErrorCodeSetIterator {}

#[cfg(test)]
mod tests {
    use crate::registry::{ErrorCode, ErrorCodeSet};
    use strum::IntoEnumIterator;

    /// Tests that the set can contain all rules
    #[test]
    fn test_all_rules() {
        for rule in ErrorCode::iter() {
            let set = ErrorCodeSet::from_error_code(rule);

            assert!(set.contains(rule));
        }

        let all_rules_set: ErrorCodeSet = ErrorCode::iter().collect();
        let all_rules: Vec<_> = all_rules_set.iter().collect();
        let expected_rules: Vec<_> = ErrorCode::iter().collect();
        assert_eq!(all_rules, expected_rules);
    }
}
