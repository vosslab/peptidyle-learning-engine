//! Public active-attempt re-resolution oracle for group and M2--M4 changes.

use super::*;
use domain::effective_assignment_policy::{
    GroupAccommodation, GroupScheduleOffset, IndividualPolicyException, PolicyModificationMode,
    PolicyPatch, PolicyPatchSet, PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::{
    AssignmentUpdate, DeleteGroupAccommodationCommand, DeleteGroupScheduleOffsetCommand,
    DeleteIndividualPolicyExceptionCommand, PutGroupAccommodationCommand,
    PutGroupScheduleOffsetCommand, PutIndividualPolicyExceptionCommand,
    StoredIndividualPolicyException,
};
use question_model::{AssignmentAudience, AssignmentPolicyExceptionId, AttemptStatus};

pub(crate) async fn exercise_active_group_reresolution_contract<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
) where
    S: Store + CatalogStore + CourseGroupManagementStore + CourseRosterStore + SessionStore,
{
    let learner = UserId::from_uuid(uuid(99_002));
    let membership = store
        .get_current_course_membership(fixture.context, fixture.course, learner)
        .await
        .expect("current learner membership")
        .expect("active learner membership");
    let student = membership.student.expect("student identity");
    let schedule = CourseGroupId::from_uuid(uuid(99_020));
    let accommodation = CourseGroupId::from_uuid(uuid(99_021));

    // The issued public receipt is generation one.  The public Store API has
    // no generation-addressed history read; PostgreSQL live SQL owns physical
    // generation-retention proof.  Keep the value as immutable evidence here.
    let first = store
        .get_issued_effective_policy_receipt(fixture.context, fixture.attempt)
        .await
        .expect("read initial receipt")
        .expect("initial current receipt");
    assert_eq!(first.generation, 1);
    assert_eq!(first, fixture.receipt);

    let mut base = store
        .get_base_assignment_policy(fixture.context, fixture.assignment)
        .await
        .expect("base policy")
        .expect("stored base policy");
    base.policy.due_at = Some(ActivityTimestamp::from_unix_millis(1_787_590_800_000));
    store
        .put_assignment_teaching_settings(
            fixture.context,
            learning_data_access::PutAssignmentTeachingSettingsCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: base.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: base.policy,
                },
            },
        )
        .await
        .expect("M1 preparation re-resolves active attempt");

    set_group_members(store, fixture, schedule, vec![membership.id]).await;
    let after_m2 = current_receipt(store, fixture).await;
    assert!(after_m2.generation > first.generation);
    assert_eq!(
        after_m2.policy.due_at.source,
        PolicySource::GroupScheduleOffsets(vec![schedule])
    );
    assert_in_progress(store, fixture, learner).await;
    let after_direct_m2_put = put_schedule_offset(store, fixture, schedule, 120).await;
    assert!(after_direct_m2_put.generation > after_m2.generation);
    assert_ne!(after_direct_m2_put.policy.due_at, after_m2.policy.due_at);
    assert_eq!(
        after_direct_m2_put.policy.due_at.source,
        PolicySource::GroupScheduleOffsets(vec![schedule])
    );
    assert_in_progress(store, fixture, learner).await;
    let after_direct_m2_delete = delete_schedule_offset(store, fixture, schedule).await;
    assert!(after_direct_m2_delete.generation > after_direct_m2_put.generation);
    assert_eq!(
        after_direct_m2_delete.policy.due_at.source,
        PolicySource::Base
    );
    assert_in_progress(store, fixture, learner).await;
    let restored_m2 = put_schedule_offset(store, fixture, schedule, 60).await;
    assert!(restored_m2.generation > after_direct_m2_delete.generation);
    assert_eq!(
        restored_m2.policy.due_at.source,
        PolicySource::GroupScheduleOffsets(vec![schedule])
    );
    assert_in_progress(store, fixture, learner).await;
    set_group_members(store, fixture, schedule, Vec::new()).await;
    let without_m2 = current_receipt(store, fixture).await;
    assert!(without_m2.generation > restored_m2.generation);
    assert_eq!(without_m2.policy.due_at.source, PolicySource::Base);

    set_group_members(store, fixture, accommodation, vec![membership.id]).await;
    let after_m3 = current_receipt(store, fixture).await;
    assert!(after_m3.generation > without_m2.generation);
    assert_eq!(
        after_m3.policy.available_at.source,
        PolicySource::GroupAccommodations(vec![accommodation])
    );
    let after_direct_m3_put = put_accommodation(store, fixture, accommodation).await;
    assert!(after_direct_m3_put.generation > after_m3.generation);
    assert_ne!(
        after_direct_m3_put.policy.available_at,
        after_m3.policy.available_at
    );
    assert_eq!(
        after_direct_m3_put.policy.available_at.source,
        PolicySource::GroupAccommodations(vec![accommodation])
    );
    assert_in_progress(store, fixture, learner).await;
    let after_direct_m3_delete = delete_accommodation(store, fixture, accommodation).await;
    assert!(after_direct_m3_delete.generation > after_direct_m3_put.generation);
    assert_eq!(
        after_direct_m3_delete.policy.available_at.source,
        PolicySource::Base
    );
    assert_in_progress(store, fixture, learner).await;
    let restored_m3 = restore_accommodation(store, fixture, accommodation).await;
    assert!(restored_m3.generation > after_direct_m3_delete.generation);
    assert_eq!(
        restored_m3.policy.available_at.source,
        PolicySource::GroupAccommodations(vec![accommodation])
    );
    assert_in_progress(store, fixture, learner).await;
    set_group_members(store, fixture, accommodation, Vec::new()).await;
    let without_m3 = current_receipt(store, fixture).await;
    assert!(without_m3.generation > restored_m3.generation);
    assert_eq!(without_m3.policy.available_at.source, PolicySource::Base);

    let revision = assignment_revision(store, fixture).await;
    store
        .put_individual_policy_exception(
            fixture.context,
            PutIndividualPolicyExceptionCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: revision,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(uuid(99_220)),
                    exception: IndividualPolicyException {
                        student,
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
        .expect("M4 override");
    let after_m4 = current_receipt(store, fixture).await;
    assert!(after_m4.generation > without_m3.generation);
    assert_eq!(
        after_m4.policy.available_at.source,
        PolicySource::IndividualException(student)
    );
    let revision = assignment_revision(store, fixture).await;
    store
        .delete_individual_policy_exception(
            fixture.context,
            DeleteIndividualPolicyExceptionCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: revision,
                student,
            },
        )
        .await
        .expect("M4 removal");
    let without_m4 = current_receipt(store, fixture).await;
    assert!(without_m4.generation > after_m4.generation);
    assert_eq!(without_m4.policy.available_at.source, PolicySource::Base);

    assert_atomic_refusals(store, fixture, learner, schedule, accommodation).await;
    assert_audience_revocation_terminalizes_current_attempt(store, fixture, learner, membership.id)
        .await;
}

