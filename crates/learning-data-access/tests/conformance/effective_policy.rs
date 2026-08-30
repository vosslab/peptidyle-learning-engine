use super::*;
use domain::effective_assignment_policy::{
    AssignmentLifecycleDenial, AuthorizationDenial, AuthorizationGate, BaseAssignmentPolicy,
    EffectivePolicyDecision, GateDenial, GroupAccommodation, GroupScheduleOffset,
    IndividualPolicyException, PolicyGate, PolicyModificationMode, PolicyPatch, PolicyPatchSet,
    PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::{
    AssignmentPoliciesUpdate, PutAssignmentTeachingSettingsCommand, PutGroupAccommodationCommand,
    PutGroupScheduleOffsetCommand, PutIndividualPolicyExceptionCommand,
    ReplaceAssignmentPoliciesCommand, ReplaceAssignmentPoliciesOutcome,
    ResolveEffectivePolicyCommand, StoredIndividualPolicyException,
};
use std::num::NonZeroU32;

pub(crate) struct EffectivePolicyFixture {
    pub context: TenantContext,
    pub instructor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub assignment_revision: learning_data_access::AssignmentRevision,
    pub attempt: QuestionAttemptId,
    pub receipt: learning_data_access::IssuedEffectivePolicyReceipt,
}

fn policy_issue(
    learner: UserId,
    course: CourseId,
    assignment: AssignmentId,
    run: RunId,
    attempt: question_model::QuestionAttemptId,
    reference: ProblemVersionRef,
    issued_question_snapshot: learning_data_access::IssuedQuestionSnapshotV1,
) -> IssueQuestionAttemptCommand {
    IssueQuestionAttemptCommand {
        actor: learner,
        binding: StudentWorkRoutingBinding::new(course, assignment),
        attempt,
        run,
        assignment_position: 0,
        problem: reference.problem,
        question_version: reference.version,
        issued_question_snapshot,
        seed: 99,
        presentation_capability: PresentationCapability::NotApplicable,
        presentation: None,
        presentation_snapshot: None,
        grading_envelope: None,
        native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: WebworkGradingCapability::NotApplicable,
        qti_grading: None,
        qti_grading_capability: QtiGradingCapability::NotApplicable,
        parameter_hash: "effective-policy-conformance".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("effective-policy-native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("effective-policy-grading"),
            rendered_question_sha256: "effective-policy-render".to_string(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    }
}

/// Exercises the Store boundary that composes an opaque S5 grant with S3
/// policy records. PostgreSQL's disposable conformance oracle may call this
/// helper after its backend is available.
pub(crate) async fn exercise_effective_policy_gate_and_materialization_contract<S>(
    store: &S,
) -> EffectivePolicyFixture
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    exercise_effective_policy_gate_and_materialization_contract_with_timing(
        store,
        question_model::run_policy::TimingPolicy::Untimed,
    )
    .await
}

pub(crate) async fn exercise_effective_policy_gate_and_materialization_contract_with_timing<S>(
    store: &S,
    timing_policy: question_model::run_policy::TimingPolicy,
) -> EffectivePolicyFixture
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(99_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(99_001));
    let learner = UserId::from_uuid(uuid(99_002));
    let other_learner = UserId::from_uuid(uuid(99_003));
    let course = CourseId::from_uuid(uuid(99_004));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Policy gate conformance".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("course");
    for (user, display_name) in [
        (learner, "Policy learner"),
        (other_learner, "Other policy learner"),
    ] {
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user,
                    display_name: display_name.to_string(),
                    roster_contact: None,
                },
            )
            .await
            .expect("learner membership");
    }
    let learner_student = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("learner membership lookup")
        .expect("learner active membership")
        .student
        .expect("learner student identity");
    let other_student = store
        .get_current_course_membership(context, course, other_learner)
        .await
        .expect("other membership lookup")
        .expect("other active membership")
        .student
        .expect("other learner identity");
    let reference = publish_assignment_version_with_timing(
        store,
        context,
        tenant,
        instructor,
        99_010,
        PublicationScope::Public,
        timing_policy,
    )
    .await;
    let assignment_a = AssignmentId::from_uuid(uuid(99_011));
    let assignment_b = AssignmentId::from_uuid(uuid(99_012));
    let mut revision = store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: assignment_a,
                tenant,
                course_id: course,
                title: "Policy target".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Draft,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment A")
        .revision;
    store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: assignment_b,
                tenant,
                course_id: course,
                title: "Grant source".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Draft,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment B");

    let schedule_group = CourseGroupId::from_uuid(uuid(99_020));
    let accommodation_group = CourseGroupId::from_uuid(uuid(99_021));
    for (id, purpose, title) in [
        (
            schedule_group,
            question_model::CourseGroupPurpose::Section,
            "Unapproved schedule scope",
        ),
        (
            accommodation_group,
            question_model::CourseGroupPurpose::Accommodation,
            "Unapproved accommodation scope",
        ),
    ] {
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id,
                        tenant,
                        course,
                        purpose,
                        title: title.to_string(),
                        members: Vec::new(),
                    },
                },
            )
            .await
            .expect("unapproved policy group");
    }
    revision = store
        .put_group_schedule_offset(
            context,
            PutGroupScheduleOffsetCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                offset: GroupScheduleOffset {
                    group: schedule_group,
                    offset_seconds: ScheduleOffsetSeconds::try_new(60)
                        .expect("nonzero schedule offset"),
                },
            },
        )
        .await
        .expect("unapproved schedule record");
    revision = store
        .put_group_accommodation(
            context,
            PutGroupAccommodationCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                accommodation: GroupAccommodation {
                    group: accommodation_group,
                    mode: PolicyModificationMode::Override,
                    patch: PolicyPatchSet {
                        available_at: PolicyPatch::Unrestricted,
                        ..PolicyPatchSet::INHERIT
                    },
                },
            },
        )
        .await
        .expect("unapproved accommodation record");
    revision = store
        .put_individual_policy_exception(
            context,
            PutIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(uuid(99_022)),
                    exception: IndividualPolicyException {
                        student: other_student,
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet {
                            available_at: PolicyPatch::Unrestricted,
                            ..PolicyPatchSet::INHERIT
                        },
                    },
                },
            },
        )
        .await
        .expect("other-student exception");
    let closed = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_590_800_000)),
                        due_at: None,
                        closes_at: None,
                        time_limit_seconds: None,
                        attempt_limit: None,
                        late_submission: question_model::LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
                },
            },
        )
        .await
        .expect("future policy");

    let grant_b = store
        .evaluate_assignment_entitlement(context, learner, course, assignment_b)
        .await
        .expect("assignment B entitlement");
    assert!(matches!(
        grant_b,
        domain::entitlement::EntitlementDecision::Granted(_)
    ));
    let lifecycle_denied = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment: assignment_a,
                entitlement: grant_b.clone(),
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(0),
                prior_run_count: 0,
            },
        )
        .await
        .expect("lifecycle denial must not inspect policy records")
        .expect("assignment exists");
    assert!(matches!(
        lifecycle_denied.decision,
        EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(AssignmentLifecycleDenial::NotPublished),
        }
    ));
    let authorization_denied = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment: assignment_a,
                entitlement: grant_b.clone(),
                authorization: AuthorizationGate::Denied(AuthorizationDenial::ActionNotPermitted),
                now: ActivityTimestamp::from_unix_millis(0),
                prior_run_count: 0,
            },
        )
        .await
        .expect("stored lifecycle denial must precede caller-supplied authorization")
        .expect("assignment exists");
    assert!(matches!(
        authorization_denied.decision,
        EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(AssignmentLifecycleDenial::NotPublished),
        }
    ));
    assert!(matches!(
        store
            .resolve_effective_policy(
                context,
                ResolveEffectivePolicyCommand {
                    assignment: assignment_a,
                    entitlement: grant_b,
                    authorization: AuthorizationGate::Authorized,
                    now: ActivityTimestamp::from_unix_millis(0),
                    prior_run_count: 0,
                },
            )
            .await
            .map(|value| value.expect("assignment exists").decision),
        Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(AssignmentLifecycleDenial::NotPublished),
        })
    ));

    let proposed_run = RunId::from_uuid(uuid(99_030));
    assert!(
        store
            .student_get_enrollment_for_assignment(context, learner, assignment_a)
            .await
            .expect("enrollment lookup before denied start")
            .is_none()
    );
    assert!(matches!(
        store
            .start_or_resume_run(
                context,
                learner,
                StudentWorkRoutingBinding::new(course, assignment_a),
                proposed_run,
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        store
            .student_get_enrollment_for_assignment(context, learner, assignment_a)
            .await
            .expect("enrollment lookup after denied start")
            .is_none()
    );
    assert!(
        store
            .get_run(context, proposed_run)
            .await
            .expect("proposed run lookup")
            .is_none()
    );

    // These unrelated M2/M3 records remain stored. An S5-entitled learner
    // whose grant excludes both scopes can still resolve, start, and issue.
    // This proves action paths consume S5's scope boundary, not every row.
    let revision = closed.revision;
    let revision = store
        .put_individual_policy_exception(
            context,
            PutIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(uuid(99_023)),
                    exception: IndividualPolicyException {
                        student: learner_student,
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet {
                            time_limit_seconds: PolicyPatch::Set(
                                NonZeroU32::new(300).expect("positive M4 limit"),
                            ),
                            ..PolicyPatchSet::INHERIT
                        },
                    },
                },
            },
        )
        .await
        .expect("store applicable M4 exception");
    let open = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: assignment_a,
                expected_revision: revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: None,
                        due_at: None,
                        closes_at: None,
                        time_limit_seconds: Some(NonZeroU32::new(120).expect("positive limit")),
                        attempt_limit: None,
                        late_submission: question_model::LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
                },
            },
        )
        .await
        .expect("open current policy");
    let allowed_grant = store
        .evaluate_assignment_entitlement(context, learner, course, assignment_a)
        .await
        .expect("assignment A entitlement");
    let allowed = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment: assignment_a,
                entitlement: allowed_grant,
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(0),
                prior_run_count: 0,
            },
        )
        .await
        .expect("allowed resolution")
        .expect("assignment exists");
    assert!(matches!(
        allowed.decision,
        EffectivePolicyDecision::Allowed {
            start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
            ..
        }
    ));
    assert_eq!(allowed.revision, open.revision);

    let run = store
        .start_or_resume_run(
            context,
            learner,
            StudentWorkRoutingBinding::new(course, assignment_a),
            RunId::from_uuid(uuid(99_031)),
        )
        .await
        .expect("allowed start materializes exactly one run");
    let issued_question_snapshot = store
        .get_catalog_problem(context, reference)
        .await
        .expect("read the published policy question")
        .expect("published policy question exists")
        .question;
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        issued_question_snapshot,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct exact policy native issued snapshot");
    let issued = store
        .issue_or_resume_question_attempt(
            context,
            policy_issue(
                learner,
                course,
                assignment_a,
                run.id,
                question_model::QuestionAttemptId::from_uuid(uuid(99_032)),
                reference,
                issued_question_snapshot,
            ),
        )
        .await
        .expect("allowed issue stores a sealed policy receipt");
    let receipt = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("receipt read")
        .expect("issued attempt has a policy receipt");
    assert_eq!(receipt.generation, 1);
    assert_eq!(
        receipt.policy.time_limit_seconds.value,
        Some(NonZeroU32::new(300).expect("positive M4 limit"))
    );
    assert_eq!(
        receipt.policy.time_limit_seconds.source,
        PolicySource::IndividualException(learner_student)
    );
    EffectivePolicyFixture {
        context,
        instructor,
        course,
        assignment: assignment_a,
        assignment_revision: open.revision,
        attempt: issued.id,
        receipt,
    }
}

