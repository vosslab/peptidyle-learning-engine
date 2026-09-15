//! Assessment Attempt, timing, and Assessment activity rules.
//!
//! The seven Assessment activity rules are independent enums that compose freely. Keeping
//! them independent lets an Instructor vary grading, Question selection, display,
//! navigation, and resumption without choosing a fixed combined mode.
//!
//! Question-level policies ([`QuestionAttemptLimit`], [`QuestionAttemptTimeLimit`]) are authored
//! with the question. Assessment-level rules are chosen per Assessment, so the same
//! published question serves a graded exam in one course and open practice in
//! another.

use serde::{Deserialize, Serialize};

use crate::{AssessmentAttempt, AssessmentEntryId, AssessmentId, AssessmentType};

/// The Assessment Attempt state or Assessment schedule point when one
/// Student-facing field may be disclosed.
///
/// Each timing is evaluated independently so an instructor can, for example,
/// show a score after submission while holding a Question Answer and its
/// Question Answer Explanation until the assessment closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentFeedbackReleaseTiming {
    /// The field is visible while a Student is working on the attempt.
    DuringAttempt,
    /// The field is visible once that Student has submitted the attempt.
    AfterSubmit,
    /// The field is visible at or after the resolved assessment due time.
    AfterDue,
    /// The field is visible at or after the resolved assessment close time.
    AfterClose,
    /// The field is never visible to a Student through this policy.
    Never,
}

/// Assessment-owned Student Feedback Release Rule.
///
/// These independently configured fields are evaluated server-side against
/// the effective assessment policy. They are intentionally separate from
/// [`AssessmentActivityRules`], whose Assessment Attempt behavior remains stable
/// and separate from Student-facing projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentFeedbackReleaseRule {
    /// When the Student may see their score.
    pub score: StudentFeedbackReleaseTiming,
    /// When the Student may see per-item correctness.
    pub per_item_correctness: StudentFeedbackReleaseTiming,
    /// When the Student may see their recorded response in a previous attempt.
    pub submitted_response: StudentFeedbackReleaseTiming,
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
    /// Returns the policy used when authoring a new assessment.
    ///
    /// This is deliberately an initializer rather than a serde compatibility
    /// fallback: an assessment payload must still carry this policy explicitly.
    fn default() -> Self {
        Self::for_assessment_type(AssessmentType::RegularAssignment)
    }
}

impl StudentFeedbackReleaseRule {
    /// Returns the established new-Assessment defaults with the one
    /// Type-specific answer timing required by Human Guidance.
    ///
    /// Question Feedback and Answer Explanation remain independent settings.
    /// Quiz and Exam answer fields use ordinary post-submit timing plus the
    /// trusted current-Course-cohort gate.
    pub fn for_assessment_type(assessment_type: AssessmentType) -> Self {
        Self {
            score: StudentFeedbackReleaseTiming::AfterSubmit,
            per_item_correctness: StudentFeedbackReleaseTiming::AfterSubmit,
            submitted_response: StudentFeedbackReleaseTiming::AfterSubmit,
            question_feedback: StudentFeedbackReleaseTiming::Never,
            question_answer: match assessment_type {
                AssessmentType::PracticeQuestionAssignment => {
                    StudentFeedbackReleaseTiming::AfterSubmit
                }
                AssessmentType::Quiz | AssessmentType::Exam => {
                    StudentFeedbackReleaseTiming::AfterSubmit
                }
                AssessmentType::RegularAssignment | AssessmentType::BonusAssignment => {
                    StudentFeedbackReleaseTiming::Never
                }
            },
            question_answer_explanation: match assessment_type {
                AssessmentType::Quiz | AssessmentType::Exam => {
                    StudentFeedbackReleaseTiming::AfterSubmit
                }
                AssessmentType::RegularAssignment
                | AssessmentType::PracticeQuestionAssignment
                | AssessmentType::BonusAssignment => StudentFeedbackReleaseTiming::Never,
            },
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

/// Which Assessment Attempt score reaches the Gradebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssessmentAttemptGradeRule {
    /// The first Assessment Attempt's score.
    First,
    /// The most recent Assessment Attempt's score.
    Latest,
    /// The best Assessment Attempt's score.
    Highest,
    /// An Assessment Attempt explicitly selected by the Instructor.
    InstructorSelected,
}

/// What a later Assessment Attempt does with Question Pool membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestionPoolReuseRule {
    /// Retain the Student's prior Question Pool Selection.
    ReuseSelection,
    /// Create a new Question Pool Selection for the later Assessment Attempt.
    SelectAgain,
}

/// What a later Assessment Attempt does with Question Variations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssessmentQuestionVariationRule {
    /// Retain each selected Question's existing Question Variation.
    ReuseVariation,
    /// Issue a fresh Question Seed for every selected Question.
    NewVariation,
}

