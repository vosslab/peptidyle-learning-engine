use std::num::NonZeroU32;

use question_model::QuestionContentBlock;
use question_model::{
    AssessmentScoringState, AssessmentType, GradingResult, LateWorkRule, QuestionAnswer,
    QuestionAnswerExplanation, QuestionFeedback, StudentFeedbackReleaseRule,
    StudentFeedbackReleaseTiming, Timestamp,
};

use super::{
    StudentFeedbackReleaseDecision, apply_all_students_completed_timing,
    evaluate_student_feedback_release, project_disclosed_support, project_student_feedback,
    project_student_response_inspection_feedback, score_current_student_feedback_release,
};
use crate::effective_assessment_properties::{
    AssessmentAccessDecision, AssessmentPolicySource, AssessmentStartDecision,
    EffectiveAssessmentPolicy, EffectiveAssessmentPolicyValue, StudentLateWorkStatus,
};

fn stamp(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

#[test]
fn all_students_completed_timing_waits_only_when_configured() {
    let released = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        submitted_response: true,
        question_answer: true,
        question_answer_explanation: true,
        class_statistics: true,
        hints: true,
        worked_solutions: true,
    };

    let cohort_rule = StudentFeedbackReleaseRule {
        question_answer: StudentFeedbackReleaseTiming::AfterAllStudentsComplete,
        question_answer_explanation: StudentFeedbackReleaseTiming::AfterAllStudentsComplete,
        ..Default::default()
    };
    let waiting = apply_all_students_completed_timing(released, cohort_rule, false);
    assert!(!waiting.question_answer);
    assert!(!waiting.question_answer_explanation);
    assert!(waiting.score);
    assert!(waiting.per_item_correctness);
    assert!(waiting.submitted_response);
    assert!(waiting.class_statistics);
    assert_eq!(
        apply_all_students_completed_timing(released, cohort_rule, true),
        released
    );

    let mut instructor_override = cohort_rule;
    instructor_override.question_answer = StudentFeedbackReleaseTiming::AfterSubmit;
    instructor_override.question_answer_explanation = StudentFeedbackReleaseTiming::AfterSubmit;
    assert_eq!(
        apply_all_students_completed_timing(released, instructor_override, false),
        released
    );
}

fn rule() -> StudentFeedbackReleaseRule {
    StudentFeedbackReleaseRule {
        per_item_correctness: StudentFeedbackReleaseTiming::AfterSubmit,
        submitted_response: StudentFeedbackReleaseTiming::AfterSubmit,
        question_answer: StudentFeedbackReleaseTiming::AfterClose,
        question_answer_explanation: StudentFeedbackReleaseTiming::AfterClose,
        class_statistics: StudentFeedbackReleaseTiming::AfterDue,
        hints: StudentFeedbackReleaseTiming::Never,
        worked_solutions: StudentFeedbackReleaseTiming::Never,
    }
}

fn allowed(due_at: Option<Timestamp>, closes_at: Option<Timestamp>) -> AssessmentAccessDecision {
    AssessmentAccessDecision::Allowed {
        policy: Box::new(EffectiveAssessmentPolicy {
            available_at: resolved(None),
            due_at: resolved(due_at),
            closes_at: resolved(closes_at),
            assessment_attempt_time_limit_seconds: resolved(None::<NonZeroU32>),
            attempt_limit: resolved(None::<NonZeroU32>),
            late_work_rule: resolved(LateWorkRule::Accept),
        }),
        start_decision: AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::OnTime,
        },
    }
}

