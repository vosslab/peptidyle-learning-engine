//! Pure Student Feedback Release evaluation.
//!
//! The assessment-policy resolver has already resolved the Student's effective
//! assessment window and access verdict. This module only
//! consumes that verdict and an authoritative supplied timestamp. It neither
//! reconstructs access decisions nor records a Student Feedback Release receipt.

use question_model::{
    AssessmentScoringState, GradingResult, QuestionAnswer, QuestionAnswerExplanation,
    QuestionContentBlock, QuestionFeedback, StudentFeedback, StudentFeedbackReleaseRule,
    StudentFeedbackReleaseTiming, StudentResponseInspectionFeedback, Timestamp,
};

use crate::effective_assessment_properties::{AssessmentAccessDecision, EffectiveAssessmentPolicy};

/// The independently evaluated Student Feedback Release fields.
///
/// A caller uses these booleans to omit protected fields from Student Feedback;
/// this type contains no protected content itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudentFeedbackReleaseDecision {
    pub score: bool,
    pub per_item_correctness: bool,
    pub submitted_response: bool,
    pub question_answer: bool,
    pub question_answer_explanation: bool,
    pub class_statistics: bool,
    /// Hints use this boolean, not question-answer disclosure.
    pub hints: bool,
    /// Worked Solutions use this boolean, not hint or question-answer disclosure.
    pub worked_solutions: bool,
}

/// Hints and Worked Solutions after their own disclosure timings are applied.
///
/// Absent content stays absent. Neither field is Question Feedback or a Question Answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisclosedSupportContent {
    pub hints: Option<Vec<QuestionContentBlock>>,
    pub worked_solutions: Option<Vec<QuestionContentBlock>>,
}

/// Applies the configured all-current-Students completion timing.
///
/// The caller supplies the completion decision from a trusted persistence
/// boundary. Only fields configured for that timing wait for the cohort, so an
/// Instructor can choose another timing for a Quiz or Exam. ASVS 2.2.2, 2.3.1.
pub fn apply_all_students_completed_timing(
    mut decision: StudentFeedbackReleaseDecision,
    rule: StudentFeedbackReleaseRule,
    all_current_students_completed: bool,
) -> StudentFeedbackReleaseDecision {
    if matches!(
        rule.question_answer,
        StudentFeedbackReleaseTiming::AfterAllStudentsComplete
    ) {
        decision.question_answer = all_current_students_completed;
    }
    if matches!(
        rule.question_answer_explanation,
        StudentFeedbackReleaseTiming::AfterAllStudentsComplete
    ) {
        decision.question_answer_explanation = all_current_students_completed;
    }
    decision
}

/// Removes score-dependent feedback fields while an assessment score is not current.
///
/// A recalculating or failed aggregate must not expose an older numeric score or
/// correctness verdict beside its current status.
pub fn score_current_student_feedback_release(
    mut decision: StudentFeedbackReleaseDecision,
    assessment_scoring_state: AssessmentScoringState,
) -> StudentFeedbackReleaseDecision {
    if !matches!(assessment_scoring_state, AssessmentScoringState::Current) {
        decision.score = false;
        decision.per_item_correctness = false;
    }
    decision
}

/// Projects Question-provided feedback plus the independently disclosed fields.
///
/// `None` means no feedback field is currently visible. Hidden fields are
/// omitted rather than represented by null or a protected-content marker.
pub fn project_student_feedback(
    decision: StudentFeedbackReleaseDecision,
    result: Option<GradingResult>,
    question_feedback: &QuestionFeedback,
    question_answer: Option<&QuestionAnswer>,
    question_answer_explanation: Option<&QuestionAnswerExplanation>,
) -> Option<StudentFeedback> {
    let mut disclosed = StudentFeedback::empty();
    if let Some(result) = result {
        if decision.per_item_correctness {
            disclosed.correctness = Some(result.correct);
        }
        if decision.score {
            disclosed.points_earned = Some(result.points_earned);
            disclosed.points_possible = Some(result.points_possible);
        }
    }
    disclosed.choice_feedback = question_feedback.choice_feedback.clone();
    disclosed.correct_feedback = question_feedback.correct_feedback.clone();
    disclosed.incorrect_feedback = question_feedback.incorrect_feedback.clone();
    if decision.question_answer {
        disclosed.question_answer = question_answer.map(|answer| answer.content().to_vec());
    }
    if decision.question_answer_explanation {
        disclosed.question_answer_explanation =
            question_answer_explanation.map(|explanation| explanation.content().to_vec());
    }
    (decision.per_item_correctness
        || decision.score
        || disclosed.choice_feedback.is_some()
        || disclosed.correct_feedback.is_some()
        || disclosed.incorrect_feedback.is_some()
        || decision.question_answer
        || decision.question_answer_explanation)
        .then_some(disclosed)
}

