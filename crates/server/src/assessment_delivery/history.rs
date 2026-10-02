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
            hint,
            worked_solution,
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
        // ASVS 8.2.3: Question Feedback follows its own rule, not answer disclosure.
        question.feedback.general_feedback = released_general_feedback(general_feedback);
        if decision.submitted_response
            && let Ok(presentation) =
                reproduce_selected_issued_presentation(presentation_evidence.clone())
        {
            project_response(question, response.clone(), &presentation);
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
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use learning_data_access::{
        CourseClassAnalysis, LiveAssessmentPreviousAttemptState,
        StudentAssessmentAttemptHistoryAssessment, StudentAssessmentAttemptHistoryCourse,
        StudentAssessmentAttemptHistoryQuestion,
    };
    use question_model::{
        AssessmentId, AssessmentType, CourseInstanceId, DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE,
        GradingResult, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber,
        StudentFeedback, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming, Theme,
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
                    theme: Theme::Forest,
                },
                assessment: StudentAssessmentAttemptHistoryAssessment {
                    id: AssessmentId::new("AABCDEFG8").expect("valid Assessment ID"),
                    assessment_type: AssessmentType::PracticeQuestionAssignment,
                    title: "Protein folding practice".to_string(),
                },
                state: LiveAssessmentPreviousAttemptState::Submitted,
                score: None,
                class_statistics: None,
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
                    hints: None,
                    worked_solution: None,
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
            course_class_analysis: None,
        }
    }

    fn analysis(cohort: u32, average: Option<f64>) -> CourseClassAnalysis {
        CourseClassAnalysis {
            completed_student_cohort_size: cohort,
            incomplete_scoring: false,
            recent_rescoring: false,
            assessment_average_score: average,
        }
    }

    fn class_statistics_field(
        evidence: &StudentAssessmentAttemptHistoryEvidence,
    ) -> Option<serde_json::Value> {
        serde_json::to_value(project_history(evidence))
            .expect("history serializes")
            .get("classStatistics")
            .cloned()
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

    #[test]
    fn course_class_statistics_stay_omitted_when_a_student_could_be_inferred() {
        let safe_cohort = DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE;
        let mut released = evidence();
        released.feedback_rule.class_statistics = StudentFeedbackReleaseTiming::AfterSubmit;
        released.course_class_analysis = Some(analysis(1, Some(0.8)));
        assert!(class_statistics_field(&released).is_none());

        released.course_class_analysis = Some(analysis(safe_cohort - 1, Some(0.8)));
        assert!(class_statistics_field(&released).is_none());

        released.course_class_analysis = Some(analysis(0, Some(0.0)));
        assert!(class_statistics_field(&released).is_none());

        released.course_class_analysis = None;
        assert!(class_statistics_field(&released).is_none());

        let mut incomplete = analysis(safe_cohort, Some(0.8));
        incomplete.incomplete_scoring = true;
        released.course_class_analysis = Some(incomplete);
        assert!(class_statistics_field(&released).is_none());

        let mut rescoring = analysis(safe_cohort, Some(0.8));
        rescoring.recent_rescoring = true;
        released.course_class_analysis = Some(rescoring);
        assert!(class_statistics_field(&released).is_none());

        released.course_class_analysis = Some(analysis(safe_cohort, Some(1.1)));
        assert!(class_statistics_field(&released).is_none());

        released.feedback_rule.class_statistics = StudentFeedbackReleaseTiming::Never;
        released.course_class_analysis = Some(analysis(safe_cohort, Some(0.8)));
        assert!(class_statistics_field(&released).is_none());

        released.feedback_rule.class_statistics = StudentFeedbackReleaseTiming::AfterSubmit;
        assert_eq!(
            class_statistics_field(&released).expect("safe cohort"),
            serde_json::json!({
                "state": "available",
                "completed_student_cohort_size": safe_cohort,
                "assessment_average_score": 0.8
            })
        );
    }

    #[test]
    fn question_feedback_is_shown_when_its_disclosure_rules_allow_it() {
        let mut question = evidence().history.questions.remove(0);
        question.feedback.general_feedback =
            released_general_feedback(Some("Keep the units.".to_owned()));
        let withheld = domain::student_feedback_release::StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: false,
            worked_solutions: false,
        };
        let answer =
            question_model::QuestionAnswer::new(vec![question_model::QuestionContentBlock::Text {
                markdown: "The hidden answer".to_owned(),
            }])
            .expect("one answer block");
        project_teaching_feedback(
            &mut question,
            withheld,
            Some(GradingResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 1.0,
            }),
            adapter_ple::question_json::PleQuestionJsonRecordedTeachingContent {
                question_feedback: Some(question_model::QuestionFeedback {
                    choice_feedback: Some(vec![question_model::QuestionContentBlock::Text {
                        markdown: "Backend choice note".to_owned(),
                    }]),
                    correct_feedback: None,
                    incorrect_feedback: None,
                }),
                question_answer: Some(answer),
                question_answer_explanation: None,
            },
        );

        assert_eq!(
            question.feedback.general_feedback,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Keep the units.".to_owned(),
            }])
        );
        assert!(question.feedback.question_answer.is_none());
        assert!(question.feedback.choice_feedback.is_some());
        assert!(released_general_feedback(None).is_none());
    }

    #[test]
    fn webwork_questions_keep_ple_managed_hints_and_worked_solutions() {
        let webwork_source = "BEGIN_HINT\nThe WeBWorK hint stays in the PG document.\nEND_HINT\nBEGIN_SOLUTION\nThe WeBWorK solution stays in the PG document.\nEND_SOLUTION\n";
        let released = domain::student_feedback_release::StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: true,
            worked_solutions: true,
        };
        let native_hint = vec![question_model::QuestionContentBlock::Text {
            markdown: "Native document hint.".to_owned(),
        }];
        let support = disclosed_revision_support(
            released,
            Some("PLE-managed membrane hint.".to_owned()),
            Some("PLE-managed membrane worked solution.".to_owned()),
            Some(&native_hint),
        );
        assert_eq!(
            support.hints,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane hint.".to_owned(),
            }])
        );
        assert_eq!(
            support.worked_solutions,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane worked solution.".to_owned(),
            }])
        );
        let rendered = format!("{support:?}");
        assert!(!rendered.contains(webwork_source));
        assert!(!rendered.contains("BEGIN_HINT"));
        assert!(!rendered.contains("BEGIN_SOLUTION"));
        assert!(!rendered.contains("Native document hint."));

        let hidden = disclosed_revision_support(
            domain::student_feedback_release::StudentFeedbackReleaseDecision {
                hints: false,
                worked_solutions: false,
                ..released
            },
            Some("PLE-managed membrane hint.".to_owned()),
            Some("PLE-managed membrane worked solution.".to_owned()),
            Some(&native_hint),
        );
        assert!(hidden.hints.is_none());
        assert!(hidden.worked_solutions.is_none());

        let native_only = disclosed_revision_support(released, None, None, Some(&native_hint));
        assert_eq!(native_only.hints, Some(native_hint));
        assert!(native_only.worked_solutions.is_none());
    }

    #[test]
    fn ple_managed_support_stays_separate_from_backend_interaction_feedback() {
        let mut question = evidence().history.questions.remove(0);
        let released = domain::student_feedback_release::StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: true,
            worked_solutions: true,
        };
        question.feedback.general_feedback =
            released_general_feedback(Some("Keep the units.".to_owned()));
        let support = disclosed_revision_support(
            released,
            Some("PLE-managed membrane hint.".to_owned()),
            Some("PLE-managed membrane worked solution.".to_owned()),
            None,
        );
        question.hints = support.hints;
        question.worked_solution = support.worked_solutions;
        project_teaching_feedback(
            &mut question,
            released,
            Some(GradingResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 1.0,
            }),
            adapter_ple::question_json::PleQuestionJsonRecordedTeachingContent {
                question_feedback: Some(question_model::QuestionFeedback {
                    choice_feedback: Some(vec![question_model::QuestionContentBlock::Text {
                        markdown: "Backend interaction choice note.".to_owned(),
                    }]),
                    correct_feedback: Some(vec![question_model::QuestionContentBlock::Text {
                        markdown: "Backend interaction correct note.".to_owned(),
                    }]),
                    incorrect_feedback: Some(vec![question_model::QuestionContentBlock::Text {
                        markdown: "Backend interaction incorrect note.".to_owned(),
                    }]),
                }),
                question_answer: None,
                question_answer_explanation: None,
            },
        );

        assert_eq!(
            question.feedback.general_feedback,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Keep the units.".to_owned(),
            }])
        );
        assert_eq!(
            question.hints,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane hint.".to_owned(),
            }])
        );
        assert_eq!(
            question.worked_solution,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane worked solution.".to_owned(),
            }])
        );
        assert_eq!(
            question.feedback.choice_feedback,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Backend interaction choice note.".to_owned(),
            }])
        );
        let ple_fields = format!(
            "{:?} {:?} {:?}",
            question.feedback.general_feedback, question.hints, question.worked_solution
        );
        assert!(!ple_fields.contains("Backend interaction"));
        let interaction = format!(
            "{:?} {:?} {:?}",
            question.feedback.choice_feedback,
            question.feedback.correct_feedback,
            question.feedback.incorrect_feedback
        );
        assert!(!interaction.contains("Keep the units."));
        assert!(!interaction.contains("PLE-managed membrane hint."));
        assert!(!interaction.contains("PLE-managed membrane worked solution."));
    }

    #[test]
    fn ple_managed_support_stays_separate_from_backend_generated_content() {
        let mut question = evidence().history.questions.remove(0);
        let released = domain::student_feedback_release::StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: true,
            question_answer_explanation: true,
            class_statistics: false,
            hints: true,
            worked_solutions: true,
        };
        question.feedback.general_feedback =
            released_general_feedback(Some("Keep the units.".to_owned()));
        let support = disclosed_revision_support(
            released,
            Some("PLE-managed membrane hint.".to_owned()),
            Some("PLE-managed membrane worked solution.".to_owned()),
            None,
        );
        question.hints = support.hints;
        question.worked_solution = support.worked_solutions;
        let answer =
            question_model::QuestionAnswer::new(vec![question_model::QuestionContentBlock::Text {
                markdown: "Backend generated answer content.".to_owned(),
            }])
            .expect("backend answer");
        let explanation = question_model::QuestionAnswerExplanation::new(vec![
            question_model::QuestionContentBlock::Text {
                markdown: "Backend generated explanation content.".to_owned(),
            },
        ])
        .expect("backend explanation");
        project_teaching_feedback(
            &mut question,
            released,
            Some(GradingResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            }),
            adapter_ple::question_json::PleQuestionJsonRecordedTeachingContent {
                question_feedback: None,
                question_answer: Some(answer),
                question_answer_explanation: Some(explanation),
            },
        );

        assert_eq!(
            question.feedback.general_feedback,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Keep the units.".to_owned(),
            }])
        );
        assert_eq!(
            question.hints,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane hint.".to_owned(),
            }])
        );
        assert_eq!(
            question.worked_solution,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "PLE-managed membrane worked solution.".to_owned(),
            }])
        );
        assert_eq!(
            question.feedback.question_answer,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Backend generated answer content.".to_owned(),
            }])
        );
        assert_eq!(
            question.feedback.question_answer_explanation,
            Some(vec![question_model::QuestionContentBlock::Text {
                markdown: "Backend generated explanation content.".to_owned(),
            }])
        );
        let ple_fields = format!(
            "{:?} {:?} {:?}",
            question.feedback.general_feedback, question.hints, question.worked_solution
        );
        assert!(!ple_fields.contains("Backend generated"));
        let backend_content = format!(
            "{:?} {:?}",
            question.feedback.question_answer, question.feedback.question_answer_explanation
        );
        assert!(!backend_content.contains("Keep the units."));
        assert!(!backend_content.contains("PLE-managed membrane hint."));
        assert!(!backend_content.contains("PLE-managed membrane worked solution."));
    }
}
