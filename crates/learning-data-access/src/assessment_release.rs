//! Session-authorized persistence for one current Course Assessment workspace.
//!
//! The workspace writes the stable Assessment aggregate with its qualified
//! Edit Number. Every entry pins an exact Question Revision; release changes
//! Assessment Status and never creates an Assessment Revision.

use std::collections::BTreeSet;

use async_trait::async_trait;
use std::num::NonZeroU32;

use question_model::{
    AccountTimeZone, AssessmentActivityRules, AssessmentEditNumber, AssessmentEntry,
    AssessmentInstructions, AssessmentOrigin, AssessmentReference, AssessmentStatus,
    AssessmentTitle, AssessmentType, CourseInstanceReference, LateWorkRule, LocalDateAndTime,
    QuestionRevisionReference, StudentFeedbackReleaseRule,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Bounded initial authored content for one new Course-owned Assessment.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLiveAssessmentInput {
    /// Instructor-selected fixed pedagogical purpose.
    pub assessment_type: AssessmentType,
    /// Instructor-facing Assessment Title.
    pub title: AssessmentTitle,
    /// Plain-text Student-facing instructions; empty text remains valid.
    pub instructions: AssessmentInstructions,
}

/// Complete current authored Assessment content saved with an exact Edit Number.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLiveAssessmentInput {
    /// Compare-and-swap precondition for the current authored Assessment.
    #[serde(skip_deserializing, default = "initial_assessment_edit_number")]
    pub expected_edit_number: AssessmentEditNumber,
    /// Instructor-facing Assessment Title.
    pub title: AssessmentTitle,
    /// Plain-text Student-facing instructions.
    pub instructions: AssessmentInstructions,
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
    /// Whole-attempt limit in seconds, when the Assessment is timed.
    #[serde(default)]
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum Assessment Attempts, when bounded.
    #[serde(default)]
    pub attempt_limit: Option<NonZeroU32>,
    /// The nine independent Assessment activity rules.
    #[serde(default)]
    pub activity_rules: AssessmentActivityRules,
    /// The six independently configured Student feedback timings.
    #[serde(default)]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Ordered normalized current Assessment Entries. Every entry pins exact
    /// Question Revision content for future Attempts.
    pub entries: Vec<AssessmentEntry>,
}

/// The policy-owned slice of one current Assessment, saved with its exact Edit Number.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveBaseAssessmentPolicyInput {
    #[serde(skip_deserializing, default = "initial_assessment_edit_number")]
    pub expected_edit_number: AssessmentEditNumber,
    pub instructions: AssessmentInstructions,
    #[serde(default)]
    pub due_at: Option<LocalDateAndTime>,
    #[serde(default)]
    pub available_at: Option<LocalDateAndTime>,
    #[serde(default)]
    pub closes_at: Option<LocalDateAndTime>,
    #[serde(default = "default_late_work_rule")]
    pub late_work_rule: LateWorkRule,
    #[serde(default)]
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    #[serde(default)]
    pub attempt_limit: Option<NonZeroU32>,
    #[serde(default)]
    pub activity_rules: AssessmentActivityRules,
    #[serde(default)]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

fn initial_assessment_edit_number() -> AssessmentEditNumber {
    AssessmentEditNumber::INITIAL
}

fn default_late_work_rule() -> LateWorkRule {
    LateWorkRule::Reject
}

impl SaveLiveAssessmentInput {
    /// Rejects only the bounded invalid selection shapes this first workspace owns.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.entries.len() > 1024 {
            return Err(StoreError::InvalidRecord(
                "Assessment Workspace may select at most 1,024 Assessment Entries".to_string(),
            ));
        }
        let mut identifiers = BTreeSet::new();
        if self
            .entries
            .iter()
            .any(|entry| !identifiers.insert(entry_id(entry)))
        {
            return Err(StoreError::InvalidRecord(
                "Assessment Workspace repeats an Assessment Entry identity".to_string(),
            ));
        }
        Ok(())
    }
}

/// Browser-safe Available Published Question row used by the bounded picker.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentQuestionPickerEntry {
    /// Exact currently accepted Question Revision that a new Entry will pin.
    pub reference: QuestionRevisionReference,
    /// Answer-free Question description supplied by the current Question Library metadata.
    pub description: String,
}

