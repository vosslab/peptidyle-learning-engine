use super::*;

/// The maintained reference backend must not turn opaque locators into authority.
pub(super) async fn exercise_navigation_reference_authority(store: &MemoryStore) {
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(2)));
    let course = CourseId::from_uuid(uuid(17));
    let assignment = AssignmentId::from_uuid(uuid(8));
    let run = RunId::from_uuid(uuid(25));
    let workspace = WorkspaceId::from_uuid(uuid(24));
    let instructor = UserId::from_uuid(uuid(18));
    let student = UserId::from_uuid(uuid(14));
    let another_student = UserId::from_uuid(uuid(20));
    let outsider = UserId::from_uuid(uuid(23));
    let workspace_owner = UserId::from_uuid(uuid(16));

    let started = store
        .start_or_resume_run(context, student, assignment, run)
        .await
        .expect("reference fixture run");
    store
        .upsert_draft(
            context,
            workspace_owner,
            None,
            DraftRecord {
                tenant,
                question: draft_question(workspace),
                derived_from: None,
            },
        )
        .await
        .expect("reference fixture workspace");

    let course_reference = store
        .course_reference(context, instructor, course)
        .await
        .expect("course reference lookup")
        .expect("instructor course reference");
    assert_eq!(
        store.course_reference(context, instructor, course).await,
        Ok(Some(course_reference)),
        "course allocation is stable"
    );
    let assignment_reference = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment reference lookup")
        .expect("instructor assignment reference");
    let workspace_reference = store
        .workspace_reference(context, workspace_owner, workspace)
        .await
        .expect("workspace reference lookup")
        .expect("workspace owner reference");
    let run_reference = store
        .run_reference(context, student, run)
        .await
        .expect("student run reference lookup")
        .expect("current owning student run reference");
    assert_eq!(
        store.run_reference(context, student, run).await,
        Ok(Some(run_reference)),
        "run allocation is stable"
    );
    assert_eq!(
        store
            .resolve_course_reference(context, student, course_reference)
            .await,
        Ok(Some(course))
    );
    assert_eq!(
        store
            .resolve_assignment_reference(context, student, assignment_reference)
            .await
            .map(|result| result.map(|identity| (identity.course, identity.assignment))),
        Ok(Some((course, assignment)))
    );
    let expected_run = RunRouteIdentity {
        course,
        assignment,
        enrollment: started.enrollment,
        run,
    };
    assert_eq!(
        store
            .resolve_run_reference(context, student, run_reference)
            .await,
        Ok(Some(expected_run))
    );
    assert_eq!(
        store
            .resolve_run_reference(context, instructor, run_reference)
            .await,
        Ok(Some(expected_run)),
        "an instructor must be a current member of this exact assignment course"
    );
    assert_eq!(
        store
            .resolve_workspace_reference(context, workspace_owner, workspace_reference)
            .await,
        Ok(Some(workspace))
    );

    assert_eq!(
        store.course_reference(context, outsider, course).await,
        Ok(None)
    );
    assert_eq!(
        store
            .assignment_reference(context, outsider, assignment)
            .await,
        Ok(None)
    );
    assert_eq!(store.run_reference(context, outsider, run).await, Ok(None));
    assert_eq!(
        store
            .workspace_reference(context, outsider, workspace)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_run_reference(context, outsider, run_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .course_reference(context, another_student, course)
            .await,
        Ok(Some(course_reference))
    );
    assert_eq!(
        store
            .assignment_reference(context, another_student, assignment)
            .await,
        Ok(Some(assignment_reference))
    );
    assert_eq!(
        store.run_reference(context, another_student, run).await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_run_reference(context, another_student, run_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .workspace_reference(context, another_student, workspace)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .course_reference(foreign_context, instructor, course)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .assignment_reference(foreign_context, instructor, assignment)
            .await,
        Ok(None)
    );
    assert_eq!(
        store.run_reference(foreign_context, student, run).await,
        Ok(None)
    );
    assert_eq!(
        store
            .workspace_reference(foreign_context, workspace_owner, workspace)
            .await,
        Ok(None)
    );
}
