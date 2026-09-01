use std::num::NonZeroU32;

use question_model::envelope::QuestionContentBlock;
use question_model::{
    AssignmentDeadlineRule, AssignmentScoringState, GradingResult, LateWorkRule, QuestionAnswer,
    QuestionAnswerExplanation, QuestionFeedback, QuestionPostGradingContent,
    StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming, Timestamp,
};

use super::{
    StudentFeedbackReleaseDecision, evaluate_student_feedback_release, project_student_feedback,
    project_student_response_inspection_feedback, score_current_student_feedback_release,
};
use crate::effective_assignment_policy::{
    AssignmentAccessDecision, AssignmentStartDecision, EffectiveAssignmentPolicy,
    EffectiveAssignmentPolicyValue, PolicySource, StudentLateWorkStatus,
};

fn stamp(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn rule() -> StudentFeedbackReleaseRule {
    StudentFeedbackReleaseRule {
        score: StudentFeedbackReleaseTiming::DuringAttempt,
        per_item_correctness: StudentFeedbackReleaseTiming::AfterSubmit,
        feedback_text: StudentFeedbackReleaseTiming::AfterDue,
        question_answer: StudentFeedbackReleaseTiming::AfterClose,
        question_answer_explanation: StudentFeedbackReleaseTiming::AfterClose,
        class_statistics: StudentFeedbackReleaseTiming::Never,
    }
}

fn allowed(due_at: Option<Timestamp>, closes_at: Option<Timestamp>) -> AssignmentAccessDecision {
    AssignmentAccessDecision::Allowed {
        policy: Box::new(EffectiveAssignmentPolicy {
            available_at: resolved(None),
            due_at: resolved(due_at),
            closes_at: resolved(closes_at),
            assignment_attempt_time_limit_seconds: resolved(None::<NonZeroU32>),
            attempt_limit: resolved(None::<NonZeroU32>),
            late_work_rule: resolved(LateWorkRule::Accept),
            assignment_deadline_rule: resolved(AssignmentDeadlineRule::AutoSubmit),
        }),
        start_decision: AssignmentStartDecision::MayStart {
            late_work_status: StudentLateWorkStatus::OnTime,
        },
    }
}

fn resolved<T>(value: T) -> EffectiveAssignmentPolicyValue<T> {
    EffectiveAssignmentPolicyValue {
        value,
        source: PolicySource::Base,
    }
}

#[test]
fn independent_fields_follow_their_own_timings() {
    let decision = evaluate_student_feedback_release(
        rule(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        None,
    )
    .expect("allowed Student has a disclosure decision");

    assert_eq!(
        decision,
        StudentFeedbackReleaseDecision {
            score: true,
            per_item_correctness: false,
            feedback_text: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
        }
    );
}

#[test]
fn after_submit_requires_this_students_submission() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let before_submission = evaluate_student_feedback_release(rule(), &effective, stamp(10), None)
        .expect("allowed Student has a disclosure decision");
    let after_submission =
        evaluate_student_feedback_release(rule(), &effective, stamp(10), Some(stamp(9)))
            .expect("allowed Student has a disclosure decision");

    assert!(!before_submission.per_item_correctness);
    assert!(after_submission.per_item_correctness);
}

#[test]
fn due_and_close_release_at_the_exact_resolved_boundaries() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let just_before_due = evaluate_student_feedback_release(rule(), &effective, stamp(19), None)
        .expect("allowed Student has a disclosure decision");
    let at_due = evaluate_student_feedback_release(rule(), &effective, stamp(20), None)
        .expect("allowed Student has a disclosure decision");
    let just_before_close = evaluate_student_feedback_release(rule(), &effective, stamp(29), None)
        .expect("allowed Student has a disclosure decision");
    let at_close = evaluate_student_feedback_release(rule(), &effective, stamp(30), None)
        .expect("allowed Student has a disclosure decision");

    assert!(!just_before_due.feedback_text);
    assert!(at_due.feedback_text);
    assert!(!just_before_close.question_answer);
    assert!(!just_before_close.question_answer_explanation);
    assert!(at_close.question_answer);
    assert!(at_close.question_answer_explanation);
}

