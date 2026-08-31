//! Assignment Attempt, timing, and Assignment activity rules (WP-C1, WP-C3).
//!
//! The four Assignment activity rules are independent enums that compose freely. Keeping
//! them independent is what lets an instructor express "mastery required,
//! highest score kept, practice allowed after completion with fresh seeds",
//! which is the behavior the owner observed students actually using. A single
//! combined "mode" enum would offer a fixed menu instead.
//!
//! Question-level policies ([`AttemptPolicy`], [`TimingPolicy`]) are authored
//! with the question. Assignment-level rules are chosen per Assignment, so the same
//! published question serves a graded exam in one course and open practice in
//! another.

use serde::{Deserialize, Serialize};

use crate::{AssignmentAttempt, AssignmentId, AssignmentEntryId};

/// The point in an assignment lifecycle when one Student-facing field may be
/// disclosed.
///
/// Each timing is evaluated independently so an instructor can, for example,
/// show a score after submission while holding solutions until the assignment
/// closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentDisclosureTiming {
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

/// Assignment-owned Student disclosure policy.
///
/// These independently configured fields are evaluated server-side against
/// the effective assignment policy. They are intentionally separate from
/// [`AssignmentActivityRules`], whose Assignment Attempt behavior remains stable while S4 migrates
/// Student-facing projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentDisclosurePolicy {
    /// When the Student may see their score.
    pub score: StudentDisclosureTiming,
    /// When the Student may see per-item correctness.
    pub per_item_correctness: StudentDisclosureTiming,
    /// When the Student may see teaching feedback text.
    pub feedback_text: StudentDisclosureTiming,
    /// When the Student may see correct answers or solutions.
    pub solution: StudentDisclosureTiming,
    /// When the Student may see anonymous class statistics.
    pub class_statistics: StudentDisclosureTiming,
}

impl Default for StudentDisclosurePolicy {
    /// Returns the policy used when authoring a new assignment.
    ///
    /// This is deliberately an initializer rather than a serde compatibility
    /// fallback: an assignment payload must still carry this policy explicitly.
    fn default() -> Self {
        Self {
            score: StudentDisclosureTiming::AfterSubmit,
            per_item_correctness: StudentDisclosureTiming::AfterSubmit,
            feedback_text: StudentDisclosureTiming::AfterSubmit,
            solution: StudentDisclosureTiming::AfterSubmit,
            class_statistics: StudentDisclosureTiming::Never,
        }
    }
}

/// How many times a student may answer one question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttemptPolicy {
    /// Attempts permitted, or `None` for unlimited.
    ///
    /// `None` is the mastery case: retry until correct.
    pub max_attempts: Option<u32>,
}

/// Time limits applied to a question or an attempt.
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
pub enum TimingPolicy {
    /// No limit; the student works at their own pace.
    Untimed,
    /// A limit on each question, requiring the `perQuestionTiming` capability.
    PerQuestion {
        /// Seconds allowed for one question.
        seconds: u32,
        /// Extra seconds accepted after expiry, covering network delay.
        grace_seconds: u32,
    },
    /// A limit on the whole attempt.
    PerAttempt {
        /// Seconds allowed for the attempt.
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
pub enum CompletionRequirement {
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
pub enum GradePolicy {
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
pub enum ContinuedPractice {
    /// Any number of new Assignment Attempts may be started after completion.
    Unlimited,
    /// A bounded number of new Assignment Attempts may be started after completion.
    Capped {
        /// Assignment Attempts allowed after the first completed Assignment Attempt.
        max_additional_runs: u32,
    },
    /// The assignment closes once complete.
    Closed,
}

/// How much changes when a Student starts another Assignment Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum VariationPolicy {
    /// Same questions, fresh seeds, so the numbers change.
    NewSeeds,
    /// Instructor-selected Question Variants are used for the next Assignment Attempt.
    SelectedQuestionVariants,
    /// Questions are redrawn from the pool as well as reseeded.
    FullRegeneration,
}

/// Stable input for a pool draw.
///
/// The basis contains only server-owned durable identities. It chooses
/// candidate references; question issuance separately creates the fresh
/// private server seed for every selected question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolDrawBasis {
    /// Repeat draws for one Student Record retain the same candidate selection.
    StableStudentRecord {
        student_record: crate::StudentRecordId,
        assignment: AssignmentId,
        question_pool_entry: AssignmentEntryId,
    },
    /// Each new Assignment Attempt receives an independently derived candidate selection.
    RegeneratedAssignmentAttempt {
        assignment_attempt: crate::AssignmentAttemptId,
        assignment: AssignmentId,
        question_pool_entry: AssignmentEntryId,
    },
    /// An instructor-authorized, server-minted no-store preview sample.
    Preview {
        assignment: AssignmentId,
        question_pool_entry: AssignmentEntryId,
        nonce: PoolDrawPreviewNonce,
    },
}

/// Opaque server-minted entropy for an instructor pool-preview request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolDrawPreviewNonce([u8; 16]);

impl PoolDrawPreviewNonce {
    /// Builds the nonce from server-generated entropy.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the exact entropy used by the v1 derivation.
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Variation policy cannot derive a pool draw until the instructor supplies a
/// real selected-variant model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolDrawBasisError {
    /// A Question Pool has no instructor-selected variant source.
    SelectedQuestionVariantsRequireExplicitPoolSelection,
}

impl std::fmt::Display for PoolDrawBasisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedQuestionVariantsRequireExplicitPoolSelection => formatter.write_str(
                "selected Question Variants require an explicit pool-variant selection model",
            ),
        }
    }
}

