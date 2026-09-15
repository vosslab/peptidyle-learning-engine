//! Course-owned Student Record, Assessment Attempt, Issued Question, and
//! Question Attempt records.
//!
//! Completion of one Assessment Attempt does not end a Student Record. A
//! Student can begin another Assessment Attempt when its explicit continuation
//! rules permit it, and each Assessment Attempt owns its Issued Questions and
//! Question Attempts. Historical completion remains immutable.
//!
//! These are educational records. Their exact Course, Student Record,
//! Assessment, and Issued Question relationships provide authorization scope;
//! an installation-wide Account is the global product identity.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use uuid::Uuid;

mod attempt_evidence;
mod grading;
mod source_object_checksum;
#[cfg(test)]
#[path = "student_work/source_object_checksum_tests.rs"]
mod source_object_checksum_tests;

pub use attempt_evidence::{
    AssessmentAttempt, AssessmentAttemptCompletion, AssessmentAttemptEvidence,
    AssessmentAttemptPolicySource, AssessmentAttemptPolicySources, AssessmentGrade,
};
pub use grading::{GradingResult, QuestionEvaluation, QuestionEvaluationError, RecordedCredit};
pub use source_object_checksum::{SourceObjectChecksum, SourceObjectChecksumError};

use crate::assessment::{AssessmentEntryScoringRule, AssessmentPointValue};
use crate::generation::{QuestionReproduction, QuestionSourceSelection};
use crate::identity::ObjectId;
use crate::response::StudentResponse;
use crate::{AssessmentAttemptReference, QuestionRevisionReference};

/// Answer-free, server-authorized navigation state for an issued Assessment
/// Attempt. This is a projection, never a mutable "current question" record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptProgress {
    pub assessment_attempt: AssessmentAttemptReference,
    pub question_count: u32,
    pub recommended_position: Option<u32>,
    pub positions: Vec<StudentAssessmentAttemptPosition>,
}

/// One 1-based issued position in an answer-free Student navigation projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptPosition {
    pub position: u32,
    pub response_state: StudentAssessmentAttemptResponseState,
}

/// The only persistence states exposed by Assessment navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentAssessmentAttemptResponseState {
    Unanswered,
    Saved,
    Submitted,
    Closed,
}

mod identifiers;

pub use identifiers::{
    AccommodationId, AssessmentAttemptId, AssessmentEntryId, AssessmentId, CourseId,
    CourseMembershipId, IssuedQuestionId, QuestionAttemptId, QuestionPoolSelectionId,
    QuestionSubmissionId, StudentRecordId,
};

/// A timestamp supplied by the server as Unix milliseconds.
///
/// The value is carried rather than read from a process clock. PostgreSQL is
/// the authoritative clock when these records are created or transitioned.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Wraps server-supplied Unix milliseconds.
    pub fn from_unix_millis(value: i64) -> Self {
        Self(value)
    }

    /// Returns the server-supplied Unix millisecond value.
    pub fn as_unix_millis(&self) -> i64 {
        self.0
    }
}

/// One exact Question Pool Item selected in delivery order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelectedItem {
    /// Exact immutable Pool Revision member selected for delivery.
    pub pool_revision_member: crate::PoolRevisionMemberReference,
    /// Exact immutable Question Revision selected for delivery.
    pub reference: QuestionRevisionReference,
}

/// Immutable Question Pool result for one Assessment Attempt and one Question Pool Assessment Entry.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelection {
    /// Durable Question Pool Selection identity.
    pub id: QuestionPoolSelectionId,
    /// Assessment Attempt that owns this selected Question Pool Item set.
    pub assessment_attempt: AssessmentAttemptId,
    /// Question Pool Assessment Entry that supplied the Question Pool Items.
    pub question_pool_assessment_entry: AssessmentEntryId,
    /// Exact immutable Assessment-owned fork Pool Revision selected from.
    pub question_pool_revision: crate::QuestionPoolRevisionReference,
    /// Database-authoritative time at which the server selected these entries.
    pub created_at: Timestamp,
    /// Number of exact Question Pool Items selected for this Assessment Attempt.
    ///
    /// This repeats the selected Question Pool Item row cardinality so storage can reject an
    /// incomplete Selection at transaction commit without consulting mutable
    /// Assessment content.
    pub selected_question_count: u32,
    /// Earlier Selection whose exact Question Pool Items this later Assessment Attempt retained.
    ///
    /// A reused Selection is still an immutable result owned by this Assessment
    /// Attempt. This link preserves the reason its selected Question Pool Item membership repeats
    /// without treating the earlier Attempt's record as mutable shared state.
    pub reused_from_question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Exact selected Question Pool Items in their frozen delivery order.
    pub selected_items: Vec<QuestionPoolSelectedItem>,
}