/// Whether one Assessment Attempt can be left and later resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentAttemptResumeRule {
    /// A Student can leave the Assessment Attempt and return to its server-owned state.
    Resumable,
    /// A Student must finish the Assessment Attempt in its first active session.
    SingleSession,
}

/// How many Issued Questions an Assessment Attempt presents at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentQuestionDisplayRule {
    /// Present every available Issued Question together.
    AllQuestions,
    /// Present one Issued Question at a time.
    OneQuestionAtATime,
}

/// How a Student may move among available Issued Questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentNavigationRule {
    /// A Student may revisit any available Issued Question.
    FreeNavigation,
    /// A Student may advance but cannot return to an earlier Issued Question.
    ForwardOnly,
}

/// The server-owned order used after Assessment Entries expand into Issued Questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentQuestionOrderRule {
    /// Preserve the Instructor-authored Assessment Entry order.
    AuthoredOrder,
    /// Shuffle the expanded Issued Question order once for the Assessment Attempt.
    Shuffled,
}

/// Stable server-owned inputs for one Question Pool Selection.
///
/// The basis contains only server-owned durable identities. It chooses
/// Question Pool Item references; Question issuance separately creates a fresh
/// Question Seed for every selected Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionPoolSelectionInputs {
    /// Repeat selections for one Student Record retain the same Question Pool Items.
    StableStudentRecord {
        student_record: crate::StudentRecordId,
        assessment: AssessmentId,
        question_pool_assessment_entry: AssessmentEntryId,
    },
    /// Each new Assessment Attempt receives independently selected Question Pool Items.
    RegeneratedAssessmentAttempt {
        assessment_attempt: crate::AssessmentAttemptId,
        assessment: AssessmentId,
        question_pool_assessment_entry: AssessmentEntryId,
    },
    /// An instructor-authorized, server-minted no-store preview sample.
    Preview {
        assessment: AssessmentId,
        question_pool_assessment_entry: AssessmentEntryId,
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
    /// Derives the only accepted server-owned selection inputs for one Question Pool Assessment Entry.
    pub fn question_pool_selection_inputs(
        self,
        assessment: AssessmentId,
        assessment_attempt: &AssessmentAttempt,
        question_pool_assessment_entry: AssessmentEntryId,
    ) -> QuestionPoolSelectionInputs {
        match self {
            Self::ReuseSelection => QuestionPoolSelectionInputs::StableStudentRecord {
                student_record: assessment_attempt.student_record,
                assessment,
                question_pool_assessment_entry,
            },
            Self::SelectAgain => QuestionPoolSelectionInputs::RegeneratedAssessmentAttempt {
                assessment_attempt: assessment_attempt.id,
                assessment,
                question_pool_assessment_entry,
            },
        }
    }
}

impl QuestionPoolSelectionInputs {
    /// Preview inputs are for an Instructor Question Pool Preview.
    pub const fn preview(
        assessment: AssessmentId,
        question_pool_assessment_entry: AssessmentEntryId,
        nonce: QuestionPoolPreviewNonce,
    ) -> Self {
        Self::Preview {
            assessment,
            question_pool_assessment_entry,
            nonce,
        }
    }
}

