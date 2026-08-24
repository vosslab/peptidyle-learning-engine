//! Public Store conformance for the WP-PROF-T2 group-management capability.
//!
//! PostgreSQL calls this same helper once it implements
//! `CourseGroupManagementStore`; Memory is the first backend to execute it.

use super::effective_policy::exercise_effective_policy_gate_and_materialization_contract;
use super::*;
use learning_data_access::{
    CatalogStore, CourseGroupManagementStore, SessionStore, TeachingAuthorityReferenceStore,
};

#[path = "group_store_memory/active_reresolution.rs"]
mod active_reresolution;
#[path = "group_store_memory/membership_m4.rs"]
mod membership_m4;
#[path = "group_store_memory/purpose_policy.rs"]
mod purpose_policy;
#[path = "group_store_memory/student_picker.rs"]
mod student_picker;

pub(crate) async fn exercise_course_group_management_contract<S>(store: &S)
where
    S: Store
        + CatalogStore
        + CourseGroupManagementStore
        + CourseRosterStore
        + SessionStore
        + TeachingAuthorityReferenceStore,
{
    let tenant = TenantId::from_uuid(uuid(720_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(720_001));
    let outsider = UserId::from_uuid(uuid(720_002));
    let course = CourseId::from_uuid(uuid(720_003));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Group management conformance".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("course");
    let instructor_session = SessionTokenHash::compute(b"group-management-conformance-instructor");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "Group management instructor",
                vec![UserRole::Instructor],
            )
            .expect("instructor session subject"),
            SessionLifetime::from_seconds(3_600).expect("positive instructor session lifetime"),
        )
        .await
        .expect("persisted instructor session");
    student_picker::assert_student_picker_reference_contract(
        store, context, instructor, outsider, course,
    )
    .await;
    assert_multiple_membership_warning_contract(store, context, instructor, tenant, course).await;
    purpose_policy::assert_all_default_policies(
        store,
        context,
        instructor,
        instructor_session,
        outsider,
        course,
    )
    .await;
    let groups = create_and_page_groups(store, context, instructor, tenant, course).await;
    assert_delete_cleans_reference(store, context, instructor, course, groups[0].clone()).await;
    assert_group_authorization_and_membership_guards(
        store,
        context,
        instructor,
        outsider,
        tenant,
        course,
        groups[1].clone(),
    )
    .await;
    assert_referenced_group_and_modifier_guards(store).await;
}