#[tokio::test]
async fn memory_effective_policy_gate_and_materialization_conformance() {
    let store = MemoryStore::default();
    super::effective_policy_parity::exercise_effective_policy_resolution_parity(&store).await;
    let fixture = exercise_effective_policy_gate_and_materialization_contract(&store).await;
    let current = store
        .get_base_assignment_policy(fixture.context, fixture.assignment)
        .await
        .expect("read current authored policy")
        .expect("authored policy exists");
    store
        .put_assignment_teaching_settings(
            fixture.context,
            PutAssignmentTeachingSettingsCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: current.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        time_limit_seconds: Some(
                            NonZeroU32::new(180).expect("positive edit limit"),
                        ),
                        ..current.policy
                    },
                },
            },
        )
        .await
        .expect("edit future policy");
    let current = store
        .get_issued_effective_policy_receipt(fixture.context, fixture.attempt)
        .await
        .expect("read re-resolved current receipt")
        .expect("active attempt keeps a current receipt");
    assert_eq!(current.generation, fixture.receipt.generation + 1);
    assert_eq!(
        current.policy.time_limit_seconds.source,
        fixture.receipt.policy.time_limit_seconds.source
    );
    let persisted = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("read assignment after policy re-resolution")
        .expect("assignment remains persisted");
    let focused = store
        .replace_assignment_policies(
            fixture.context,
            ReplaceAssignmentPoliciesCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: persisted.revision,
                update: AssignmentPoliciesUpdate {
                    audience: persisted.record.audience.clone(),
                    disclosure_policy: persisted.record.disclosure_policy,
                    policies: persisted.record.policies,
                    teaching_settings: question_model::AssignmentTeachingSettings {
                        lifecycle: persisted.record.lifecycle,
                        instructions: persisted.record.instructions.clone(),
                        base_policy: persisted.base_policy,
                    },
                },
            },
        )
        .await
        .expect("focused policy slice");
    let focused = match focused {
        ReplaceAssignmentPoliciesOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected focused policy outcome: {other:?}"),
    };
    assert_eq!(focused.revision.value(), persisted.revision.value() + 1);
    assert_eq!(focused.record.items, persisted.record.items);
}

