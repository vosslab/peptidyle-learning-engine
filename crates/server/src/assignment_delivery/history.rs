//! Student-owned selected completed-Attempt history route.

use std::num::NonZeroU32;
use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use domain::{
    effective_assignment_policy::{
        AssignmentPolicySource, EffectiveAssignmentPolicy, EffectiveAssignmentPolicyValue,
    },
    student_feedback_release::{
        evaluate_allowed_student_feedback_release, project_student_feedback,
        score_current_student_feedback_release,
    },
};
use learning_data_access::{
    LiveAssignmentAttemptScore, LiveAssignmentDeliveryStore, StudentAssignmentAttemptHistory,
    StudentAssignmentAttemptHistoryEvidence,
};
use question_model::{AssignmentAttemptReference, QuestionFeedback, StudentFeedback};
use question_model::{AssignmentScoringState, LateWorkRule, Timestamp};

use super::{
    StateData, concealed, reproduce_selected_issued_presentation, resolve_source, store_error,
    student,
};

pub(super) async fn student_history(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assignment_attempt): Path<String>,
) -> Response {
    let assignment_attempt = match AssignmentAttemptReference::from_str(&assignment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assignment_attempt_history(token, assignment_attempt)
        .await
    {
        Ok(value) => {
            let decision = history_decision(&value);
            let mut history = project_history(&value);
            if needs_released_content(decision) {
                project_released_content(
                    &state,
                    token,
                    assignment_attempt,
                    &value,
                    decision,
                    &mut history,
                )
                .await;
            }
            crate::auth::no_store(Json(history).into_response())
        }
        Err(value) => store_error(value),
    }
}

fn needs_released_content(
    decision: domain::student_feedback_release::StudentFeedbackReleaseDecision,
) -> bool {
    decision.submitted_response
        || decision.question_feedback
        || decision.question_answer
        || decision.question_answer_explanation
}

fn project_history(
    evidence: &StudentAssignmentAttemptHistoryEvidence,
) -> StudentAssignmentAttemptHistory {
    let decision = history_decision(evidence);
    let mut history = evidence.history.clone();
    if !evidence.grading_is_current {
        return history;
    }
    // ASVS 8.2.3: independent field gates are evaluated only after the Store
    // proves the complete immutable grading chain for this exact Attempt.
    let decision =
        score_current_student_feedback_release(decision, AssignmentScoringState::Current);
    if decision.score {
        let points_earned = evidence
            .grading_results
            .iter()
            .flatten()
            .map(|result| result.points_earned)
            .sum();
        let points_possible = evidence
            .grading_results
            .iter()
            .flatten()
            .map(|result| result.points_possible)
            .sum();
        history.score = Some(LiveAssignmentAttemptScore {
            points_earned,
            points_possible,
        });
    }
    for (question, result) in history
        .questions
        .iter_mut()
        .zip(evidence.grading_results.iter().cloned())
    {
        question.feedback =
            project_student_feedback(decision, result, &QuestionFeedback::default(), None, None)
                .unwrap_or_else(StudentFeedback::empty);
    }
    history
}

fn history_decision(
    evidence: &StudentAssignmentAttemptHistoryEvidence,
) -> domain::student_feedback_release::StudentFeedbackReleaseDecision {
    evaluate_allowed_student_feedback_release(
        &history_policy(evidence.due_at, evidence.closes_at),
        evidence.feedback_rule,
        evidence.evaluated_at,
        evidence.submitted_at,
    )
}

async fn project_released_content(
    state: &StateData,
    token: learning_data_access::SessionTokenHash,
    assignment_attempt: AssignmentAttemptReference,
    evidence: &StudentAssignmentAttemptHistoryEvidence,
    decision: domain::student_feedback_release::StudentFeedbackReleaseDecision,
    history: &mut StudentAssignmentAttemptHistory,
) {
    let sources = match state
        .delivery
        .student_assignment_attempt_history_response_sources(token, assignment_attempt)
        .await
    {
        Ok(sources) => sources,
        Err(_) => return,
    };
    for source in sources {
        // ASVS 8.2.3: each protected teaching field is assigned only after
        // the server-owned disclosure decision and exact source read succeed.
        let learning_data_access::StudentAssignmentAttemptHistoryResponseSource {
            position,
            response,
            presentation_evidence,
            presentation_source,
        } = source;
        let Some(question_index) = history
            .questions
            .iter()
            .position(|question| question.position == position)
        else {
            continue;
        };
        let recorded_result = evidence
            .grading_results
            .get(question_index)
            .cloned()
            .flatten();
        let question = &mut history.questions[question_index];
        if decision.submitted_response
            && let Ok(presentation) =
                reproduce_selected_issued_presentation(presentation_evidence.clone())
        {
            project_response(question, response.clone(), &presentation);
        }
        match presentation_source {
            Some(learning_data_access::StudentAssignmentAttemptPresentationSource::Ple {
                source: ple_source,
                ..
            }) if decision.question_feedback
                || decision.question_answer
                || decision.question_answer_explanation =>
            {
                let Ok(resolved) = resolve_source(&state.objects, &ple_source).await else {
                    continue;
                };
                let teaching = adapter_ple::PleQuestionBackend::new()
                    .project_recorded_question_json_teaching_content(
                        &resolved,
                        if decision.question_feedback {
                            response.as_ref()
                        } else {
                            None
                        },
                        recorded_result,
                    );
                let Ok(teaching) = teaching else {
                    continue;
                };
                project_teaching_feedback(question, decision, recorded_result, teaching);
            }
            _ => {}
        }
    }
}

fn project_teaching_feedback(
    question: &mut learning_data_access::StudentAssignmentAttemptHistoryQuestion,
    decision: domain::student_feedback_release::StudentFeedbackReleaseDecision,
    recorded_result: Option<question_model::GradingResult>,
    teaching: adapter_ple::question_json::PleQuestionJsonRecordedTeachingContent,
) {
    let question_feedback = teaching.question_feedback.unwrap_or_default();
    let Some(feedback) = project_student_feedback(
        decision,
        recorded_result,
        &question_feedback,
        teaching.question_answer.as_ref(),
        teaching.question_answer_explanation.as_ref(),
    ) else {
        return;
    };
    question.feedback.choice_feedback = feedback.choice_feedback;
    question.feedback.correct_feedback = feedback.correct_feedback;
    question.feedback.incorrect_feedback = feedback.incorrect_feedback;
    question.feedback.question_answer = feedback.question_answer;
    question.feedback.question_answer_explanation = feedback.question_answer_explanation;
}

fn project_response(
    question: &mut learning_data_access::StudentAssignmentAttemptHistoryQuestion,
    response: Option<question_model::StudentResponse>,
    presentation: &question_model::presentation::IssuedQuestionPresentation,
) {
    let Some(response) = response else {
        return;
    };
    let Ok(response) = question_model::presentation::project_durable_response_to_presentation_response_item_references(
        &response,
        presentation,
    ) else {
        return;
    };
    let Some(response) =
        super::history_response::project(response, &presentation.presentation.response)
    else {
        return;
    };
    question.response = Some(response);
}

fn history_policy(
    due_at: Option<Timestamp>,
    closes_at: Option<Timestamp>,
) -> EffectiveAssignmentPolicy {
    EffectiveAssignmentPolicy {
        available_at: base(None),
        due_at: base(due_at),
        closes_at: base(closes_at),
        assignment_attempt_time_limit_seconds: base(None::<NonZeroU32>),
        attempt_limit: base(None::<NonZeroU32>),
        late_work_rule: base(LateWorkRule::Accept),
    }
}

fn base<T>(value: T) -> EffectiveAssignmentPolicyValue<T> {
    EffectiveAssignmentPolicyValue {
        value,
        source: AssignmentPolicySource::Base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use learning_data_access::{
        LiveAssignmentPreviousAttemptState, StudentAssignmentAttemptHistoryAssignment,
        StudentAssignmentAttemptHistoryCourse, StudentAssignmentAttemptHistoryQuestion,
    };
    use question_model::{
        AssignmentReference, CourseInstanceReference, CourseTheme, GradingResult, QuestionId,
        QuestionRevisionNumber, QuestionRevisionReference, StudentFeedback,
        StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
    };

    fn evidence() -> StudentAssignmentAttemptHistoryEvidence {
        StudentAssignmentAttemptHistoryEvidence {
            history: StudentAssignmentAttemptHistory {
                assignment_attempt: AssignmentAttemptReference::new(12).expect("valid reference"),
                attempt_number: 2,
                course: StudentAssignmentAttemptHistoryCourse {
                    reference: CourseInstanceReference::new(3).expect("valid reference"),
                    short_name: "Mol Bio".to_string(),
                    long_name: "Molecular biology".to_string(),
                    theme: CourseTheme::Forest,
                },
                assignment: StudentAssignmentAttemptHistoryAssignment {
                    reference: AssignmentReference::new(7).expect("valid reference"),
                    title: "Protein folding practice".to_string(),
                },
                state: LiveAssignmentPreviousAttemptState::Submitted,
                score: None,
                questions: vec![StudentAssignmentAttemptHistoryQuestion {
                    position: 1,
                    question_revision: QuestionRevisionReference {
                        question_id: QuestionId::from_canonical_parts("ABCDEF", '1')
                            .expect("Question ID"),
                        revision_number: QuestionRevisionNumber::new(3)
                            .expect("Question Revision Number"),
                    },
                    response_state: LiveAssignmentPreviousAttemptState::Submitted,
                    response: None,
                    feedback: StudentFeedback::empty(),
                }],
            },
            feedback_rule: StudentFeedbackReleaseRule::default(),
            due_at: None,
            closes_at: None,
            submitted_at: Some(Timestamp::from_unix_millis(1)),
            evaluated_at: Timestamp::from_unix_millis(2),
            grading_is_current: true,
            grading_results: vec![Some(GradingResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 2.0,
            })],
        }
    }

    #[test]
    fn current_grading_preserves_a_genuine_zero_score() {
        let history = project_history(&evidence());

        assert_eq!(history.score.expect("disclosed score").points_earned, 0.0);
        assert_eq!(history.questions[0].feedback.correctness, Some(false));
    }

    #[test]
    fn score_and_correctness_remain_independent() {
        let mut evidence = evidence();
        evidence.feedback_rule.score = StudentFeedbackReleaseTiming::Never;
        let history = project_history(&evidence);

        assert!(history.score.is_none());
        assert_eq!(history.questions[0].feedback.correctness, Some(false));
    }

    #[test]
    fn submitted_response_release_does_not_wait_for_current_grading() {
        let mut evidence = evidence();
        evidence.grading_is_current = false;
        evidence.feedback_rule.submitted_response = StudentFeedbackReleaseTiming::AfterSubmit;

        assert!(history_decision(&evidence).submitted_response);
        assert!(project_history(&evidence).score.is_none());
    }

    #[test]
    fn answer_explanation_release_reads_retained_exact_source() {
        let mut evidence = evidence();
        evidence.feedback_rule.question_answer_explanation =
            StudentFeedbackReleaseTiming::AfterSubmit;

        let decision = history_decision(&evidence);

        assert!(decision.question_answer_explanation);
        assert!(needs_released_content(decision));
    }

    #[test]
    fn history_wire_keeps_the_exact_issued_question_revision() {
        let wire = serde_json::to_value(project_history(&evidence())).expect("history serializes");

        assert_eq!(
            wire["questions"][0]["questionRevision"]["questionId"],
            "ABC-DEF1"
        );
        assert_eq!(
            wire["questions"][0]["questionRevision"]["revisionNumber"],
            3
        );
    }
}
