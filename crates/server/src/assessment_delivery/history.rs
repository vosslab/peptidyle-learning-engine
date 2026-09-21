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
    effective_assessment_properties::{
        AssessmentPolicySource, EffectiveAssessmentPolicy, EffectiveAssessmentPolicyValue,
    },
    student_feedback_release::{
        evaluate_allowed_student_feedback_release, gate_quiz_exam_answers_for_current_cohort,
        project_student_feedback, score_current_student_feedback_release,
    },
};
use learning_data_access::{
    LiveAssessmentAttemptScore, LiveAssessmentDeliveryStore, StoreError,
    StudentAssessmentAttemptHistory, StudentAssessmentAttemptHistoryEvidence,
};
use question_model::{AssessmentAttemptId, QuestionFeedback, StudentFeedback};
use question_model::{AssessmentScoringState, LateWorkRule, Timestamp};

use super::{
    StateData, concealed, reproduce_selected_issued_presentation, resolve_source, store_error,
    student,
};

pub(super) async fn student_history(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assessment_attempt): Path<String>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptId::from_str(&assessment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assessment_attempt_history(token, assessment_attempt)
        .await
    {
        Ok(value) => {
            let decision = history_decision(&value);
            let mut history = project_history(&value);
            if let Err(value) = project_released_content(
                &state,
                token,
                assessment_attempt,
                &value,
                decision,
                &mut history,
            )
            .await
            {
                return store_error(value);
            }
            crate::auth::no_store(Json(history).into_response())
        }
        Err(value) => store_error(value),
    }
}

