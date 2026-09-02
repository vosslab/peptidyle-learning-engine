//! Assignment Attempt, timing, and Assignment activity rules (WP-C1, WP-C3).
//!
//! The eight Assignment activity rules are independent enums that compose freely. Keeping
//! them independent is what lets an instructor express "mastery required,
//! highest score kept, practice allowed after completion with fresh seeds",
//! which is the behavior the owner observed students actually using. A single
//! combined "mode" enum would offer a fixed menu instead.
//!
//! Question-level policies ([`QuestionAttemptLimit`], [`QuestionAttemptTimeLimit`]) are authored
//! with the question. Assignment-level rules are chosen per Assignment, so the same
//! published question serves a graded exam in one course and open practice in
//! another.

use serde::{Deserialize, Serialize};

use crate::{AssignmentAttempt, AssignmentEntryId, AssignmentId};

/// The Assignment Attempt state or Assignment schedule point when one
/// Student-facing field may be disclosed.
///
/// Each timing is evaluated independently so an instructor can, for example,
/// show a score after submission while holding a Question Answer and its
/// Question Answer Explanation until the assignment closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentFeedbackReleaseTiming {
    /// The field is visible while a Student is working on the attempt.
    DuringAttempt,
    /// The field is visible once that Student has submitted the attempt.
    AfterSubmit,
    /// The field is visible at or after the resolved assignment due time.
    AfterDue,
    /// The field is visible at or after the resolved assignment close time.
    AfterClose,
    /// The field is never visible to a Student through this policy.
    Never,
}

/// Assignment-owned Student Feedback Release Rule.
///
/// These independently configured fields are evaluated server-side against
/// the effective assignment policy. They are intentionally separate from
/// [`AssignmentActivityRules`], whose Assignment Attempt behavior remains stable while S4 migrates
/// Student-facing projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentFeedbackReleaseRule {
    /// When the Student may see their score.
    pub score: StudentFeedbackReleaseTiming,
    /// When the Student may see per-item correctness.
    pub per_item_correctness: StudentFeedbackReleaseTiming,
    /// When the Student may see Question Feedback.
    pub question_feedback: StudentFeedbackReleaseTiming,
    /// When the Student may see the display-ready Question Answer.
    pub question_answer: StudentFeedbackReleaseTiming,
    /// When the Student may see the Question Answer Explanation.
    pub question_answer_explanation: StudentFeedbackReleaseTiming,
    /// When the Student may see anonymous class statistics.
    pub class_statistics: StudentFeedbackReleaseTiming,
}

impl Default for StudentFeedbackReleaseRule {
    /// Returns the policy used when authoring a new assignment.
    ///
    /// This is deliberately an initializer rather than a serde compatibility
    /// fallback: an assignment payload must still carry this policy explicitly.
    fn default() -> Self {
        Self {
            score: StudentFeedbackReleaseTiming::AfterSubmit,
            per_item_correctness: StudentFeedbackReleaseTiming::AfterSubmit,
            question_feedback: StudentFeedbackReleaseTiming::AfterSubmit,
            question_answer: StudentFeedbackReleaseTiming::AfterSubmit,
            question_answer_explanation: StudentFeedbackReleaseTiming::AfterSubmit,
            class_statistics: StudentFeedbackReleaseTiming::Never,
        }
    }
}

/// How many times a student may answer one question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionAttemptLimit {
    /// Attempts permitted, or `None` for unlimited.
    ///
    /// `None` is the mastery case: retry until correct.
    pub max_attempts: Option<u32>,
}

/// Time limit for one Question Attempt.
///
/// Server time is authoritative; a browser clock is display only. Keeping the
/// limit in the model and the verdict in `crates/domain` is what makes the
/// outcome invariant under client clock skew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionAttemptTimeLimit {
    /// No limit; the Student works at their own pace.
    Unlimited,
    /// Seconds allowed for one issued Question Attempt.
    Limited {
        seconds: u32,
        /// Extra seconds accepted after expiry, covering network delay.
        grace_seconds: u32,
    },
}

/// What a Student must achieve for an Assignment Attempt to count as complete.
///
/// `PartialEq` without `Eq`, because a threshold is a fraction and floating
/// point has no total equality. Comparisons on thresholds go through the
/// scoring rules in `crates/domain`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssignmentCompletionRule {
    /// Answering every Question completes the Assignment Attempt, whatever the score.
    AnswerAll,
    /// Every Question must be answered correctly.
    AllCorrect,
    /// A score at or above a threshold completes the Assignment Attempt.
    ScoreAtLeast {
        /// Threshold as a fraction, where 0.8 means eighty percent.
        fraction: f64,
    },
}

