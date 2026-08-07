//! Attempt lifecycle and pure state transitions (MOD-STATE).
//!
//! The server is authoritative for every event. The browser may project a
//! disclosed state, but it cannot mark an answer correct, exhaust attempts, or
//! override a timeout. Keeping the transition function free of clocks and
//! storage lets the API apply the same rule before committing an event.

use serde::{Deserialize, Serialize};

/// Current lifecycle state of one logical question inside a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttemptState {
    /// No question instance has been issued yet.
    NotStarted,
    /// A server-issued question instance is accepting a response.
    Active,
    /// A response arrived and awaits authoritative server grading.
    Submitted,
    /// The server graded the response as correct.
    Correct,
    /// The server graded the response as incorrect and has not applied retry policy yet.
    Incorrect,
    /// Policy permits the server to issue another question attempt.
    RetryAvailable,
    /// Policy permits no further response for this logical question.
    Exhausted,
    /// The authoritative timing rule rejected further responses.
    TimedOut,
    /// The student used an instructor-permitted give-up action.
    Abandoned,
}

/// One requested attempt-lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttemptEvent {
    /// Issue the first question instance.
    Start,
    /// Record a student response for server grading.
    Submit,
    /// Record an authoritative correct grade.
    GradeCorrect,
    /// Record an authoritative incorrect grade.
    GradeIncorrect,
    /// Apply policy that permits another issued attempt.
    AllowRetry,
    /// Apply policy that permits no more attempts.
    ExhaustRetries,
    /// Issue the policy-permitted retry as a new question attempt.
    StartRetry,
    /// Apply the authoritative timer verdict.
    TimeOut,
    /// Apply an instructor-permitted give-up action.
    Abandon,
}

/// A state/event pair that the lifecycle refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptTransitionError {
    /// State that rejected the event.
    pub state: AttemptState,
    /// Event that cannot apply to that state.
    pub event: AttemptEvent,
}

impl std::fmt::Display for AttemptTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "attempt event {:?} does not apply to state {:?}",
            self.event, self.state
        )
    }
}

impl std::error::Error for AttemptTransitionError {}

/// Applies one event to one attempt state.
///
/// `StartRetry` returns the logical question to `Active`, but it does not
/// mutate the earlier [`question_model::QuestionAttempt`]. The API must issue a
/// new record with a fresh server-owned seed and retain the earlier attempt as
/// history.
///
/// # Errors
///
/// Returns [`AttemptTransitionError`] when the event does not apply to the
/// supplied state. Terminal states accept no further events.
pub fn apply(
    state: AttemptState,
    event: AttemptEvent,
) -> Result<AttemptState, AttemptTransitionError> {
    match (state, event) {
        (AttemptState::NotStarted, AttemptEvent::Start)
        | (AttemptState::RetryAvailable, AttemptEvent::StartRetry) => Ok(AttemptState::Active),
        (AttemptState::Active, AttemptEvent::Submit) => Ok(AttemptState::Submitted),
        (AttemptState::Submitted, AttemptEvent::GradeCorrect) => Ok(AttemptState::Correct),
        (AttemptState::Submitted, AttemptEvent::GradeIncorrect) => Ok(AttemptState::Incorrect),
        (AttemptState::Incorrect, AttemptEvent::AllowRetry) => Ok(AttemptState::RetryAvailable),
        (AttemptState::Incorrect, AttemptEvent::ExhaustRetries) => Ok(AttemptState::Exhausted),
        (AttemptState::Active, AttemptEvent::TimeOut) => Ok(AttemptState::TimedOut),
        (AttemptState::Active | AttemptState::RetryAvailable, AttemptEvent::Abandon) => {
            Ok(AttemptState::Abandoned)
        }
        _ => Err(AttemptTransitionError { state, event }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_legal_transition_has_one_explicit_result() {
        let transitions = [
            (
                AttemptState::NotStarted,
                AttemptEvent::Start,
                AttemptState::Active,
            ),
            (
                AttemptState::Active,
                AttemptEvent::Submit,
                AttemptState::Submitted,
            ),
            (
                AttemptState::Submitted,
                AttemptEvent::GradeCorrect,
                AttemptState::Correct,
            ),
            (
                AttemptState::Submitted,
                AttemptEvent::GradeIncorrect,
                AttemptState::Incorrect,
            ),
            (
                AttemptState::Incorrect,
                AttemptEvent::AllowRetry,
                AttemptState::RetryAvailable,
            ),
            (
                AttemptState::Incorrect,
                AttemptEvent::ExhaustRetries,
                AttemptState::Exhausted,
            ),
            (
                AttemptState::RetryAvailable,
                AttemptEvent::StartRetry,
                AttemptState::Active,
            ),
            (
                AttemptState::Active,
                AttemptEvent::TimeOut,
                AttemptState::TimedOut,
            ),
            (
                AttemptState::Active,
                AttemptEvent::Abandon,
                AttemptState::Abandoned,
            ),
            (
                AttemptState::RetryAvailable,
                AttemptEvent::Abandon,
                AttemptState::Abandoned,
            ),
        ];

        for (state, event, expected) in transitions {
            assert_eq!(apply(state, event), Ok(expected), "{state:?} + {event:?}");
        }
    }

    #[test]
    fn grading_cannot_skip_the_submitted_state() {
        assert_eq!(
            apply(AttemptState::Active, AttemptEvent::GradeCorrect),
            Err(AttemptTransitionError {
                state: AttemptState::Active,
                event: AttemptEvent::GradeCorrect,
            })
        );
    }

    #[test]
    fn terminal_states_refuse_later_events() {
        for terminal in [
            AttemptState::Correct,
            AttemptState::Exhausted,
            AttemptState::TimedOut,
            AttemptState::Abandoned,
        ] {
            assert!(apply(terminal, AttemptEvent::StartRetry).is_err());
        }
    }

    #[test]
    fn serde_uses_the_lower_camel_case_wire_contract() {
        assert_eq!(
            serde_json::to_string(&AttemptState::RetryAvailable).expect("state should serialize"),
            r#""retryAvailable""#
        );
        assert_eq!(
            serde_json::to_string(&AttemptEvent::StartRetry).expect("event should serialize"),
            r#""startRetry""#
        );
    }
}
