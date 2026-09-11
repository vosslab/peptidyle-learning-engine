//! Session-authorized Assignment Workspace persistence for the Live Demo.
//!
//! This is deliberately the first small Course-owned Assignment boundary: an
//! Instructor selects a bounded ordered set of currently Available Published
//! Questions, saves with an Assignment Edit Number, validates, previews, and
//! releases one immutable Assignment Revision. Student delivery uses its own boundary.

use std::collections::BTreeSet;

use async_trait::async_trait;
use std::num::NonZeroU32;

use question_model::{
    AccountTimeZone, AssignmentActivityRules, AssignmentEditNumber, AssignmentInstructions,
    AssignmentReference, AssignmentStatus, AssignmentTitle, CourseInstanceReference, LateWorkRule,
    LocalDateAndTime, QuestionId, StudentFeedbackReleaseRule,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Bounded initial authored content for one new Course-owned Assignment.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLiveAssignmentInput {
    /// Instructor-facing Assignment Title.
    pub title: AssignmentTitle,
    /// Plain-text Student-facing instructions; empty text remains valid.
    pub instructions: AssignmentInstructions,
}

/// Complete current authored Assignment content saved with an exact Edit Number.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLiveAssignmentInput {
    /// Compare-and-swap precondition for the current authored Assignment.
    #[serde(skip_deserializing, default = "initial_assignment_edit_number")]
    pub expected_edit_number: AssignmentEditNumber,
    /// Instructor-facing Assignment Title.
    pub title: AssignmentTitle,
    /// Plain-text Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Optional exact local Due at; the Store resolves it in the authenticated Instructor zone.
    #[serde(default)]
    pub due_at: Option<LocalDateAndTime>,
    /// Student-work rule captured by a later immutable Assignment Revision.
    #[serde(default = "default_late_work_rule")]
    pub late_work_rule: LateWorkRule,
    /// Whole-attempt limit in seconds, when the Assignment is timed.
    #[serde(default)]
    pub assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum Assignment Attempts, when bounded.
    #[serde(default)]
    pub attempt_limit: Option<NonZeroU32>,
    /// The nine independent Assignment activity rules.
    #[serde(default)]
    pub activity_rules: AssignmentActivityRules,
    /// The seven independently configured Student feedback timings.
    #[serde(default)]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Ordered Available Published Questions selected for the Assignment.
    pub question_ids: Vec<QuestionId>,
}

fn initial_assignment_edit_number() -> AssignmentEditNumber {
    AssignmentEditNumber::INITIAL
}

fn default_late_work_rule() -> LateWorkRule {
    LateWorkRule::Reject
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
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionPickerEntry {
    /// Stable global Published Question ID.
    pub question_id: QuestionId,
    /// Answer-free Question description supplied by the current Question Library metadata.
    pub description: String,
}

/// Browser-safe current authored fixed-Question selection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredAssignmentQuestion {
    /// Stable global Published Question ID.
    pub question_id: QuestionId,
    /// Answer-free Question description for Instructor review and Assignment Preview.
    pub description: String,
}

/// Browser-safe Assignment summary for one direct Course Instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseAssignmentSummary {
    /// Public Assignment Reference; internal Assignment identity remains server-side.
    pub reference: AssignmentReference,
    /// Current Instructor-authored Assignment Title.
    pub title: AssignmentTitle,
    /// Optional current Due at projected in the authenticated Instructor zone.
    pub due_at: Option<LocalDateAndTime>,
    /// Authenticated Instructor zone governing the returned local Due at.
    pub display_time_zone: AccountTimeZone,
    /// Stable Assignment lifecycle, separate from Student Assignment Access.
    pub status: AssignmentStatus,
    /// Exact compare-and-swap value for the current authored content.
    pub edit_number: AssignmentEditNumber,
}

/// One answer-free Assignment due in the current Instructor's rolling next-seven-days window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueSoonAssignmentSummary {
    /// Public Course Instance reference; internal Course identity remains server-side.
    pub course_reference: CourseInstanceReference,
    /// Descriptive Course Instance name for cross-Course lists.
    pub course_long_name: String,
    /// Public Assignment reference; internal Assignment identity remains server-side.
    pub assignment_reference: AssignmentReference,
    /// Current Instructor-authored Assignment title.
    pub assignment_title: AssignmentTitle,
    /// Current Assignment lifecycle, limited by the reader to ordinary actionable states.
    pub assignment_status: AssignmentStatus,
    /// Stored deadline instant as Unix milliseconds, not a Course-local wall-clock value.
    pub due_at_millis: i64,
}