fn resolved<T>(value: T) -> EffectiveAssessmentPolicyValue<T> {
    EffectiveAssessmentPolicyValue {
        value,
        source: AssessmentPolicySource::Base,
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
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: false,
            worked_solutions: false,
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
    assert!(!before_submission.submitted_response);
    assert!(after_submission.per_item_correctness);
    assert!(after_submission.submitted_response);
}

#[test]
fn submitted_response_never_remains_withheld_after_submission() {
    let mut response_withheld = rule();
    response_withheld.submitted_response = StudentFeedbackReleaseTiming::Never;

    let decision = evaluate_student_feedback_release(
        response_withheld,
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        Some(stamp(9)),
    )
    .expect("allowed Student has a disclosure decision");

    assert!(!decision.submitted_response);
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

    assert!(!just_before_due.class_statistics);
    assert!(at_due.class_statistics);
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

    assert!(!decision.class_statistics);
    assert!(!decision.question_answer);
    assert!(!decision.question_answer_explanation);
}

#[test]
fn never_stays_hidden_after_every_other_release() {
    let mut release_rule = rule();
    release_rule.class_statistics = StudentFeedbackReleaseTiming::Never;
    let decision = evaluate_student_feedback_release(
        release_rule,
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(30),
        Some(stamp(1)),
    )
    .expect("allowed Student has a disclosure decision");

    assert!(decision.score);
    assert!(decision.per_item_correctness);
    assert!(decision.question_answer);
    assert!(decision.question_answer_explanation);
    assert!(!decision.class_statistics);
}

#[test]
fn denied_s3_verdict_has_no_student_feedback_release_decision() {
    let denied = AssessmentAccessDecision::Denied {
        gate: crate::effective_assessment_properties::PolicyGate::Authorization,
        reason: crate::effective_assessment_properties::GateDenial::Authorization(
            crate::effective_assessment_properties::AuthorizationDenial::ActionNotPermitted,
        ),
    };

    assert!(
        evaluate_student_feedback_release(rule(), &denied, stamp(100), Some(stamp(1))).is_none()
    );
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
fn feedback_projection_allowlists_each_released_field() {
    let decision = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        submitted_response: true,
        question_answer: true,
        question_answer_explanation: true,
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
fn withheld_question_answer_is_absent_while_provided_feedback_is_shown() {
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

    assert!(disclosed.choice_feedback.is_some());
    assert!(disclosed.correct_feedback.is_some());
    assert!(disclosed.incorrect_feedback.is_some());
    assert!(disclosed.question_answer.is_none());
    assert!(disclosed.question_answer_explanation.is_none());
    let public = serde_json::to_value(disclosed).expect("Student Feedback serializes");
    assert!(public.get("questionAnswer").is_none());
    assert!(public.get("questionAnswerExplanation").is_none());
}

#[test]
fn independently_derived_answer_explanation_releases_without_an_answer_wrapper() {
    let decision = StudentFeedbackReleaseDecision {
        score: false,
        per_item_correctness: false,
        submitted_response: false,
        question_answer: false,
        question_answer_explanation: true,
        class_statistics: false,
        hints: false,
        worked_solutions: false,
    };
    let explanation = question_answer_explanation();

    let disclosed = project_student_feedback(
        decision,
        Some(result()),
        &QuestionFeedback::default(),
        None,
        Some(&explanation),
    )
    .expect("an authorized Answer Explanation produces Student Feedback");

    assert!(disclosed.question_answer.is_none());
    assert_eq!(
        disclosed.question_answer_explanation,
        Some(vec![QuestionContentBlock::Text {
            markdown: "Answer explanation".to_string(),
        }])
    );
}

#[test]
fn student_response_inspection_projects_only_permitted_correctness_and_score() {
    let decision = StudentFeedbackReleaseDecision {
        score: true,
        per_item_correctness: true,
        submitted_response: true,
        question_answer: true,
        question_answer_explanation: true,
        class_statistics: false,
        hints: false,
        worked_solutions: false,
    };
    for status in [
        AssessmentScoringState::Current,
        AssessmentScoringState::Recalculating,
        AssessmentScoringState::Failed,
    ] {
        let disclosed =
            project_student_response_inspection_feedback(decision, status, Some(result()));
        if status == AssessmentScoringState::Current {
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
        submitted_response: true,
        question_answer: false,
        question_answer_explanation: false,
        class_statistics: false,
        hints: false,
        worked_solutions: false,
    };
    let stale =
        score_current_student_feedback_release(decision, AssessmentScoringState::Recalculating);
    assert!(!stale.score);
    assert!(!stale.per_item_correctness);
}

#[test]
fn weekly_and_bonus_keep_the_correct_answer_hidden_after_submission() {
    for assessment_type in [
        AssessmentType::RegularAssignment,
        AssessmentType::BonusAssignment,
    ] {
        let rule = StudentFeedbackReleaseRule::for_assessment_type(assessment_type);
        assert_eq!(rule.question_answer, StudentFeedbackReleaseTiming::Never);
        let decision = evaluate_student_feedback_release(
            rule,
            &allowed(Some(stamp(20)), Some(stamp(30))),
            stamp(40),
            Some(stamp(10)),
        )
        .expect("allowed Student has a disclosure decision");
        assert!(decision.per_item_correctness);
        assert!(decision.submitted_response);
        assert!(!decision.question_answer);
        let disclosed = project_student_feedback(
            decision,
            Some(result()),
            &question_feedback(),
            Some(&question_answer()),
            Some(&question_answer_explanation()),
        )
        .expect("submitted feedback");
        assert_eq!(disclosed.correctness, Some(true));
        assert!(disclosed.question_answer.is_none());
    }
}

#[test]
fn hints_and_worked_solutions_use_their_own_disclosure_settings() {
    let mut release_rule = rule();
    release_rule.hints = StudentFeedbackReleaseTiming::DuringAttempt;
    release_rule.worked_solutions = StudentFeedbackReleaseTiming::Never;
    release_rule.question_answer = StudentFeedbackReleaseTiming::AfterSubmit;
    let during_attempt = evaluate_student_feedback_release(
        release_rule,
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        None,
    )
    .expect("allowed Student has a disclosure decision");
    assert!(during_attempt.hints);
    assert!(!during_attempt.worked_solutions);
    assert!(!during_attempt.question_answer);

    release_rule.hints = StudentFeedbackReleaseTiming::Never;
    release_rule.worked_solutions = StudentFeedbackReleaseTiming::AfterSubmit;
    let after_submit = evaluate_student_feedback_release(
        release_rule,
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        Some(stamp(9)),
    )
    .expect("allowed Student has a disclosure decision");
    assert!(!after_submit.hints);
    assert!(after_submit.worked_solutions);
    assert!(after_submit.question_answer);

    let hint = vec![QuestionContentBlock::Text {
        markdown: "Count the carbons.".to_owned(),
    }];
    let worked_solution = vec![QuestionContentBlock::Text {
        markdown: "The carbonyl carbon is electrophilic.".to_owned(),
    }];
    let shown = project_disclosed_support(during_attempt, Some(&hint), Some(&worked_solution));
    assert_eq!(shown.hints, Some(hint.clone()));
    assert!(shown.worked_solutions.is_none());
    let solution_only =
        project_disclosed_support(after_submit, Some(&hint), Some(&worked_solution));
    assert!(solution_only.hints.is_none());
    assert_eq!(solution_only.worked_solutions, Some(worked_solution));

    let answer = project_student_feedback(
        during_attempt,
        None,
        &QuestionFeedback::default(),
        Some(&question_answer()),
        None,
    );
    assert!(answer.is_none() || answer.expect("feedback").question_answer.is_none());

    let mut cohort_rule = rule();
    cohort_rule.question_answer = StudentFeedbackReleaseTiming::AfterAllStudentsComplete;
    let waiting = apply_all_students_completed_timing(after_submit, cohort_rule, false);
    assert!(waiting.worked_solutions);
    assert!(!waiting.hints);
    assert!(!waiting.question_answer);
}
