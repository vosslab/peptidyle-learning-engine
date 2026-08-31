//! Bounded scalar values used by curriculum-adoption commands and receipts.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{BlueprintCourseTitleError, validate_blueprint_course_title};

/// Largest browser-supplied idempotency key accepted by one B2 write.
pub const MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES: usize = 128;

/// A validated opaque browser key that binds a completed adoption retry.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintOperationRetryToken(String);

impl BlueprintOperationRetryToken {
    /// Parses the bounded opaque key used for one completed retry.
    pub fn parse(value: &str) -> Result<Self, BlueprintOperationRetryTokenError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        valid
            .then(|| Self(value.to_owned()))
            .ok_or(BlueprintOperationRetryTokenError)
    }

    /// Returns the opaque browser value without assigning any authority to it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BlueprintOperationRetryToken {
    type Error = BlueprintOperationRetryTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<BlueprintOperationRetryToken> for String {
    fn from(value: BlueprintOperationRetryToken) -> Self {
        value.0
    }
}

impl std::fmt::Debug for BlueprintOperationRetryToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BlueprintOperationRetryToken([opaque])")
    }
}

/// An idempotency key was blank, oversized, or outside the opaque-key alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintOperationRetryTokenError;

impl std::fmt::Display for BlueprintOperationRetryTokenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum adoption idempotency key is invalid")
    }
}

impl std::error::Error for BlueprintOperationRetryTokenError {}

/// Strong revision evidence for one durable curriculum import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurriculumImportRevision(NonZeroU64);

impl CurriculumImportRevision {
    /// Builds a positive revision that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact persistence revision scalar.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for CurriculumImportRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

impl FromStr for CurriculumImportRevision {
    type Err = CurriculumImportRevisionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CurriculumImportRevisionError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(CurriculumImportRevisionError)
    }
}

impl TryFrom<String> for CurriculumImportRevision {
    type Error = CurriculumImportRevisionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CurriculumImportRevision> for String {
    fn from(value: CurriculumImportRevision) -> Self {
        value.to_string()
    }
}

/// An import revision was not one canonical positive PostgreSQL-bigint decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumImportRevisionError;

impl std::fmt::Display for CurriculumImportRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum import revision must be a canonical positive decimal")
    }
}

impl std::error::Error for CurriculumImportRevisionError {}

/// A validated display title for a new ordinary course or BlueprintCourse fork.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurriculumAdoptionTitle(String);

impl CurriculumAdoptionTitle {
    /// Builds one bounded, trimmed nonblank title using the shared curriculum rule.
    pub fn parse(value: &str) -> Result<Self, CurriculumAdoptionTitleError> {
        validate_blueprint_course_title(value).map_err(CurriculumAdoptionTitleError::from)?;
        Ok(Self(value.to_owned()))
    }

    /// Returns the validated display title.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CurriculumAdoptionTitle {
    type Error = CurriculumAdoptionTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CurriculumAdoptionTitle> for String {
    fn from(value: CurriculumAdoptionTitle) -> Self {
        value.0
    }
}

/// A destination title did not satisfy the shared curriculum text rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumAdoptionTitleError;

impl From<BlueprintCourseTitleError> for CurriculumAdoptionTitleError {
    fn from(_: BlueprintCourseTitleError) -> Self {
        Self
    }
}

impl std::fmt::Display for CurriculumAdoptionTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum adoption title is invalid")
    }
}

impl std::error::Error for CurriculumAdoptionTitleError {}