/// Bounded cross-Course Due Soon projection for one authenticated Instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueSoonAssignments {
    /// Answer-free Assignment rows ordered by their stored due instant.
    pub items: Vec<DueSoonAssignmentSummary>,
    /// This bounded reader has no pagination cursor.
    pub next_cursor: Option<String>,
    /// Authenticated Account-owned zone for the browser's due-instant display.
    pub display_time_zone: AccountTimeZone,
}

/// The only mutable fields exposed by the Course Assignment-list inline save.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLiveAssignmentInlineInput {
    // ASVS 1.5.2 and 2.2.1: accept only this closed, validated request shape.
    /// Instructor-facing Assignment Title.
    pub title: AssignmentTitle,
    /// Optional raw local Due at; the Store resolves it in the Instructor zone.
    #[serde(deserialize_with = "deserialize_required_option")]
    pub due_at: Option<LocalDateAndTime>,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

/// Complete current Assignment Workspace projection for one direct Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Optional exact Due at projected in the authenticated Instructor zone.
    pub due_at: Option<LocalDateAndTime>,
    /// Current canonical Late Work Rule.
    pub late_work_rule: LateWorkRule,
    /// Whole-attempt limit in seconds, when configured.
    pub assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum Assignment Attempts, when configured.
    pub attempt_limit: Option<NonZeroU32>,
    /// The complete current activity policy.
    pub activity_rules: AssignmentActivityRules,
    /// The complete current disclosure policy.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Authenticated Instructor display zone; never accepted in a request.
    pub display_time_zone: AccountTimeZone,
    /// Ordered current fixed-Question selection.
    pub questions: Vec<AuthoredAssignmentQuestion>,
}

/// One complete calculated Assignment Release Issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentReleaseIssue {
    /// An Assignment Attempt duration must be chosen before release.
    TimeLimitRequired,
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

#[cfg(test)]
mod tests {
    use super::{AssignmentReleaseIssue, SaveLiveAssignmentInlineInput};

    #[test]
    fn time_limit_required_is_a_stable_browser_issue() {
        assert_eq!(
            serde_json::to_value(AssignmentReleaseIssue::TimeLimitRequired)
                .expect("release issue serializes"),
            serde_json::json!("timeLimitRequired")
        );
    }

    #[test]
    fn inline_save_input_accepts_only_the_closed_validated_shape() {
        assert!(
            serde_json::from_value::<SaveLiveAssignmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds"
            }))
            .is_err()
        );

        let input: SaveLiveAssignmentInlineInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "dueAt": "2026-09-11T14:30:00.000"
        }))
        .expect("valid inline save input deserializes");
        assert_eq!(input.title.as_str(), "Peptide bonds");
        assert!(input.due_at.is_some());

        let input: SaveLiveAssignmentInlineInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "dueAt": null
        }))
        .expect("explicit null inline due date deserializes");
        assert!(input.due_at.is_none());

        assert!(
            serde_json::from_value::<SaveLiveAssignmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds",
                "dueAt": null,
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SaveLiveAssignmentInlineInput>(serde_json::json!({
                "title": "",
                "dueAt": null
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SaveLiveAssignmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds",
                "dueAt": "invalid"
            }))
            .is_err()
        );
    }
}

/// Answer-free Instructor-authorized Assignment Preview for the current Assignment.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Lists ordinary current Assignments due in the caller's rolling next-seven-days window.
    async fn list_assignments_due_soon(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<DueSoonAssignments, StoreError>;

    /// Lists only Assignments owned by one exact authorized Course Instance.
    async fn list_course_assignments(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseAssignmentSummary>, StoreError>;

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

    /// Saves only the current Title and Due at with an exact Edit Number.
    async fn save_live_assignment_inline(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected_edit_number: AssignmentEditNumber,
        input: SaveLiveAssignmentInlineInput,
    ) -> Result<CourseAssignmentSummary, StoreError>;

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