/// Browser-safe current authored fixed-Question selection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredAssessmentQuestion {
    /// Exact Question Revision retained by this current Assessment entry.
    pub reference: QuestionRevisionReference,
    /// Answer-free Question description for Instructor review and Assessment Preview.
    pub description: String,
}

/// Browser-safe Assessment summary for one direct Course Instructor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseAssessmentSummary {
    /// Public Assessment Reference; internal Assessment identity remains server-side.
    pub reference: AssessmentReference,
    /// Fixed pedagogical purpose of this Assessment.
    pub assessment_type: AssessmentType,
    /// Current Instructor-authored Assessment Title.
    pub title: AssessmentTitle,
    /// Optional current Due at projected in the authenticated Instructor zone.
    pub due_at: Option<LocalDateAndTime>,
    /// Authenticated Instructor zone governing the returned local Due at.
    pub display_time_zone: AccountTimeZone,
    /// Stable Assessment lifecycle, separate from Student Assessment Access.
    pub status: AssessmentStatus,
    /// Exact compare-and-swap value for the current authored content.
    pub edit_number: AssessmentEditNumber,
}

/// One answer-free Assessment due in the current Instructor's rolling next-seven-days window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueSoonAssessmentSummary {
    /// Public Course Instance reference; internal Course identity remains server-side.
    pub course_reference: CourseInstanceReference,
    /// Descriptive Course Instance name for cross-Course lists.
    pub course_long_name: String,
    /// Public Assessment reference; internal Assessment identity remains server-side.
    pub assessment_reference: AssessmentReference,
    /// Fixed pedagogical purpose of this Assessment.
    pub assessment_type: AssessmentType,
    /// Current Instructor-authored Assessment title.
    pub assessment_title: AssessmentTitle,
    /// Current Assessment lifecycle, limited by the reader to ordinary actionable states.
    pub assessment_status: AssessmentStatus,
    /// Stored deadline instant as Unix milliseconds, not a Course-local wall-clock value.
    pub due_at_millis: i64,
}

/// Bounded cross-Course Due Soon projection for one authenticated Instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueSoonAssessments {
    /// Answer-free Assessment rows ordered by their stored due instant.
    pub items: Vec<DueSoonAssessmentSummary>,
    /// This bounded reader has no pagination cursor.
    pub next_cursor: Option<String>,
    /// Authenticated Account-owned zone for the browser's due-instant display.
    pub display_time_zone: AccountTimeZone,
}

/// The only mutable fields exposed by the Course Assessment-list inline save.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLiveAssessmentInlineInput {
    // ASVS 1.5.2 and 2.2.1: accept only this closed, validated request shape.
    /// Instructor-facing Assessment Title.
    pub title: AssessmentTitle,
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

/// Complete current Assessment Workspace projection for one direct Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentWorkspace {
    /// Public Assessment Reference; internal Assessment identity remains server-side.
    pub reference: AssessmentReference,
    /// Exact compare-and-swap value for the current authored content.
    pub edit_number: AssessmentEditNumber,
    /// Stable Assessment lifecycle, separate from future Assessment Access.
    pub status: AssessmentStatus,
    /// Immutable database-derived creation origin; browser requests cannot supply it.
    pub origin: AssessmentOrigin,
    /// Fixed pedagogical purpose, independent of editable Assessment Properties.
    pub assessment_type: AssessmentType,
    /// Current Instructor-authored title.
    pub title: AssessmentTitle,
    /// Current Student-facing instructions.
    pub instructions: AssessmentInstructions,
    /// Optional exact Due at projected in the authenticated Instructor zone.
    pub due_at: Option<LocalDateAndTime>,
    /// Optional first local instant at which future Attempts may begin.
    pub available_at: Option<LocalDateAndTime>,
    /// Optional local hard-close instant for future Attempts.
    pub closes_at: Option<LocalDateAndTime>,
    /// Current canonical Late Work Rule.
    pub late_work_rule: LateWorkRule,
    /// Whole-attempt limit in seconds, when configured.
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum Assessment Attempts, when configured.
    pub attempt_limit: Option<NonZeroU32>,
    /// The complete current activity policy.
    pub activity_rules: AssessmentActivityRules,
    /// The complete current disclosure policy.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
    /// Authenticated Instructor display zone; never accepted in a request.
    pub display_time_zone: AccountTimeZone,
    /// Ordered complete normalized entries for future Assessment Attempts.
    pub entries: Vec<AssessmentEntry>,
    /// Ordered current selected exact Question Revision pins.
    pub questions: Vec<AuthoredAssessmentQuestion>,
}

