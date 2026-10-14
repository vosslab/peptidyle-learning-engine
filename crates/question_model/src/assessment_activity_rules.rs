//! Assessment Attempt, timing, and Assessment activity rules.
//!
//! The six Assessment activity rules are independent enums that compose freely. Keeping
//! them independent lets an Instructor vary grading, Question variation, display,
//! navigation, and resumption without choosing a fixed combined mode.
//!
//! Question-level policies ([`QuestionAttemptLimit`], [`QuestionAttemptTimeLimit`]) are authored
//! with the question. Assessment-level rules are chosen per Assessment, so the same
//! published question serves a graded exam in one course and open practice in
//! another.

use serde::{Deserialize, Serialize};

use crate::AssessmentType;

/// The Assessment Attempt state or Assessment schedule point when one
/// Student-facing field may be disclosed.
///
/// Each timing is evaluated independently so an instructor can, for example,
/// show a Question Answer after submission while holding its explanation until
/// the assessment closes.
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
    /// The field is visible once every current Student has completed the Assessment.
    AfterAllStudentsComplete,
    /// The field is never visible to a Student through this policy.
    Never,
}

/// Assessment-owned optional Student Feedback Release Rule.
///
/// These independently configured fields are evaluated server-side against
/// the effective assessment policy. They are intentionally separate from
/// [`AssessmentActivityRules`], whose Assessment Attempt behavior remains stable
/// and separate from Student-facing projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StudentFeedbackReleaseRule {
    /// When the Student may see the display-ready Question Answer.
    pub question_answer: StudentFeedbackReleaseTiming,
    /// When the Student may see the Question Answer Explanation.
    pub question_answer_explanation: StudentFeedbackReleaseTiming,
    /// When the Student may see anonymous class statistics.
    pub class_statistics: StudentFeedbackReleaseTiming,
    /// When the Student may see PLE-managed Hints.
    ///
    /// ASVS 8.2.3: this timing is independent of Question Answer disclosure.
    pub hints: StudentFeedbackReleaseTiming,
    /// When the Student may see PLE-managed Worked Solutions.
    ///
    /// ASVS 8.2.3: this timing is independent of Hints and of Question Answer disclosure.
    pub worked_solutions: StudentFeedbackReleaseTiming,
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
    /// Answer and Answer Explanation remain independent settings. Quiz and Exam
    /// default to current-Course completion, which an Instructor can replace.
    pub fn for_assessment_type(assessment_type: AssessmentType) -> Self {
        Self {
            question_answer: match assessment_type {
                AssessmentType::PracticeQuestionAssignment => {
                    StudentFeedbackReleaseTiming::AfterSubmit
                }
                AssessmentType::Quiz | AssessmentType::Exam => {
                    StudentFeedbackReleaseTiming::AfterAllStudentsComplete
                }
                AssessmentType::RegularAssignment | AssessmentType::BonusAssignment => {
                    StudentFeedbackReleaseTiming::Never
                }
            },
            question_answer_explanation: match assessment_type {
                AssessmentType::Quiz | AssessmentType::Exam => {
                    StudentFeedbackReleaseTiming::AfterAllStudentsComplete
                }
                AssessmentType::RegularAssignment
                | AssessmentType::PracticeQuestionAssignment
                | AssessmentType::BonusAssignment => StudentFeedbackReleaseTiming::Never,
            },
            class_statistics: StudentFeedbackReleaseTiming::Never,
            // Human Guidance assigns these their own settings and does not name
            // a more permissive default, so both stay hidden until an Instructor
            // chooses a timing.
            hints: StudentFeedbackReleaseTiming::Never,
            worked_solutions: StudentFeedbackReleaseTiming::Never,
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

/// What a later Assessment Attempt does with Question Variations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssessmentQuestionVariationRule {
    /// Retain each selected Question's existing Question Variation.
    ReuseVariation,
    /// Issue a fresh Question Seed for every selected Question.
    NewVariation,
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

/// The Assessment's grading and activity settings, gathered for convenience.
///
/// A struct of independent enums rather than one combined enum: the rules vary
/// independently, and all combinations are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentActivityRules {
    /// Whether recorded Backend fractions below one earn proportional points.
    /// When disabled, only a stored fraction of one earns points.
    pub partial_credit_enabled: bool,
    /// Whether a later Assessment Attempt reuses each selected Question Variation.
    pub question_variation_rule: AssessmentQuestionVariationRule,
    /// The server-owned Issued Question order for one Assessment Attempt.
    pub assessment_question_order_rule: AssessmentQuestionOrderRule,
}

impl Default for AssessmentActivityRules {
    fn default() -> Self {
        Self {
            partial_credit_enabled: true,
            question_variation_rule: AssessmentQuestionVariationRule::NewVariation,
            assessment_question_order_rule: AssessmentQuestionOrderRule::Shuffled,
        }
    }
}
