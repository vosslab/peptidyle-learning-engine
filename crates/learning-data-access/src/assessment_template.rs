//! Session-authorized persistence for private Instructor-owned Assessment Templates.

use async_trait::async_trait;
use question_model::{
    AssessmentTemplate, AssessmentTemplateEditNumber, AssessmentTemplateId, AssessmentTemplateName,
    AssessmentTemplateSettings, AssessmentType,
};

use crate::{SessionTokenHash, StoreError};

/// Complete replacement accepted for one current Assessment Template.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveAssessmentTemplateInput {
    /// Private server-issued identity of the owned Template.
    pub id: AssessmentTemplateId,
    /// Exact current compare-and-swap value.
    pub expected_edit_number: AssessmentTemplateEditNumber,
    /// Replacement Instructor-facing name.
    pub name: AssessmentTemplateName,
    /// Replacement pedagogical purpose; settings remain independently supplied.
    pub assessment_type: AssessmentType,
    /// Complete reusable settings copied into future Assessments.
    pub settings: AssessmentTemplateSettings,
}

/// Persistence boundary for current private Assessment Template aggregates.
#[async_trait]
pub trait AssessmentTemplateStore: Send + Sync {
    /// Lists only Templates owned by the current active Instructor.
    async fn list_assessment_templates(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<AssessmentTemplate>, StoreError>;

    /// Reads one Template only when it belongs to the current active Instructor.
    async fn read_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        id: AssessmentTemplateId,
    ) -> Result<AssessmentTemplate, StoreError>;

    /// Creates one server-identified Template for the current active Instructor.
    async fn create_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        template: AssessmentTemplate,
    ) -> Result<AssessmentTemplate, StoreError>;

    /// Replaces one owned Template under its exact current Edit Number.
    async fn save_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAssessmentTemplateInput,
    ) -> Result<AssessmentTemplate, StoreError>;
}
