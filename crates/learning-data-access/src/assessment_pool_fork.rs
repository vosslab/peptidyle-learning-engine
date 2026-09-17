//! Trusted import of an immutable Question Pool fork for one Assessment Entry.

use async_trait::async_trait;
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentEntryScoringRule, AssessmentPointValue,
    AssessmentReference, CourseInstanceReference, QuestionId, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionPoolSelectedQuestionOrder, QuestionRevisionReference,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Server-resolved exact source and server-issued identity for one Pool import.
///
/// This is deliberately not browser input: the server resolves the public
/// source Pool ID to its current immutable root Revision and validates the
/// fresh child public ID before calling the Store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAssessmentPoolForkInput {
    /// Trusted classification receipt for the exact imported Pool candidate.
    pub bloom_preparation_receipt_id: crate::BloomPreparationReceiptId,
    /// Opaque route references; the Store resolves their authorized internal
    /// Assessment identity in the same transaction as the fork import.
    pub course: CourseInstanceReference,
    pub assessment: AssessmentReference,
    pub assessment_entry: AssessmentEntryId,
    pub expected_assessment_edit_number: AssessmentEditNumber,
    pub fork_question_pool_id: Uuid,
    pub fork_public_question_pool_id: QuestionId,
    pub source_public_question_pool_id: QuestionId,
    pub authored_position: u32,
    pub selection_count: std::num::NonZeroU32,
    pub points_per_item: AssessmentPointValue,
    pub selected_question_order: QuestionPoolSelectedQuestionOrder,
    pub scoring_rule: AssessmentEntryScoringRule,
}

/// Immutable fork reference created and associated in one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAssessmentPoolFork {
    pub assessment_entry: AssessmentEntryId,
    pub question_pool_revision: QuestionPoolRevisionReference,
    pub assessment_edit_number: AssessmentEditNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendAssessmentPoolForkRevisionInput {
    /// Trusted classification receipt for the exact replacement Pool candidate.
    pub bloom_preparation_receipt_id: crate::BloomPreparationReceiptId,
    /// Opaque route references; no browser supplies an internal Assessment ID.
    pub course: CourseInstanceReference,
    pub assessment: AssessmentReference,
    pub assessment_entry: AssessmentEntryId,
    pub expected_assessment_edit_number: AssessmentEditNumber,
    pub expected_pool_metadata_etag: Uuid,
    pub members: Vec<QuestionRevisionReference>,
    pub interchangeability_attested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendedAssessmentPoolForkRevision {
    pub assessment_entry: AssessmentEntryId,
    pub question_pool_revision_number: QuestionPoolRevisionNumber,
    pub metadata_etag: Uuid,
    pub assessment_edit_number: AssessmentEditNumber,
}

/// Session-authorized atomic Assessment Pool import boundary.
#[async_trait]
pub trait AssessmentPoolForkStore: Send + Sync {
    /// Creates fork Revision 1 and associates it with exactly one Assessment Entry.
    async fn import_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        input: ImportAssessmentPoolForkInput,
    ) -> Result<ImportedAssessmentPoolFork, StoreError>;

    async fn append_assessment_question_pool_fork_revision(
        &self,
        session_token_hash: SessionTokenHash,
        input: AppendAssessmentPoolForkRevisionInput,
    ) -> Result<AppendedAssessmentPoolForkRevision, StoreError>;
}
