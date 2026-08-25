//! Attempt, timing, and run policies (WP-C1, WP-C3, MOD-RUN).
//!
//! The four run policies are independent enums that compose freely. Keeping
//! them independent is what lets an instructor express "mastery required,
//! highest score kept, practice allowed after completion with fresh seeds",
//! which is the behavior the owner observed students actually using. A single
//! combined "mode" enum would offer a fixed menu instead.
//!
//! Question-level policies ([`AttemptPolicy`], [`TimingPolicy`]) are authored
//! with the question. Run-level policies are chosen per assignment, so the same
//! published question serves a graded exam in one course and open practice in
//! another.

use serde::{Deserialize, Serialize};

use crate::{AssignmentId, AssignmentRun, AssignmentSelectionGroupId};

/// The point in an assignment lifecycle when one learner-facing field may be
/// disclosed.
///
/// Each timing is evaluated independently so an instructor can, for example,
/// show a score after submission while holding solutions until the assignment
/// closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LearnerDisclosureTiming {
    /// The field is visible while a learner is working on the attempt.
    DuringAttempt,
    /// The field is visible once that learner has submitted the attempt.
    AfterSubmit,
    /// The field is visible at or after the resolved assignment due time.
    AfterDue,
    /// The field is visible at or after the resolved assignment close time.
    AfterClose,
    /// The field is never visible to a learner through this policy.
    Never,
}

/// Assignment-owned learner disclosure policy.
///
/// These independently configured fields are evaluated server-side against
/// the effective assignment policy. They are intentionally separate from
/// [`RunPolicies`], whose run behavior remains stable while S4 migrates
/// learner-facing projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnerDisclosurePolicy {
    /// When the learner may see their score.
    pub score: LearnerDisclosureTiming,
    /// When the learner may see per-item correctness.
    pub per_item_correctness: LearnerDisclosureTiming,
    /// When the learner may see teaching feedback text.
    pub feedback_text: LearnerDisclosureTiming,
    /// When the learner may see correct answers or solutions.
    pub solution: LearnerDisclosureTiming,
    /// When the learner may see anonymous class statistics.
    pub class_statistics: LearnerDisclosureTiming,
}

impl Default for LearnerDisclosurePolicy {
    /// Returns the policy used when authoring a new assignment.
    ///
    /// This is deliberately an initializer rather than a serde compatibility
    /// fallback: an assignment payload must still carry this policy explicitly.
    fn default() -> Self {
        Self {
            score: LearnerDisclosureTiming::AfterSubmit,
            per_item_correctness: LearnerDisclosureTiming::AfterSubmit,
            feedback_text: LearnerDisclosureTiming::AfterSubmit,
            solution: LearnerDisclosureTiming::AfterSubmit,
            class_statistics: LearnerDisclosureTiming::Never,
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

/// What a student must achieve for a run to count as complete.
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
    /// Answering every question completes the run, whatever the score.
    AnswerAll,
    /// Every question must be answered correctly.
    AllCorrect,
    /// A score at or above a threshold completes the run.
    ScoreAtLeast {
        /// Threshold as a fraction, where 0.8 means eighty percent.
        fraction: f64,
    },
}

/// Which run's score reaches the gradebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum GradePolicy {
    /// The first run's score.
    First,
    /// The most recent run's score.
    Latest,
    /// The best run's score.
    Highest,
    /// A run explicitly selected by the instructor.
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
    /// Any number of new runs may be started after completion.
    Unlimited,
    /// A bounded number of new runs may be started after completion.
    Capped {
        /// Runs allowed after the first completed run.
        max_additional_runs: u32,
    },
    /// The assignment closes once complete.
    Closed,
}

/// How much changes when a student starts another run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum VariationPolicy {
    /// Same questions, fresh seeds, so the numbers change.
    NewSeeds,
    /// Instructor-selected problem variants are used for the next run.
    SelectedProblemVariants,
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
    /// Repeat draws for one enrollment retain the same candidate selection.
    StableEnrollment {
        enrollment: crate::EnrollmentId,
        assignment: AssignmentId,
        group: AssignmentSelectionGroupId,
    },
    /// Each new run receives an independently derived candidate selection.
    RegeneratedRun {
        run: crate::RunId,
        assignment: AssignmentId,
        group: AssignmentSelectionGroupId,
    },
    /// An instructor-authorized, server-minted no-store preview sample.
    Preview {
        assignment: AssignmentId,
        group: AssignmentSelectionGroupId,
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
    /// A selection group has no instructor-selected variant source.
    SelectedProblemVariantsRequireExplicitPoolSelection,
}

impl std::fmt::Display for PoolDrawBasisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedProblemVariantsRequireExplicitPoolSelection => formatter.write_str(
                "selected problem variants require an explicit pool-variant selection model",
            ),
        }
    }
}

