use std::num::NonZeroU32;

use question_model::envelope::ContentBlock;
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AttemptResult, FeedbackContent,
    LateSubmissionPolicy, ScoringStatus, StudentDisclosurePolicy, StudentDisclosureTiming,
};

use super::{
    StudentDisclosureDecision, evaluate_student_disclosure, project_disclosed_feedback,
    project_inspected_student_score_feedback, score_current_disclosure,
};
use crate::effective_assignment_policy::{
    EffectiveAssignmentPolicy, EffectivePolicyDecision, LateVerdict, PolicySource, ResolvedField,
    StartVerdict,
};

fn stamp(value: i64) -> ActivityTimestamp {
    ActivityTimestamp::from_unix_millis(value)
}

fn policy() -> StudentDisclosurePolicy {
    StudentDisclosurePolicy {
        score: StudentDisclosureTiming::DuringAttempt,
        per_item_correctness: StudentDisclosureTiming::AfterSubmit,
        feedback_text: StudentDisclosureTiming::AfterDue,
        solution: StudentDisclosureTiming::AfterClose,
        class_statistics: StudentDisclosureTiming::Never,
    }
}

fn allowed(
    due_at: Option<ActivityTimestamp>,
    closes_at: Option<ActivityTimestamp>,
) -> EffectivePolicyDecision {
    EffectivePolicyDecision::Allowed {
        policy: Box::new(EffectiveAssignmentPolicy {
            available_at: resolved(None),
            due_at: resolved(due_at),
            closes_at: resolved(closes_at),
            time_limit_seconds: resolved(None::<NonZeroU32>),
            attempt_limit: resolved(None::<NonZeroU32>),
            late_submission: resolved(LateSubmissionPolicy::Accept),
            deadline_behavior: resolved(AssignmentDeadlineBehavior::AutoSubmit),
        }),
        start: StartVerdict::MayStart {
            late: LateVerdict::OnTime,
        },
    }
}

fn resolved<T>(value: T) -> ResolvedField<T> {
    ResolvedField {
        value,
        source: PolicySource::Base,
    }
}

#[test]
fn independent_fields_follow_their_own_timings() {
    let decision = evaluate_student_disclosure(
        policy(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        None,
    )
    .expect("allowed Student has a disclosure decision");

    assert_eq!(
        decision,
        StudentDisclosureDecision {
            score: true,
            per_item_correctness: false,
            feedback_text: false,
            solution: false,
            class_statistics: false,
        }
    );
}

#[test]
fn after_submit_requires_this_students_submission() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let before_submission = evaluate_student_disclosure(policy(), &effective, stamp(10), None)
        .expect("allowed Student has a disclosure decision");
    let after_submission =
        evaluate_student_disclosure(policy(), &effective, stamp(10), Some(stamp(9)))
            .expect("allowed Student has a disclosure decision");

    assert!(!before_submission.per_item_correctness);
    assert!(after_submission.per_item_correctness);
}

#[test]
fn due_and_close_release_at_the_exact_resolved_boundaries() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let just_before_due = evaluate_student_disclosure(policy(), &effective, stamp(19), None)
        .expect("allowed Student has a disclosure decision");
    let at_due = evaluate_student_disclosure(policy(), &effective, stamp(20), None)
        .expect("allowed Student has a disclosure decision");
    let just_before_close = evaluate_student_disclosure(policy(), &effective, stamp(29), None)
        .expect("allowed Student has a disclosure decision");
    let at_close = evaluate_student_disclosure(policy(), &effective, stamp(30), None)
        .expect("allowed Student has a disclosure decision");

    assert!(!just_before_due.feedback_text);
    assert!(at_due.feedback_text);
    assert!(!just_before_close.solution);
    assert!(at_close.solution);
}

#[test]
fn absent_due_and_close_do_not_release_timed_fields() {
    let decision =
        evaluate_student_disclosure(policy(), &allowed(None, None), stamp(100), Some(stamp(1)))
            .expect("allowed Student has a disclosure decision");

    assert!(!decision.feedback_text);
    assert!(!decision.solution);
}

#[test]
fn never_stays_hidden_after_every_other_release() {
    let decision = evaluate_student_disclosure(
        policy(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(30),
        Some(stamp(1)),
    )
    .expect("allowed Student has a disclosure decision");

    assert!(decision.score);
    assert!(decision.per_item_correctness);
    assert!(decision.feedback_text);
    assert!(decision.solution);
    assert!(!decision.class_statistics);
}

#[test]
fn denied_s3_verdict_has_no_student_disclosure_decision() {
    let denied = EffectivePolicyDecision::Denied {
        gate: crate::effective_assignment_policy::PolicyGate::Authorization,
        reason: crate::effective_assignment_policy::GateDenial::Authorization(
            crate::effective_assignment_policy::AuthorizationDenial::ActionNotPermitted,
        ),
    };

    assert!(evaluate_student_disclosure(policy(), &denied, stamp(100), Some(stamp(1))).is_none());
}

fn feedback() -> FeedbackContent {
    FeedbackContent {
        hint: Some(vec![ContentBlock::Text {
            markdown: "Hint".to_string(),
        }]),
        rationale: Some(vec![ContentBlock::Text {
            markdown: "Rationale".to_string(),
        }]),
        correct_response: Some(vec![ContentBlock::Text {
            markdown: "Correct response".to_string(),
        }]),
    }
}

fn result() -> AttemptResult {
    AttemptResult {
        correct: true,
        points_earned: 2.0,
        points_possible: 2.0,
    }
}

#[test]
fn feedback_projection_allowlists_each_released_field() {
    let decision = StudentDisclosureDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: true,
        solution: true,
        class_statistics: false,
    };
    let disclosed = project_disclosed_feedback(decision, Some(result()), &feedback())
        .expect("released fields produce feedback");
    assert_eq!(disclosed.correctness, Some(true));
    assert_eq!(disclosed.points_earned, Some(2.0));
    assert!(disclosed.hint.is_some());
    assert!(disclosed.rationale.is_some());
    assert!(disclosed.correct_response.is_some());
}

#[test]
fn inspection_projects_only_score_fields_and_hides_stale_values() {
    let decision = StudentDisclosureDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: true,
        solution: true,
        class_statistics: false,
    };
    for status in [
        ScoringStatus::Current,
        ScoringStatus::Recalculating,
        ScoringStatus::Failed,
    ] {
        let disclosed = project_inspected_student_score_feedback(decision, status, Some(result()));
        if status == ScoringStatus::Current {
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
    let decision = StudentDisclosureDecision {
        score: true,
        per_item_correctness: true,
        feedback_text: false,
        solution: false,
        class_statistics: false,
    };
    let stale = score_current_disclosure(decision, ScoringStatus::Recalculating);
    assert!(!stale.score);
    assert!(!stale.per_item_correctness);
}
