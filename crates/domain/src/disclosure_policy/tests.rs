use std::num::NonZeroU32;

use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, LateSubmissionPolicy, LearnerDisclosurePolicy,
    LearnerDisclosureTiming,
};

use super::{LearnerDisclosureDecision, evaluate_learner_disclosure};
use crate::effective_assignment_policy::{
    EffectiveAssignmentPolicy, EffectivePolicyDecision, LateVerdict, PolicySource, ResolvedField,
    StartVerdict,
};

fn stamp(value: i64) -> ActivityTimestamp {
    ActivityTimestamp::from_unix_millis(value)
}

fn policy() -> LearnerDisclosurePolicy {
    LearnerDisclosurePolicy {
        score: LearnerDisclosureTiming::DuringAttempt,
        per_item_correctness: LearnerDisclosureTiming::AfterSubmit,
        feedback_text: LearnerDisclosureTiming::AfterDue,
        solution: LearnerDisclosureTiming::AfterClose,
        class_statistics: LearnerDisclosureTiming::Never,
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
    let decision = evaluate_learner_disclosure(
        policy(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(10),
        None,
    )
    .expect("allowed learner has a disclosure decision");

    assert_eq!(
        decision,
        LearnerDisclosureDecision {
            score: true,
            per_item_correctness: false,
            feedback_text: false,
            solution: false,
            class_statistics: false,
        }
    );
}

#[test]
fn after_submit_requires_this_learners_submission() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let before_submission = evaluate_learner_disclosure(policy(), &effective, stamp(10), None)
        .expect("allowed learner has a disclosure decision");
    let after_submission =
        evaluate_learner_disclosure(policy(), &effective, stamp(10), Some(stamp(9)))
            .expect("allowed learner has a disclosure decision");

    assert!(!before_submission.per_item_correctness);
    assert!(after_submission.per_item_correctness);
}

#[test]
fn due_and_close_release_at_the_exact_resolved_boundaries() {
    let effective = allowed(Some(stamp(20)), Some(stamp(30)));

    let just_before_due = evaluate_learner_disclosure(policy(), &effective, stamp(19), None)
        .expect("allowed learner has a disclosure decision");
    let at_due = evaluate_learner_disclosure(policy(), &effective, stamp(20), None)
        .expect("allowed learner has a disclosure decision");
    let just_before_close = evaluate_learner_disclosure(policy(), &effective, stamp(29), None)
        .expect("allowed learner has a disclosure decision");
    let at_close = evaluate_learner_disclosure(policy(), &effective, stamp(30), None)
        .expect("allowed learner has a disclosure decision");

    assert!(!just_before_due.feedback_text);
    assert!(at_due.feedback_text);
    assert!(!just_before_close.solution);
    assert!(at_close.solution);
}

#[test]
fn absent_due_and_close_do_not_release_timed_fields() {
    let decision =
        evaluate_learner_disclosure(policy(), &allowed(None, None), stamp(100), Some(stamp(1)))
            .expect("allowed learner has a disclosure decision");

    assert!(!decision.feedback_text);
    assert!(!decision.solution);
}

#[test]
fn never_stays_hidden_after_every_other_release() {
    let decision = evaluate_learner_disclosure(
        policy(),
        &allowed(Some(stamp(20)), Some(stamp(30))),
        stamp(30),
        Some(stamp(1)),
    )
    .expect("allowed learner has a disclosure decision");

    assert!(decision.score);
    assert!(decision.per_item_correctness);
    assert!(decision.feedback_text);
    assert!(decision.solution);
    assert!(!decision.class_statistics);
}

#[test]
fn denied_s3_verdict_has_no_learner_disclosure_decision() {
    let denied = EffectivePolicyDecision::Denied {
        gate: crate::effective_assignment_policy::PolicyGate::Authorization,
        reason: crate::effective_assignment_policy::GateDenial::Authorization(
            crate::effective_assignment_policy::AuthorizationDenial::ActionNotPermitted,
        ),
    };

    assert!(evaluate_learner_disclosure(policy(), &denied, stamp(100), Some(stamp(1))).is_none());
}
