//! Canonical optimistic-concurrency revision for editable assignments.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Server-issued optimistic-concurrency value for one editable assignment.
///
/// Assignment definitions are tenant-owned course artifacts. Their selected
/// published versions stay immutable, while the ordered selection and policies
/// change only through this compare-and-swap token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentRevision(NonZeroU64);

impl AssignmentRevision {
    /// Initial revision for a newly persisted assignment definition.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Rebuilds a positive revision that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact positive persistence revision scalar.
    pub fn value(self) -> u64 {
        self.0.get()
    }

    /// Advances without exceeding the PostgreSQL `BIGINT` persistence boundary.
    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

impl FromStr for AssignmentRevision {
    type Err = AssignmentRevisionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AssignmentRevisionError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(AssignmentRevisionError)
    }
}

impl TryFrom<String> for AssignmentRevision {
    type Error = AssignmentRevisionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssignmentRevision> for String {
    fn from(value: AssignmentRevision) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for AssignmentRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

/// An assignment revision was not one canonical positive PostgreSQL-`BIGINT` decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentRevisionError;

impl std::fmt::Display for AssignmentRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment revision must be a canonical positive decimal")
    }
}

impl std::error::Error for AssignmentRevisionError {}