/// A requested later-Attempt Selection does not match its earlier source.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionPoolSelectionReuseError {
    /// A Question Pool Selection can only be retained for the same Assessment Entry.
    DifferentQuestionPoolAssessmentEntry,
    /// A Selection cannot use itself as an earlier Assessment Attempt's source.
    SameAssessmentAttempt,
}

impl std::fmt::Display for QuestionPoolSelectionReuseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DifferentQuestionPoolAssessmentEntry => formatter
                .write_str("Question Pool Selection reuse requires the same Assessment Entry"),
            Self::SameAssessmentAttempt => formatter
                .write_str("Question Pool Selection reuse requires a later Assessment Attempt"),
        }
    }
}

impl std::error::Error for QuestionPoolSelectionReuseError {}

impl QuestionPoolSelection {
    /// Copies this immutable Question Pool Selection into a later Assessment Attempt.
    ///
    /// Storage additionally verifies that both attempts belong to the same
    /// Student and Assessment, and that the target attempt number is later.
    /// Keeping those relational checks in storage prevents a caller from
    /// supplying an unrelated Student Work identifier.
    pub fn reused_for_later_attempt(
        &self,
        id: QuestionPoolSelectionId,
        assessment_attempt: AssessmentAttemptId,
        question_pool_assessment_entry: AssessmentEntryId,
        created_at: Timestamp,
    ) -> Result<Self, QuestionPoolSelectionReuseError> {
        if question_pool_assessment_entry != self.question_pool_assessment_entry {
            return Err(QuestionPoolSelectionReuseError::DifferentQuestionPoolAssessmentEntry);
        }
        if assessment_attempt == self.assessment_attempt {
            return Err(QuestionPoolSelectionReuseError::SameAssessmentAttempt);
        }
        Ok(Self {
            id,
            assessment_attempt,
            question_pool_assessment_entry,
            question_pool_revision: self.question_pool_revision.clone(),
            created_at,
            selected_question_count: self.selected_question_count,
            reused_from_question_pool_selection: Some(self.id),
            selected_items: self.selected_items.clone(),
        })
    }
}

/// Immutable question selection and issued order for one Assessment Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestion {
    /// Durable issued-question identity.
    pub id: IssuedQuestionId,
    /// Assessment Attempt whose future sequencing is frozen by this record.
    pub assessment_attempt: AssessmentAttemptId,
    /// Stable fixed-question or Question Pool Assessment Entry identity.
    pub assessment_entry: AssessmentEntryId,
    /// Entry index in the Assessment Content when the Assessment Attempt began.
    pub assessment_content_entry_index: u32,
    /// Expanded zero-based delivery order inside this Assessment Attempt.
    pub issued_position: u32,
    /// Exact immutable Question Library version selected for delivery.
    pub reference: QuestionRevisionReference,
    /// Pre-render static or seeded source selection. A complete reproduction
    /// descriptor begins only after rendering records its parameter checksum.
    pub source_selection: QuestionSourceSelection,
    /// Exact rendering and reproduction binding retained for this issue.
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Exact Assessment-owned point value frozen when this Question was issued.
    pub point_value: AssessmentPointValue,
    /// Exact Assessment scoring treatment frozen when this Question was issued.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Whether this Issued Question may contribute to cross-course learning evidence.
    ///
    /// The value is frozen when the Assessment Attempt begins so later assessment scoring
    /// changes cannot rewrite the validity of an observed student response.
    pub question_statistics_eligibility: bool,
    /// Immutable Question Pool Selection that produced this Issued Question, if it was drawn.
    pub question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Exact immutable Pool Revision member in that Question Pool Selection, if drawn.
    pub pool_revision_member: Option<crate::PoolRevisionMemberReference>,
}

