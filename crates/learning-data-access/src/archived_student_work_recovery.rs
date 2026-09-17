//! Deliberate, read-only retained Work recovery. No ordinary-history capability.

use async_trait::async_trait;
use question_model::{AssessmentAttemptReference, CourseInstanceReference};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Minimized retained selection; roster labels are Course-local and may be absent.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySummary {
    pub course: CourseInstanceReference,
    pub roster_id: Option<String>,
    pub assessment: question_model::AssessmentReference,
    pub assessment_title: String,
    pub assessment_attempt: AssessmentAttemptReference,
    pub assessment_attempt_number: u32,
    pub started_at: String,
    pub submitted_at: Option<String>,
    pub student_data_archived_at: String,
    pub delete_due_at: String,
}

/// Materialized evidence, not restoration, current scoring or executable content.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredAttempt {
    pub course: CourseInstanceReference,
    pub roster_id: Option<String>,
    pub assessment: question_model::AssessmentReference,
    pub assessment_attempt: AssessmentAttemptReference,
    pub assessment_attempt_number: u32,
    pub started_at: String,
    pub expires_at: Option<String>,
    pub student_data_archived_at: String,
    pub delete_due_at: String,
    pub assessment_title: String,
    pub attempt_facts_text: String,
    pub submission_text: Option<String>,
    pub questions: Vec<RecoveredQuestion>,
}

/// Text fields contain only inert, typed allowlisted evidence, never raw SQL JSON.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredQuestion {
    pub issued_position: u32,
    pub question_id: String,
    pub revision_number: u32,
    pub delivery_text: String,
    pub pool_text: Option<String>,
    pub attempt_text: Option<String>,
    pub presentation_text: Option<String>,
    pub reproduction_text: Option<String>,
    pub backend_document_text: Option<String>,
    pub saved_response_text: Option<String>,
    pub finalized_response_text: Option<String>,
    pub grading_text: Option<String>,
    pub unavailable_evidence: Vec<String>,
}

/// Both operations independently reauthorize in the originating session transaction.
#[async_trait]
pub trait ArchivedStudentWorkRecoveryStore: Send + Sync {
    async fn select_retained_work(
        &self,
        session: SessionTokenHash,
        course: CourseInstanceReference,
        after: Option<AssessmentAttemptReference>,
    ) -> Result<Vec<RecoverySummary>, StoreError>;

    async fn recover_retained_work(
        &self,
        session: SessionTokenHash,
        course: CourseInstanceReference,
        attempt: AssessmentAttemptReference,
    ) -> Result<RecoveredAttempt, StoreError>;
}

// ASVS 1.5.2/8.2.3/14.2.6: explicit typed allowlist for retained JSON.
// SQL's extra trusted-server fields are deliberately not serialized to browsers.
#[derive(Deserialize, Serialize)]
pub(crate) struct AttemptFacts {
    pub assessment_title: String,
    assessment_instructions: String,
    available_at: Option<String>,
    due_at: Option<String>,
    closes_at: Option<String>,
    assessment_attempt_time_limit_seconds: Option<u32>,
    assessment_attempt_limit: Option<u32>,
    late_work_rule: String,
    question_variation_rule: String,
    assessment_question_order_rule: String,
    feedback_score: String,
    feedback_per_item_correctness: String,
    feedback_submitted_response: String,
    feedback_question_answer: String,
    feedback_question_answer_explanation: String,
    feedback_class_statistics: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Submission {
    submitted_at: String,
    finalization_kind: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Delivery {
    assessment_content_entry_index: u32,
    pub issued_position: u32,
    pub question_id: String,
    pub revision_number: u32,
    point_value: f64,
    scoring_rule: String,
    question_attempt_limit: Option<u32>,
    question_attempt_time_limit_seconds: Option<u32>,
    question_attempt_grace_seconds: Option<u32>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PoolSelection {
    question_pool_id: String,
    question_pool_revision_number: u32,
    question_pool_member_position: u32,
    selection_position: u32,
    selected_question_count: u32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct AttemptTiming {
    issued_at: String,
    deadline_at: Option<String>,
    finalized_at: Option<String>,
    question_attempt_state: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Grading {
    grading_state: String,
    created_at: String,
    completed_at: Option<String>,
    normalized_credit: Option<f64>,
    recorded_at: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RetainedPresentation {
    pub presentation: question_model::presentation::QuestionPresentation,
    #[serde(skip_serializing)]
    pub backend_document: Option<String>,
    descriptor_version: u32,
    presentation_nonce: String,
    presentation_checksum: String,
    response_item_bindings: Vec<ResponseItemBinding>,
    asset_renditions: Vec<AssetRendition>,
}

#[derive(Deserialize, Serialize)]
struct ResponseItemBinding {
    presentation_response_item_reference: String,
    response_item_reference: String,
}
#[derive(Deserialize, Serialize)]
struct AssetRendition {
    asset_id: String,
    question_asset_checksum: String,
    rendition_checksum: String,
    intrinsic_width: u32,
    intrinsic_height: u32,
}

/// Immutable interpretation identities, not source bytes, credentials or object URLs.
#[derive(Deserialize, Serialize)]
pub(crate) struct Reproduction {
    backend_name: String,
    backend_version: String,
    renderer_name: Option<String>,
    renderer_version: Option<String>,
    grader_name: Option<String>,
    grader_version: Option<String>,
    source_object_id: Option<String>,
    source_object_checksum: Option<String>,
    question_seed: Option<String>,
    generated_parameter_sha256: Option<String>,
    rendered_question_sha256: Option<String>,
    issued_capability: String,
    webwork_pg_path: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RetainedResponse {
    pub student_response: question_model::response::StudentResponse,
    pub saved_at: Option<String>,
    pub finalized_at: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RetainedQuestion {
    pub delivery: Delivery,
    pub pool: Option<PoolSelection>,
    pub attempt: Option<AttemptTiming>,
    pub presentation: Option<RetainedPresentation>,
    pub reproduction: Option<Reproduction>,
    pub saved_response: Option<RetainedResponse>,
    pub finalized_response: Option<RetainedResponse>,
    pub grading: Option<Grading>,
}
