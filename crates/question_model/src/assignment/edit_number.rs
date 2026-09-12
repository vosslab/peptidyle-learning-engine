//! Canonical current Assignment Edit Number.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Positive compare-and-swap number for one replaceable Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentEditNumber(NonZeroU64);

impl AssignmentEditNumber {
    /// First edit number for a newly created Assignment.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Rebuilds a positive edit number that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact positive persistence value.
    pub fn value(self) -> u64 {
        self.0.get()
    }

    /// Advances one successful Assignment replacement.
    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

impl FromStr for AssignmentEditNumber {
    type Err = AssignmentEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AssignmentEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(AssignmentEditNumberError)
    }
}

impl TryFrom<String> for AssignmentEditNumber {
    type Error = AssignmentEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssignmentEditNumber> for String {
    fn from(value: AssignmentEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for AssignmentEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// An Assignment Edit Number was not one canonical positive PostgreSQL-`BIGINT` decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentEditNumberError;

impl std::fmt::Display for AssignmentEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment edit number must be a canonical positive decimal")
    }
}

impl std::error::Error for AssignmentEditNumberError {}