/// Which Assignment Attempt score reaches the Gradebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssignmentAttemptGradeRule {
    /// The first Assignment Attempt's score.
    First,
    /// The most recent Assignment Attempt's score.
    Latest,
    /// The best Assignment Attempt's score.
    Highest,
    /// An Assignment Attempt explicitly selected by the Instructor.
    InstructorSelected,
}

/// What a student may do after completing an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssignmentAttemptContinuationRule {
    /// Any number of new Assignment Attempts may be started after completion.
    Unlimited,
    /// A bounded number of new Assignment Attempts may be started after completion.
    Capped {
        /// Assignment Attempts allowed after the first completed Assignment Attempt.
        max_additional_assignment_attempts: u32,
    },
    /// The assignment closes once complete.
    Closed,
}

/// What a later Assignment Attempt does with Question Pool membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestionPoolReuseRule {
    /// Retain the Student's prior Question Pool Selection.
    ReuseSelection,
    /// Create a new Question Pool Selection for the later Assignment Attempt.
    SelectAgain,
}

/// What a later Assignment Attempt does with Question Variations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssignmentQuestionVariationRule {
    /// Retain each selected Question's existing Question Variation.
    ReuseVariation,
    /// Issue a fresh Question Seed for every selected Question.
    NewVariation,
}

/// Whether one Assignment Attempt can be left and later resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAttemptResumeRule {
    /// A Student can leave the Assignment Attempt and return to its server-owned state.
    Resumable,
    /// A Student must finish the Assignment Attempt in its first active session.
    SingleSession,
}

/// How many Issued Questions an Assignment Attempt presents at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentQuestionDisplayRule {
    /// Present every available Issued Question together.
    AllQuestions,
    /// Present one Issued Question at a time.
    OneQuestionAtATime,
}

/// How a Student may move among available Issued Questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentNavigationRule {
    /// A Student may revisit any available Issued Question.
    FreeNavigation,
    /// A Student may advance but cannot return to an earlier Issued Question.
    ForwardOnly,
}

/// The server-owned order used after Assignment Entries expand into Issued Questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentQuestionOrderRule {
    /// Preserve the Instructor-authored Assignment Entry order.
    AuthoredOrder,
    /// Shuffle the expanded Issued Question order once for the Assignment Attempt.
    Shuffled,
}

/// Stable server-owned inputs for one Question Pool Selection.
///
/// The basis contains only server-owned durable identities. It chooses
/// Question Pool Item references; question issuance separately creates the fresh
/// private server seed for every selected question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionPoolSelectionInputs {
    /// Repeat selections for one Student Record retain the same Question Pool Items.
    StableStudentRecord {
        student_record: crate::StudentRecordId,
        assignment: AssignmentId,
        question_pool_assignment_entry: AssignmentEntryId,
    },
    /// Each new Assignment Attempt receives independently selected Question Pool Items.
    RegeneratedAssignmentAttempt {
        assignment_attempt: crate::AssignmentAttemptId,
        assignment: AssignmentId,
        question_pool_assignment_entry: AssignmentEntryId,
    },
    /// An instructor-authorized, server-minted no-store preview sample.
    Preview {
        assignment: AssignmentId,
        question_pool_assignment_entry: AssignmentEntryId,
        nonce: QuestionPoolPreviewNonce,
    },
}

/// Opaque server-minted entropy for one Instructor Question Pool Preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionPoolPreviewNonce([u8; 16]);

