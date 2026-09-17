//! Count-only editing for one Assessment-owned Question Pool entry.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentQuestionPoolSelectionCountReceipt,
    AssessmentReference, CourseInstanceReference,
};

use crate::{SessionTokenHash, StoreError};

/// Closed input for one optimistic-concurrency Pool selection-count change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentPoolSelectionCountInput {
    /// Opaque Course reference from the canonical route.
    pub course: CourseInstanceReference,
    /// Opaque Assessment reference from the canonical route.
    pub assessment: AssessmentReference,
    /// Stable Assessment Entry selected by the route.
    pub assessment_entry: AssessmentEntryId,
    /// Strong current Assessment Edit Number from `If-Match`.
    pub expected_assessment_edit_number: AssessmentEditNumber,
    /// Positive count selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
}

/// Session-authorized atomic count-only mutation boundary.
#[async_trait]
pub trait AssessmentPoolSelectionCountStore: Send + Sync {
    /// Updates only the owned Pool Entry count and advances its Assessment edit once.
    async fn update_assessment_question_pool_selection_count(
        &self,
        session_token_hash: SessionTokenHash,
        input: AssessmentPoolSelectionCountInput,
    ) -> Result<AssessmentQuestionPoolSelectionCountReceipt, StoreError>;
}
