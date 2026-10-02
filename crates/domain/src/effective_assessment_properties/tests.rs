use super::*;
use crate::active_student_course_membership::{
    ActiveStudentCourseMembershipFacts, ActiveStudentMembership,
    HypotheticalStudentViewScenarioAdmissionFacts, admit_hypothetical_student_view_scenario,
    evaluate_active_student_course_membership,
};
use question_model::{AccountId, AssessmentId, CourseInstanceId, CourseMembershipId};
use std::num::NonZeroU32;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn student_record(value: u128) -> StudentRecordId {
    StudentRecordId::from_uuid(id(value))
}

fn stamp(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn base() -> BaseAssessmentPolicy {
    BaseAssessmentPolicy {
        available_at: Some(stamp(10_000)),
        due_at: Some(stamp(20_000)),
        closes_at: Some(stamp(30_000)),
        assessment_attempt_time_limit_seconds: NonZeroU32::new(60),
        attempt_limit: NonZeroU32::new(2),
        late_work_rule: LateWorkRule::Reject,
    }
}

fn active_student_course_membership() -> ActiveStudentCourseMembershipDecision {
    evaluate_active_student_course_membership(ActiveStudentCourseMembershipFacts {
        course_instance_id: CourseInstanceId::from_debug_serial(2),
        assessment_id: AssessmentId::from_debug_serial(3),
        student_account: AccountId::from_debug_serial(4),
        membership: Some(ActiveStudentMembership {
            id: CourseMembershipId::from_uuid(id(5)),
            student_record: student_record(6),
        }),
    })
}

fn input() -> ResolveEffectivePolicyInput {
    ResolveEffectivePolicyInput {
        assessment_status: AssessmentStatusGate::Open,
        active_student_course_membership: active_student_course_membership(),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_assessment_attempt_count: 0,
        base: base(),
        accommodation: None,
    }
}

#[test]
fn active_student_course_membership_resolves_base_policy() {
    let AssessmentAccessDecision::Allowed {
        policy,
        start_decision,
    } = resolve_effective_policy(input()).expect("valid direct policy")
    else {
        panic!("active student should receive an assessment policy");
    };
    assert_eq!(policy.due_at.value, Some(stamp(20_000)));
    assert_eq!(policy.due_at.source, AssessmentPolicySource::Base);
    assert_eq!(
        start_decision,
        AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::OnTime
        }
    );
}

#[test]
fn base_duration_validation_rejects_non_minute_values() {
    let mut value = base();
    value.assessment_attempt_time_limit_seconds = NonZeroU32::new(90);
    assert_eq!(
        validate_base_assessment_policy(value),
        Err(EffectivePolicyError::BaseAssessmentAttemptAssessmentAttemptTimeLimitOutOfRange)
    );
}

fn start_decision_at(now: i64, completed_attempt_count: u32) -> AssessmentStartDecision {
    let mut value = input();
    value.now = stamp(now);
    value.prior_assessment_attempt_count = completed_attempt_count;
    let AssessmentAccessDecision::Allowed { start_decision, .. } =
        resolve_effective_policy(value).expect("valid boundary policy")
    else {
        panic!("active Student should receive an Assessment Start Decision");
    };
    start_decision
}

#[test]
fn assessment_start_decision_uses_exact_schedule_boundaries() {
    assert_eq!(
        start_decision_at(9_999, 0),
        AssessmentStartDecision::NotYetAvailable
    );
    assert_eq!(
        start_decision_at(10_000, 0),
        AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::OnTime,
        }
    );
    assert_eq!(
        start_decision_at(20_000, 0),
        AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::OnTime,
        }
    );
    assert_eq!(
        start_decision_at(20_001, 0),
        AssessmentStartDecision::LateWorkRefused
    );
    assert_eq!(
        start_decision_at(30_000, 0),
        AssessmentStartDecision::Closed
    );
}

#[test]
fn assessment_start_decision_checks_attempt_limit_before_late_work() {
    assert_eq!(
        start_decision_at(20_001, 2),
        AssessmentStartDecision::AttemptLimitReached
    );
}

