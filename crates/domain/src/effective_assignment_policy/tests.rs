use super::*;
use crate::entitlement::{
    ActiveStudentMembership, EntitlementFacts, evaluate_assignment_entitlement,
};
use question_model::{
    AssignmentAudience, AssignmentId, CourseGroupPurpose, CourseId, CourseMembershipId, TenantId,
    UserId,
};
use uuid::Uuid;

fn stamp(value: i64) -> ActivityTimestamp {
    ActivityTimestamp::from_unix_millis(value)
}
fn group(value: u128) -> CourseGroupId {
    CourseGroupId::from_uuid(Uuid::from_u128(value))
}
fn student(value: u128) -> StudentId {
    StudentId::from_uuid(Uuid::from_u128(value))
}
fn seconds(value: i32) -> ScheduleOffsetSeconds {
    ScheduleOffsetSeconds::try_new(value).expect("valid test offset")
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
fn grant(groups: Vec<(CourseGroupId, CourseGroupPurpose)>) -> EntitlementDecision {
    evaluate_assignment_entitlement(EntitlementFacts {
        tenant: TenantId::from_uuid(Uuid::from_u128(1)),
        course: CourseId::from_uuid(Uuid::from_u128(2)),
        assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
        learner: UserId::from_uuid(Uuid::from_u128(4)),
        membership: Some(ActiveStudentMembership {
            id: CourseMembershipId::from_uuid(Uuid::from_u128(5)),
            student: student(6),
        }),
        audience: AssignmentAudience::CourseWide,
        current_groups: groups,
    })
}
fn input(groups: Vec<(CourseGroupId, CourseGroupPurpose)>) -> ResolveEffectivePolicyInput {
    ResolveEffectivePolicyInput {
        lifecycle: AssignmentLifecycleGate::Open,
        entitlement: grant(groups),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        group_schedule_offsets: Vec::new(),
        group_accommodations: Vec::new(),
        individual_exception: None,
    }
}

#[test]
fn gates_short_circuit_before_modifier_validation() {
    let mut value = input(Vec::new());
    value.lifecycle = AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::NotPublished);
    value.group_schedule_offsets.push(GroupScheduleOffset {
        group: group(1),
        offset_seconds: seconds(1),
    });
    assert_eq!(
        resolve_effective_policy(value),
        Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(AssignmentLifecycleDenial::NotPublished)
        })
    );
}

#[test]
fn policy_scope_kind_controls_modifier_kind() {
    let section = group(10);
    let accommodation = group(11);
    let mut schedule = input(vec![
        (section, CourseGroupPurpose::Section),
        (accommodation, CourseGroupPurpose::Accommodation),
    ]);
    schedule.group_schedule_offsets.push(GroupScheduleOffset {
        group: section,
        offset_seconds: seconds(1),
    });
    assert!(matches!(
        resolve_effective_policy(schedule),
        Ok(EffectivePolicyDecision::Allowed { .. })
    ));

    let mut wrong_kind = input(vec![(accommodation, CourseGroupPurpose::Accommodation)]);
    wrong_kind.group_schedule_offsets.push(GroupScheduleOffset {
        group: accommodation,
        offset_seconds: seconds(1),
    });
    assert_eq!(
        resolve_effective_policy(wrong_kind),
        Err(EffectivePolicyError::UnapprovedScheduleScope(accommodation))
    );
}

#[test]
fn lab_and_cohort_scopes_allow_schedule_offsets() {
    let lab = group(12);
    let cohort = group(13);
    let mut value = input(vec![
        (lab, CourseGroupPurpose::Lab),
        (cohort, CourseGroupPurpose::Cohort),
    ]);
    value.group_schedule_offsets = vec![
        GroupScheduleOffset {
            group: lab,
            offset_seconds: seconds(1),
        },
        GroupScheduleOffset {
            group: cohort,
            offset_seconds: seconds(-1),
        },
    ];
    assert!(matches!(
        resolve_effective_policy(value),
        Ok(EffectivePolicyDecision::Allowed { .. })
    ));
}