#[tokio::test]
async fn memory_start_rejects_valid_assignment_bound_to_different_course_without_mutation() {
    let store = MemoryStore::default();
    let fixture = exercise_effective_policy_gate_and_materialization_contract(&store).await;
    let learner = UserId::from_uuid(uuid(99_002));
    let asserted_course = CourseId::from_uuid(uuid(99_040));
    let proposed_run = RunId::from_uuid(uuid(99_041));
    let asserted_course_creation_authority = sysadmin_course_creation_authority(
        &store,
        fixture.context.tenant_id(),
        asserted_course,
        fixture.instructor,
    )
    .await;

    store
        .create_course(
            fixture.context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: asserted_course,
                    title: "Unrelated asserted course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid asserted course term"),
                },
                authority: asserted_course_creation_authority,
            },
        )
        .await
        .expect("asserted course");

    let existing_enrollment = store
        .student_get_enrollment_for_assignment(fixture.context, learner, fixture.assignment)
        .await
        .expect("existing enrollment lookup")
        .expect("fixture enrollment");
    let existing_run_id = RunId::from_uuid(uuid(99_031));
    let existing_run = store
        .get_run(fixture.context, existing_run_id)
        .await
        .expect("existing run lookup")
        .expect("fixture run");

    assert_eq!(
        store
            .start_or_resume_run(
                fixture.context,
                learner,
                StudentWorkRoutingBinding::new(asserted_course, fixture.assignment),
                proposed_run,
            )
            .await,
        Err(StoreError::NotFound),
        "course authorization must conceal an assignment from a non-member",
    );

    assert_eq!(
        store
            .student_get_enrollment_for_assignment(fixture.context, learner, fixture.assignment)
            .await
            .expect("enrollment lookup after rejected start"),
        Some(existing_enrollment),
        "rejected routing must not mutate the existing enrollment",
    );
    assert_eq!(
        store
            .get_run(fixture.context, existing_run_id)
            .await
            .expect("existing run lookup after rejected start"),
        Some(existing_run),
        "rejected routing must not mutate the existing run",
    );
    assert_eq!(
        store
            .get_run(fixture.context, proposed_run)
            .await
            .expect("proposed run lookup after rejected start"),
        None,
        "rejected routing must not create the proposed run",
    );
}