fn start_decision_for(
    attempt_limit: Option<NonZeroU32>,
    prior_assessment_attempt_count: u32,
) -> AssessmentStartDecision {
    let mut value = input();
    value.base.attempt_limit = attempt_limit;
    value.prior_assessment_attempt_count = prior_assessment_attempt_count;
    let AssessmentAccessDecision::Allowed { start_decision, .. } =
        resolve_effective_policy(value).expect("repeat policy")
    else {
        panic!("an active Student should receive a start decision");
    };
    start_decision
}

#[test]
fn student_may_repeat_while_the_saved_attempt_limit_allows_it() {
    let another = AssessmentStartDecision::MayStart {
        student_late_work_status: StudentLateWorkStatus::OnTime,
    };
    assert_eq!(start_decision_for(None, 7), another);
    assert_eq!(start_decision_for(NonZeroU32::new(3), 2), another);
    assert_eq!(
        start_decision_for(NonZeroU32::new(3), 3),
        AssessmentStartDecision::AttemptLimitReached
    );
}

#[test]
fn direct_student_accommodation_extends_due_time() {
    let mut value = input();
    value.accommodation = Some(Accommodation {
        student_record: student_record(6),
        mode: AccommodationApplicationRule::ExtendOnly,
        adjustment: AccommodationAdjustment {
            due_at: AccommodationAdjustmentValue::Set(stamp(25_000)),
            ..AccommodationAdjustment::INHERIT
        },
    });
    let AssessmentAccessDecision::Allowed { policy, .. } =
        resolve_effective_policy(value).expect("valid accommodation")
    else {
        panic!("active student should receive an assessment policy");
    };
    assert_eq!(policy.due_at.value, Some(stamp(25_000)));
    assert_eq!(
        policy.due_at.source,
        AssessmentPolicySource::Accommodation(student_record(6))
    );
}

#[test]
fn accommodation_must_belong_to_the_entitled_student() {
    let mut value = input();
    value.accommodation = Some(Accommodation {
        student_record: student_record(7),
        mode: AccommodationApplicationRule::Replace,
        adjustment: AccommodationAdjustment::INHERIT,
    });
    assert_eq!(
        resolve_effective_policy(value),
        Err(EffectivePolicyError::AccommodationStudentRecordMismatch {
            granted: student_record(6),
            modifier: student_record(7),
        })
    );
}

#[test]
fn assessment_status_denial_precedes_policy_evaluation() {
    let mut value = input();
    value.assessment_status = AssessmentStatusGate::Denied(AssessmentStatusDenial::Unreleased);
    assert_eq!(
        resolve_effective_policy(value),
        Ok(AssessmentAccessDecision::Denied {
            gate: PolicyGate::AssessmentStatus,
            reason: GateDenial::AssessmentStatus(AssessmentStatusDenial::Unreleased),
        })
    );
}

#[test]
fn unrelease_is_a_released_to_unreleased_state_transition() {
    assert!(is_legal_assessment_status_transition(
        AssessmentStatus::Released,
        AssessmentStatus::Unreleased,
    ));
}

#[test]
fn hypothetical_student_view_scenario_can_apply_direct_modifiers() {
    let decision = resolve_hypothetical_student_view_scenario_policy(
        ResolveHypotheticalStudentViewScenarioPolicyInput {
            assessment_status: AssessmentStatusGate::Open,
            hypothetical_student_view_scenario_admission: admit_hypothetical_student_view_scenario(
                HypotheticalStudentViewScenarioAdmissionFacts::new(
                    CourseInstanceId::from_debug_serial(2),
                    AssessmentId::from_debug_serial(3),
                ),
            ),
            authorization: AuthorizationGate::Authorized,
            now: stamp(20_000),
            prior_assessment_attempt_count: 0,
            base: base(),
            hypothetical_student_view_scenario_modifiers: Some(
                HypotheticalStudentViewScenarioModifiers {
                    mode: AccommodationApplicationRule::ExtendOnly,
                    adjustment: AccommodationAdjustment {
                        attempt_limit: AccommodationAdjustmentValue::Set(
                            NonZeroU32::new(3).expect("non-zero"),
                        ),
                        ..AccommodationAdjustment::INHERIT
                    },
                },
            ),
        },
    )
    .expect("valid hypothetical Student View Scenario policy");
    let HypotheticalStudentViewScenarioPolicyDecision::Allowed { policy, .. } = decision else {
        panic!("hypothetical Student View Scenario should be authorized");
    };
    assert_eq!(policy.attempt_limit.value, NonZeroU32::new(3));
    assert_eq!(
        policy.attempt_limit.source,
        AssessmentPolicySource::HypotheticalStudentViewScenario
    );
}