async fn put_schedule_offset<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
    seconds: i32,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    store
        .put_group_schedule_offset(
            fixture.context,
            PutGroupScheduleOffsetCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment_revision(store, fixture).await,
                offset: GroupScheduleOffset {
                    group,
                    offset_seconds: ScheduleOffsetSeconds::try_new(seconds).expect("offset"),
                },
            },
        )
        .await
        .expect("direct M2 put");
    current_receipt(store, fixture).await
}

async fn delete_schedule_offset<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    store
        .delete_group_schedule_offset(
            fixture.context,
            DeleteGroupScheduleOffsetCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment_revision(store, fixture).await,
                group,
            },
        )
        .await
        .expect("direct M2 delete");
    current_receipt(store, fixture).await
}

async fn put_accommodation<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    put_group_accommodation(
        store,
        fixture,
        group,
        PolicyPatch::Set(ActivityTimestamp::from_unix_millis(0)),
    )
    .await
}

async fn restore_accommodation<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    put_group_accommodation(store, fixture, group, PolicyPatch::Unrestricted).await
}

async fn put_group_accommodation<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
    available_at: PolicyPatch<ActivityTimestamp>,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    store
        .put_group_accommodation(
            fixture.context,
            PutGroupAccommodationCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment_revision(store, fixture).await,
                accommodation: GroupAccommodation {
                    group,
                    mode: PolicyModificationMode::Override,
                    patch: PolicyPatchSet {
                        available_at,
                        ..PolicyPatchSet::INHERIT
                    },
                },
            },
        )
        .await
        .expect("direct M3 put");
    current_receipt(store, fixture).await
}

async fn delete_accommodation<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    store
        .delete_group_accommodation(
            fixture.context,
            DeleteGroupAccommodationCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment_revision(store, fixture).await,
                group,
            },
        )
        .await
        .expect("direct M3 delete");
    current_receipt(store, fixture).await
}

async fn set_group_members<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    group: CourseGroupId,
    members: Vec<question_model::CourseMembershipId>,
) where
    S: Store + CourseGroupManagementStore,
{
    let mut current = store
        .get_course_group(fixture.context, group)
        .await
        .expect("group read")
        .expect("fixture group");
    current.record.members = members;
    store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: Some(current.revision),
                record: current.record,
            },
        )
        .await
        .expect("group membership CAS");
}

async fn current_receipt<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
) -> learning_data_access::IssuedEffectivePolicyReceipt
where
    S: Store,
{
    store
        .get_issued_effective_policy_receipt(fixture.context, fixture.attempt)
        .await
        .expect("current receipt read")
        .expect("active attempt current receipt")
}

async fn assignment_revision<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
) -> learning_data_access::AssignmentRevision
where
    S: Store,
{
    store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment edit read")
        .expect("assignment")
        .revision
}

