//! Browser-safe contracts for an Instructor's non-mutating Question Pool Preview.
//!
//! A preview describes a saved Question Pool. It deliberately contains no
//! selected Question Pool Item identity, Question Pool Preview Nonce, Answer Key, Student identity,
//! or issued-work record. The server owns the temporary preview nonce and discards it after the
//! response is produced.

use serde::{Deserialize, Serialize};

use crate::{AssessmentEditNumber, AssessmentEntryId, AssessmentId, PublishedQuestionId};

/// Strict request body for an Instructor's one-off sample of a saved pool.
/// The route owns course and assessment identity; the Assessment edit number comes from the
/// `If-Match` header, so the browser can select only one saved Assessment Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreviewRequest {
    pub assessment_entry_id: AssessmentEntryId,
}

/// The public Question Library identity and Question Title that are safe in an Instructor
/// preview.  A Question ID remains the sole human-facing question identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreviewItem {
    pub question_id: PublishedQuestionId,
    pub question_title: String,
}

/// One no-store preview result for a saved assessment Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreview {
    pub assessment_id: AssessmentId,
    pub assessment_edit_number: AssessmentEditNumber,
    pub assessment_entry_id: AssessmentEntryId,
    /// Stable presentation label derived from the saved Question Pool
    /// Assessment Entry order. Question Pool Assessment Entries have no
    /// Instructor-authored label in v1.
    pub question_pool_label: String,
    pub selection_count: u32,
    pub items: Vec<QuestionPoolPreviewItem>,
    pub selected_items: Vec<QuestionPoolPreviewItem>,
}