/// Projects permitted correctness and score fields for one Student Response Inspection.
///
/// Inspection can show only the current score and correctness permitted by
/// assessment disclosure. Question Hint, Question Answer, and Question Answer
/// Explanation have no representation in this detail capability.
pub fn project_student_response_inspection_feedback(
    decision: StudentFeedbackReleaseDecision,
    assessment_scoring_state: AssessmentScoringState,
    result: Option<GradingResult>,
) -> StudentResponseInspectionFeedback {
    let decision = score_current_student_feedback_release(decision, assessment_scoring_state);
    let mut inspection_feedback = StudentResponseInspectionFeedback::empty();
    if let Some(result) = result {
        if decision.per_item_correctness {
            inspection_feedback.correctness = Some(result.correct);
        }
        if decision.score {
            inspection_feedback.points_earned = Some(result.points_earned);
            inspection_feedback.points_possible = Some(result.points_possible);
        }
    }
    inspection_feedback
}

/// Evaluates Student Feedback Release from one already-resolved assessment policy verdict.
///
/// A denied assessment-policy verdict produces no Student decision. For allowed verdicts,
/// every field is evaluated independently using the supplied server timestamp
/// and the resolved due/close times, if present. A submission timestamp is
/// evidence that the current Student submitted; it is not read from storage.
pub fn evaluate_student_feedback_release(
    rule: StudentFeedbackReleaseRule,
    effective_policy: &AssessmentAccessDecision,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
) -> Option<StudentFeedbackReleaseDecision> {
    let AssessmentAccessDecision::Allowed { policy, .. } = effective_policy else {
        return None;
    };

    Some(evaluate_allowed_student_feedback_release(
        policy,
        rule,
        now,
        submitted_at,
    ))
}