/// One complete calculated Assessment Release Issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentReleaseIssue {
    /// An Assessment Attempt duration must be chosen before release.
    TimeLimitRequired,
    /// The current Assessment requires at least one selected Question.
    NoPublishedQuestions,
    /// A previously selected Question Revision is no longer Available for release.
    QuestionUnavailable,
    /// Release requires a Due date.
    DueDateRequired,
    /// A new or changed Due date must be at least 24 hours ahead.
    DueDateLessThan24HoursAhead,
    /// The Due date cannot extend past the Course's immutable Active cutoff.
    DueDateAfterCourseActiveUntil,
    /// Student availability cannot begin after the Due date.
    AvailabilityAfterDueDate,
    /// The Due date cannot occur after the closing time.
    DueDateAfterClose,
}

/// Calculated release validation changes neither Assessment nor Student work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentReleaseValidation {
    /// True only when the current authored Assessment may create a Revision.
    pub can_release: bool,
    /// All current release blockers in the small release boundary.
    pub issues: Vec<AssessmentReleaseIssue>,
}

#[cfg(test)]
mod tests {
    use super::{
        AssessmentUnreleaseImpact, SaveLiveAssessmentInlineInput, SaveLiveAssessmentInput,
    };
    use question_model::{AssessmentEditNumber, AssessmentTitle};

    #[test]
    fn time_limit_required_is_a_stable_browser_issue() {
        assert_eq!(
            serde_json::to_value(super::AssessmentReleaseIssue::TimeLimitRequired)
                .expect("release issue serializes"),
            serde_json::json!("timeLimitRequired")
        );
    }

    #[test]
    fn release_date_blockers_are_stable_browser_issues() {
        let issues = [
            super::AssessmentReleaseIssue::DueDateRequired,
            super::AssessmentReleaseIssue::DueDateLessThan24HoursAhead,
            super::AssessmentReleaseIssue::DueDateAfterCourseActiveUntil,
            super::AssessmentReleaseIssue::AvailabilityAfterDueDate,
            super::AssessmentReleaseIssue::DueDateAfterClose,
        ];
        assert_eq!(
            serde_json::to_value(issues).expect("release issues serialize"),
            serde_json::json!([
                "dueDateRequired",
                "dueDateLessThan24HoursAhead",
                "dueDateAfterCourseActiveUntil",
                "availabilityAfterDueDate",
                "dueDateAfterClose"
            ])
        );
    }