#[test]
fn accommodations_require_an_approved_accommodation_scope() {
    let accommodation = group(11);
    let mut valid = input(vec![(accommodation, CourseGroupPurpose::Accommodation)]);
    valid.group_accommodations.push(GroupAccommodation {
        group: accommodation,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            attempt_limit: PolicyPatch::Unrestricted,
            ..PolicyPatchSet::INHERIT
        },
    });
    assert!(matches!(
        resolve_effective_policy(valid),
        Ok(EffectivePolicyDecision::Allowed { .. })
    ));

    let section = group(10);
    let mut wrong_kind = input(vec![(section, CourseGroupPurpose::Section)]);
    wrong_kind.group_accommodations.push(GroupAccommodation {
        group: section,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            attempt_limit: PolicyPatch::Unrestricted,
            ..PolicyPatchSet::INHERIT
        },
    });
    assert_eq!(
        resolve_effective_policy(wrong_kind),
        Err(EffectivePolicyError::UnapprovedAccommodationScope(section))
    );
}

#[test]
fn modifiers_for_groups_outside_the_grant_are_refused() {
    let outsider = group(99);
    let mut value = input(Vec::new());
    value.group_accommodations.push(GroupAccommodation {
        group: outsider,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            attempt_limit: PolicyPatch::Unrestricted,
            ..PolicyPatchSet::INHERIT
        },
    });
    assert_eq!(
        resolve_effective_policy(value),
        Err(EffectivePolicyError::UnapprovedAccommodationScope(outsider))
    );
}

#[test]
fn individual_exception_must_bind_the_granted_student() {
    let mut value = input(Vec::new());
    value.individual_exception = Some(IndividualPolicyException {
        student: student(7),
        mode: PolicyModificationMode::Override,
        patch: PolicyPatchSet {
            closes_at: PolicyPatch::Set(stamp(25_000)),
            ..PolicyPatchSet::INHERIT
        },
    });
    assert_eq!(
        resolve_effective_policy(value),
        Err(EffectivePolicyError::IndividualExceptionStudentMismatch {
            granted: student(6),
            modifier: student(7),
        })
    );
}

#[test]
fn individual_exception_for_the_granted_student_applies_without_receipt_authority() {
    let mut value = input(Vec::new());
    value.individual_exception = Some(IndividualPolicyException {
        student: student(6),
        mode: PolicyModificationMode::Override,
        patch: PolicyPatchSet {
            closes_at: PolicyPatch::Set(stamp(25_000)),
            ..PolicyPatchSet::INHERIT
        },
    });
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) = resolve_effective_policy(value)
    else {
        panic!("grant-bound individual exception should resolve");
    };
    assert_eq!(
        policy.closes_at.source,
        PolicySource::IndividualException(student(6))
    );
}

#[test]
fn schedule_offsets_use_validated_seconds_and_sorted_provenance() {
    let first = group(20);
    let second = group(10);
    let mut value = input(vec![
        (first, CourseGroupPurpose::Lab),
        (second, CourseGroupPurpose::Cohort),
    ]);
    value.group_schedule_offsets = vec![
        GroupScheduleOffset {
            group: first,
            offset_seconds: seconds(1),
        },
        GroupScheduleOffset {
            group: second,
            offset_seconds: seconds(2),
        },
    ];
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) = resolve_effective_policy(value)
    else {
        panic!("valid policy should resolve");
    };
    assert_eq!(policy.available_at.value, Some(stamp(13_000)));
    assert_eq!(
        policy.available_at.source,
        PolicySource::GroupScheduleOffsets(vec![second, first])
    );
}

#[test]
fn schedule_offset_constructor_refuses_non_persistable_values() {
    assert_eq!(
        ScheduleOffsetSeconds::try_new(0),
        Err(ScheduleOffsetSecondsError::Zero)
    );
    assert_eq!(
        ScheduleOffsetSeconds::try_new(MAX_SCHEDULE_OFFSET_SECONDS + 1),
        Err(ScheduleOffsetSecondsError::OutOfRange)
    );
}

