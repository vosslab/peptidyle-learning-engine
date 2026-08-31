//! Canonical immutable revision number within one stable Assignment.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Positive number for one immutable Assignment Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentRevisionNumber(NonZeroU64);

impl AssignmentRevisionNumber {
    /// Initial immutable revision number for a newly persisted assignment definition.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Rebuilds a positive revision that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact positive persistence revision number.
    pub fn value(self) -> u64 {
        self.0.get()
    }

    /// Advances without exceeding the PostgreSQL `BIGINT` persistence boundary.
    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

impl FromStr for AssignmentRevisionNumber {
    type Err = AssignmentRevisionNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AssignmentRevisionNumberError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(AssignmentRevisionNumberError)
    }
}

impl TryFrom<String> for AssignmentRevisionNumber {
    type Error = AssignmentRevisionNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssignmentRevisionNumber> for String {
    fn from(value: AssignmentRevisionNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for AssignmentRevisionNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

/// An Assignment Revision Number was not one canonical positive PostgreSQL-`BIGINT` decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentRevisionNumberError;

impl std::fmt::Display for AssignmentRevisionNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment revision number must be a canonical positive decimal")
    }
}

impl std::error::Error for AssignmentRevisionNumberError {}
