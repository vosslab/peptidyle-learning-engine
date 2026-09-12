//! Session-authorized persistence for one current Course Assignment workspace.
//!
//! The workspace writes the stable Assignment aggregate with its qualified
//! Edit Number. Every entry pins an exact Question Revision; release changes
//! Assignment Status and never creates an Assignment Revision.

use std::collections::BTreeSet;

use async_trait::async_trait;
use std::num::NonZeroU32;

use question_model::{
    AccountTimeZone, AssignmentActivityRules, AssignmentEditNumber, AssignmentEntry,
    AssignmentInstructions, AssignmentReference, AssignmentStatus, AssignmentTitle,
    BlueprintAssignmentSource, CourseInstanceReference, LateWorkRule, LocalDateAndTime,
    QuestionRevisionReference, StudentFeedbackReleaseRule,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Bounded initial authored content for one new Course-owned Assignment.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLiveAssignmentInput {
    /// Stable Blueprint Assignment selected from the source Blueprint Revision.
    pub blueprint_assignment_reference: Uuid,
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
    /// Optional local first instant at which future Attempts may begin.
    #[serde(default)]
    pub available_at: Option<LocalDateAndTime>,
    /// Optional local hard-close instant for future Attempts.
    #[serde(default)]
    pub closes_at: Option<LocalDateAndTime>,
    /// Current late-work rule used by future Attempts.
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
    /// Ordered normalized current Assignment Entries. Every entry pins exact
    /// Question Revision content for future Attempts.
    pub entries: Vec<AssignmentEntry>,
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
        if self.entries.len() > 1024 {
            return Err(StoreError::InvalidRecord(
                "Assignment Workspace may select at most 1,024 Assignment Entries".to_string(),
            ));
        }
        let mut identifiers = BTreeSet::new();
        if self
            .entries
            .iter()
            .any(|entry| !identifiers.insert(entry_id(entry)))
        {
            return Err(StoreError::InvalidRecord(
                "Assignment Workspace repeats an Assignment Entry identity".to_string(),
            ));
        }
        Ok(())
    }
}

/// Browser-safe Available Published Question row used by the bounded picker.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionPickerEntry {
    /// Exact currently accepted Question Revision that a new Entry will pin.
    pub reference: QuestionRevisionReference,
    /// Answer-free Question description supplied by the current Question Library metadata.
    pub description: String,
}

/// One ordinary Blueprint Assignment available to create an Assignment in a
/// specific Course.  Its exact Blueprint Revision is inherited from the
/// Course, rather than selected independently by the browser.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseAssignmentSourceChoice {
    /// Exact immutable reusable-content provenance retained by a created Assignment.
    pub source: BlueprintAssignmentSource,
    /// Answer-free human-readable selection label from that immutable Blueprint Revision.
    pub label: String,
}

/// Browser-safe current authored fixed-Question selection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredAssignmentQuestion {
    /// Exact Question Revision retained by this current Assignment entry.
    pub reference: QuestionRevisionReference,
    /// Answer-free Question description for Instructor review and Assignment Preview.
    pub description: String,
}

/// Browser-safe Assignment summary for one direct Course Instructor.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Immutable reusable-content provenance selected when this Assignment was created.
    /// This database-derived value is read-only; browser requests cannot supply it.
    pub source: BlueprintAssignmentSource,
    /// Current Instructor-authored title.
    pub title: AssignmentTitle,
    /// Current Student-facing instructions.
    pub instructions: AssignmentInstructions,
    /// Optional exact Due at projected in the authenticated Instructor zone.
    pub due_at: Option<LocalDateAndTime>,
    /// Optional first local instant at which future Attempts may begin.
    pub available_at: Option<LocalDateAndTime>,
    /// Optional local hard-close instant for future Attempts.
    pub closes_at: Option<LocalDateAndTime>,
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
    /// Ordered complete normalized entries for future Assignment Attempts.
    pub entries: Vec<AssignmentEntry>,
    /// Ordered current selected exact Question Revision pins.
    pub questions: Vec<AuthoredAssignmentQuestion>,
}

/// One complete calculated Assignment Release Issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentReleaseIssue {
    /// An Assignment Attempt duration must be chosen before release.
    TimeLimitRequired,
    /// The current Assignment requires at least one selected Question.
    NoPublishedQuestions,
    /// A previously selected Question Revision is no longer Available for release.
    QuestionUnavailable,
}