#[test]
fn offset_timestamp_overflow_is_refused() {
    let section = group(10);
    let mut value = input(vec![(section, CourseGroupPurpose::Section)]);
    value.base.available_at = Some(stamp(i64::MAX));
    value.base.due_at = None;
    value.base.closes_at = None;
    value.group_schedule_offsets.push(GroupScheduleOffset {
        group: section,
        offset_seconds: seconds(1),
    });
    assert_eq!(
        resolve_effective_policy(value),
        Err(EffectivePolicyError::ScheduleOffsetOverflow)
    );
}

#[test]
fn extend_only_accommodations_reduce_by_permissiveness_independent_of_order() {
    let first = group(20);
    let second = group(10);
    let accommodation = |group, closes| GroupAccommodation {
        group,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            closes_at: PolicyPatch::Set(stamp(closes)),
            ..PolicyPatchSet::INHERIT
        },
    };
    let groups = vec![
        (first, CourseGroupPurpose::Accommodation),
        (second, CourseGroupPurpose::Accommodation),
    ];
    let mut forward = input(groups.clone());
    forward.group_accommodations =
        vec![accommodation(first, 40_000), accommodation(second, 50_000)];
    let mut reverse = input(groups);
    reverse.group_accommodations =
        vec![accommodation(second, 50_000), accommodation(first, 40_000)];
    let Ok(EffectivePolicyDecision::Allowed {
        policy: forward, ..
    }) = resolve_effective_policy(forward)
    else {
        panic!("forward policy should resolve");
    };
    let Ok(EffectivePolicyDecision::Allowed {
        policy: reverse, ..
    }) = resolve_effective_policy(reverse)
    else {
        panic!("reverse policy should resolve");
    };
    assert_eq!(forward.closes_at, reverse.closes_at);
    assert_eq!(forward.closes_at.value, Some(stamp(50_000)));
}

#[test]
fn equally_permissive_extensions_retain_all_winning_sources() {
    let first = group(20);
    let second = group(10);
    let mut value = input(vec![
        (first, CourseGroupPurpose::Accommodation),
        (second, CourseGroupPurpose::Accommodation),
    ]);
    value.group_accommodations = vec![
        GroupAccommodation {
            group: first,
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(4).unwrap()),
                ..PolicyPatchSet::INHERIT
            },
        },
        GroupAccommodation {
            group: second,
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(4).unwrap()),
                ..PolicyPatchSet::INHERIT
            },
        },
    ];
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) = resolve_effective_policy(value)
    else {
        panic!("valid policy should resolve");
    };
    assert_eq!(
        policy.attempt_limit.source,
        PolicySource::GroupAccommodations(vec![second, first])
    );
}

#[test]
fn one_override_wins_but_every_tightening_extension_is_refused() {
    let extension = group(10);
    let override_group = group(20);
    let mut value = input(vec![
        (extension, CourseGroupPurpose::Accommodation),
        (override_group, CourseGroupPurpose::Accommodation),
    ]);
    value.group_accommodations = vec![
        GroupAccommodation {
            group: extension,
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                closes_at: PolicyPatch::Set(stamp(40_000)),
                ..PolicyPatchSet::INHERIT
            },
        },
        GroupAccommodation {
            group: override_group,
            mode: PolicyModificationMode::Override,
            patch: PolicyPatchSet {
                closes_at: PolicyPatch::Set(stamp(25_000)),
                ..PolicyPatchSet::INHERIT
            },
        },
    ];
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) = resolve_effective_policy(value)
    else {
        panic!("valid policy should resolve");
    };
    assert_eq!(policy.closes_at.value, Some(stamp(25_000)));
    assert_eq!(
        policy.closes_at.source,
        PolicySource::GroupAccommodations(vec![override_group])
    );
}