fn project_history(
    evidence: &StudentAssessmentAttemptHistoryEvidence,
) -> StudentAssessmentAttemptHistory {
    let decision = history_decision(evidence);
    let mut history = evidence.history.clone();
    if !evidence.grading_is_current {
        return history;
    }
    // ASVS 8.2.3: independent field gates are evaluated only after the Store
    // proves the complete immutable grading chain for this exact Attempt.
    let decision =
        score_current_student_feedback_release(decision, AssessmentScoringState::Current);
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
        history.score = Some(LiveAssessmentAttemptScore {
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

pub(crate) fn history_decision(
    evidence: &StudentAssessmentAttemptHistoryEvidence,
) -> domain::student_feedback_release::StudentFeedbackReleaseDecision {
    let decision = evaluate_allowed_student_feedback_release(
        &history_policy(evidence.due_at, evidence.closes_at),
        evidence.feedback_rule,
        evidence.evaluated_at,
        evidence.submitted_at,
    );
    // ASVS 8.2.3 and 8.3.1: Quiz and Exam answer fields require the
    // database-authorized current-Course cohort decision; browser state cannot
    // weaken this field-level gate.
    gate_quiz_exam_answers_for_current_cohort(
        decision,
        evidence.assessment_type,
        evidence.all_students_completed,
    )
}

async fn project_released_content(
    state: &StateData,
    token: learning_data_access::SessionTokenHash,
    assessment_attempt: AssessmentAttemptId,
    evidence: &StudentAssessmentAttemptHistoryEvidence,
    decision: domain::student_feedback_release::StudentFeedbackReleaseDecision,
    history: &mut StudentAssessmentAttemptHistory,
) -> Result<(), StoreError> {
    let sources = state
        .delivery
        .student_assessment_attempt_history_response_sources(token, assessment_attempt)
        .await?;
    for source in sources {
        // ASVS 8.2.3: each protected teaching field is assigned only after
        // the server-owned disclosure decision and exact source read succeed.
        let learning_data_access::StudentAssessmentAttemptHistoryResponseSource {
            position,
            response,
            general_feedback,
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
        if decision.question_answer
            && matches!(
                presentation_source,
                Some(
                    learning_data_access::StudentAssessmentAttemptPresentationSource::Webwork { .. }
                )
            )
        {
            question.backend_answer_review =
                Some(learning_data_access::BackendAnswerReviewAvailability::Available);
        }
        // General feedback is exact PLE-authored Revision metadata. It is
        // shown when provided and has no separate delayed-release policy.
        question.feedback.general_feedback = general_feedback
            .map(|markdown| vec![question_model::QuestionContentBlock::Text { markdown }]);
        if decision.submitted_response
            && let Ok(presentation) =
                reproduce_selected_issued_presentation(presentation_evidence.clone())
        {
            project_response(question, response.clone(), &presentation);
        }
        if let Some(learning_data_access::StudentAssessmentAttemptPresentationSource::Ple {
            source: ple_source,
            ..
        }) = presentation_source
        {
            let Ok(resolved) = resolve_source(&state.objects, &ple_source).await else {
                continue;
            };
            let teaching = adapter_ple::PleQuestionBackend::new()
                .project_recorded_question_json_teaching_content(
                    &resolved,
                    response.as_ref(),
                    recorded_result,
                );
            let Ok(teaching) = teaching else {
                continue;
            };
            project_teaching_feedback(question, decision, recorded_result, teaching);
        }
    }
    Ok(())
}

fn project_teaching_feedback(
    question: &mut learning_data_access::StudentAssessmentAttemptHistoryQuestion,
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
    question: &mut learning_data_access::StudentAssessmentAttemptHistoryQuestion,
    response: Option<question_model::StudentResponse>,
    presentation: &question_model::presentation::IssuedQuestionPresentation,
) {
    let Some(response) = response else {
        return;
    };
    let Ok(response) =
        question_model::presentation::project_durable_response_to_presentation_response_item_ids(
            &response,
            presentation,
        )
    else {
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
) -> EffectiveAssessmentPolicy {
    EffectiveAssessmentPolicy {
        available_at: base(None),
        due_at: base(due_at),
        closes_at: base(closes_at),
        assessment_attempt_time_limit_seconds: base(None::<NonZeroU32>),
        attempt_limit: base(None::<NonZeroU32>),
        late_work_rule: base(LateWorkRule::Accept),
    }
}

fn base<T>(value: T) -> EffectiveAssessmentPolicyValue<T> {
    EffectiveAssessmentPolicyValue {
        value,
        source: AssessmentPolicySource::Base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use learning_data_access::{
        LiveAssessmentPreviousAttemptState, StudentAssessmentAttemptHistoryAssessment,
        StudentAssessmentAttemptHistoryCourse, StudentAssessmentAttemptHistoryQuestion,
    };
    use question_model::{
        AssessmentId, AssessmentType, CourseInstanceId, CourseTheme, GradingResult,
        PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber,
        StudentFeedback, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
    };

    fn evidence() -> StudentAssessmentAttemptHistoryEvidence {
        StudentAssessmentAttemptHistoryEvidence {
            history: StudentAssessmentAttemptHistory {
                assessment_attempt_id: AssessmentAttemptId::from_uuid(uuid::Uuid::from_u128(12)),
                attempt_number: 2,
                course: StudentAssessmentAttemptHistoryCourse {
                    id: CourseInstanceId::new("CIABCDEFGS").expect("valid Course Instance ID"),
                    short_name: "Mol Bio".to_string(),
                    long_name: "Molecular biology".to_string(),
                    theme: CourseTheme::Forest,
                },
                assessment: StudentAssessmentAttemptHistoryAssessment {
                    id: AssessmentId::new("AABCDEFG8").expect("valid Assessment ID"),
                    title: "Protein folding practice".to_string(),
                },
                state: LiveAssessmentPreviousAttemptState::Submitted,
                score: None,
                questions: vec![StudentAssessmentAttemptHistoryQuestion {
                    position: 1,
                    published_question_revision_tuple: PublishedQuestionRevisionTuple {
                        published_question_id: PublishedQuestionId::from_random_identifier(
                            "ABCDEF1",
                        )
                        .expect("Question ID"),
                        revision_number: QuestionRevisionNumber::new(3)
                            .expect("Question Revision Number"),
                    },
                    response_state: LiveAssessmentPreviousAttemptState::Submitted,
                    response: None,
                    backend_answer_review: None,
                    feedback: StudentFeedback::empty(),
                }],
            },
            assessment_type: AssessmentType::RegularAssignment,
            feedback_rule: StudentFeedbackReleaseRule::default(),
            due_at: None,
            closes_at: None,
            submitted_at: Some(Timestamp::from_unix_millis(1)),
            evaluated_at: Timestamp::from_unix_millis(2),
            all_students_completed: true,
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
    }

    #[test]
    fn quiz_answer_release_uses_the_current_course_cohort_decision() {
        let mut evidence = evidence();
        evidence.assessment_type = AssessmentType::Quiz;
        evidence.all_students_completed = false;
        evidence.feedback_rule.question_answer = StudentFeedbackReleaseTiming::AfterSubmit;
        evidence.feedback_rule.question_answer_explanation =
            StudentFeedbackReleaseTiming::AfterSubmit;

        let waiting = history_decision(&evidence);

        assert!(!waiting.question_answer);
        assert!(!waiting.question_answer_explanation);

        evidence.all_students_completed = true;
        let released = history_decision(&evidence);
        assert!(released.question_answer);
        assert!(released.question_answer_explanation);
    }

    #[test]
    fn history_wire_keeps_the_exact_issued_question_revision() {
        let evidence = evidence();
        let expected = evidence.history.questions[0]
            .published_question_revision_tuple
            .clone();
        let wire = serde_json::to_value(project_history(&evidence)).expect("history serializes");

        assert_eq!(
            wire["questions"][0]["publishedQuestionRevisionTuple"]["publishedQuestionId"],
            expected.published_question_id.to_string()
        );
        assert_eq!(
            wire["questions"][0]["publishedQuestionRevisionTuple"]["revisionNumber"],
            expected.revision_number.get()
        );
        assert!(wire["questions"][0].get("backendAnswerReview").is_none());
        let mut history = project_history(&evidence);
        history.questions[0].backend_answer_review =
            Some(learning_data_access::BackendAnswerReviewAvailability::Available);
        let wire = serde_json::to_value(history).unwrap();
        assert_eq!(wire["questions"][0]["backendAnswerReview"], "available");
    }
}