async fn assert_group_authorization_and_membership_guards<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    outsider: UserId,
    tenant: TenantId,
    course: CourseId,
    existing: learning_data_access::CourseGroupView,
) where
    S: Store
        + CourseGroupManagementStore
        + CourseRosterStore
        + TeachingAuthorityReferenceStore
        + SessionStore,
{
    let student = UserId::from_uuid(uuid(720_070));
    store
        .upsert_course_member(
            context,
            instructor,
            learning_data_access::UpsertCourseMember {
                course,
                user: student,
                display_name: "Group guard student".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership");
    let membership_record = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("student lookup")
        .expect("active student");
    let membership = membership_record.id;
    let group_with_member = CourseGroupRecord {
        members: vec![membership],
        ..existing.group.record.clone()
    };
    let updated_group = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(existing.group.revision),
                record: group_with_member,
            },
        )
        .await
        .expect("add active student to the exact group");
    let group_members = store
        .list_course_group_membership_reference_views(
            context,
            instructor,
            course,
            existing.reference,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("direct Instructor group-target projection");
    assert_eq!(updated_group.record.members, vec![membership]);
    assert_eq!(group_members.items.len(), 1);
    assert_eq!(group_members.items[0].display_name, "Group guard student");
    assert_eq!(
        group_members.items[0].role,
        question_model::CourseMembershipRole::Student
    );
    let target = store
        .resolve_active_student_target_reference(
            context,
            instructor,
            course,
            group_members.items[0].reference,
        )
        .await
        .expect("authorized student target resolution")
        .expect("active Student membership is targetable");
    assert_eq!(target.course, course);
    assert_eq!(target.membership, membership);
    assert_eq!(target.user, student);
    assert_eq!(
        target.student,
        membership_record.student.expect("student identity")
    );
    assert_eq!(
        store
            .active_student_membership_reference_view(context, instructor, course, target.student)
            .await
            .expect("authorized Student reverse projection"),
        Some(group_members.items[0].clone()),
    );
    assert_eq!(
        store
            .get_course_group_by_id_for_instructor(
                context,
                instructor,
                course,
                updated_group.record.id,
            )
            .await
            .expect("authorized group reverse projection")
            .expect("group remains in exact course")
            .reference,
        existing.reference,
    );
    assert!(matches!(
        store
            .resolve_active_student_target_reference(
                context,
                outsider,
                course,
                group_members.items[0].reference,
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let duplicate = CourseGroupRecord {
        id: CourseGroupId::from_uuid(uuid(720_071)),
        tenant,
        course,
        purpose: question_model::CourseGroupPurpose::Section,
        title: "Duplicate members".into(),
        members: vec![membership, membership],
    };
    assert!(matches!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: duplicate.clone(),
                }
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store.get_course_group(context, duplicate.id).await,
        Ok(None)
    );

    for actor in [outsider, student] {
        assert_eq!(
            store
                .list_course_groups(
                    context,
                    actor,
                    course,
                    PageRequest::first(PageSize::new(1).expect("size"))
                )
                .await,
            Err(StoreError::NotFound)
        );
        assert_eq!(
            store
                .get_course_group_by_reference(context, actor, course, existing.reference)
                .await,
            Err(StoreError::NotFound)
        );
        assert_eq!(
            store
                .delete_course_group(
                    context,
                    actor,
                    course,
                    existing.group.record.id,
                    existing.group.revision
                )
                .await,
            Err(StoreError::NotFound)
        );
    }
    let second_course = CourseId::from_uuid(uuid(720_072));
    let second_course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, second_course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: second_course,
                    tenant,
                    title: "Second group course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: second_course_creation_authority,
            },
        )
        .await
        .expect("second course");
    let other = UserId::from_uuid(uuid(720_073));
    store
        .upsert_course_member(
            context,
            instructor,
            learning_data_access::UpsertCourseMember {
                course: second_course,
                user: other,
                display_name: "Other course student".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("other member");
    let foreign_membership = store
        .get_current_course_membership(context, second_course, other)
        .await
        .expect("other lookup")
        .expect("other active")
        .id;
    let cross_course = CourseGroupRecord {
        id: CourseGroupId::from_uuid(uuid(720_074)),
        tenant,
        course,
        purpose: question_model::CourseGroupPurpose::Lab,
        title: "Cross course".into(),
        members: vec![foreign_membership],
    };
    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: cross_course.clone()
                }
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store.get_course_group(context, cross_course.id).await,
        Ok(None)
    );
    assert_eq!(
        store
            .get_course_group_by_reference(context, instructor, second_course, existing.reference)
            .await,
        Ok(None)
    );
}

async fn assert_referenced_group_and_modifier_guards<S>(store: &S)
where
    S: Store + CatalogStore + CourseGroupManagementStore + CourseRosterStore + SessionStore,
{
    use domain::effective_assignment_policy::{
        GroupAccommodation, GroupScheduleOffset, PolicyModificationMode, PolicyPatch,
        PolicyPatchSet, ScheduleOffsetSeconds,
    };
    use learning_data_access::{PutGroupAccommodationCommand, PutGroupScheduleOffsetCommand};

    let fixture = exercise_effective_policy_gate_and_materialization_contract(store).await;
    active_reresolution::exercise_active_group_reresolution_contract(store, &fixture).await;
    let tenant = fixture.context.tenant_id();
    let assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("stored assignment");
    let audience_group = CourseGroupId::from_uuid(uuid(99_090));
    let work_group = CourseGroupId::from_uuid(uuid(99_091));
    for (id, purpose) in [
        (audience_group, question_model::CourseGroupPurpose::Section),
        (work_group, question_model::CourseGroupPurpose::Work),
    ] {
        store
            .put_course_group(
                fixture.context,
                PutCourseGroupCommand {
                    actor: fixture.instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id,
                        tenant,
                        course: fixture.course,
                        purpose,
                        title: format!("Guard {purpose:?}"),
                        members: Vec::new(),
                    },
                },
            )
            .await
            .expect("guard group");
    }
    let update = learning_data_access::AssignmentUpdate {
        title: assignment.record.title.clone(),
        audience: question_model::AssignmentAudience::any_of_groups(vec![audience_group])
            .expect("audience"),
        items: assignment.record.items.clone(),
        selection_groups: assignment.record.selection_groups.clone(),
        disclosure_policy: assignment.record.disclosure_policy,
        policies: assignment.record.policies,
    };
    let audience_assignment = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment.revision,
                update,
            },
        )
        .await
        .expect("audience update");
    let audience_view = store
        .get_course_group(fixture.context, audience_group)
        .await
        .expect("audience group")
        .expect("exists");
    let mut forbidden = audience_view.record.clone();
    forbidden.purpose = question_model::CourseGroupPurpose::Accommodation;
    assert_eq!(
        store
            .put_course_group(
                fixture.context,
                PutCourseGroupCommand {
                    actor: fixture.instructor,
                    expected_revision: Some(audience_view.revision),
                    record: forbidden
                }
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store
            .delete_course_group(
                fixture.context,
                fixture.instructor,
                fixture.course,
                audience_group,
                audience_view.revision
            )
            .await,
        Err(StoreError::Conflict)
    );

    let schedule_group = CourseGroupId::from_uuid(uuid(99_020));
    let schedule_view = store
        .get_course_group(fixture.context, schedule_group)
        .await
        .expect("schedule group")
        .expect("exists");
    let mut invalid_schedule = schedule_view.record.clone();
    invalid_schedule.purpose = question_model::CourseGroupPurpose::Work;
    assert_eq!(
        store
            .put_course_group(
                fixture.context,
                PutCourseGroupCommand {
                    actor: fixture.instructor,
                    expected_revision: Some(schedule_view.revision),
                    record: invalid_schedule
                }
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store
            .delete_course_group(
                fixture.context,
                fixture.instructor,
                fixture.course,
                schedule_group,
                schedule_view.revision
            )
            .await,
        Err(StoreError::Conflict)
    );

    let accommodation_group = CourseGroupId::from_uuid(uuid(99_021));
    let accommodation_view = store
        .get_course_group(fixture.context, accommodation_group)
        .await
        .expect("accommodation group")
        .expect("exists");
    let mut invalid_accommodation = accommodation_view.record.clone();
    invalid_accommodation.purpose = question_model::CourseGroupPurpose::Section;
    assert_eq!(
        store
            .put_course_group(
                fixture.context,
                PutCourseGroupCommand {
                    actor: fixture.instructor,
                    expected_revision: Some(accommodation_view.revision),
                    record: invalid_accommodation
                }
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store
            .delete_course_group(
                fixture.context,
                fixture.instructor,
                fixture.course,
                accommodation_group,
                accommodation_view.revision
            )
            .await,
        Err(StoreError::Conflict)
    );

    let work = store
        .get_course_group(fixture.context, work_group)
        .await
        .expect("work")
        .expect("work exists");
    let mut legal = work.record.clone();
    legal.purpose = question_model::CourseGroupPurpose::Section;
    let legal = store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: Some(work.revision),
                record: legal,
            },
        )
        .await
        .expect("legal purpose transition");
    assert_eq!(
        store
            .delete_course_group(
                fixture.context,
                fixture.instructor,
                fixture.course,
                work_group,
                work.revision
            )
            .await,
        Err(StoreError::Conflict)
    );

    let mut revision = audience_assignment.revision;
    for (id, purpose) in [
        (
            CourseGroupId::from_uuid(uuid(99_092)),
            question_model::CourseGroupPurpose::Lab,
        ),
        (
            CourseGroupId::from_uuid(uuid(99_093)),
            question_model::CourseGroupPurpose::Cohort,
        ),
    ] {
        store
            .put_course_group(
                fixture.context,
                PutCourseGroupCommand {
                    actor: fixture.instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id,
                        tenant,
                        course: fixture.course,
                        purpose,
                        title: format!("Schedule {purpose:?}"),
                        members: Vec::new(),
                    },
                },
            )
            .await
            .expect("schedule-capable group");
        revision = store
            .put_group_schedule_offset(
                fixture.context,
                PutGroupScheduleOffsetCommand {
                    actor: fixture.instructor,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    expected_revision: revision,
                    offset: GroupScheduleOffset {
                        group: id,
                        offset_seconds: ScheduleOffsetSeconds::try_new(30).expect("offset"),
                    },
                },
            )
            .await
            .expect("valid M2 scope");
    }
    let rejected_work_group = CourseGroupId::from_uuid(uuid(99_094));
    store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: rejected_work_group,
                    tenant,
                    course: fixture.course,
                    purpose: question_model::CourseGroupPurpose::Work,
                    title: "Rejected schedule work scope".into(),
                    members: Vec::new(),
                },
            },
        )
        .await
        .expect("work scope");
    for group in [accommodation_group, rejected_work_group] {
        let invalid_m2 = PutGroupScheduleOffsetCommand {
            actor: fixture.instructor,
            course: fixture.course,
            assignment: fixture.assignment,
            expected_revision: revision,
            offset: GroupScheduleOffset {
                group,
                offset_seconds: ScheduleOffsetSeconds::try_new(30).expect("offset"),
            },
        };
        assert!(matches!(
            store
                .put_group_schedule_offset(fixture.context, invalid_m2)
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        assert_eq!(
            store
                .get_assignment_for_edit(fixture.context, fixture.assignment)
                .await
                .expect("assignment")
                .expect("exists")
                .revision,
            revision
        );
    }
    for group in [
        schedule_group,
        CourseGroupId::from_uuid(uuid(99_092)),
        CourseGroupId::from_uuid(uuid(99_093)),
        rejected_work_group,
    ] {
        let invalid_m3 = PutGroupAccommodationCommand {
            actor: fixture.instructor,
            course: fixture.course,
            assignment: fixture.assignment,
            expected_revision: revision,
            accommodation: GroupAccommodation {
                group,
                mode: PolicyModificationMode::Override,
                patch: PolicyPatchSet {
                    available_at: PolicyPatch::Unrestricted,
                    ..PolicyPatchSet::INHERIT
                },
            },
        };
        assert!(matches!(
            store
                .put_group_accommodation(fixture.context, invalid_m3)
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        assert_eq!(
            store
                .get_assignment_for_edit(fixture.context, fixture.assignment)
                .await
                .expect("assignment")
                .expect("exists")
                .revision,
            revision
        );
    }
    assert!(
        store
            .delete_course_group(
                fixture.context,
                fixture.instructor,
                fixture.course,
                work_group,
                legal.revision
            )
            .await
            .expect("delete unreferenced")
    );
    membership_m4::assert_individual_policy_exception_membership_guards(store, &fixture).await;
}

async fn assert_multiple_membership_warning_contract<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    tenant: TenantId,
    course: CourseId,
) where
    S: Store + CourseGroupManagementStore + CourseRosterStore,
{
    let learner = UserId::from_uuid(uuid(720_050));
    store
        .upsert_course_member(
            context,
            instructor,
            learning_data_access::UpsertCourseMember {
                course,
                user: learner,
                display_name: "Warning learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership");
    let membership = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("membership lookup")
        .expect("active membership")
        .id;
    for (offset, purpose) in [
        (60_u128, question_model::CourseGroupPurpose::Section),
        (61, question_model::CourseGroupPurpose::Section),
        (62, question_model::CourseGroupPurpose::Lab),
        (63, question_model::CourseGroupPurpose::Lab),
    ] {
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id: CourseGroupId::from_uuid(uuid(720_000 + offset)),
                        tenant,
                        course,
                        purpose,
                        title: format!("{purpose:?} {offset}"),
                        members: vec![membership],
                    },
                },
            )
            .await
            .expect("multiple membership remains a valid write");
    }
    let warnings = store
        .course_group_membership_warnings(context, instructor, course)
        .await
        .expect("warnings");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].membership, membership);
    assert_eq!(
        warnings[0].purpose,
        question_model::CourseGroupPurpose::Section
    );
    assert_eq!(warnings[0].membership_count, 2);
    assert!(matches!(
        warnings[0].disposition,
        question_model::MultipleMembershipDisposition::AllowedWithWarning
    ));
}

