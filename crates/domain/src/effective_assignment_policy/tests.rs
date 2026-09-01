use super::*;
use crate::active_student_course_membership::{
    ActiveStudentCourseMembershipFacts, ActiveStudentMembership, SyntheticPreviewAdmissionFacts,
    admit_synthetic_preview, evaluate_active_student_course_membership,
};
use question_model::{AccountId, AssignmentId, CourseId, CourseMembershipId};
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

fn base() -> BaseAssignmentPolicy {
    BaseAssignmentPolicy {
        available_at: Some(stamp(10_000)),
        due_at: Some(stamp(20_000)),
        closes_at: Some(stamp(30_000)),
        assignment_attempt_time_limit_seconds: NonZeroU32::new(60),
        attempt_limit: NonZeroU32::new(2),
        late_work_rule: LateWorkRule::Reject,
        assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
    }
}

fn active_student_course_membership() -> ActiveStudentCourseMembershipDecision {
    evaluate_active_student_course_membership(ActiveStudentCourseMembershipFacts {
        course: CourseId::from_uuid(id(2)),
        assignment: AssignmentId::from_uuid(id(3)),
        student_account: AccountId::from_uuid(id(4)),
        membership: Some(ActiveStudentMembership {
            id: CourseMembershipId::from_uuid(id(5)),
            student_record: student_record(6),
        }),
    })
}

fn input() -> ResolveEffectivePolicyInput {
    ResolveEffectivePolicyInput {
        assignment_status: AssignmentStatusGate::Open,
        active_student_course_membership: active_student_course_membership(),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        accommodation: None,
    }
}

#[test]
fn active_student_course_membership_resolves_base_policy() {
    let AssignmentAccessDecision::Allowed {
        policy,
        start_decision,
    } = resolve_effective_policy(input()).expect("valid direct policy")
    else {
        panic!("active student should receive an assignment policy");
    };
    assert_eq!(policy.due_at.value, Some(stamp(20_000)));
    assert_eq!(policy.due_at.source, PolicySource::Base);
    assert_eq!(
        start_decision,
        AssignmentStartDecision::MayStart {
            late_work_status: StudentLateWorkStatus::OnTime
        }
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
    let AssignmentAccessDecision::Allowed { policy, .. } =
        resolve_effective_policy(value).expect("valid accommodation")
    else {
        panic!("active student should receive an assignment policy");
    };
    assert_eq!(policy.due_at.value, Some(stamp(25_000)));
    assert_eq!(
        policy.due_at.source,
        PolicySource::Accommodation(student_record(6))
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
fn assignment_status_denial_precedes_policy_evaluation() {
    let mut value = input();
    value.assignment_status = AssignmentStatusGate::Denied(AssignmentStatusDenial::Unreleased);
    assert_eq!(
        resolve_effective_policy(value),
        Ok(AssignmentAccessDecision::Denied {
            gate: PolicyGate::AssignmentStatus,
            reason: GateDenial::AssignmentStatus(AssignmentStatusDenial::Unreleased),
        })
    );
}

#[test]
fn synthetic_preview_can_apply_a_hypothetical_accommodation() {
    let decision = resolve_synthetic_preview_policy(ResolveSyntheticPreviewPolicyInput {
        assignment_status: AssignmentStatusGate::Open,
        active_student_course_membership: admit_synthetic_preview(
            SyntheticPreviewAdmissionFacts::new(
                CourseId::from_uuid(id(2)),
                AssignmentId::from_uuid(id(3)),
            ),
        ),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        hypothetical_accommodation: Some(HypotheticalAccommodation {
            mode: AccommodationApplicationRule::ExtendOnly,
            adjustment: AccommodationAdjustment {
                attempt_limit: AccommodationAdjustmentValue::Set(
                    NonZeroU32::new(3).expect("non-zero"),
                ),
                ..AccommodationAdjustment::INHERIT
            },
        }),
    })
    .expect("valid synthetic policy");
    let AssignmentAccessDecision::Allowed { policy, .. } = decision else {
        panic!("synthetic preview should be authorized");
    };
    assert_eq!(policy.attempt_limit.value, NonZeroU32::new(3));
    assert_eq!(
        policy.attempt_limit.source,
        PolicySource::HypotheticalAccommodation
    );
}
