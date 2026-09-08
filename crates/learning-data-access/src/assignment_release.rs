//! Session-authorized Assignment Workspace persistence for the Live Demo.
//!
//! This is deliberately the first small Course-owned Assignment boundary: an
//! Instructor selects a bounded ordered set of currently Available Published
//! Questions, saves with an Assignment Edit Number, validates, previews, and
//! releases one immutable Assignment Revision. Student delivery uses its own boundary.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    AssignmentEditNumber, AssignmentInstructions, AssignmentReference, AssignmentStatus,
    AssignmentTitle, CourseInstanceReference, CourseLocalDateAndTime, LateWorkRule, QuestionId,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Bounded initial authored content for one new Course-owned Assignment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLiveAssignmentInput {
    /// Instructor-facing Assignment Title.
    pub title: AssignmentTitle,
    /// Plain-text Student-facing instructions; empty text remains valid.
    pub instructions: AssignmentInstructions,
}

/// Complete current authored Assignment content saved with an exact Edit Number.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLiveAssignmentInput {
    /// Compare-and-swap precondition for the current authored Assignment.
    #[serde(skip_deserializing, default = "initial_assignment_edit_number")]
    pub expected_edit_number: AssignmentEditNumber,
    /// Instructor-facing Assignment Title.
    pub title: AssignmentTitle,
    /// Plain-text Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Optional strict local Due at in the Course's authoritative IANA zone.
    #[serde(default)]
    pub due_at: Option<CourseLocalDateAndTime>,
    /// Student-work rule captured by a later immutable Assignment Revision.
    #[serde(default = "default_late_work_rule")]
    pub late_work_rule: LateWorkRule,
    /// Ordered Available Published Questions selected for the Assignment.
    pub question_ids: Vec<QuestionId>,
}

fn initial_assignment_edit_number() -> AssignmentEditNumber {
    AssignmentEditNumber::INITIAL
}

fn default_late_work_rule() -> LateWorkRule {
    LateWorkRule::Accept
}

impl SaveLiveAssignmentInput {
    /// Rejects only the bounded invalid selection shapes this first workspace owns.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.question_ids.len() > 25 {
            return Err(StoreError::InvalidRecord(
                "Assignment Workspace may select at most twenty-five Published Questions"
                    .to_string(),
            ));
        }
        let mut identifiers = BTreeSet::new();
        if self
            .question_ids
            .iter()
            .map(ToString::to_string)
            .any(|question_id| !identifiers.insert(question_id))
        {
            return Err(StoreError::InvalidRecord(
                "Assignment Workspace repeats a Published Question".to_string(),
            ));
        }
        Ok(())
    }
}

/// Browser-safe Available Published Question row used by the bounded picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionPickerEntry {
    /// Stable global Published Question ID.
    pub question_id: QuestionId,
    /// Answer-free Question description supplied by the current Question Library metadata.
    pub description: String,
}

/// Browser-safe current authored fixed-Question selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredAssignmentQuestion {
    /// Stable global Published Question ID.
    pub question_id: QuestionId,
    /// Answer-free Question description for Instructor review and Assignment Preview.
    pub description: String,
}

/// Complete current Assignment Workspace projection for one direct Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssignmentWorkspace {
    /// Public Assignment Reference; internal Assignment identity remains server-side.
    pub reference: AssignmentReference,
    /// Exact compare-and-swap value for the current authored content.
    pub edit_number: AssignmentEditNumber,
    /// Stable Assignment lifecycle, separate from future Assignment Access.
    pub status: AssignmentStatus,
    /// Current Instructor-authored title.
    pub title: AssignmentTitle,
    /// Current Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Optional exact Due at in the authoritative Course zone.
    pub due_at: Option<CourseLocalDateAndTime>,
    /// Current canonical Late Work Rule.
    pub late_work_rule: LateWorkRule,
    /// Ordered current fixed-Question selection.
    pub questions: Vec<AuthoredAssignmentQuestion>,
}

/// One complete calculated Assignment Release Issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentReleaseIssue {
    /// An immutable Assignment Revision requires at least one selected Question.
    NoPublishedQuestions,
    /// A previously selected Question Revision is no longer Available for release.
    QuestionUnavailable,
}

/// Calculated release validation; it creates no Revision or Student work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentReleaseValidation {
    /// True only when the current authored Assignment may create a Revision.
    pub can_release: bool,
    /// All current release blockers in the small release boundary.
    pub issues: Vec<AssignmentReleaseIssue>,
}

/// Answer-free Instructor-authorized Assignment Preview for the current Assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentPreview {
    /// Current Student-visible Assignment title.
    pub title: AssignmentTitle,
    /// Current Student-visible instructions.
    pub instructions: AssignmentInstructions,
    /// Ordered answer-free Question descriptions; this does not create Student delivery.
    pub questions: Vec<AuthoredAssignmentQuestion>,
}

/// Result of one successful immutable Assignment Release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasedLiveAssignment {
    /// The Assignment that selected the new immutable Revision.
    pub reference: AssignmentReference,
    /// Positive immutable revision number selected for future Student delivery.
    pub revision_number: u64,
}

/// Store boundary for Assignment Workspace and release operations.
#[async_trait]
pub trait LiveAssignmentStore: Send + Sync {
    /// Lists answer-free currently Available Published Questions for one authorized picker.
    async fn list_assignment_question_picker(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<AssignmentQuestionPickerEntry>, StoreError>;

    /// Creates one Unreleased Assignment without Question selection or Student activity.
    async fn create_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        input: CreateLiveAssignmentInput,
    ) -> Result<LiveAssignmentWorkspace, StoreError>;

    /// Loads one Assignment only through the caller's direct Instructor Course Membership.
    async fn load_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentWorkspace, StoreError>;

    /// Saves complete authored content with its exact Assignment Edit Number.
    async fn save_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        input: SaveLiveAssignmentInput,
    ) -> Result<LiveAssignmentWorkspace, StoreError>;

    /// Calculates the current small release boundary without mutating state.
    async fn validate_live_assignment_release(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentReleaseValidation, StoreError>;

    /// Loads the narrow answer-free Assignment Preview without creating Student work.
    async fn load_live_assignment_preview(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentPreview, StoreError>;

    /// Creates the next immutable Assignment Revision after exact Edit Number validation.
    async fn release_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected_edit_number: AssignmentEditNumber,
    ) -> Result<ReleasedLiveAssignment, StoreError>;
}