/// Server-recorded timing inputs for one issued question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTiming {
    /// Server time at which the question became available.
    pub issued_at: Timestamp,
    /// Server-owned base deadline before authorized pauses, or `None` when untimed.
    pub deadline: Option<Timestamp>,
    /// Server time at which the response arrived, if submitted.
    pub submitted_at: Option<Timestamp>,
}

/// Current operational state of one issued Question Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAttemptState {
    /// The student may still submit a response.
    Open,
    /// The server accepted one Question Submission.
    SubmissionAccepted,
    /// The effective deadline closed this Question Attempt without a submission.
    ClosedAtDeadline,
}

/// One immutable accepted Student Response and its current grading result.
///
/// The containing Question Attempt supplies the exact issue-time reproduction details
/// that the grading result reproduces. A Question Attempt has at most one
/// accepted Question Submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionSubmission {
    /// Durable Question Submission identity.
    pub id: QuestionSubmissionId,
    /// Exact Question Attempt that accepted this response.
    pub question_attempt: QuestionAttemptId,
    /// Immutable Student Response accepted by the server.
    pub response: StudentResponse,
    /// Server time when the response was accepted.
    pub submitted_at: Timestamp,
    /// Present only after grading produced a result for this submission.
    pub grading_result: Option<GradingResult>,
}

/// Exact Question Backend Version recorded with one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionBackendVersion {
    /// Stable Question Backend implementation name.
    pub name: String,
    /// Exact Question Backend software version.
    pub version: String,
}

/// Exact Question Grader Version recorded with one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionGraderVersion {
    /// Stable Question Grader implementation name.
    pub name: String,
    /// Exact Question Grader software version.
    pub version: String,
}

/// Exact Question Renderer Version recorded with one Question Attempt.
///
/// Question Renderer Version has a distinct role from the Question Backend and
/// Question Grader versions that the same attempt also records.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRendererVersion {
    /// Stable Question Renderer implementation name.
    pub name: String,
    /// Exact Question Renderer software version.
    pub version: String,
}

/// Source Object Reference captured for a reproducible Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceObjectReference {
    /// Immutable Object Record containing the source bytes.
    pub object: ObjectId,
}

/// Versions and object identities required to reproduce one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptReproductionDetails {
    /// Question Backend that loaded and interpreted the question.
    pub backend: QuestionBackendVersion,
    /// Renderer used to produce the typed Question Content Blocks, when the backend has one.
    pub renderer_version: Option<QuestionRendererVersion>,
    /// Exact source object, when the backend stores source bytes.
    pub source_object_reference: Option<SourceObjectReference>,
    /// SHA-256 integrity evidence for the exact source object.
    pub source_object_checksum: Option<SourceObjectChecksum>,
    /// Objects referenced by the rendered question.
    pub asset_objects: Vec<ObjectId>,
    /// Server-only Question Grader that produced the result.
    pub grader: QuestionGraderVersion,
    /// SHA-256 of the rendered question delivered for this attempt.
    pub rendered_question_sha256: String,
}

/// Immutable issued-presentation capability recorded inside the checksummed attempt payload.
///
/// The database keeps the corresponding private presentation and grading
/// payloads in dedicated protected columns. This tag binds their required or
/// not-applicable shape to the attempt itself, so a damaged column cannot
/// downgrade a PLE Question JSON or WeBWorK attempt into a current Question Library recovery path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssuedAttemptCapability {
    /// A browser-safe `QuestionPresentation` with no format-specific
    /// private first-grade contract.
    QuestionPresentation,
    /// A PLE Question JSON presentation and its required private grading contract.
    PleQuestionJsonPresentation,
    /// The WeBWorK capability requires a backend-owned document and opaque Student Response.
    WebworkPresentation,
    /// iMathAS session and launch lifecycle state without a format-specific
    /// private grading contract. Student delivery remains a Question Presentation.
    NotApplicable,
}