#[test]
fn absent_due_and_close_do_not_release_timed_fields() {
    let decision =
        evaluate_student_feedback_release(rule(), &allowed(None, None), stamp(100), Some(stamp(1)))
            .expect("allowed Student has a disclosure decision");

    assert!(!decision.feedback_text);
    assert!(!decision.question_answer);
    assert!(!decision.question_answer_explanation);
}

#[test]
fn never_stays_hidden_after_every_other_release() {
    let decision = evaluate_student_feedback_release(
        rule(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(30),
        Some(stamp(1)),
    )
    .expect("allowed Student has a disclosure decision");

    assert!(decision.score);
    assert!(decision.per_item_correctness);
    assert!(decision.feedback_text);
    assert!(decision.question_answer);
    assert!(decision.question_answer_explanation);
    assert!(!decision.class_statistics);
}

#[test]
fn denied_s3_verdict_has_no_student_feedback_release_decision() {
    let denied = AssignmentAccessDecision::Denied {
        gate: crate::effective_assignment_policy::PolicyGate::Authorization,
        reason: crate::effective_assignment_policy::GateDenial::Authorization(
            crate::effective_assignment_policy::AuthorizationDenial::ActionNotPermitted,
        ),
    };

    assert!(
        evaluate_student_feedback_release(rule(), &denied, stamp(100), Some(stamp(1))).is_none()
    );
}

fn post_grading_content() -> QuestionPostGradingContent {
    let question_answer = QuestionAnswer::new(vec![QuestionContentBlock::Text {
        markdown: "Correct response".to_string(),
    }])
    .expect("one answer block is non-empty");
    let question_answer_explanation =
        QuestionAnswerExplanation::new(vec![QuestionContentBlock::Text {
            markdown: "Answer explanation".to_string(),
        }])
        .expect("one explanation block is non-empty");
    QuestionPostGradingContent {
        question_feedback: QuestionFeedback {
            choice_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Choice feedback".to_string(),
            }]),
            correct_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Correct feedback".to_string(),
            }]),
            incorrect_feedback: Some(vec![QuestionContentBlock::Text {
                markdown: "Incorrect feedback".to_string(),
            }]),
        },
        question_answer: Some(question_answer),
        question_answer_explanation: Some(question_answer_explanation),
    }
}

fn result() -> GradingResult {
    GradingResult {
        correct: true,
        points_earned: 2.0,
        points_possible: 2.0,
    }
}

#[test]
fn feedback_projection_allowlists_each_released_field() {
    let decision = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: true,
        question_answer: true,
        question_answer_explanation: true,
        class_statistics: false,
    };
    let disclosed = project_student_feedback(decision, Some(result()), &post_grading_content())
        .expect("released fields produce feedback");
    assert_eq!(disclosed.correctness, Some(true));
    assert_eq!(disclosed.points_earned, Some(2.0));
    assert!(disclosed.choice_feedback.is_some());
    assert!(disclosed.correct_feedback.is_some());
    assert!(disclosed.incorrect_feedback.is_some());
    assert!(disclosed.question_answer.is_some());
    assert!(disclosed.question_answer_explanation.is_some());
}

#[test]
fn inspection_projects_only_score_fields_and_hides_stale_values() {
    let decision = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: true,
        question_answer: true,
        question_answer_explanation: true,
        class_statistics: false,
    };
    for status in [
        AssignmentScoringState::Current,
        AssignmentScoringState::Recalculating,
        AssignmentScoringState::Failed,
    ] {
        let disclosed =
            project_student_response_inspection_feedback(decision, status, Some(result()));
        if status == AssignmentScoringState::Current {
            assert_eq!(disclosed.correctness, Some(true));
            assert_eq!(disclosed.points_earned, Some(2.0));
        } else {
            assert_eq!(disclosed.correctness, None);
            assert_eq!(disclosed.points_earned, None);
            assert_eq!(disclosed.points_possible, None);
        }
    }
}

#[test]
fn stale_scoring_removes_both_score_and_correctness_permissions() {
    let decision = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: false,
        question_answer: false,
        question_answer_explanation: false,
        class_statistics: false,
    };
    let stale =
        score_current_student_feedback_release(decision, AssignmentScoringState::Recalculating);
    assert!(!stale.score);
    assert!(!stale.per_item_correctness);
}