impl QuestionPoolPreviewNonce {
    /// Builds the nonce from server-generated entropy.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the exact entropy used by the v1 derivation.
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl QuestionPoolReuseRule {
    /// Derives the only accepted server-owned selection inputs for one Question Pool entry.
    pub fn question_pool_selection_inputs(
        self,
        assignment: AssignmentId,
        assignment_attempt: &AssignmentAttempt,
        question_pool_assignment_entry: AssignmentEntryId,
    ) -> QuestionPoolSelectionInputs {
        match self {
            Self::ReuseSelection => QuestionPoolSelectionInputs::StableStudentRecord {
                student_record: assignment_attempt.student_record,
                assignment,
                question_pool_assignment_entry,
            },
            Self::SelectAgain => QuestionPoolSelectionInputs::RegeneratedAssignmentAttempt {
                assignment_attempt: assignment_attempt.id,
                assignment,
                question_pool_assignment_entry,
            },
        }
    }
}

impl QuestionPoolSelectionInputs {
    /// Creates independent no-store selection inputs for a saved definition preview.
    pub const fn preview(
        assignment: AssignmentId,
        question_pool_assignment_entry: AssignmentEntryId,
        nonce: QuestionPoolPreviewNonce,
    ) -> Self {
        Self::Preview {
            assignment,
            question_pool_assignment_entry,
            nonce,
        }
    }
}

/// The nine explicit Assignment activity rules an Assignment chooses, gathered for convenience.
///
/// A struct of independent enums rather than one combined enum: the rules vary
/// independently, and all combinations are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentActivityRules {
    /// What one Assignment Attempt must satisfy to be complete.
    pub assignment_completion_rule: AssignmentCompletionRule,
    /// Which completed Assignment Attempt score reaches the Gradebook.
    pub assignment_attempt_grade_rule: AssignmentAttemptGradeRule,
    /// Whether another Assignment Attempt may start after completion.
    pub assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule,
    /// Whether a later Assignment Attempt reuses its Question Pool Selection.
    pub question_pool_reuse_rule: QuestionPoolReuseRule,
    /// Whether a later Assignment Attempt reuses each selected Question Variation.
    pub question_variation_rule: AssignmentQuestionVariationRule,
    /// Whether the current Assignment Attempt can be resumed after leaving.
    pub assignment_attempt_resume_rule: AssignmentAttemptResumeRule,
    /// How many Issued Questions appear together.
    pub assignment_question_display_rule: AssignmentQuestionDisplayRule,
    /// How a Student may move through available Issued Questions.
    pub assignment_navigation_rule: AssignmentNavigationRule,
    /// The server-owned Issued Question order for one Assignment Attempt.
    pub assignment_question_order_rule: AssignmentQuestionOrderRule,
}

