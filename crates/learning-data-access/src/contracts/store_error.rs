use super::*;

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active ownership boundary or Question Library.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A record disagrees with authenticated ownership.
    OwnershipMismatch,
    /// Stored state changed after a caller validated its expected value.
    Conflict,
    /// PostgreSQL aborted the whole transaction due to a serialization or deadlock conflict.
    RetryableTransaction,
    /// Authenticated identity lacks ownership or role for the operation.
    Forbidden,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity evaluation rejected the transition.
    AssignmentActivity(AssignmentActivityError),
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
            Self::OwnershipMismatch => write!(formatter, "record ownership does not match context"),
            Self::Conflict => write!(formatter, "record changed before the operation committed"),
            Self::RetryableTransaction => write!(formatter, "transaction must be retried"),
            Self::Forbidden => write!(formatter, "operation is not authorized"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::AssignmentActivity(error) => {
                write!(formatter, "activity transition rejected: {error}")
            }
            Self::TimedOut => write!(formatter, "question attempt timed out"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<AssignmentActivityError> for StoreError {
    fn from(error: AssignmentActivityError) -> Self {
        Self::AssignmentActivity(error)
    }
}