impl std::error::Error for PoolDrawBasisError {}

impl VariationPolicy {
    /// Derives the only accepted server-owned basis for one pool group.
    pub fn pool_draw_basis(
        self,
        assignment: AssignmentId,
        run: &AssignmentRun,
        group: AssignmentSelectionGroupId,
    ) -> Result<PoolDrawBasis, PoolDrawBasisError> {
        match self {
            Self::NewSeeds => Ok(PoolDrawBasis::StableEnrollment {
                enrollment: run.enrollment,
                assignment,
                group,
            }),
            Self::FullRegeneration => Ok(PoolDrawBasis::RegeneratedRun {
                run: run.id,
                assignment,
                group,
            }),
            Self::SelectedProblemVariants => {
                Err(PoolDrawBasisError::SelectedProblemVariantsRequireExplicitPoolSelection)
            }
        }
    }
}

impl PoolDrawBasis {
    /// Creates the independent no-store preview basis for a saved definition.
    pub const fn preview(
        assignment: AssignmentId,
        group: AssignmentSelectionGroupId,
        nonce: PoolDrawPreviewNonce,
    ) -> Self {
        Self::Preview {
            assignment,
            group,
            nonce,
        }
    }
}

/// The four run policies an assignment chooses, gathered for convenience.
///
/// A struct of independent enums rather than one combined enum: the four vary
/// independently, and all combinations are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPolicies {
    /// What completion requires.
    pub completion: CompletionRequirement,
    /// Which run's score is recorded.
    pub grade: GradePolicy,
    /// Whether practice continues after completion.
    pub continued_practice: ContinuedPractice,
    /// How much changes between runs.
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
    fn the_four_run_policies_compose_freely() {
        let mastery_with_practice = RunPolicies {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        };
        let json =
            serde_json::to_string(&mastery_with_practice).expect("serialization should succeed");
        let restored: RunPolicies =
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
    fn learner_disclosure_policy_serializes_independent_camel_case_fields() {
        let policy = LearnerDisclosurePolicy {
            score: LearnerDisclosureTiming::AfterSubmit,
            per_item_correctness: LearnerDisclosureTiming::AfterDue,
            feedback_text: LearnerDisclosureTiming::DuringAttempt,
            solution: LearnerDisclosureTiming::AfterClose,
            class_statistics: LearnerDisclosureTiming::Never,
        };

        let json = serde_json::to_string(&policy).expect("serialization should succeed");

        assert!(json.contains(r#""perItemCorrectness":"afterDue""#));
        assert!(json.contains(r#""classStatistics":"never""#));
    }

    #[test]
    fn default_learner_disclosure_policy_releases_feedback_after_submission() {
        let policy = LearnerDisclosurePolicy::default();

        assert_eq!(policy.score, LearnerDisclosureTiming::AfterSubmit);
        assert_eq!(
            policy.per_item_correctness,
            LearnerDisclosureTiming::AfterSubmit
        );
        assert_eq!(policy.feedback_text, LearnerDisclosureTiming::AfterSubmit);
        assert_eq!(policy.solution, LearnerDisclosureTiming::AfterSubmit);
        assert_eq!(policy.class_statistics, LearnerDisclosureTiming::Never);
    }
}
