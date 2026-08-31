use super::*;
use crate::entitlement::{
    evaluate_assignment_entitlement, evaluate_synthetic_preview_entitlement, ActiveStudentMembership,
    EntitlementFacts, SyntheticPreviewEntitlementFacts,
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

fn stamp(value: i64) -> ActivityTimestamp {
    ActivityTimestamp::from_unix_millis(value)
}

fn base() -> BaseAssignmentPolicy {
    BaseAssignmentPolicy {
        available_at: Some(stamp(10_000)),
        due_at: Some(stamp(20_000)),
        closes_at: Some(stamp(30_000)),
        time_limit_seconds: NonZeroU32::new(60),
        attempt_limit: NonZeroU32::new(2),
        late_submission: LateSubmissionPolicy::Reject,
        deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
    }
}

fn entitlement() -> EntitlementDecision {
    evaluate_assignment_entitlement(EntitlementFacts {
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
        lifecycle: AssignmentLifecycleGate::Open,
        entitlement: entitlement(),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        accommodation: None,
    }
}

#[test]
fn active_student_course_membership_resolves_base_policy() {
    let EffectivePolicyDecision::Allowed { policy, start } =
        resolve_effective_policy(input()).expect("valid direct policy")
    else {
        panic!("active student should receive an assignment policy");
    };
    assert_eq!(policy.due_at.value, Some(stamp(20_000)));
    assert_eq!(policy.due_at.source, PolicySource::Base);
    assert_eq!(start, StartVerdict::MayStart { late: LateVerdict::OnTime });
}

#[test]
fn direct_student_accommodation_extends_due_time() {
    let mut value = input();
    value.accommodation = Some(Accommodation {
        student_record: student_record(6),
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            due_at: PolicyPatch::Set(stamp(25_000)),
            ..PolicyPatchSet::INHERIT
        },
    });
    let EffectivePolicyDecision::Allowed { policy, .. } =
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
        mode: PolicyModificationMode::Override,
        patch: PolicyPatchSet::INHERIT,
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
fn lifecycle_denial_precedes_policy_evaluation() {
    let mut value = input();
    value.lifecycle = AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::NotPublished);
    assert_eq!(
        resolve_effective_policy(value),
        Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(AssignmentLifecycleDenial::NotPublished),
        })
    );
}

#[test]
fn synthetic_preview_can_apply_a_hypothetical_accommodation() {
    let decision = resolve_synthetic_preview_policy(ResolveSyntheticPreviewPolicyInput {
        lifecycle: AssignmentLifecycleGate::Open,
        entitlement: evaluate_synthetic_preview_entitlement(SyntheticPreviewEntitlementFacts::new(
            CourseId::from_uuid(id(2)),
            AssignmentId::from_uuid(id(3)),
        )),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        hypothetical_accommodation: Some(HypotheticalAccommodation {
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(3).expect("non-zero")),
                ..PolicyPatchSet::INHERIT
            },
        }),
    })
    .expect("valid synthetic policy");
    let EffectivePolicyDecision::Allowed { policy, .. } = decision else {
        panic!("synthetic preview should be authorized");
    };
    assert_eq!(policy.attempt_limit.value, NonZeroU32::new(3));
    assert_eq!(
        policy.attempt_limit.source,
        PolicySource::HypotheticalAccommodation
    );
}
