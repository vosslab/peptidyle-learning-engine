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

/// When a student may see feedback about a submitted response.
///
/// Disclosure is a policy in its own right because the same question serves a
/// practice set, where immediate feedback teaches, and an exam, where it would
/// leak answers between students.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FeedbackDisclosure {
    /// Correctness appears as soon as a response is submitted.
    Immediate,
    /// Correctness appears once the whole attempt is submitted.
    AfterAttempt,
    /// Correctness appears after the assignment's due date passes.
    AfterDueDate,
    /// Correctness stays withheld from the student view.
    Withheld,
}

/// How many times a student may answer one question, and what they learn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptPolicy {
    /// Attempts permitted, or `None` for unlimited.
    ///
    /// `None` is the mastery case: retry until correct.
    pub max_attempts: Option<u32>,
    /// When correctness is disclosed.
    pub feedback: FeedbackDisclosure,
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
    /// The mean across runs.
    Average,
}

/// What a student may do after completing an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ContinuedPractice {
    /// Further runs are available and leave the recorded grade untouched.
    Allowed,
    /// The assignment closes once complete.
    Closed,
}

/// How much changes when a student starts another run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum VariationPolicy {
    /// Same questions, fresh seeds, so the numbers change.
    NewSeeds,
    /// Questions redrawn from the pool as well as reseeded.
    FullRegeneration,
    /// Identical content to the previous run.
    Identical,
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
        let policy = AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::Immediate,
        };
        let json = serde_json::to_string(&policy).expect("serialization should succeed");
        assert!(json.contains(r#""maxAttempts":null"#));
    }

    #[test]
    fn the_four_run_policies_compose_freely() {
        let mastery_with_practice = RunPolicies {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Allowed,
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
}