/// One server-issued try under an exact Issued Question.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttempt {
    /// Durable question-attempt identity.
    pub id: QuestionAttemptId,
    /// Immutable delivered Question that owns this attempt.
    pub issued_question: IssuedQuestionId,
    /// Explicit static or seeded facts used to reproduce this Question Attempt.
    /// This server model is never a browser wire contract.
    pub reproduction: QuestionReproduction,
    /// Immutable accepted Student Response, when the server accepted one.
    pub submission: Option<QuestionSubmission>,
    /// Current operational state, independent of retained response evidence.
    pub state: QuestionAttemptState,
    /// Server-owned timing record.
    pub timing: QuestionAttemptTiming,
    /// Exact reproduction details required to reproduce this Question Attempt.
    #[serde(skip_serializing)]
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapability,
}

/// Answer-free Student read of one Question Attempt.
///
/// The server constructs this from the durable Question Attempt after it has
/// applied the Student's disclosure and scoring policy. Reproduction details
/// and generated-parameter evidence intentionally have no representation here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentQuestionAttemptView {
    /// Durable Question Attempt identity.
    pub id: QuestionAttemptId,
    /// Immutable delivered Question that owns this attempt.
    pub issued_question: IssuedQuestionId,
    /// Immutable accepted Student Response, when the server accepted one.
    pub submission: Option<QuestionSubmission>,
    /// Current operational state.
    pub state: QuestionAttemptState,
    /// Student-visible timing record.
    pub timing: QuestionAttemptTiming,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapability,
}

impl From<&QuestionAttempt> for StudentQuestionAttemptView {
    fn from(attempt: &QuestionAttempt) -> Self {
        Self {
            id: attempt.id,
            issued_question: attempt.issued_question,
            submission: attempt.submission.clone(),
            state: attempt.state,
            timing: attempt.timing,
            issued_capability: attempt.issued_capability,
        }
    }
}

/// Compact Assessment Progress Record read by course pages and the Gradebook.
///
/// Historical Assessment Attempts remain separate. Updating this Assessment Progress Record from the same
/// Assessment Attempt transition lets storage commit the history and progress atomically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssessmentProgressRecord {
    /// Student Record summarized by this view.
    pub student_record: StudentRecordId,
    /// Assessment summarized by this view.
    pub assessment: AssessmentId,
    /// Number of completed Assessment Attempts, including continued Student work.
    pub completed_assessment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assessment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-supplied Student Work timestamp.
    pub last_activity_at: Option<Timestamp>,
}

/// Browser-safe status of the Student's Assessment Grade.
///
/// This is a presentation state, not an authorization input.  The server
/// derives it from the current assessment disclosure policy and never sends
/// the policy, clock, or Student Record to the browser for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentGradeScoreState {
    /// The Student has not submitted any response for this assessment.
    NoActivity,
    /// The Student has activity, but the current policy withholds scores.
    Withheld,
    /// The current policy permits score disclosure.
    Available,
}

/// Key-free Student Assessment Grade for one Assessment.
///
/// It deliberately excludes the Student Record and Assessment identifiers
/// carried by [`AssessmentGrade`]. The server derives disclosure and sends no
/// score totals while the current Assessment settings withhold them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentAssessmentGrade {
    /// Whether aggregate score values are absent because there is no submitted
    /// response, are currently withheld, or are available for display.
    pub score_state: AssessmentGradeScoreState,
    /// Current freshness and visibility of the assessment's computed scores.
    pub assessment_scoring_state: crate::AssessmentScoringState,
    /// Score selected by the assessment's grade policy when available.
    pub current_score: Option<f64>,
    /// Highest completed Assessment Attempt score when available.
    pub best_score: Option<f64>,
    /// Most recently completed Assessment Attempt score when available.
    pub latest_score: Option<f64>,
    /// Current anonymous class statistics when the assessment policy permits
    /// their disclosure. Absent means the server withholds Class Statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_statistics: Option<crate::ClassStatistics>,
}

/// Key-free Student Assessment activity facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssessmentProgress {
    /// Number of completed Assessment Attempts, including continued Student work.
    pub completed_assessment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assessment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-recorded Student Work time, if any.
    pub last_activity_at: Option<Timestamp>,
}

/// Key-free Student response that keeps Assessment activity and grade facts separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentAssessmentProgress {
    /// Student-safe activity facts for this Assessment.
    pub assessment_progress: AssessmentProgress,
    /// Student-safe disclosed grade facts for this Assessment.
    pub student_assessment_grade: StudentAssessmentGrade,
}

