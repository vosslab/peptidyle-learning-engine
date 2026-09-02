//! Bounded scalar values used by Blueprint-operation commands and receipts.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Largest browser-supplied Request Retry Token accepted by one write request.
pub const MAX_REQUEST_RETRY_TOKEN_BYTES: usize = 128;

/// Exact SHA-256 checksum supplied by the trusted server request boundary for one operation.
///
/// This server-held scalar keeps request-integrity evidence distinct from object
/// checksums, presentation checksums, and content-derived identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestChecksum([u8; 32]);

impl RequestChecksum {
    /// Records the complete SHA-256 checksum supplied by the request boundary.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact bytes for trusted server persistence.
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// A validated opaque browser token for one exact retried Instructor write request.
///
/// The authenticated Account, exact Request Checksum, and typed request/Receipt context bind
/// the token. It grants no authority by itself.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RequestRetryToken(String);

impl RequestRetryToken {
    /// Parses the bounded opaque token used for one completed request retry.
    pub fn parse(value: &str) -> Result<Self, RequestRetryTokenError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_REQUEST_RETRY_TOKEN_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        valid
            .then(|| Self(value.to_owned()))
            .ok_or(RequestRetryTokenError)
    }

    /// Returns the opaque browser value without assigning any authority to it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RequestRetryToken {
    type Error = RequestRetryTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<RequestRetryToken> for String {
    fn from(value: RequestRetryToken) -> Self {
        value.0
    }
}

impl std::fmt::Debug for RequestRetryToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RequestRetryToken([opaque])")
    }
}

/// A Request Retry Token was blank, oversized, or outside the opaque-token alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestRetryTokenError;

impl std::fmt::Display for RequestRetryTokenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("request retry token is invalid")
    }
}

impl std::error::Error for RequestRetryTokenError {}

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
