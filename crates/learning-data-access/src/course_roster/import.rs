//! Replica-safe staged roster-import contract.

use std::num::NonZeroU32;

use objects::Sha256Digest;
use question_model::{ActivityTimestamp, CourseId};
use uuid::Uuid;

use super::{
    AuthenticationEmail, CourseInvitation, CourseInvitationLifetime, CourseInvitationSecretHash,
    CourseRosterId, RosterIdempotencyKey, RosterRevision, StoreError,
};

/// Largest accepted roster data-row count.
pub const MAX_ROSTER_IMPORT_ROWS: usize = 500;

/// Server-minted identity for one normalized, short-lived preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseRosterImportId(Uuid);

impl CourseRosterImportId {
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!("roster import ID randomness unavailable: {error}"))
        })
        .map(Self)
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Strong optimistic-concurrency token for one staged import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RosterImportRevision(u64);

impl RosterImportRevision {
    pub const INITIAL: Self = Self(1);

    pub fn from_stored(value: i64) -> Result<Self, StoreError> {
        u64::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .map(Self)
            .ok_or_else(|| {
                StoreError::Unavailable("stored roster import revision is invalid".to_string())
            })
    }

    pub fn value(self) -> u64 {
        self.0
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0.checked_add(1).map(Self).ok_or(StoreError::Conflict)
    }
}

/// Short lifetime for normalized preview rows; raw CSV bytes are never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseRosterImportLifetime(NonZeroU32);

impl CourseRosterImportLifetime {
    pub const MAX_SECONDS: u32 = 24 * 60 * 60;

    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= Self::MAX_SECONDS)
            .map(Self)
    }

    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Safe row status shown without echoing malformed raw cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterImportRowStatus {
    ReadyToInvite,
    AlreadyMember,
    AlreadyPending,
    Duplicate,
    Invalid,
}

/// Normalized row supplied after the HTTP parser discards the raw CSV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRosterImportRowInput {
    pub row_number: u16,
    pub email: Option<AuthenticationEmail>,
    pub roster_id: Option<CourseRosterId>,
}

impl CourseRosterImportRowInput {
    pub fn validate_shape(&self) -> Result<(), StoreError> {
        if self.row_number < 2 {
            return Err(StoreError::InvalidRecord(
                "roster import row number is invalid".to_string(),
            ));
        }
        match (&self.email, &self.roster_id) {
            (Some(_), Some(_)) | (None, None) => Ok(()),
            _ => Err(StoreError::InvalidRecord(
                "roster import row is only partly normalized".to_string(),
            )),
        }
    }
}

/// One protected normalized preview row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRosterImportRow {
    pub row_number: u16,
    pub email: Option<AuthenticationEmail>,
    pub roster_id: Option<CourseRosterId>,
    pub status: RosterImportRowStatus,
}

/// Lifecycle of one normalized staged preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseRosterImportState {
    Preview,
    Committed,
}

/// Protected preview safe only for an authorized course instructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRosterImportPreview {
    pub id: CourseRosterImportId,
    pub course: CourseId,
    pub roster_revision: RosterRevision,
    pub revision: RosterImportRevision,
    pub state: CourseRosterImportState,
    pub expires_at: ActivityTimestamp,
    pub rows: Vec<CourseRosterImportRow>,
}

/// Stages one bounded normalized import under the current roster revision.
#[derive(Debug, Clone)]
pub struct StageCourseRosterImport {
    pub course: CourseId,
    pub expected_roster_revision: RosterRevision,
    pub normalized_digest: Sha256Digest,
    pub idempotency_key: RosterIdempotencyKey,
    pub rows: Vec<CourseRosterImportRowInput>,
    pub lifetime: CourseRosterImportLifetime,
}

/// Server-issued invitation binding for one ready row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterImportInvitation {
    pub row_number: u16,
    pub token_hash: CourseInvitationSecretHash,
    pub idempotency_key: RosterIdempotencyKey,
    pub lifetime: CourseInvitationLifetime,
}

/// Commits exactly the ready rows from one staged preview.
#[derive(Debug, Clone)]
pub struct CommitCourseRosterImport {
    pub course: CourseId,
    pub import: CourseRosterImportId,
    pub expected_import_revision: RosterImportRevision,
    pub idempotency_key: RosterIdempotencyKey,
    pub invitations: Vec<RosterImportInvitation>,
}

/// Atomic commit result used by the server-only delivery loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCourseRosterImport {
    pub import: CourseRosterImportId,
    pub import_revision: RosterImportRevision,
    pub roster_revision: RosterRevision,
    pub invitations: Vec<(u16, CourseInvitation)>,
}