impl StudentAssessmentGrade {
    /// Projects an entitled Student's Assessment Grade before the first durable
    /// educational receipt exists. Reading the grade must not create an
    /// Assessment Attempt merely to represent the valid no-Student-work state.
    pub fn no_activity(assessment_scoring_state: crate::AssessmentScoringState) -> Self {
        Self {
            score_state: AssessmentGradeScoreState::NoActivity,
            assessment_scoring_state,
            current_score: None,
            best_score: None,
            latest_score: None,
            class_statistics: None,
        }
    }

    /// Projects the internal Assessment Grade after the server has made its disclosure
    /// decision. No-activity takes precedence over the disclosure setting.
    pub fn from_assessment_grade(
        grade: &AssessmentGrade,
        progress: &AssessmentProgressRecord,
        score_disclosed: bool,
        assessment_scoring_state: crate::AssessmentScoringState,
    ) -> Self {
        let score_state = if progress.total_question_attempts == 0 {
            AssessmentGradeScoreState::NoActivity
        } else if score_disclosed {
            AssessmentGradeScoreState::Available
        } else {
            AssessmentGradeScoreState::Withheld
        };
        let scores = matches!(score_state, AssessmentGradeScoreState::Available)
            && matches!(
                assessment_scoring_state,
                crate::AssessmentScoringState::Current
            );
        Self {
            score_state,
            assessment_scoring_state,
            current_score: scores.then_some(grade.current_score).flatten(),
            best_score: scores.then_some(grade.best_score).flatten(),
            latest_score: scores.then_some(grade.latest_score).flatten(),
            class_statistics: None,
        }
    }
}

impl From<&AssessmentProgressRecord> for AssessmentProgress {
    fn from(progress: &AssessmentProgressRecord) -> Self {
        Self {
            completed_assessment_attempt_count: progress.completed_assessment_attempt_count,
            total_question_attempts: progress.total_question_attempts,
            last_activity_at: progress.last_activity_at,
        }
    }
}

impl AssessmentProgressRecord {
    /// Creates the empty Student Work view for one Student Record and Assessment.
    pub fn empty(student_record: StudentRecordId, assessment: AssessmentId) -> Self {
        Self {
            student_record,
            assessment,
            completed_assessment_attempt_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
        }
    }
}