impl Default for AssignmentActivityRules {
    fn default() -> Self {
        Self {
            assignment_completion_rule: AssignmentCompletionRule::AnswerAll,
            assignment_attempt_grade_rule: AssignmentAttemptGradeRule::Highest,
            assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule::Unlimited,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssignmentQuestionVariationRule::NewVariation,
            assignment_attempt_resume_rule: AssignmentAttemptResumeRule::Resumable,
            assignment_question_display_rule: AssignmentQuestionDisplayRule::AllQuestions,
            assignment_navigation_rule: AssignmentNavigationRule::FreeNavigation,
            assignment_question_order_rule: AssignmentQuestionOrderRule::AuthoredOrder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn unlimited_attempts_are_expressed_as_none() {
        let policy = QuestionAttemptLimit { max_attempts: None };
        let json = serde_json::to_string(&policy).expect("serialization should succeed");
        assert!(json.contains(r#""maxAttempts":null"#));
    }

    #[test]
    fn question_attempt_limit_refuses_the_removed_feedback_member() {
        let error = serde_json::from_str::<QuestionAttemptLimit>(
            r#"{"maxAttempts":1,"feedback":"immediateFull"}"#,
        )
        .expect_err("legacy feedback must not be accepted");
        assert!(error.to_string().contains("feedback"));
    }

    #[test]
    fn the_nine_assignment_activity_rules_compose_freely() {
        let mastery_with_practice = AssignmentActivityRules {
            assignment_completion_rule: AssignmentCompletionRule::AllCorrect,
            assignment_attempt_grade_rule: AssignmentAttemptGradeRule::Highest,
            assignment_attempt_continuation_rule: AssignmentAttemptContinuationRule::Unlimited,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssignmentQuestionVariationRule::NewVariation,
            assignment_attempt_resume_rule: AssignmentAttemptResumeRule::Resumable,
            assignment_question_display_rule: AssignmentQuestionDisplayRule::AllQuestions,
            assignment_navigation_rule: AssignmentNavigationRule::FreeNavigation,
            assignment_question_order_rule: AssignmentQuestionOrderRule::AuthoredOrder,
        };
        let json =
            serde_json::to_string(&mastery_with_practice).expect("serialization should succeed");
        let restored: AssignmentActivityRules =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, mastery_with_practice);
        assert!(json.contains(r#""questionPoolReuseRule":"reuseSelection""#));
        assert!(json.contains(r#""questionVariationRule":"newVariation""#));
        assert!(serde_json::from_str::<AssignmentActivityRules>(
            r#"{"completion":{"kind":"allCorrect"},"grade":"highest","continuedPractice":{"kind":"unlimited"},"variation":"newSeeds"}"#,
        )
        .is_err());
        assert!(serde_json::from_str::<AssignmentActivityRules>(
            r#"{"completion":{"kind":"allCorrect"},"grade":"highest","continuedPractice":{"kind":"unlimited"},"questionVariationRule":"invalidValue"}"#,
        )
        .is_err());
        assert!(serde_json::from_str::<AssignmentActivityRules>(
            r#"{"assignmentCompletionRule":{"kind":"allCorrect"},"grade":"highest","continuedPractice":{"kind":"unlimited"},"questionVariationRule":"invalidValue"}"#,
        )
        .is_err());
        assert!(serde_json::from_str::<AssignmentActivityRules>(
            r#"{"assignmentCompletionRule":{"kind":"allCorrect"},"assignmentAttemptGradeRule":"highest","continuedPractice":{"kind":"unlimited"},"questionVariationRule":"invalidValue"}"#,
        )
        .is_err());
    }

    #[test]
    fn every_pool_reuse_and_variation_combination_is_an_explicit_policy() {
        for question_pool_reuse_rule in [
            QuestionPoolReuseRule::ReuseSelection,
            QuestionPoolReuseRule::SelectAgain,
        ] {
            for question_variation_rule in [
                AssignmentQuestionVariationRule::ReuseVariation,
                AssignmentQuestionVariationRule::NewVariation,
            ] {
                let rules = AssignmentActivityRules {
                    question_pool_reuse_rule,
                    question_variation_rule,
                    ..AssignmentActivityRules::default()
                };
                let json = serde_json::to_string(&rules).expect("policy combination serializes");
                let restored: AssignmentActivityRules =
                    serde_json::from_str(&json).expect("policy combination deserializes");
                assert_eq!(restored, rules);
            }
        }
    }

    #[test]
    fn pool_reuse_rule_derives_the_exact_selection_identity() {
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(1));
        let assignment_attempt = AssignmentAttempt {
            id: crate::AssignmentAttemptId::from_uuid(Uuid::from_u128(2)),
            reference: crate::AssignmentAttemptReference::new(1).expect("valid attempt reference"),
            student_record: crate::StudentRecordId::from_uuid(Uuid::from_u128(3)),
            assignment,
            assignment_revision: crate::AssignmentRevisionReference {
                assignment: crate::AssignmentReference::new(1).expect("valid assignment reference"),
                revision_number: crate::AssignmentRevisionNumber::INITIAL,
            },
            attempt_number: 2,
            started_at: crate::Timestamp::from_unix_millis(1),
            completed_at: None,
            score: None,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssignmentQuestionVariationRule::NewVariation,
        };
        let entry = AssignmentEntryId::from_uuid(Uuid::from_u128(4));

        assert_eq!(
            QuestionPoolReuseRule::ReuseSelection.question_pool_selection_inputs(
                assignment,
                &assignment_attempt,
                entry,
            ),
            QuestionPoolSelectionInputs::StableStudentRecord {
                student_record: assignment_attempt.student_record,
                assignment,
                question_pool_assignment_entry: entry,
            }
        );
        assert_eq!(
            QuestionPoolReuseRule::SelectAgain.question_pool_selection_inputs(
                assignment,
                &assignment_attempt,
                entry,
            ),
            QuestionPoolSelectionInputs::RegeneratedAssignmentAttempt {
                assignment_attempt: assignment_attempt.id,
                assignment,
                question_pool_assignment_entry: entry,
            }
        );
    }

    #[test]
    fn timing_carries_a_grace_period() {
        let policy = QuestionAttemptTimeLimit::Limited {
            seconds: 1800,
            grace_seconds: 30,
        };
        let json = serde_json::to_string(&policy).expect("serialization should succeed");
        assert!(json.contains(r#""graceSeconds":30"#));
    }

    #[test]
    fn student_feedback_release_rule_serializes_independent_snake_case_fields() {
        let rule = StudentFeedbackReleaseRule {
            score: StudentFeedbackReleaseTiming::AfterSubmit,
            per_item_correctness: StudentFeedbackReleaseTiming::AfterDue,
            question_feedback: StudentFeedbackReleaseTiming::DuringAttempt,
            question_answer: StudentFeedbackReleaseTiming::AfterClose,
            question_answer_explanation: StudentFeedbackReleaseTiming::AfterClose,
            class_statistics: StudentFeedbackReleaseTiming::Never,
        };

        let json = serde_json::to_string(&rule).expect("serialization should succeed");

        assert!(json.contains(r#""per_item_correctness":"after_due""#));
        assert!(json.contains(r#""class_statistics":"never""#));
    }

    #[test]
    fn default_student_feedback_release_rule_releases_feedback_after_submission() {
        let rule = StudentFeedbackReleaseRule::default();

        assert_eq!(rule.score, StudentFeedbackReleaseTiming::AfterSubmit);
        assert_eq!(
            rule.per_item_correctness,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(
            rule.question_feedback,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(
            rule.question_answer,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(
            rule.question_answer_explanation,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(rule.class_statistics, StudentFeedbackReleaseTiming::Never);
    }
}