/// The seven explicit Assessment activity rules an Assessment chooses, gathered for convenience.
///
/// A struct of independent enums rather than one combined enum: the rules vary
/// independently, and all combinations are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentActivityRules {
    /// Which completed Assessment Attempt score reaches the Gradebook.
    pub assessment_attempt_grade_rule: AssessmentAttemptGradeRule,
    /// Whether a later Assessment Attempt reuses its Question Pool Selection.
    pub question_pool_reuse_rule: QuestionPoolReuseRule,
    /// Whether a later Assessment Attempt reuses each selected Question Variation.
    pub question_variation_rule: AssessmentQuestionVariationRule,
    /// Whether the current Assessment Attempt can be resumed after leaving.
    pub assessment_attempt_resume_rule: AssessmentAttemptResumeRule,
    /// How many Issued Questions appear together.
    pub assessment_question_display_rule: AssessmentQuestionDisplayRule,
    /// How a Student may move through available Issued Questions.
    pub assessment_navigation_rule: AssessmentNavigationRule,
    /// The server-owned Issued Question order for one Assessment Attempt.
    pub assessment_question_order_rule: AssessmentQuestionOrderRule,
}

impl Default for AssessmentActivityRules {
    fn default() -> Self {
        Self {
            assessment_attempt_grade_rule: AssessmentAttemptGradeRule::Highest,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
            assessment_attempt_resume_rule: AssessmentAttemptResumeRule::Resumable,
            assessment_question_display_rule: AssessmentQuestionDisplayRule::OneQuestionAtATime,
            assessment_navigation_rule: AssessmentNavigationRule::FreeNavigation,
            assessment_question_order_rule: AssessmentQuestionOrderRule::Shuffled,
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
    fn new_assessment_defaults_shuffle_one_question_at_a_time() {
        let rules = AssessmentActivityRules::default();

        assert_eq!(
            rules.assessment_question_display_rule,
            AssessmentQuestionDisplayRule::OneQuestionAtATime
        );
        assert_eq!(
            rules.assessment_question_order_rule,
            AssessmentQuestionOrderRule::Shuffled
        );
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
    fn activity_rules_round_trip_the_closed_seven_field_contract() {
        let rules = AssessmentActivityRules {
            assessment_attempt_grade_rule: AssessmentAttemptGradeRule::Highest,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
            assessment_attempt_resume_rule: AssessmentAttemptResumeRule::Resumable,
            assessment_question_display_rule: AssessmentQuestionDisplayRule::AllQuestions,
            assessment_navigation_rule: AssessmentNavigationRule::FreeNavigation,
            assessment_question_order_rule: AssessmentQuestionOrderRule::AuthoredOrder,
        };
        let json = serde_json::to_string(&rules).expect("serialization should succeed");
        let restored: AssessmentActivityRules =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, rules);
        assert!(json.contains(r#""questionPoolReuseRule":"reuseSelection""#));
        assert!(json.contains(r#""questionVariationRule":"newVariation""#));
        assert!(
            serde_json::from_str::<AssessmentActivityRules>(
                r#"{"completion":{"kind":"allCorrect"}}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn every_pool_reuse_and_variation_combination_is_an_explicit_policy() {
        for question_pool_reuse_rule in [
            QuestionPoolReuseRule::ReuseSelection,
            QuestionPoolReuseRule::SelectAgain,
        ] {
            for question_variation_rule in [
                AssessmentQuestionVariationRule::ReuseVariation,
                AssessmentQuestionVariationRule::NewVariation,
            ] {
                let rules = AssessmentActivityRules {
                    question_pool_reuse_rule,
                    question_variation_rule,
                    ..AssessmentActivityRules::default()
                };
                let json = serde_json::to_string(&rules).expect("policy combination serializes");
                let restored: AssessmentActivityRules =
                    serde_json::from_str(&json).expect("policy combination deserializes");
                assert_eq!(restored, rules);
            }
        }
    }

    #[test]
    fn pool_reuse_rule_derives_the_exact_selection_identity() {
        let assessment = AssessmentId::from_uuid(Uuid::from_u128(1));
        let assessment_attempt = AssessmentAttempt {
            id: crate::AssessmentAttemptId::from_uuid(Uuid::from_u128(2)),
            reference: crate::AssessmentAttemptReference::new(1).expect("valid attempt reference"),
            student_record: crate::StudentRecordId::from_uuid(Uuid::from_u128(3)),
            assessment,
            evidence: crate::AssessmentAttemptEvidence {
                title: crate::AssessmentTitle::try_new("Assessment".to_string()).expect("title"),
                instructions: crate::AssessmentInstructions::default(),
                base_policy: crate::BaseAssessmentPolicy::default(),
                activity_rules: AssessmentActivityRules::default(),
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                effective_policy_sources: crate::AssessmentAttemptPolicySources::default(),
            },
            attempt_number: 2,
            started_at: crate::Timestamp::from_unix_millis(1),
            submitted_at: None,
            score: None,
        };
        let entry = AssessmentEntryId::from_uuid(Uuid::from_u128(4));

        assert_eq!(
            QuestionPoolReuseRule::ReuseSelection.question_pool_selection_inputs(
                assessment,
                &assessment_attempt,
                entry,
            ),
            QuestionPoolSelectionInputs::StableStudentRecord {
                student_record: assessment_attempt.student_record,
                assessment,
                question_pool_assessment_entry: entry,
            }
        );
        assert_eq!(
            QuestionPoolReuseRule::SelectAgain.question_pool_selection_inputs(
                assessment,
                &assessment_attempt,
                entry,
            ),
            QuestionPoolSelectionInputs::RegeneratedAssessmentAttempt {
                assessment_attempt: assessment_attempt.id,
                assessment,
                question_pool_assessment_entry: entry,
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
            submitted_response: StudentFeedbackReleaseTiming::AfterSubmit,
            question_feedback: StudentFeedbackReleaseTiming::DuringAttempt,
            question_answer: StudentFeedbackReleaseTiming::AfterClose,
            question_answer_explanation: StudentFeedbackReleaseTiming::AfterClose,
            class_statistics: StudentFeedbackReleaseTiming::Never,
        };

        let json = serde_json::to_string(&rule).expect("serialization should succeed");

        assert!(json.contains(r#""per_item_correctness":"after_due""#));
        assert!(json.contains(r#""submitted_response":"after_submit""#));
        assert!(json.contains(r#""class_statistics":"never""#));
    }

    #[test]
    fn default_student_feedback_releases_response_and_correctness_after_submission() {
        let rule = StudentFeedbackReleaseRule::default();

        assert_eq!(rule.score, StudentFeedbackReleaseTiming::AfterSubmit);
        assert_eq!(
            rule.per_item_correctness,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(
            rule.submitted_response,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(rule.question_feedback, StudentFeedbackReleaseTiming::Never);
        assert_eq!(rule.question_answer, StudentFeedbackReleaseTiming::Never);
        assert_eq!(
            rule.question_answer_explanation,
            StudentFeedbackReleaseTiming::Never
        );
        assert_eq!(rule.class_statistics, StudentFeedbackReleaseTiming::Never);
    }

    #[test]
    fn practice_defaults_release_the_answer_without_releasing_question_feedback() {
        let practice = StudentFeedbackReleaseRule::for_assessment_type(
            AssessmentType::PracticeQuestionAssignment,
        );

        assert_eq!(
            practice.question_answer,
            StudentFeedbackReleaseTiming::AfterSubmit
        );
        assert_eq!(
            practice.question_feedback,
            StudentFeedbackReleaseTiming::Never
        );
        assert_eq!(
            practice.question_answer_explanation,
            StudentFeedbackReleaseTiming::Never
        );
    }

    #[test]
    fn quiz_and_exam_defaults_release_answers_only_after_submission() {
        for assessment_type in [AssessmentType::Quiz, AssessmentType::Exam] {
            let rule = StudentFeedbackReleaseRule::for_assessment_type(assessment_type);
            assert_eq!(
                rule.question_answer,
                StudentFeedbackReleaseTiming::AfterSubmit
            );
            assert_eq!(
                rule.question_answer_explanation,
                StudentFeedbackReleaseTiming::AfterSubmit
            );
        }
    }
}