impl AssessmentGrade {
    /// Creates the empty selected-grade record for one Student Record and Assessment.
    pub fn empty(student_record: StudentRecordId, assessment: AssessmentId) -> Self {
        Self {
            student_record,
            assessment,
            first_completed_at: None,
            current_assessment_attempt: None,
            current_score: None,
            best_assessment_attempt: None,
            best_score: None,
            latest_assessment_attempt: None,
            latest_score: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt_evidence() -> AssessmentAttemptEvidence {
        AssessmentAttemptEvidence {
            title: crate::AssessmentTitle::try_new("Assessment".to_string()).expect("valid title"),
            instructions: crate::AssessmentInstructions::default(),
            base_policy: crate::BaseAssessmentPolicy::default(),
            activity_rules: crate::AssessmentActivityRules::default(),
            student_feedback_release_rule: crate::StudentFeedbackReleaseRule::default(),
            effective_policy_sources: AssessmentAttemptPolicySources::default(),
        }
    }

    fn reproduction_details() -> QuestionAttemptReproductionDetails {
        QuestionAttemptReproductionDetails {
            backend: QuestionBackendVersion {
                name: "ple".to_string(),
                version: "test".to_string(),
            },
            renderer_version: None,
            source_object_reference: None,
            source_object_checksum: None,
            asset_objects: vec![],
            grader: QuestionGraderVersion {
                name: "ple".to_string(),
                version: "test".to_string(),
            },
            rendered_question_sha256: "0".repeat(64),
        }
    }

    #[test]
    fn assessment_attempt_retains_interpretation_evidence() {
        let accommodation = AccommodationId::from_uuid(Uuid::from_u128(4));
        let mut evidence = attempt_evidence();
        evidence.effective_policy_sources.schedule =
            AssessmentAttemptPolicySource::Accommodation { accommodation };
        evidence.activity_rules.question_pool_reuse_rule =
            crate::QuestionPoolReuseRule::SelectAgain;
        evidence.activity_rules.question_variation_rule =
            crate::AssessmentQuestionVariationRule::ReuseVariation;
        let attempt = AssessmentAttempt {
            id: AssessmentAttemptId::from_uuid(Uuid::from_u128(1)),
            reference: AssessmentAttemptReference::new(1).expect("valid attempt reference"),
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(2)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(3)),
            evidence,
            attempt_number: 1,
            started_at: Timestamp::from_unix_millis(1_000),
            completed_at: None,
            score: None,
        };

        assert_eq!(attempt.student_record.as_uuid(), Uuid::from_u128(2));
        assert_eq!(attempt.assessment.as_uuid(), Uuid::from_u128(3));
        assert_eq!(attempt.evidence.title.as_str(), "Assessment");
        assert_eq!(
            attempt.question_pool_reuse_rule(),
            crate::QuestionPoolReuseRule::SelectAgain
        );
        assert_eq!(
            attempt.question_variation_rule(),
            crate::AssessmentQuestionVariationRule::ReuseVariation
        );
        assert_eq!(
            attempt.evidence.effective_policy_sources.schedule,
            AssessmentAttemptPolicySource::Accommodation { accommodation }
        );
        assert_eq!(
            serde_json::to_value(attempt.evidence.effective_policy_sources)
                .expect("qualified evidence serializes")["schedule"],
            serde_json::json!({
                "kind": "accommodation",
                "accommodation": accommodation.to_string(),
            })
        );
    }
    #[test]
    fn question_pool_selection_retains_exact_entries_and_issued_question_link() {
        let selection_id = QuestionPoolSelectionId::from_uuid(Uuid::from_u128(10));
        let pool_revision = crate::QuestionPoolRevisionReference {
            question_pool_id: "7654-X321".parse().expect("valid Pool ID"),
            revision_number: crate::QuestionPoolRevisionNumber::new(1)
                .expect("positive Pool Revision"),
        };
        let pool_revision_member = crate::PoolRevisionMemberReference {
            question_pool_revision: pool_revision.clone(),
            member_position: 0,
        };
        let reference = QuestionRevisionReference {
            question_id: "1234-X567".parse().expect("valid Question ID"),
            revision_number: crate::QuestionRevisionNumber::new(1).expect("positive version"),
        };
        let selection = QuestionPoolSelection {
            id: selection_id,
            assessment_attempt: AssessmentAttemptId::from_uuid(Uuid::from_u128(12)),
            question_pool_assessment_entry: AssessmentEntryId::from_uuid(Uuid::from_u128(13)),
            question_pool_revision: pool_revision,
            created_at: Timestamp::from_unix_millis(1_000),
            selected_question_count: 1,
            reused_from_question_pool_selection: None,
            selected_items: vec![QuestionPoolSelectedItem {
                pool_revision_member: pool_revision_member.clone(),
                reference: reference.clone(),
            }],
        };
        let issued_question = IssuedQuestion {
            id: IssuedQuestionId::from_uuid(Uuid::from_u128(14)),
            assessment_attempt: selection.assessment_attempt,
            assessment_entry: selection.question_pool_assessment_entry,
            assessment_content_entry_index: 0,
            issued_position: 0,
            reference,
            source_selection: QuestionSourceSelection::Static,
            reproduction_details: reproduction_details(),
            point_value: crate::AssessmentPointValue::from_whole(1),
            scoring_rule: crate::AssessmentEntryScoringRule::Normal,
            question_statistics_eligibility: true,
            question_pool_selection: Some(selection_id),
            pool_revision_member: Some(pool_revision_member.clone()),
        };

        assert_eq!(selection.selected_items.len(), 1);
        assert_eq!(issued_question.question_pool_selection, Some(selection.id));
        assert_eq!(
            issued_question.pool_revision_member,
            Some(pool_revision_member)
        );

        let reused = selection
            .reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(15)),
                AssessmentAttemptId::from_uuid(Uuid::from_u128(16)),
                selection.question_pool_assessment_entry,
                Timestamp::from_unix_millis(2_000),
            )
            .expect("same Question Pool may retain its exact Question Pool Items");
        assert_eq!(
            reused.reused_from_question_pool_selection,
            Some(selection.id)
        );
        assert_eq!(reused.selected_items, selection.selected_items);
    }
    #[test]
    fn question_pool_selection_refuses_reuse_for_a_different_entry_or_same_attempt() {
        let selection = QuestionPoolSelection {
            id: QuestionPoolSelectionId::from_uuid(Uuid::from_u128(1)),
            assessment_attempt: AssessmentAttemptId::from_uuid(Uuid::from_u128(2)),
            question_pool_assessment_entry: AssessmentEntryId::from_uuid(Uuid::from_u128(3)),
            question_pool_revision: crate::QuestionPoolRevisionReference {
                question_pool_id: "7654-X321".parse().expect("valid Pool ID"),
                revision_number: crate::QuestionPoolRevisionNumber::new(1)
                    .expect("positive Pool Revision"),
            },
            created_at: Timestamp::from_unix_millis(1_000),
            selected_question_count: 1,
            reused_from_question_pool_selection: None,
            selected_items: Vec::new(),
        };

        assert_eq!(
            selection.reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(4)),
                selection.assessment_attempt,
                selection.question_pool_assessment_entry,
                Timestamp::from_unix_millis(2_000),
            ),
            Err(QuestionPoolSelectionReuseError::SameAssessmentAttempt),
        );
        assert_eq!(
            selection.reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(4)),
                AssessmentAttemptId::from_uuid(Uuid::from_u128(5)),
                AssessmentEntryId::from_uuid(Uuid::from_u128(6)),
                Timestamp::from_unix_millis(2_000),
            ),
            Err(QuestionPoolSelectionReuseError::DifferentQuestionPoolAssessmentEntry),
        );
    }

    #[test]
    fn student_assessment_progress_separates_activity_from_disclosed_grade() {
        assert_eq!(
            StudentAssessmentGrade::no_activity(crate::AssessmentScoringState::Current),
            StudentAssessmentGrade {
                score_state: AssessmentGradeScoreState::NoActivity,
                assessment_scoring_state: crate::AssessmentScoringState::Current,
                current_score: None,
                best_score: None,
                latest_score: None,
                class_statistics: None,
            }
        );
        let mut progress = AssessmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssessmentId::from_uuid(Uuid::from_u128(3)),
        );
        let mut grade = AssessmentGrade::empty(progress.student_record, progress.assessment);
        assert_eq!(
            StudentAssessmentGrade::from_assessment_grade(
                &grade,
                &progress,
                true,
                crate::AssessmentScoringState::Current
            )
            .score_state,
            AssessmentGradeScoreState::NoActivity
        );

        progress.total_question_attempts = 1;
        grade.current_score = Some(0.5);
        grade.best_score = Some(0.5);
        grade.latest_score = Some(0.5);
        let withheld = StudentAssessmentGrade::from_assessment_grade(
            &grade,
            &progress,
            false,
            crate::AssessmentScoringState::Current,
        );
        assert_eq!(withheld.score_state, AssessmentGradeScoreState::Withheld);
        assert_eq!(
            (
                withheld.current_score,
                withheld.best_score,
                withheld.latest_score
            ),
            (None, None, None)
        );

        let available = StudentAssessmentGrade::from_assessment_grade(
            &grade,
            &progress,
            true,
            crate::AssessmentScoringState::Current,
        );
        assert_eq!(available.score_state, AssessmentGradeScoreState::Available);
        assert_eq!(available.current_score, Some(0.5));
        assert!(available.class_statistics.is_none());
    }

    #[test]
    fn student_assessment_grade_hides_scores_while_scoring_is_not_current() {
        let mut progress = AssessmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssessmentId::from_uuid(Uuid::from_u128(3)),
        );
        progress.total_question_attempts = 1;
        let mut grade = AssessmentGrade::empty(progress.student_record, progress.assessment);
        grade.current_score = Some(0.5);
        for assessment_scoring_state in [
            crate::AssessmentScoringState::Recalculating,
            crate::AssessmentScoringState::Failed,
        ] {
            let student_grade = StudentAssessmentGrade::from_assessment_grade(
                &grade,
                &progress,
                true,
                assessment_scoring_state,
            );
            assert_eq!(
                student_grade.score_state,
                AssessmentGradeScoreState::Available
            );
            assert_eq!(
                student_grade.assessment_scoring_state,
                assessment_scoring_state
            );
            assert_eq!(student_grade.current_score, None);
        }
    }

    #[test]
    fn every_activity_identifier_stays_distinct_but_round_trips() {
        let raw = Uuid::from_u128(7);
        let assessment_attempt = AssessmentAttemptId::from_uuid(raw);
        let attempt = QuestionAttemptId::from_uuid(raw);

        assert_eq!(
            (assessment_attempt.as_uuid(), attempt.as_uuid()),
            (raw, raw)
        );
    }

    #[test]
    fn question_attempt_state_uses_the_closed_operational_wire_vocabulary() {
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::Open).expect("open state serializes"),
            serde_json::json!("open")
        );
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::SubmissionAccepted)
                .expect("accepted-submission state serializes"),
            serde_json::json!("submission_accepted")
        );
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::ClosedAtDeadline)
                .expect("deadline-closed state serializes"),
            serde_json::json!("closed_at_deadline")
        );
    }

    #[test]
    fn reproduction_details_serialize_role_specific_versions() {
        let record = QuestionAttemptReproductionDetails {
            backend: QuestionBackendVersion {
                name: "ple-question-backend".to_string(),
                version: "1".to_string(),
            },
            renderer_version: None,
            source_object_reference: Some(SourceObjectReference {
                object: ObjectId::from_uuid(Uuid::from_u128(7)),
            }),
            source_object_checksum: Some(
                SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum"),
            ),
            asset_objects: Vec::new(),
            grader: QuestionGraderVersion {
                name: "generic-grader".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "a".repeat(64),
        };

        let wire = serde_json::to_value(record).expect("reproduction details serialize");
        assert!(wire.get("backend").is_some());
        assert!(wire.get("grader").is_some());
        assert_eq!(
            wire["sourceObjectReference"],
            serde_json::json!({ "object": "00000000-0000-0000-0000-000000000007" })
        );
        assert_eq!(
            wire["sourceObjectChecksum"],
            serde_json::json!("a".repeat(64))
        );
        assert!(wire.get("adapter").is_none());
        assert!(wire.get("grading").is_none());
    }

    #[test]
    fn question_attempt_browser_wire_omits_reproduction_details() {
        let attempt = QuestionAttempt {
            id: QuestionAttemptId::from_uuid(Uuid::from_u128(1)),
            issued_question: IssuedQuestionId::from_uuid(Uuid::from_u128(2)),
            reproduction: QuestionReproduction::Static,
            submission: None,
            state: QuestionAttemptState::Open,
            timing: QuestionAttemptTiming {
                issued_at: Timestamp::from_unix_millis(4),
                deadline: None,
                submitted_at: None,
            },
            reproduction_details: QuestionAttemptReproductionDetails {
                backend: QuestionBackendVersion {
                    name: "ple-question-backend".to_string(),
                    version: "1".to_string(),
                },
                renderer_version: None,
                source_object_reference: None,
                source_object_checksum: None,
                asset_objects: Vec::new(),
                grader: QuestionGraderVersion {
                    name: "generic-grader".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: "b".repeat(64),
            },
            issued_capability: IssuedAttemptCapability::NotApplicable,
        };

        let view = StudentQuestionAttemptView::from(&attempt);
        let wire = serde_json::to_value(view).expect("Student Question Attempt View serializes");
        assert!(wire.get("parameterHash").is_none());
        assert!(wire.get("questionSeed").is_none());
        assert!(wire.get("reproduction").is_none());
        assert!(wire.get("reproductionDetails").is_none());
        assert_eq!(
            wire.get("id"),
            Some(&serde_json::json!(attempt.id.to_string()))
        );
        assert_eq!(
            wire.get("issuedQuestion"),
            Some(&serde_json::json!(attempt.issued_question.to_string()))
        );
        assert!(wire.get("issuedCapability").is_some());
    }

    #[test]
    fn saved_assessment_attempt_navigation_state_serializes() {
        assert_eq!(
            serde_json::to_value(StudentAssessmentAttemptResponseState::Saved)
                .expect("saved response state serializes"),
            serde_json::json!("saved")
        );
    }
}
