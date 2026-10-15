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
        apply_all_students_completed_timing, evaluate_allowed_student_feedback_release,
        project_disclosed_support, project_student_feedback,
        score_current_student_feedback_release,
    },
};
use learning_data_access::{
    LiveAssessmentAttemptScore, StoreError, StudentAssessmentAttemptHistory,
    StudentAssessmentAttemptHistoryEvidence,
};
use question_model::{AssessmentAttemptId, ClassStatistics, QuestionFeedback, StudentFeedback};
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
    // ASVS 14.2.6 and 8.2.3: a course average is absent unless this policy
    // releases it and the cohort cannot identify one Student.
    history.class_statistics =
        disclose_course_class_statistics(decision.class_statistics, evidence.course_class_analysis);
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
    // ASVS 2.2.2 and 2.3.1: only an explicitly configured all-Students timing
    // uses this database-authorized current-Course completion decision.
    apply_all_students_completed_timing(
        decision,
        evidence.feedback_rule,
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
    let mut disclosed = Vec::new();
    for source in sources {
        // ASVS 8.2.3: each protected teaching field is assigned only after
        // the server-owned disclosure decision and exact source read succeed.
        let learning_data_access::StudentAssessmentAttemptHistoryResponseSource {
            position,
            response,
            general_feedback,
            hint,
            worked_solution,
            presentation_evidence,
            presentation_source,
            ..
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
        let revision = presentation_evidence
            .published_question_revision_tuple
            .clone();
        let presentation =
            reproduce_selected_issued_presentation(presentation_evidence.clone()).ok();
        match presentation_source.as_ref() {
            Some(learning_data_access::StudentAssessmentAttemptPresentationSource::Ple {
                ..
            }) => {
                // The retained native presentation contains the prompt and its
                // original option arrangement. It is Student Work, not an
                // answer or feedback disclosure.
                question.presentation = presentation
                    .as_ref()
                    .map(|value| value.presentation.clone());
            }
            Some(learning_data_access::StudentAssessmentAttemptPresentationSource::Webwork {
                ..
            }) => {
                // The WeBWorK renderer owns its own read-only review document.
                // Availability is independent of correct-answer disclosure.
                question.backend_review =
                    Some(learning_data_access::BackendReviewAvailability::Available);
            }
            None => {}
        }
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
        // ASVS 8.2.3: Question Feedback follows its own rule, not answer disclosure.
        question.feedback.general_feedback = released_general_feedback(general_feedback);
        if decision.submitted_response
            && let Some(presentation) = presentation.as_ref()
        {
            project_response(question, response.clone(), presentation);
        }
        let mut native_hint = None;
        let ple_resolved = match presentation_source.as_ref() {
            Some(learning_data_access::StudentAssessmentAttemptPresentationSource::Ple {
                source: ple_source,
                ..
            }) => match resolve_source(&state.objects, ple_source).await {
                Ok(resolved) => {
                    native_hint = resolved
                        .question_hint()
                        .map(|question_hint| question_hint.content().to_vec());
                    Some(resolved)
                }
                Err(_) => None,
            },
            _ => None,
        };
        // ASVS 8.2.3: Hints and Worked Solutions follow their own timings.
        // ASVS 14.2.6: the WeBWorK document is not copied into these fields.
        let support =
            disclosed_revision_support(decision, hint, worked_solution, native_hint.as_deref());
        question.hints = support.hints;
        question.worked_solution = support.worked_solutions;
        if let Some(resolved) = ple_resolved {
            let teaching = adapter_ple::PleQuestionBackend::new()
                .project_recorded_question_json_teaching_content(
                    &resolved,
                    response.as_ref(),
                    recorded_result,
                );
            if let Ok(teaching) = teaching {
                project_teaching_feedback(question, decision, recorded_result, teaching);
            }
        }
        // ASVS 8.2.3/8.3.1: grant image access only for the support fields
        // actually returned after the existing domain disclosure projection.
        let images = super::support_images::disclosed_images(question);
        if !images.is_empty() {
            disclosed.push(
                learning_data_access::StudentAssessmentAttemptSupportImages {
                    position,
                    revision,
                    images,
                },
            );
        }
    }
    state
        .delivery
        .register_student_assessment_attempt_support_images(token, assessment_attempt, &disclosed)
        .await?;
    Ok(())
}

/// Shows PLE-managed Question Feedback when the Revision provides it.
///
/// Assessment correct-answer disclosure does not apply. Absent feedback stays
/// absent rather than becoming an empty block.
fn released_general_feedback(
    general_feedback: Option<String>,
) -> Option<Vec<question_model::QuestionContentBlock>> {
    general_feedback.map(|markdown| vec![question_model::QuestionContentBlock::Text { markdown }])
}

/// Projects PLE-managed Hint and Worked Solution text stored on the Question
/// Revision. A native Question Hint is used only when the Revision Hint is
/// absent. WeBWorK source text is not a parameter and is never substituted.
///
/// ASVS 8.2.3: each field appears only when its own timing allows it.
/// ASVS 14.2.6: withheld text is omitted rather than copied from another source.
fn disclosed_revision_support(
    decision: domain::student_feedback_release::StudentFeedbackReleaseDecision,
    revision_hint: Option<String>,
    revision_worked_solution: Option<String>,
    native_hint: Option<&[question_model::QuestionContentBlock]>,
) -> domain::student_feedback_release::DisclosedSupportContent {
    let hint_blocks = match revision_hint {
        Some(markdown) => Some(support_text_blocks(&markdown)),
        None => native_hint.map(|blocks| blocks.to_vec()),
    };
    let worked_blocks = revision_worked_solution.as_deref().map(support_text_blocks);
    project_disclosed_support(decision, hint_blocks.as_deref(), worked_blocks.as_deref())
}

fn support_text_blocks(markdown: &str) -> Vec<question_model::QuestionContentBlock> {
    vec![question_model::QuestionContentBlock::Text {
        markdown: markdown.to_owned(),
    }]
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

/// Omits course-specific analysis when a Student could be inferred from it.
///
/// ASVS 14.2.6: the response keeps the minimum sensitive aggregate. ASVS 8.2.3:
/// a small cohort, a missing report, incomplete scoring, and a policy that has
/// not released class statistics are all an absent field. An unavailable
/// object is never returned.
fn disclose_course_class_statistics(
    policy_releases_class_statistics: bool,
    analysis: Option<learning_data_access::CourseClassAnalysis>,
) -> Option<ClassStatistics> {
    if !policy_releases_class_statistics {
        return None;
    }
    let analysis = analysis?;
    match ClassStatistics::from_current_analysis(
        analysis.completed_student_cohort_size,
        analysis.incomplete_scoring,
        analysis.recent_rescoring,
        analysis.assessment_average_score,
    ) {
        available @ ClassStatistics::Available { .. } => Some(available),
        ClassStatistics::Unavailable => None,
    }
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
