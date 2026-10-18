//! Explicit maintenance of missing artifacts for existing immutable Revisions.

use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::PublishedQuestionRevisionTuple;

use crate::{PreparedQuestionThumbnailPublication, PublishedQuestionThumbnail, StoreError};

/// Exact registered source facts, including an optional existing artifact.
#[derive(Debug, Clone)]
pub struct QuestionThumbnailBackfillEvidence {
    pub source_record: ObjectRecord,
    pub webwork_pg_path: String,
    pub thumbnail: Option<PublishedQuestionThumbnail>,
}

/// Keeps malformed evidence attached to its exact Revision rather than
/// silently excluding it from the finite maintenance snapshot.
#[derive(Debug, Clone)]
pub struct QuestionThumbnailBackfillRevision {
    pub revision: PublishedQuestionRevisionTuple,
    pub evidence: Result<QuestionThumbnailBackfillEvidence, StoreError>,
}

/// Only confirmed noncommit permits deletion of a prewritten candidate.
#[derive(Debug)]
pub enum QuestionThumbnailBackfillRegistrationError {
    NotCommitted(StoreError),
    Unknown(StoreError),
}

/// Maintenance authority is the trusted migrator, never an application session.
#[async_trait]
pub trait QuestionThumbnailBackfillStore: Send + Sync {
    /// Includes historical and archived WeBWorK Revisions in exact tuple order.
    async fn snapshot(&self) -> Result<Vec<QuestionThumbnailBackfillRevision>, StoreError>;

    /// Reads a concurrent winner through the same exact maintenance projection.
    async fn load_revision(
        &self,
        revision: &PublishedQuestionRevisionTuple,
    ) -> Result<QuestionThumbnailBackfillEvidence, StoreError>;

    /// Returns true only when this candidate was registered. False means a
    /// concurrent immutable relation already exists and must be verified.
    async fn register(
        &self,
        revision: &PublishedQuestionRevisionTuple,
        source: &ObjectRecord,
        thumbnail: &PreparedQuestionThumbnailPublication,
    ) -> Result<bool, QuestionThumbnailBackfillRegistrationError>;
}