async fn assert_in_progress<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    learner: UserId,
) where
    S: Store,
{
    assert_eq!(
        store
            .student_get_question_attempt(fixture.context, learner, fixture.attempt)
            .await
            .expect("learner attempt read")
            .expect("learner retains active attempt")
            .status,
        AttemptStatus::InProgress
    );
}

async fn assert_atomic_refusals<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    learner: UserId,
    schedule: CourseGroupId,
    accommodation: CourseGroupId,
) where
    S: Store + CourseGroupManagementStore,
{
    let before = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("stored assignment");
    // Establish a newer revision so the following delete has a reachable stale
    // CAS token, without manufacturing an unreachable resolver failure.
    store
        .put_group_schedule_offset(
            fixture.context,
            PutGroupScheduleOffsetCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: before.revision,
                offset: GroupScheduleOffset {
                    group: schedule,
                    offset_seconds: ScheduleOffsetSeconds::try_new(31).expect("offset"),
                },
            },
        )
        .await
        .expect("valid modifier update before stale CAS");
    let assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("stored assignment");
    let group = store
        .get_course_group(fixture.context, schedule)
        .await
        .expect("group")
        .expect("schedule group");
    let attempt = store
        .student_get_question_attempt(fixture.context, learner, fixture.attempt)
        .await
        .expect("attempt")
        .expect("attempt");
    let receipt = current_receipt(store, fixture).await;
    let wrong_purpose = store
        .put_group_schedule_offset(
            fixture.context,
            PutGroupScheduleOffsetCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: before.revision,
                offset: GroupScheduleOffset {
                    group: accommodation,
                    offset_seconds: ScheduleOffsetSeconds::try_new(30).expect("offset"),
                },
            },
        )
        .await;
    assert!(matches!(wrong_purpose, Err(StoreError::InvalidRecord(_))));
    let stale = store
        .delete_group_schedule_offset(
            fixture.context,
            DeleteGroupScheduleOffsetCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: before.revision,
                group: schedule,
            },
        )
        .await;
    assert_eq!(stale, Err(StoreError::Conflict));
    let missing = store
        .put_individual_policy_exception(
            fixture.context,
            PutIndividualPolicyExceptionCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment.revision,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(uuid(99_221)),
                    exception: IndividualPolicyException {
                        student: question_model::StudentId::from_uuid(uuid(99_222)),
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet::INHERIT,
                    },
                },
            },
        )
        .await;
    assert_eq!(missing, Err(StoreError::NotFound));
    assert_eq!(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment"),
        Some(assignment)
    );
    assert_eq!(
        store
            .get_course_group(fixture.context, schedule)
            .await
            .expect("group"),
        Some(group)
    );
    assert_eq!(
        store
            .student_get_question_attempt(fixture.context, learner, fixture.attempt)
            .await
            .expect("attempt"),
        Some(attempt)
    );
    assert_eq!(current_receipt(store, fixture).await, receipt);
}

async fn assert_audience_revocation_terminalizes_current_attempt<S>(
    store: &S,
    fixture: &super::super::effective_policy::EffectivePolicyFixture,
    learner: UserId,
    membership: question_model::CourseMembershipId,
) where
    S: Store + CourseGroupManagementStore,
{
    let audience_group = CourseGroupId::from_uuid(uuid(99_230));
    let created = store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: audience_group,
                    tenant: fixture.context.tenant_id(),
                    course: fixture.course,
                    purpose: question_model::CourseGroupPurpose::Section,
                    title: "Active audience".into(),
                    members: vec![membership],
                },
            },
        )
        .await
        .expect("audience group");
    let current = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("assignment");
    let assigned = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: current.revision,
                update: assignment_update(
                    &current.record,
                    AssignmentAudience::any_of_groups(vec![audience_group])
                        .expect("one audience group"),
                ),
            },
        )
        .await
        .expect("audience narrowing keeps member active");
    let before_removal = current_receipt(store, fixture).await;
    assert!(before_removal.generation > 1);
    let mut changed = created.record;
    changed.members.clear();
    store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: Some(created.revision),
                record: changed,
            },
        )
        .await
        .expect("group removal re-resolves audience attempt");
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(fixture.context, fixture.attempt)
            .await
            .expect("receipt read"),
        None
    );
    assert_eq!(
        store
            .get_question_attempt(fixture.context, fixture.attempt)
            .await
            .expect("attempt read")
            .expect("terminal attempt")
            .status,
        AttemptStatus::AutoSubmitted
    );
    assert_eq!(
        store
            .start_or_resume_run(
                fixture.context,
                learner,
                StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
                RunId::from_uuid(uuid(99_231))
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert!(assigned.revision.value() > 0);
}

fn assignment_update(record: &AssignmentRecord, audience: AssignmentAudience) -> AssignmentUpdate {
    AssignmentUpdate {
        title: record.title.clone(),
        audience,
        items: record.items.clone(),
        selection_groups: record.selection_groups.clone(),
        disclosure_policy: record.disclosure_policy,
        policies: record.policies,
    }
}