#[test]
fn multiple_overrides_and_tightening_extensions_name_the_field_and_source() {
    let first = group(20);
    let second = group(10);
    let groups = vec![
        (first, CourseGroupPurpose::Accommodation),
        (second, CourseGroupPurpose::Accommodation),
    ];
    let mut overrides = input(groups.clone());
    overrides.group_accommodations = vec![
        GroupAccommodation {
            group: first,
            mode: PolicyModificationMode::Override,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(3).unwrap()),
                ..PolicyPatchSet::INHERIT
            },
        },
        GroupAccommodation {
            group: second,
            mode: PolicyModificationMode::Override,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(4).unwrap()),
                ..PolicyPatchSet::INHERIT
            },
        },
    ];
    assert_eq!(
        resolve_effective_policy(overrides),
        Err(EffectivePolicyError::MultipleAccommodationOverrides {
            field: PolicyField::AttemptLimit,
            sources: vec![second, first],
        })
    );

    let mut tightening = input(groups);
    tightening.group_accommodations.push(GroupAccommodation {
        group: first,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            closes_at: PolicyPatch::Set(stamp(25_000)),
            ..PolicyPatchSet::INHERIT
        },
    });
    assert_eq!(
        resolve_effective_policy(tightening),
        Err(EffectivePolicyError::ExtendOnlyViolation {
            field: PolicyField::ClosesAt,
            source: ModifierSource::Group(first),
        })
    );
}

#[test]
fn disjoint_accommodation_fields_compose() {
    let first = group(10);
    let second = group(20);
    let mut value = input(vec![
        (first, CourseGroupPurpose::Accommodation),
        (second, CourseGroupPurpose::Accommodation),
    ]);
    value.group_accommodations = vec![
        GroupAccommodation {
            group: first,
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                closes_at: PolicyPatch::Set(stamp(40_000)),
                ..PolicyPatchSet::INHERIT
            },
        },
        GroupAccommodation {
            group: second,
            mode: PolicyModificationMode::ExtendOnly,
            patch: PolicyPatchSet {
                attempt_limit: PolicyPatch::Set(NonZeroU32::new(4).unwrap()),
                ..PolicyPatchSet::INHERIT
            },
        },
    ];
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) = resolve_effective_policy(value)
    else {
        panic!("disjoint accommodations should compose");
    };
    assert_eq!(policy.closes_at.value, Some(stamp(40_000)));
    assert_eq!(policy.attempt_limit.value, NonZeroU32::new(4));
}

#[test]
fn availability_due_close_and_attempt_boundaries_are_authoritative() {
    let mut not_yet = input(Vec::new());
    not_yet.now = stamp(9_999);
    assert!(matches!(
        resolve_effective_policy(not_yet),
        Ok(EffectivePolicyDecision::Allowed {
            start: StartVerdict::NotYetAvailable,
            ..
        })
    ));

    let mut available = input(Vec::new());
    available.now = stamp(10_000);
    assert!(matches!(
        resolve_effective_policy(available),
        Ok(EffectivePolicyDecision::Allowed {
            start: StartVerdict::MayStart {
                late: LateVerdict::OnTime
            },
            ..
        })
    ));

    let mut due = input(Vec::new());
    due.now = stamp(20_001);
    assert!(matches!(
        resolve_effective_policy(due),
        Ok(EffectivePolicyDecision::Allowed {
            start: StartVerdict::DueDateRejectsNewRun,
            ..
        })
    ));

    let mut closed = input(Vec::new());
    closed.now = stamp(30_000);
    assert!(matches!(
        resolve_effective_policy(closed),
        Ok(EffectivePolicyDecision::Allowed {
            start: StartVerdict::Closed,
            ..
        })
    ));

    let mut exhausted = input(Vec::new());
    exhausted.prior_run_count = 2;
    assert!(matches!(
        resolve_effective_policy(exhausted),
        Ok(EffectivePolicyDecision::Allowed {
            start: StartVerdict::AttemptLimitReached,
            ..
        })
    ));
}