    #[test]
    fn unrelease_impact_exposes_only_aggregate_deletion_counts() {
        let impact = AssessmentUnreleaseImpact {
            confirmation_title: AssessmentTitle::try_new("Peptide bonds".to_string())
                .expect("valid title"),
            edit_number: AssessmentEditNumber::new(3).expect("valid edit number"),
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
            serde_json::from_value::<SaveLiveAssessmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds"
            }))
            .is_err()
        );

        let input: SaveLiveAssessmentInlineInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "dueAt": "2026-09-11T14:30:00.000"
        }))
        .expect("valid inline save input deserializes");
        assert_eq!(input.title.as_str(), "Peptide bonds");
        assert!(input.due_at.is_some());

        let input: SaveLiveAssessmentInlineInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "dueAt": null
        }))
        .expect("explicit null inline due date deserializes");
        assert!(input.due_at.is_none());

        assert!(
            serde_json::from_value::<SaveLiveAssessmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds",
                "dueAt": null,
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SaveLiveAssessmentInlineInput>(serde_json::json!({
                "title": "",
                "dueAt": null
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SaveLiveAssessmentInlineInput>(serde_json::json!({
                "title": "Peptide bonds",
                "dueAt": "invalid"
            }))
            .is_err()
        );
    }

    #[test]
    fn workspace_save_requires_an_exact_question_revision_pin() {
        let input: SaveLiveAssessmentInput = serde_json::from_value(serde_json::json!({
            "title": "Peptide bonds",
            "instructions": "Answer every question.",
            "entries": [{
                "kind": "fixedQuestion",
                "id": "00000000-0000-0000-0000-000000000001",
                "reference": { "questionId": "7K3M-X9QP", "revisionNumber": 1 },
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

/// Aggregate, non-identifying Student Work affected by a proposed Unrelease.
///
/// This projection exists only while the current Assessment is Released.  It
/// intentionally contains no Student, response, or grade detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentUnreleaseImpact {
    /// Current Assessment Title that the Instructor must repeat to confirm.
    pub confirmation_title: AssessmentTitle,
    /// Exact compare-and-swap value required by the destructive transition.
    pub edit_number: AssessmentEditNumber,
    /// Assessment Attempts that the transition will delete.
    pub attempt_count: u64,
    /// All Question and Assessment submissions that the transition will delete.
    pub submission_count: u64,
    /// Grading results that the transition will delete.
    pub grade_count: u64,
}

/// Result of one accepted Assessment Unrelease transition.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreleasedLiveAssessment {
    /// The complete current Assessment aggregate after it becomes Unreleased.
    pub assessment: LiveAssessmentWorkspace,
    /// Aggregate deletion receipt; detailed Student Work never leaves the database.
    pub deleted: AssessmentUnreleaseImpact,
}

/// Store boundary for Assessment Workspace and release operations.
#[async_trait]
pub trait LiveAssessmentStore: Send + Sync {
    /// Lists ordinary current Assessments due in the caller's rolling next-seven-days window.
    async fn list_assessments_due_soon(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<DueSoonAssessments, StoreError>;

    /// Lists only Assessments owned by one exact authorized Course Instance.
    async fn list_course_assessments(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseAssessmentSummary>, StoreError>;

    /// Lists answer-free currently Available Published Questions for one authorized picker.
    async fn list_assessment_question_picker(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<AssessmentQuestionPickerEntry>, StoreError>;

    /// Creates one Unreleased Assessment without Question selection or Student activity.
    async fn create_live_assessment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        input: CreateLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError>;

    /// Loads one Assessment only through the caller's direct Instructor Course Membership.
    async fn load_live_assessment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
    ) -> Result<LiveAssessmentWorkspace, StoreError>;

    /// Saves complete authored content with its exact Assessment Edit Number.
    async fn save_live_assessment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
        input: SaveLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError>;

    /// Saves only the current Title and Due at with an exact Edit Number.
    async fn save_live_assessment_inline(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
        expected_edit_number: AssessmentEditNumber,
        input: SaveLiveAssessmentInlineInput,
    ) -> Result<CourseAssessmentSummary, StoreError>;

    /// Saves policy fields only; title and normalized Entries remain untouched.
    async fn save_base_assessment_policy(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
        input: SaveBaseAssessmentPolicyInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError>;

    /// Calculates the current small release boundary without mutating state.
    async fn validate_live_assessment_release(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
    ) -> Result<AssessmentReleaseValidation, StoreError>;

    /// Transitions the current Assessment from Unreleased to Released after
    /// exact Edit Number and current-content validation.
    async fn release_live_assessment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
        expected_edit_number: AssessmentEditNumber,
    ) -> Result<LiveAssessmentWorkspace, StoreError>;

    /// Reads the exact, aggregate impact of Unrelease for a currently Released Assessment.
    async fn read_live_assessment_unrelease_impact(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
    ) -> Result<AssessmentUnreleaseImpact, StoreError>;

    /// Atomically deletes rooted Student Work and restores the current Assessment
    /// to Unreleased after exact Edit Number and title confirmation.
    async fn unrelease_live_assessment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assessment: AssessmentReference,
        expected_edit_number: AssessmentEditNumber,
        confirmation_title: AssessmentTitle,
    ) -> Result<UnreleasedLiveAssessment, StoreError>;
}

fn entry_id(entry: &AssessmentEntry) -> String {
    match entry {
        AssessmentEntry::FixedQuestion(value) => value.id.to_string(),
        AssessmentEntry::QuestionPool(value) => value.id.to_string(),
    }
}