fn scheduled_default_policy() -> BaseAssessmentPolicy {
    BaseAssessmentPolicy {
        available_at: Some(stamp(10_000)),
        due_at: Some(stamp(20_000)),
        closes_at: Some(stamp(30_000)),
        ..BaseAssessmentPolicy::default()
    }
}

fn decide(
    now: i64,
    status: AssessmentStatus,
    base: BaseAssessmentPolicy,
) -> AssessmentAccessDecision {
    let mut value = input();
    value.now = stamp(now);
    value.base = base;
    value.assessment_status = assessment_status_gate(status);
    resolve_effective_policy(value).expect("scheduled policy")
}

fn released_start(now: i64, base: BaseAssessmentPolicy) -> AssessmentStartDecision {
    let AssessmentAccessDecision::Allowed { start_decision, .. } =
        decide(now, AssessmentStatus::Released, base)
    else {
        panic!("released student should receive a start decision");
    };
    start_decision
}

fn availability(
    now: i64,
    status: AssessmentStatus,
    base: BaseAssessmentPolicy,
) -> question_model::InstructorAssessmentAvailabilityView {
    let settings = question_model::AssessmentAuthoredContent {
        instructions: question_model::AssessmentInstructions::default(),
        base_policy: base,
        activity_rules: question_model::AssessmentActivityRules::default(),
    };
    question_model::derive_instructor_assessment_availability(
        &question_model::CourseTerm::from_parts("2026-01-01", "2026-12-31").expect("term"),
        &question_model::AccountTimeZone::parse("UTC").expect("zone"),
        status,
        &settings,
        stamp(now),
    )
    .expect("availability")
}

#[test]
fn default_reject_policy_opens_released_work_until_the_due_date() {
    assert_eq!(
        BaseAssessmentPolicy::default().late_work_rule,
        LateWorkRule::Reject
    );
    for assessment_type in question_model::AssessmentType::ALL {
        assert_eq!(
            question_model::AssessmentTemplateSettings::for_assessment_type(assessment_type)
                .late_work_rule,
            LateWorkRule::Reject
        );
    }

    let policy = scheduled_default_policy();
    assert_eq!(policy.late_work_rule, LateWorkRule::Reject);
    assert_eq!(
        decide(15_000, AssessmentStatus::Unreleased, policy),
        AssessmentAccessDecision::Denied {
            gate: PolicyGate::AssessmentStatus,
            reason: GateDenial::AssessmentStatus(AssessmentStatusDenial::Unreleased),
        }
    );
    assert_eq!(
        availability(15_000, AssessmentStatus::Unreleased, policy),
        question_model::InstructorAssessmentAvailabilityView::Unreleased
    );
    assert_eq!(
        released_start(9_999, policy),
        AssessmentStartDecision::NotYetAvailable
    );
    let AssessmentAccessDecision::Allowed {
        policy: effective, ..
    } = decide(15_000, AssessmentStatus::Released, policy)
    else {
        panic!("released student should receive the default policy");
    };
    assert_eq!(effective.late_work_rule.value, LateWorkRule::Reject);
    assert_eq!(
        released_start(15_000, policy),
        AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::OnTime,
        }
    );
    assert_eq!(
        released_start(20_001, policy),
        AssessmentStartDecision::LateWorkRefused
    );
    assert_eq!(
        released_start(30_000, policy),
        AssessmentStartDecision::Closed
    );
    assert_eq!(
        availability(19_999, AssessmentStatus::Released, policy),
        question_model::InstructorAssessmentAvailabilityView::Available
    );
    assert!(matches!(
        availability(20_000, AssessmentStatus::Released, policy),
        question_model::InstructorAssessmentAvailabilityView::Closed { closed_at: Some(_) }
    ));

    let mut accepting = policy;
    accepting.late_work_rule = LateWorkRule::Accept;
    accepting.closes_at = None;
    assert_eq!(
        released_start(20_001, accepting),
        AssessmentStartDecision::MayStart {
            student_late_work_status: StudentLateWorkStatus::AcceptedLate,
        }
    );
    assert_eq!(
        availability(20_000, AssessmentStatus::Released, accepting),
        question_model::InstructorAssessmentAvailabilityView::Available
    );
}