async fn create_and_page_groups<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    tenant: TenantId,
    course: CourseId,
) -> Vec<learning_data_access::CourseGroupView>
where
    S: Store + CourseGroupManagementStore,
{
    for number in 0..12_u128 {
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id: CourseGroupId::from_uuid(uuid(720_100 + number)),
                        tenant,
                        course,
                        purpose: question_model::CourseGroupPurpose::Section,
                        title: format!("Section {number}"),
                        members: Vec::new(),
                    },
                },
            )
            .await
            .expect("group create");
    }
    assert_group_put_revision_and_idempotency(store, context, instructor, tenant, course).await;
    let size = PageSize::new(5).expect("page size");
    let first = store
        .list_course_groups(context, instructor, course, PageRequest::first(size))
        .await
        .expect("first");
    let second = store
        .list_course_groups(
            context,
            instructor,
            course,
            PageRequest::after(first.next_cursor.clone().expect("cursor"), size),
        )
        .await
        .expect("second");
    let third = store
        .list_course_groups(
            context,
            instructor,
            course,
            PageRequest::after(second.next_cursor.clone().expect("cursor"), size),
        )
        .await
        .expect("third");
    let fourth = store
        .list_course_groups(
            context,
            instructor,
            course,
            PageRequest::after(third.next_cursor.clone().expect("cursor"), size),
        )
        .await
        .expect("fourth");
    let mut groups = first.items;
    groups.extend(second.items);
    groups.extend(third.items);
    groups.extend(fourth.items);
    assert_eq!(groups.len(), 16);
    for pair in groups.windows(2) {
        assert!(pair[0].reference.number() < pair[1].reference.number());
    }
    assert_eq!(
        store
            .get_course_group_by_reference(context, instructor, course, groups[0].reference)
            .await,
        Ok(Some(groups[0].clone()))
    );
    groups
}

