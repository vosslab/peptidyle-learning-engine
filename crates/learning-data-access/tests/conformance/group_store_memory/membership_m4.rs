//! Revoked-membership and M4 guards shared by Store conformance backends.

use super::*;
use domain::effective_assignment_policy::{
    AuthorizationGate, IndividualPolicyException, PolicyModificationMode, PolicyPatch,
    PolicyPatchSet,
};
use learning_data_access::{
    AssignmentRevision, DeleteIndividualPolicyExceptionCommand,
    PutIndividualPolicyExceptionCommand, RevokeCourseMember, SessionLifetime, SessionStore,
    SessionSubject, StoredIndividualPolicyException,
};
use question_model::StudentId;

pub(crate) async fn assert_individual_policy_exception_membership_guards<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
) where
    S: Store + CatalogStore + CourseGroupManagementStore + CourseRosterStore + SessionStore,
{
    assert_revoked_membership_group_and_policy_guards(store).await;
    assert_m4_student_membership_guards(store, fixture).await;
}

async fn assert_revoked_membership_group_and_policy_guards<S>(store: &S)
where
    S: Store + CourseGroupManagementStore + CourseRosterStore + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(720_300));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(720_301));
    let student = UserId::from_uuid(uuid(720_302));
    let outsider = UserId::from_uuid(uuid(720_303));
    let course = CourseId::from_uuid(uuid(720_304));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Revoked group guard".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    let member = store
        .upsert_course_member(
            context,
            learning_data_access::UpsertCourseMember {
                course,
                user: student,
                display_name: "Revoked student".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership");
    let episode = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("student lookup")
        .expect("active student")
        .id;

    for actor in [student, outsider] {
        for purpose in [
            question_model::CourseGroupPurpose::Section,
            question_model::CourseGroupPurpose::Lab,
            question_model::CourseGroupPurpose::Cohort,
            question_model::CourseGroupPurpose::Accommodation,
            question_model::CourseGroupPurpose::Work,
        ] {
            assert_eq!(
                store
                    .get_course_group_purpose_policy(context, actor, course, purpose)
                    .await,
                Err(StoreError::NotFound)
            );
        }
        assert_eq!(
            store
                .update_course_group_purpose_policy(
                    context,
                    UpdateCourseGroupPurposePolicyCommand {
                        actor,
                        course,
                        expected_revision: CourseGroupPurposePolicyRevision::INITIAL,
                        policy: question_model::CourseGroupPurposePolicy::default_for_purpose(
                            question_model::CourseGroupPurpose::Section,
                        ),
                    },
                )
                .await,
            Err(StoreError::NotFound)
        );
    }
    let explicit_policy = store
        .get_course_group_purpose_policy(
            context,
            instructor,
            course,
            question_model::CourseGroupPurpose::Section,
        )
        .await
        .expect("instructor policy lookup")
        .expect("initialized policy is explicit and fail closed when missing");
    assert_eq!(
        explicit_policy.policy,
        question_model::CourseGroupPurposePolicy::default_for_purpose(
            question_model::CourseGroupPurpose::Section,
        )
    );

    let instructor_session = SessionTokenHash::compute(b"revoked-group-instructor");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "Revoked group instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    store
        .revoke_course_member(
            context,
            instructor_session,
            RevokeCourseMember {
                course,
                member: member.member.id,
                expected_revision: member.roster_revision,
            },
        )
        .await
        .expect("revoke exact membership episode");

    let rejected = CourseGroupRecord {
        id: CourseGroupId::from_uuid(uuid(720_305)),
        tenant,
        course,
        purpose: question_model::CourseGroupPurpose::Section,
        title: "Revoked episode".into(),
        members: vec![episode],
    };
    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: rejected.clone(),
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(store.get_course_group(context, rejected.id).await, Ok(None));
    assert!(
        store
            .list_course_groups(
                context,
                instructor,
                course,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .expect("authorized group list")
            .items
            .iter()
            .all(|view| view.group.record.id != rejected.id)
    );
}

async fn assert_m4_student_membership_guards<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
) where
    S: Store + CatalogStore + CourseGroupManagementStore + CourseRosterStore + SessionStore,
{
    let tenant = fixture.context.tenant_id();
    let learner = UserId::from_uuid(uuid(99_002));
    let current_student = store
        .get_current_course_membership(fixture.context, fixture.course, learner)
        .await
        .expect("current learner lookup")
        .expect("current learner")
        .student
        .expect("student identity");
    let baseline_revision = assignment_revision(store, fixture.context, fixture.assignment).await;
    let baseline_policy = resolved_policy(store, fixture, learner).await;

    let missing = StudentId::from_uuid(uuid(99_100));
    assert_invalid_m4_preserves_state(
        store,
        fixture,
        learner,
        baseline_revision,
        baseline_policy.clone(),
        missing,
        99_101,
    )
    .await;

    let revoked_user = UserId::from_uuid(uuid(99_102));
    let revoked = store
        .upsert_course_member(
            fixture.context,
            learning_data_access::UpsertCourseMember {
                course: fixture.course,
                user: revoked_user,
                display_name: "Revoked M4 student".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("revoked-member setup");
    let revoked_student = store
        .get_current_course_membership(fixture.context, fixture.course, revoked_user)
        .await
        .expect("revoked student lookup")
        .expect("active revoked setup")
        .student
        .expect("student identity");
    let session = SessionTokenHash::compute(b"m4-revoked-student-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                fixture.instructor,
                "M4 instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    store
        .revoke_course_member(
            fixture.context,
            session,
            RevokeCourseMember {
                course: fixture.course,
                member: revoked.member.id,
                expected_revision: revoked.roster_revision,
            },
        )
        .await
        .expect("revoke inactive M4 student");
    assert_invalid_m4_preserves_state(
        store,
        fixture,
        learner,
        baseline_revision,
        baseline_policy.clone(),
        revoked_student,
        99_103,
    )
    .await;

    let other_course = CourseId::from_uuid(uuid(99_104));
    let foreign_user = UserId::from_uuid(uuid(99_105));
    store
        .create_course(
            fixture.context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: other_course,
                    tenant,
                    title: "M4 foreign course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                initial_instructor: fixture.instructor,
            },
        )
        .await
        .expect("foreign course");
    store
        .upsert_course_member(
            fixture.context,
            learning_data_access::UpsertCourseMember {
                course: other_course,
                user: foreign_user,
                display_name: "Foreign M4 student".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("foreign student");
    let foreign_student = store
        .get_current_course_membership(fixture.context, other_course, foreign_user)
        .await
        .expect("foreign student lookup")
        .expect("foreign student active")
        .student
        .expect("student identity");
    assert_invalid_m4_preserves_state(
        store,
        fixture,
        learner,
        baseline_revision,
        baseline_policy.clone(),
        foreign_student,
        99_106,
    )
    .await;

    let valid = m4_command(fixture, current_student, 99_107, baseline_revision);
    let updated_revision = store
        .put_individual_policy_exception(fixture.context, valid)
        .await
        .expect("current same-course M4 put");
    assert!(updated_revision.value() > baseline_revision.value());
    let deleted_revision = store
        .delete_individual_policy_exception(
            fixture.context,
            DeleteIndividualPolicyExceptionCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: updated_revision,
                student: current_student,
            },
        )
        .await
        .expect("current same-course M4 delete");
    assert!(deleted_revision.value() > updated_revision.value());
}

async fn assert_invalid_m4_preserves_state<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    learner: UserId,
    expected_revision: AssignmentRevision,
    expected_policy: learning_data_access::EffectivePolicyResolution,
    student: StudentId,
    id: u128,
) where
    S: Store,
{
    assert_eq!(
        store
            .put_individual_policy_exception(
                fixture.context,
                m4_command(fixture, student, id, expected_revision),
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        assignment_revision(store, fixture.context, fixture.assignment).await,
        expected_revision
    );
    assert_eq!(
        resolved_policy(store, fixture, learner).await,
        expected_policy
    );
}

fn m4_command(
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    student: StudentId,
    id: u128,
    expected_revision: AssignmentRevision,
) -> PutIndividualPolicyExceptionCommand {
    PutIndividualPolicyExceptionCommand {
        actor: fixture.instructor,
        course: fixture.course,
        assignment: fixture.assignment,
        expected_revision,
        exception: StoredIndividualPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(uuid(id)),
            exception: IndividualPolicyException {
                student,
                mode: PolicyModificationMode::Override,
                patch: PolicyPatchSet {
                    available_at: PolicyPatch::Unrestricted,
                    ..PolicyPatchSet::INHERIT
                },
            },
        },
    }
}

async fn assignment_revision<S>(
    store: &S,
    context: TenantContext,
    assignment: AssignmentId,
) -> AssignmentRevision
where
    S: Store,
{
    store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment lookup")
        .expect("assignment exists")
        .revision
}

async fn resolved_policy<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    learner: UserId,
) -> learning_data_access::EffectivePolicyResolution
where
    S: Store,
{
    let entitlement = store
        .evaluate_assignment_entitlement(
            fixture.context,
            learner,
            fixture.course,
            fixture.assignment,
        )
        .await
        .expect("current entitlement");
    store
        .resolve_effective_policy(
            fixture.context,
            learning_data_access::ResolveEffectivePolicyCommand {
                assignment: fixture.assignment,
                entitlement,
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(0),
                prior_run_count: 0,
            },
        )
        .await
        .expect("policy resolution")
        .expect("assignment policy")
}
