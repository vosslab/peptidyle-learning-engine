use super::assignments::publish_assignment_version;
use super::*;

pub(super) async fn exercise_assignment_workspace_slices<S>(store: &S)
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(70_300));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(70_301));
    let learner = UserId::from_uuid(uuid(70_302));
    let course = CourseId::from_uuid(uuid(70_303));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Assignment workspace course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("workspace fixture term"),
                },
                authority: sysadmin_course_creation_authority(store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("workspace course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "Workspace learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("workspace learner");
    let reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_310,
        PublicationScope::Public,
    )
    .await;
    let replacement_reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_311,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(70_320));
    let draft = store
        .create_assignment_draft(
            context,
            CreateAssignmentDraftCommand {
                actor: instructor,
                course,
                assignment,
                title: "Workspace draft".to_string(),
            },
        )
        .await
        .expect("empty assignment draft");
    assert!(draft.record.items.is_empty());
    assert!(draft.record.selection_groups.is_empty());
    assert_eq!(draft.revision.value(), 1);
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("reload empty draft"),
        Some(draft.clone())
    );

    let publish_settings = question_model::AssignmentTeachingSettings {
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::default(),
        base_policy: question_model::BaseAssignmentPolicy::default(),
    };
    assert!(matches!(
        store
            .replace_assignment_policies(
                context,
                ReplaceAssignmentPoliciesCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: draft.revision,
                    update: AssignmentPoliciesUpdate {
                        audience: draft.record.audience.clone(),
                        disclosure_policy: draft.record.disclosure_policy,
                        policies: draft.record.policies,
                        teaching_settings: publish_settings.clone(),
                    },
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("reload refused publication"),
        Some(draft.clone())
    );

    let content = store
        .replace_assignment_content(
            context,
            ReplaceAssignmentContentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: draft.revision,
                update: AssignmentContentUpdate {
                    title: "Workspace questions".to_string(),
                    items: fixed_items(vec![reference]),
                    selection_groups: Vec::new(),
                },
            },
        )
        .await
        .expect("save Questions slice");
    let content = match content {
        ReplaceAssignmentContentOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected content outcome: {other:?}"),
    };
    assert_eq!(content.revision.value(), 2);
    assert_eq!(content.record.policies, draft.record.policies);
    assert_eq!(content.record.audience, draft.record.audience);
    assert_eq!(content.base_policy, draft.base_policy);

    let policy = store
        .replace_assignment_policies(
            context,
            ReplaceAssignmentPoliciesCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: content.revision,
                update: AssignmentPoliciesUpdate {
                    audience: content.record.audience.clone(),
                    disclosure_policy: content.record.disclosure_policy,
                    policies: RunPolicies {
                        grade: GradePolicy::Latest,
                        ..content.record.policies
                    },
                    teaching_settings: question_model::AssignmentTeachingSettings {
                        lifecycle: question_model::AssignmentLifecycle::Published,
                        instructions: question_model::AssignmentInstructions::try_new(
                            "Read each question carefully.".to_string(),
                        )
                        .expect("workspace instructions"),
                        base_policy: question_model::BaseAssignmentPolicy::default(),
                    },
                },
            },
        )
        .await
        .expect("save Policies slice");
    let policy = match policy {
        ReplaceAssignmentPoliciesOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected policy outcome: {other:?}"),
    };
    assert_eq!(policy.revision.value(), 3);
    assert_eq!(policy.record.title, content.record.title);
    assert_eq!(policy.record.items, content.record.items);
    assert_eq!(
        policy.record.selection_groups,
        content.record.selection_groups
    );
    assert_eq!(
        policy.record.lifecycle,
        question_model::AssignmentLifecycle::Published
    );

    let stale_content = store
        .replace_assignment_content(
            context,
            ReplaceAssignmentContentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: content.revision,
                update: AssignmentContentUpdate {
                    title: "Stale Questions tab".to_string(),
                    items: policy.record.items.clone(),
                    selection_groups: policy.record.selection_groups.clone(),
                },
            },
        )
        .await
        .expect("stale content response");
    assert_eq!(
        stale_content,
        ReplaceAssignmentContentOutcome::RevisionConflict
    );
    let current = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("reload after stale content")
        .expect("workspace assignment remains");
    assert_eq!(current.revision, policy.revision);

    let advanced = store
        .replace_assignment_content(
            context,
            ReplaceAssignmentContentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: policy.revision,
                update: AssignmentContentUpdate {
                    title: "Workspace questions revised".to_string(),
                    items: policy.record.items.clone(),
                    selection_groups: policy.record.selection_groups.clone(),
                },
            },
        )
        .await
        .expect("one-revision content advancement");
    let advanced = match advanced {
        ReplaceAssignmentContentOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected advanced content outcome: {other:?}"),
    };
    assert_eq!(advanced.revision.value(), policy.revision.value() + 1);

    store
        .start_or_resume_run(
            context,
            learner,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(uuid(70_330)),
        )
        .await
        .expect("workspace learner run");
    let nonstructural = store
        .replace_assignment_content(
            context,
            ReplaceAssignmentContentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: advanced.revision,
                update: AssignmentContentUpdate {
                    title: "Workspace questions clarified".to_string(),
                    items: advanced.record.items.clone(),
                    selection_groups: advanced.record.selection_groups.clone(),
                },
            },
        )
        .await
        .expect("post-run nonstructural Questions save");
    let nonstructural = match nonstructural {
        ReplaceAssignmentContentOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected nonstructural content outcome: {other:?}"),
    };
    assert_eq!(
        nonstructural.revision.value(),
        advanced.revision.value() + 1
    );
    let fenced = store
        .replace_assignment_content(
            context,
            ReplaceAssignmentContentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: nonstructural.revision,
                update: AssignmentContentUpdate {
                    title: nonstructural.record.title.clone(),
                    items: fixed_items(vec![replacement_reference]),
                    selection_groups: Vec::new(),
                },
            },
        )
        .await
        .expect("structural replacement fence");
    assert_eq!(fenced, ReplaceAssignmentContentOutcome::Issued);

    let closed = store
        .replace_assignment_policies(
            context,
            ReplaceAssignmentPoliciesCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: nonstructural.revision,
                update: AssignmentPoliciesUpdate {
                    audience: nonstructural.record.audience.clone(),
                    disclosure_policy: nonstructural.record.disclosure_policy,
                    policies: nonstructural.record.policies,
                    teaching_settings: question_model::AssignmentTeachingSettings {
                        lifecycle: question_model::AssignmentLifecycle::Closed,
                        instructions: nonstructural.record.instructions.clone(),
                        base_policy: nonstructural.base_policy,
                    },
                },
            },
        )
        .await
        .expect("close preserves historical content");
    let closed = match closed {
        ReplaceAssignmentPoliciesOutcome::Replaced(stored) => *stored,
        other => panic!("unexpected close outcome: {other:?}"),
    };
    assert_eq!(
        closed.record.lifecycle,
        question_model::AssignmentLifecycle::Closed
    );
    assert_eq!(closed.record.items, nonstructural.record.items);
}
