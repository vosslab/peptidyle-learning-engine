//! Pure, allowlist-only projection of trusted feedback for browser responses.
//!
//! This module owns no persistence and performs no grading. It accepts a
//! trusted server-side result plus already-sanitized teaching blocks, then
//! returns exactly the fields the assignment policy permits. Route and store
//! integration deliberately come later so idempotent replay can project the
//! same persisted private record.

use question_model::{
    AttemptResult, DisclosedFeedback, FeedbackContent, run_policy::FeedbackDisclosure,
};

/// Facts needed to decide whether a delayed feedback policy has unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackDisclosureState {
    /// The whole run has completed, as recorded by the authoritative store.
    pub run_completed: bool,
    /// An authorized instructor release record permits disclosure.
    pub released: bool,
}

/// Projects only the fields permitted by `policy`.
///
/// `None` means feedback remains locked. The function is total and contains
/// no browser, provider, key, or persistence behavior; callers must pass only
/// feedback that a trusted backend has verified and sanitized.
pub fn project_feedback(
    policy: FeedbackDisclosure,
    state: FeedbackDisclosureState,
    result: Option<AttemptResult>,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    let full = match policy {
        FeedbackDisclosure::ImmediateFull => true,
        FeedbackDisclosure::ImmediateCorrectness => false,
        FeedbackDisclosure::Deferred => state.run_completed,
        FeedbackDisclosure::OnRelease => state.released,
    };
    let unlocked = matches!(
        policy,
        FeedbackDisclosure::ImmediateFull | FeedbackDisclosure::ImmediateCorrectness
    ) || full;
    if !unlocked {
        return None;
    }

    let mut disclosed = DisclosedFeedback::empty();
    if let Some(result) = result {
        disclosed.correctness = Some(result.correct);
        if full {
            disclosed.points_earned = Some(result.points_earned);
            disclosed.points_possible = Some(result.points_possible);
        }
    }
    disclosed.hint = content.hint.clone();
    if full {
        disclosed.correct_response = content.correct_response.clone();
        disclosed.rationale = content.rationale.clone();
    }
    Some(disclosed)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use question_model::envelope::ContentBlock;

    fn content() -> FeedbackContent {
        FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "Check the units first.".to_string(),
            }]),
            correct_response: Some(vec![ContentBlock::Math {
                latex: "x = 2".to_string(),
                description: "x equals two".to_string(),
            }]),
            rationale: Some(vec![ContentBlock::Text {
                markdown: "Substitution satisfies the equation.".to_string(),
            }]),
        }
    }

    fn result() -> AttemptResult {
        AttemptResult {
            correct: false,
            points_earned: 0.0,
            points_possible: 2.0,
        }
    }

    fn state(run_completed: bool, released: bool) -> FeedbackDisclosureState {
        FeedbackDisclosureState {
            run_completed,
            released,
        }
    }

    fn keys(disclosed: &DisclosedFeedback) -> Vec<String> {
        let Value::Object(object) =
            serde_json::to_value(disclosed).expect("public feedback should serialize")
        else {
            panic!("feedback must serialize as an object");
        };
        let mut keys: Vec<_> = object.into_iter().map(|(key, _)| key).collect();
        keys.sort();
        keys
    }

    #[test]
    fn policy_matrix_is_an_exact_allowlist() {
        let cases = [
            (
                FeedbackDisclosure::ImmediateCorrectness,
                false,
                false,
                Some(vec!["correctness", "hint"]),
            ),
            (
                FeedbackDisclosure::ImmediateCorrectness,
                true,
                true,
                Some(vec!["correctness", "hint"]),
            ),
            (
                FeedbackDisclosure::ImmediateFull,
                false,
                false,
                Some(vec![
                    "correctResponse",
                    "correctness",
                    "hint",
                    "pointsEarned",
                    "pointsPossible",
                    "rationale",
                ]),
            ),
            (
                FeedbackDisclosure::ImmediateFull,
                true,
                true,
                Some(vec![
                    "correctResponse",
                    "correctness",
                    "hint",
                    "pointsEarned",
                    "pointsPossible",
                    "rationale",
                ]),
            ),
            (FeedbackDisclosure::Deferred, false, false, None),
            (
                FeedbackDisclosure::Deferred,
                true,
                false,
                Some(vec![
                    "correctResponse",
                    "correctness",
                    "hint",
                    "pointsEarned",
                    "pointsPossible",
                    "rationale",
                ]),
            ),
            (FeedbackDisclosure::OnRelease, false, false, None),
            (
                FeedbackDisclosure::OnRelease,
                false,
                true,
                Some(vec![
                    "correctResponse",
                    "correctness",
                    "hint",
                    "pointsEarned",
                    "pointsPossible",
                    "rationale",
                ]),
            ),
        ];

        for (policy, completed, released, expected) in cases {
            let projected = project_feedback(
                policy,
                state(completed, released),
                Some(result()),
                &content(),
            );
            match expected {
                Some(expected) => {
                    assert_eq!(keys(&projected.expect("feedback should unlock")), expected)
                }
                None => assert!(projected.is_none(), "{policy:?} should remain locked"),
            }
        }
    }

    #[test]
    fn release_never_unlocks_deferred_and_completion_never_unlocks_on_release() {
        assert!(
            project_feedback(
                FeedbackDisclosure::Deferred,
                state(false, true),
                Some(result()),
                &content(),
            )
            .is_none()
        );
        assert!(
            project_feedback(
                FeedbackDisclosure::OnRelease,
                state(true, false),
                Some(result()),
                &content(),
            )
            .is_none()
        );
    }

    #[test]
    fn ungraded_work_never_fabricates_correctness_or_points() {
        let disclosed = project_feedback(
            FeedbackDisclosure::ImmediateFull,
            state(false, false),
            None,
            &content(),
        )
        .expect("the unlocked projection exists");
        assert_eq!(
            keys(&disclosed),
            vec!["correctResponse", "hint", "rationale"]
        );
    }

    #[test]
    fn serialized_public_projection_has_no_private_contract_fields() {
        let disclosed = project_feedback(
            FeedbackDisclosure::ImmediateFull,
            state(false, false),
            Some(result()),
            &content(),
        )
        .expect("immediate full feedback should unlock");
        let json = serde_json::to_string(&disclosed).expect("feedback should serialize");
        for forbidden in [
            "answerKey",
            "expectedValue",
            "checkerState",
            "providerTranscript",
            "sourcePackage",
            "solutionUrl",
            "launchUrl",
            "credential",
            "token",
        ] {
            assert!(
                !json.contains(forbidden),
                "public feedback must not serialize {forbidden}"
            );
        }
    }
}