async fn assert_group_put_revision_and_idempotency<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    tenant: TenantId,
    course: CourseId,
) where
    S: Store + CourseGroupManagementStore,
{
    let group = CourseGroupId::from_uuid(uuid(720_100));
    let created = store
        .get_course_group(context, group)
        .await
        .expect("created group read")
        .expect("created group");
    let mut changed = created.record;
    changed.title = "Revised section".into();
    let updated = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(created.revision),
                record: changed,
            },
        )
        .await
        .expect("group update");
    assert_eq!(updated.record.tenant, tenant);
    assert_eq!(updated.record.course, course);

    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(created.revision),
                    record: updated.record.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store.get_course_group(context, group).await,
        Ok(Some(updated.clone()))
    );

    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(updated.revision),
                    record: updated.record.clone(),
                },
            )
            .await,
        Ok(updated)
    );
}

async fn assert_delete_cleans_reference<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    course: CourseId,
    view: learning_data_access::CourseGroupView,
) where
    S: Store + CourseGroupManagementStore,
{
    assert!(
        store
            .delete_course_group(
                context,
                instructor,
                course,
                view.group.record.id,
                view.group.revision
            )
            .await
            .expect("delete")
    );
    assert_eq!(
        store
            .get_course_group_by_reference(context, instructor, course, view.reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store.get_course_group(context, view.group.record.id).await,
        Ok(None)
    );
}