/// Calculated release validation changes neither Assignment nor Student work.
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
    use super::{
        AssignmentUnreleaseImpact, SaveLiveAssignmentInlineInput, SaveLiveAssignmentInput,
    };
    use question_model::{AssignmentEditNumber, AssignmentTitle};

    #[test]
    fn time_limit_required_is_a_stable_browser_issue() {
        assert_eq!(
            serde_json::to_value(super::AssignmentReleaseIssue::TimeLimitRequired)
                .expect("release issue serializes"),
            serde_json::json!("timeLimitRequired")
        );
    }

    #[test]
    fn unrelease_impact_exposes_only_aggregate_deletion_counts() {
        let impact = AssignmentUnreleaseImpact {
            confirmation_title: AssignmentTitle::try_new("Peptide bonds".to_string())
                .expect("valid title"),
            edit_number: AssignmentEditNumber::new(3).expect("valid edit number"),
            attempt_count: 2,
            submission_count: 5,
            grade_count: 3,
        };
        assert_eq!(
            serde_json::to_value(impact).expect("impact serializes"),
            serde_json::json!({
                "confirmationTitle": "Peptide bonds",
                "editNumber": "3",
                "attemptCount": 2,
                "submissionCount": 5,
                "gradeCount": 3
            })
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

    #[test]
    fn workspace_save_requires_an_exact_question_revision_pin() {
        let input: SaveLiveAssignmentInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "instructions": "Answer every question.",
            "entries": [{
                "kind": "fixedQuestion",
                "id": "00000000-0000-0000-0000-000000000001",
                "reference": { "questionId": "7K3-M9QP", "revisionNumber": 1 },
                "pointsPossible": "1",
                "availability": "available",
                "scoringRule": "normal",
                "questionAttemptLimit": { "maxAttempts": null },
                "questionAttemptTimeLimit": { "kind": "unlimited" }
            }]
        }))
        .expect("exact Question Revision selection deserializes");
        assert_eq!(input.entries.len(), 1);
        assert!(input.validate().is_ok());
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

/// Aggregate, non-identifying Student Work affected by a proposed Unrelease.
///
/// This projection exists only while the current Assignment is Released.  It
/// intentionally contains no Student, response, or grade detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentUnreleaseImpact {
    /// Current Assignment Title that the Instructor must repeat to confirm.
    pub confirmation_title: AssignmentTitle,
    /// Exact compare-and-swap value required by the destructive transition.
    pub edit_number: AssignmentEditNumber,
    /// Assignment Attempts that the transition will delete.
    pub attempt_count: u64,
    /// All Question and Assignment submissions that the transition will delete.
    pub submission_count: u64,
    /// Grading results that the transition will delete.
    pub grade_count: u64,
}

/// Result of one accepted Assignment Unrelease transition.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreleasedLiveAssignment {
    /// The complete current Assignment aggregate after it becomes Unreleased.
    pub assignment: LiveAssignmentWorkspace,
    /// Aggregate deletion receipt; detailed Student Work never leaves the database.
    pub deleted: AssignmentUnreleaseImpact,
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

    /// Lists the stable Blueprint Assignments in this Course's exact pinned
    /// Blueprint Revision.  The browser selects one stable member identity;
    /// PostgreSQL derives and retains the complete exact provenance.
    async fn list_course_assignment_source_choices(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseAssignmentSourceChoice>, StoreError>;

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

    /// Transitions the current Assignment from Unreleased to Released after
    /// exact Edit Number and current-content validation.
    async fn release_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected_edit_number: AssignmentEditNumber,
    ) -> Result<LiveAssignmentWorkspace, StoreError>;

    /// Reads the exact, aggregate impact of Unrelease for a currently Released Assignment.
    async fn read_live_assignment_unrelease_impact(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentUnreleaseImpact, StoreError>;

    /// Atomically deletes rooted Student Work and restores the current Assignment
    /// to Unreleased after exact Edit Number and title confirmation.
    async fn unrelease_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected_edit_number: AssignmentEditNumber,
        confirmation_title: AssignmentTitle,
    ) -> Result<UnreleasedLiveAssignment, StoreError>;
}

fn entry_id(entry: &AssignmentEntry) -> String {
    match entry {
        AssignmentEntry::FixedQuestion(value) => value.id.to_string(),
        AssignmentEntry::QuestionPool(value) => value.id.to_string(),
    }
}