/// Evaluates Student Feedback Release from an already-authorized effective assessment policy.
///
/// Store receipt projections may retain the resolved policy without its
/// Assessment Access decision. Callers that still have the decision should prefer
/// [`evaluate_student_feedback_release`]; this helper does not authorize access.
pub fn evaluate_allowed_student_feedback_release(
    policy: &EffectiveAssessmentPolicy,
    rule: StudentFeedbackReleaseRule,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
) -> StudentFeedbackReleaseDecision {
    StudentFeedbackReleaseDecision {
        // Automated scores have no Instructor timing control. A submitted,
        // current grading result is disclosed by the caller.
        score: submitted_at.is_some(),
        // Completed review always includes the Student's own recorded work and
        // correctness. Correct answers and all optional teaching content retain
        // their independent Instructor-controlled disclosure timings.
        per_item_correctness: submitted_at.is_some(),
        submitted_response: submitted_at.is_some(),
        question_answer: timing_released(
            rule.question_answer,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        question_answer_explanation: timing_released(
            rule.question_answer_explanation,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        class_statistics: timing_released(
            rule.class_statistics,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        hints: timing_released(
            rule.hints,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
        worked_solutions: timing_released(
            rule.worked_solutions,
            now,
            submitted_at,
            policy.due_at.value,
            policy.closes_at.value,
        ),
    }
}

/// Shows Hints and Worked Solutions only when each field's own timing allows it.
///
/// ASVS 8.2.3: a released Question Answer does not reveal either field, and
/// one field's timing does not reveal the other. Empty content stays absent.
pub fn project_disclosed_support(
    decision: StudentFeedbackReleaseDecision,
    hints: Option<&[QuestionContentBlock]>,
    worked_solutions: Option<&[QuestionContentBlock]>,
) -> DisclosedSupportContent {
    DisclosedSupportContent {
        hints: disclosed_blocks(decision.hints, hints),
        worked_solutions: disclosed_blocks(decision.worked_solutions, worked_solutions),
    }
}

fn disclosed_blocks(
    released: bool,
    blocks: Option<&[QuestionContentBlock]>,
) -> Option<Vec<QuestionContentBlock>> {
    if !released {
        return None;
    }
    blocks.and_then(|blocks| (!blocks.is_empty()).then(|| blocks.to_vec()))
}

fn timing_released(
    timing: StudentFeedbackReleaseTiming,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
    due_at: Option<Timestamp>,
    closes_at: Option<Timestamp>,
) -> bool {
    match timing {
        StudentFeedbackReleaseTiming::DuringAttempt => true,
        StudentFeedbackReleaseTiming::AfterSubmit => submitted_at.is_some(),
        StudentFeedbackReleaseTiming::AfterDue => due_at.is_some_and(|due_at| now >= due_at),
        StudentFeedbackReleaseTiming::AfterClose => {
            closes_at.is_some_and(|closes_at| now >= closes_at)
        }
        StudentFeedbackReleaseTiming::AfterAllStudentsComplete => false,
        StudentFeedbackReleaseTiming::Never => false,
    }
}

#[cfg(test)]
mod tests {
    use question_model::{
        GradingResult, LateWorkRule, QuestionAnswer, QuestionAnswerExplanation,
        QuestionContentBlock, QuestionFeedback, StudentFeedbackReleaseRule,
        StudentFeedbackReleaseTiming, Timestamp,
    };

    use crate::effective_assessment_properties::{
        AssessmentPolicySource, EffectiveAssessmentPolicy, EffectiveAssessmentPolicyValue,
    };

    use super::{
        StudentFeedbackReleaseDecision, evaluate_allowed_student_feedback_release,
        project_student_feedback,
    };

    fn resolved<T>(value: T) -> EffectiveAssessmentPolicyValue<T> {
        EffectiveAssessmentPolicyValue {
            value,
            source: AssessmentPolicySource::Base,
        }
    }

    fn policy() -> EffectiveAssessmentPolicy {
        EffectiveAssessmentPolicy {
            available_at: resolved(None),
            due_at: resolved(None),
            closes_at: resolved(None),
            assessment_attempt_time_limit_seconds: resolved(None),
            attempt_limit: resolved(None),
            late_work_rule: resolved(LateWorkRule::Reject),
        }
    }

    fn question_feedback() -> QuestionFeedback {
        QuestionFeedback {
            choice_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Choice feedback".to_string(),
            }]),
            correct_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Correct feedback".to_string(),
            }]),
            incorrect_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Incorrect feedback".to_string(),
            }]),
        }
    }

    fn question_answer() -> QuestionAnswer {
        QuestionAnswer::new(vec![QuestionContentBlock::Text {
            markdown: "Display-ready answer".to_string(),
        }])
        .expect("one answer block is non-empty")
    }

    fn question_answer_explanation() -> QuestionAnswerExplanation {
        QuestionAnswerExplanation::new(vec![QuestionContentBlock::Text {
            markdown: "Answer explanation".to_string(),
        }])
        .expect("one explanation block is non-empty")
    }

    fn result() -> GradingResult {
        GradingResult {
            correct: true,
            points_earned: 2.0,
            points_possible: 2.0,
        }
    }

    #[test]
    fn withheld_question_answer_is_absent_from_student_feedback() {
        let decision = StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: false,
            worked_solutions: false,
        };
        let answer = question_answer();
        let explanation = question_answer_explanation();
        let disclosed = project_student_feedback(
            decision,
            Some(result()),
            &question_feedback(),
            Some(&answer),
            Some(&explanation),
        )
        .expect("feedback disclosure produces a Student Feedback view");

        assert!(disclosed.question_answer.is_none());
        assert!(disclosed.question_answer_explanation.is_none());
        let public = serde_json::to_value(disclosed).expect("Student Feedback serializes");
        assert!(public.get("questionAnswer").is_none());
        assert!(public.get("questionAnswerExplanation").is_none());
    }

    #[test]
    fn completed_review_releases_own_work_while_optional_content_remains_hidden() {
        let rule = StudentFeedbackReleaseRule {
            question_answer: StudentFeedbackReleaseTiming::Never,
            question_answer_explanation: StudentFeedbackReleaseTiming::Never,
            class_statistics: StudentFeedbackReleaseTiming::Never,
            hints: StudentFeedbackReleaseTiming::Never,
            worked_solutions: StudentFeedbackReleaseTiming::Never,
        };
        let now = Timestamp::from_unix_millis(1_000);

        let active = evaluate_allowed_student_feedback_release(&policy(), rule, now, None);
        assert!(!active.score);
        assert!(!active.per_item_correctness);
        assert!(!active.submitted_response);
        assert!(!active.question_answer);
        assert!(!active.question_answer_explanation);

        let completed = evaluate_allowed_student_feedback_release(&policy(), rule, now, Some(now));
        assert!(completed.score);
        assert!(completed.per_item_correctness);
        assert!(completed.submitted_response);
        assert!(!completed.question_answer);
        assert!(!completed.question_answer_explanation);
        assert!(!completed.class_statistics);
        assert!(!completed.hints);
        assert!(!completed.worked_solutions);
    }
}
