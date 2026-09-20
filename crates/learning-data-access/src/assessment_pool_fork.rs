//! Trusted import of an immutable Question Pool fork for one Assessment Entry.

use async_trait::async_trait;
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentEntryScoringRule, AssessmentId,
    AssessmentPointValue, CourseInstanceId, QuestionId, QuestionPoolEditNumber,
    QuestionPoolSelectedQuestionOrder, QuestionRevisionTuple,
};

use crate::{SessionTokenHash, StoreError};

/// Server-issued Pool ID and source Pool ID for one Assessment-owned fork import.
///
/// This is deliberately not browser input: the server issues the child Pool ID
/// and the Store copies current source membership in one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAssessmentPoolForkInput {
    pub course_instance_id: CourseInstanceId,
    pub assessment_id: AssessmentId,
    pub assessment_entry: AssessmentEntryId,
    pub expected_assessment_edit_number: AssessmentEditNumber,
    pub fork_question_pool_id: QuestionId,
    pub source_question_pool_id: QuestionId,
    pub authored_position: u32,
    pub selection_count: std::num::NonZeroU32,
    pub points_per_item: AssessmentPointValue,
    pub selected_question_order: QuestionPoolSelectedQuestionOrder,
    pub scoring_rule: AssessmentEntryScoringRule,
}

/// Immutable fork identity created and associated in one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAssessmentPoolFork {
    pub assessment_entry: AssessmentEntryId,
    pub question_pool_id: QuestionId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub assessment_edit_number: AssessmentEditNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendAssessmentPoolForkMembersInput {
    /// Opaque route IDs; no browser supplies an internal Assessment ID.
    pub course_instance_id: CourseInstanceId,
    pub assessment_id: AssessmentId,
    pub assessment_entry: AssessmentEntryId,
    pub expected_assessment_edit_number: AssessmentEditNumber,
    pub expected_question_pool_edit_number: QuestionPoolEditNumber,
    pub members: Vec<QuestionRevisionTuple>,
    pub interchangeability_attested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendedAssessmentPoolFork {
    pub assessment_entry: AssessmentEntryId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    pub assessment_edit_number: AssessmentEditNumber,
}

/// Session-authorized atomic Assessment Pool import boundary.
#[async_trait]
pub trait AssessmentPoolForkStore: Send + Sync {
    /// Creates a forked Pool at Edit Number 1 and associates it with one Assessment Entry.
    async fn import_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        input: ImportAssessmentPoolForkInput,
    ) -> Result<ImportedAssessmentPoolFork, StoreError>;

    async fn append_assessment_question_pool_fork_members(
        &self,
        session_token_hash: SessionTokenHash,
        input: AppendAssessmentPoolForkMembersInput,
    ) -> Result<AppendedAssessmentPoolFork, StoreError>;
}