impl std::error::Error for PoolDrawBasisError {}

impl VariationPolicy {
    /// Derives the only accepted server-owned basis for one Question Pool entry.
    pub fn pool_draw_basis(
        self,
        assignment: AssignmentId,
        assignment_attempt: &AssignmentAttempt,
        question_pool_entry: AssignmentEntryId,
    ) -> Result<PoolDrawBasis, PoolDrawBasisError> {
        match self {
            Self::NewSeeds => Ok(PoolDrawBasis::StableStudentRecord {
                student_record: assignment_attempt.student_record,
                assignment,
                question_pool_entry,
            }),
            Self::FullRegeneration => Ok(PoolDrawBasis::RegeneratedAssignmentAttempt {
                assignment_attempt: assignment_attempt.id,
                assignment,
                question_pool_entry,
            }),
            Self::SelectedQuestionVariants => {
                Err(PoolDrawBasisError::SelectedQuestionVariantsRequireExplicitPoolSelection)
            }
        }
    }
}

impl PoolDrawBasis {
    /// Creates the independent no-store preview basis for a saved definition.
    pub const fn preview(
        assignment: AssignmentId,
        question_pool_entry: AssignmentEntryId,
        nonce: PoolDrawPreviewNonce,
    ) -> Self {
        Self::Preview {
            assignment,
            question_pool_entry,
            nonce,
        }
    }
}

/// The four explicit Assignment activity rules an Assignment chooses, gathered for convenience.
///
/// A struct of independent enums rather than one combined enum: the four vary
/// independently, and all combinations are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentActivityRules {
    /// What completion requires.
    pub completion: CompletionRequirement,
    /// Which Assignment Attempt score is recorded.
    pub grade: GradePolicy,
    /// Whether practice continues after completion.
    pub continued_practice: ContinuedPractice,
    /// How much changes between Assignment Attempts.
    pub variation: VariationPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_attempts_are_expressed_as_none() {
        let policy = AttemptPolicy { max_attempts: None };
        let json = serde_json::to_string(&policy).expect("serialization should succeed");
        assert!(json.contains(r#""maxAttempts":null"#));
    }

    #[test]
    fn attempt_policy_refuses_the_removed_feedback_member() {
        let error = serde_json::from_str::<AttemptPolicy>(
            r#"{"maxAttempts":1,"feedback":"immediateFull"}"#,
        )
        .expect_err("legacy feedback must not be accepted");
        assert!(error.to_string().contains("feedback"));
    }

    #[test]
    fn the_four_assignment_activity_rules_compose_freely() {
        let mastery_with_practice = AssignmentActivityRules {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        };
        let json =
            serde_json::to_string(&mastery_with_practice).expect("serialization should succeed");
        let restored: AssignmentActivityRules =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, mastery_with_practice);
    }

    #[test]
    fn timing_carries_a_grace_period() {
        let policy = TimingPolicy::PerAttempt {
            seconds: 1800,
            grace_seconds: 30,
        };
        let json = serde_json::to_string(&policy).expect("serialization should succeed");
        assert!(json.contains(r#""graceSeconds":30"#));
    }

    #[test]
    fn student_disclosure_policy_serializes_independent_snake_case_fields() {
        let policy = StudentDisclosurePolicy {
            score: StudentDisclosureTiming::AfterSubmit,
            per_item_correctness: StudentDisclosureTiming::AfterDue,
            feedback_text: StudentDisclosureTiming::DuringAttempt,
            solution: StudentDisclosureTiming::AfterClose,
            class_statistics: StudentDisclosureTiming::Never,
        };

        let json = serde_json::to_string(&policy).expect("serialization should succeed");

        assert!(json.contains(r#""per_item_correctness":"after_due""#));
        assert!(json.contains(r#""class_statistics":"never""#));
    }

    #[test]
    fn default_student_disclosure_policy_releases_feedback_after_submission() {
        let policy = StudentDisclosurePolicy::default();

        assert_eq!(policy.score, StudentDisclosureTiming::AfterSubmit);
        assert_eq!(
            policy.per_item_correctness,
            StudentDisclosureTiming::AfterSubmit
        );
        assert_eq!(policy.feedback_text, StudentDisclosureTiming::AfterSubmit);
        assert_eq!(policy.solution, StudentDisclosureTiming::AfterSubmit);
        assert_eq!(policy.class_statistics, StudentDisclosureTiming::Never);
    }
}
