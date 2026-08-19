use super::*;

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active tenant or shared catalog.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A tenant-owned record disagrees with authenticated context.
    TenantMismatch,
    /// Stored state changed after a caller validated its expected value.
    Conflict,
    /// PostgreSQL aborted the whole transaction due to a serialization or deadlock conflict.
    RetryableTransaction,
    /// Authenticated identity lacks ownership or role for the operation.
    Forbidden,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity projection rejected the transition.
    RunModel(RunModelError),
    /// The database-authoritative timer no longer accepts this response.
    TimedOut,
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "record not found"),
            Self::AlreadyExists => write!(formatter, "immutable record already exists"),
            Self::TenantMismatch => write!(formatter, "record tenant does not match context"),
            Self::Conflict => write!(formatter, "record changed before the operation committed"),
            Self::RetryableTransaction => write!(formatter, "transaction must be retried"),
            Self::Forbidden => write!(formatter, "operation is not authorized"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::RunModel(error) => write!(formatter, "activity transition rejected: {error}"),
            Self::TimedOut => write!(formatter, "question attempt timed out"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<RunModelError> for StoreError {
    fn from(error: RunModelError) -> Self {
        Self::RunModel(error)
    }
}
