use super::*;
use crate::entitlement::{
    ActiveStudentMembership, EntitlementFacts, SyntheticPreviewEntitlementFacts,
    evaluate_assignment_entitlement, evaluate_synthetic_preview_entitlement,
};
use chrono::TimeZone;
use question_model::{
    AssignmentAudience, AssignmentId, AssignmentLifecycle, CourseGroupPurpose, CourseId,
    CourseMembershipId, TenantId, UserId,
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
        student_user: UserId::from_uuid(Uuid::from_u128(4)),
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

fn synthetic_input(
    groups: Vec<(CourseGroupId, CourseGroupPurpose)>,
) -> ResolveSyntheticPreviewPolicyInput {
    ResolveSyntheticPreviewPolicyInput {
        lifecycle: AssignmentLifecycleGate::Open,
        entitlement: evaluate_synthetic_preview_entitlement(SyntheticPreviewEntitlementFacts::new(
            TenantId::from_uuid(Uuid::from_u128(1)),
            CourseId::from_uuid(Uuid::from_u128(2)),
            AssignmentId::from_uuid(Uuid::from_u128(3)),
            AssignmentAudience::CourseWide,
            groups,
        )),
        authorization: AuthorizationGate::Authorized,
        now: stamp(20_000),
        prior_run_count: 0,
        base: base(),
        group_schedule_offsets: Vec::new(),
        group_accommodations: Vec::new(),
        hypothetical_individual_exception: None,
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
fn lifecycle_intent_maps_to_the_first_policy_gate() {
    assert_eq!(
        assignment_lifecycle_gate(AssignmentLifecycle::Published),
        AssignmentLifecycleGate::Open
    );
    assert_eq!(
        assignment_lifecycle_gate(AssignmentLifecycle::Draft),
        AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::NotPublished)
    );
    for lifecycle in [AssignmentLifecycle::Closed, AssignmentLifecycle::Archived] {
        assert_eq!(
            assignment_lifecycle_gate(lifecycle),
            AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::Retired)
        );
    }
}

#[test]
fn lifecycle_transitions_are_closed_and_archival_is_terminal() {
    use AssignmentLifecycle::{Archived, Closed, Draft, Published};

    for lifecycle in [Draft, Published, Closed, Archived] {
        assert!(is_legal_assignment_lifecycle_transition(
            lifecycle, lifecycle
        ));
    }
    for transition in [
        (Draft, Published),
        (Draft, Archived),
        (Published, Closed),
        (Published, Archived),
        (Closed, Published),
        (Closed, Archived),
    ] {
        assert!(is_legal_assignment_lifecycle_transition(
            transition.0,
            transition.1
        ));
    }
    for transition in [
        (Published, Draft),
        (Closed, Draft),
        (Archived, Draft),
        (Archived, Published),
        (Archived, Closed),
    ] {
        assert!(!is_legal_assignment_lifecycle_transition(
            transition.0,
            transition.1
        ));
    }
}

#[test]
fn base_policy_validation_rejects_unpersistable_limits_and_schedule_order() {
    let mut invalid_schedule = base();
    invalid_schedule.available_at = Some(stamp(30_001));
    assert_eq!(
        validate_base_assignment_policy(invalid_schedule),
        Err(EffectivePolicyError::InvalidScheduleOrder)
    );

    let mut invalid_time_limit = base();
    invalid_time_limit.time_limit_seconds =
        NonZeroU32::new(question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS + 1);
    assert_eq!(
        validate_base_assignment_policy(invalid_time_limit),
        Err(EffectivePolicyError::BaseTimeLimitOutOfRange)
    );

    let mut invalid_attempt_limit = base();
    invalid_attempt_limit.attempt_limit =
        NonZeroU32::new(question_model::MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1);
    assert_eq!(
        validate_base_assignment_policy(invalid_attempt_limit),
        Err(EffectivePolicyError::BaseAttemptLimitOutOfRange)
    );

    let mut postgres_boundary = base();
    postgres_boundary.time_limit_seconds =
        NonZeroU32::new(question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS);
    postgres_boundary.attempt_limit = NonZeroU32::new(question_model::MAX_ASSIGNMENT_ATTEMPT_LIMIT);
    assert_eq!(validate_base_assignment_policy(postgres_boundary), Ok(()));
}

#[test]
fn absolute_base_schedule_stays_inside_the_course_term_in_its_authoritative_zone() {
    let term =
        question_model::CourseTerm::from_parts("2026-08-24", "2026-08-24", "America/Chicago")
            .expect("one-day course term");
    let zone = chrono_tz::America::Chicago;
    let valid = ActivityTimestamp::from_unix_millis(
        zone.with_ymd_and_hms(2026, 8, 24, 23, 59, 59)
            .single()
            .expect("unambiguous local instant")
            .timestamp_millis(),
    );
    let mut policy = base();
    policy.available_at = Some(valid);
    policy.due_at = Some(valid);
    policy.closes_at = Some(valid);
    assert_eq!(
        validate_base_assignment_policy_for_course_term(policy, &term),
        Ok(())
    );

    let previous_day = ActivityTimestamp::from_unix_millis(
        zone.with_ymd_and_hms(2026, 8, 23, 23, 59, 59)
            .single()
            .expect("unambiguous local instant")
            .timestamp_millis(),
    );
    policy.available_at = Some(previous_day);
    policy.due_at = Some(valid);
    policy.closes_at = Some(valid);
    assert_eq!(
        validate_base_assignment_policy_for_course_term(policy, &term),
        Err(EffectivePolicyError::BaseTimestampOutsideCourseTerm(
            PolicyField::AvailableAt
        ))
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
fn synthetic_policy_shares_student_base_m2_and_m3_precedence() {
    let schedule = group(10);
    let accommodation = group(11);
    let groups = vec![
        (schedule, CourseGroupPurpose::Section),
        (accommodation, CourseGroupPurpose::Accommodation),
    ];
    let offsets = vec![GroupScheduleOffset {
        group: schedule,
        offset_seconds: seconds(100),
    }];
    let accommodations = vec![GroupAccommodation {
        group: accommodation,
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            attempt_limit: PolicyPatch::Unrestricted,
            ..PolicyPatchSet::INHERIT
        },
    }];
    let mut student_input = input(groups.clone());
    student_input.group_schedule_offsets = offsets.clone();
    student_input.group_accommodations = accommodations.clone();
    let mut synthetic = synthetic_input(groups);
    synthetic.group_schedule_offsets = offsets;
    synthetic.group_accommodations = accommodations;
    assert_eq!(
        resolve_effective_policy(student_input),
        resolve_synthetic_preview_policy(synthetic)
    );
}

#[test]
fn hypothetical_individual_modifier_uses_the_existing_policy_rules() {
    let mut extend_only = synthetic_input(Vec::new());
    extend_only.hypothetical_individual_exception = Some(HypotheticalIndividualPolicyException {
        mode: PolicyModificationMode::ExtendOnly,
        patch: PolicyPatchSet {
            closes_at: PolicyPatch::Set(stamp(25_000)),
            ..PolicyPatchSet::INHERIT
        },
    });
    assert_eq!(
        resolve_synthetic_preview_policy(extend_only),
        Err(EffectivePolicyError::ExtendOnlyViolation {
            field: PolicyField::ClosesAt,
            source: ModifierSource::HypotheticalIndividual,
        })
    );

    let mut override_value = synthetic_input(Vec::new());
    override_value.hypothetical_individual_exception =
        Some(HypotheticalIndividualPolicyException {
            mode: PolicyModificationMode::Override,
            patch: PolicyPatchSet {
                closes_at: PolicyPatch::Set(stamp(25_000)),
                ..PolicyPatchSet::INHERIT
            },
        });
    let Ok(EffectivePolicyDecision::Allowed { policy, .. }) =
        resolve_synthetic_preview_policy(override_value)
    else {
        panic!("hypothetical override should resolve");
    };
    assert_eq!(
        policy.closes_at.source,
        PolicySource::HypotheticalIndividualException
    );

    let mut invalid_schedule = synthetic_input(Vec::new());
    invalid_schedule.hypothetical_individual_exception =
        Some(HypotheticalIndividualPolicyException {
            mode: PolicyModificationMode::Override,
            patch: PolicyPatchSet {
                closes_at: PolicyPatch::Set(stamp(15_000)),
                ..PolicyPatchSet::INHERIT
            },
        });
    assert_eq!(
        resolve_synthetic_preview_policy(invalid_schedule),
        Err(EffectivePolicyError::InvalidScheduleOrder)
    );
}

#[test]
fn synthetic_policy_rejects_unapproved_scopes_without_student_authority() {
    let outsider = group(99);
    let mut value = synthetic_input(Vec::new());
    value.group_schedule_offsets.push(GroupScheduleOffset {
        group: outsider,
        offset_seconds: seconds(1),
    });
    assert_eq!(
        resolve_synthetic_preview_policy(value),
        Err(EffectivePolicyError::UnapprovedScheduleScope(outsider))
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
