use super::*;

pub(super) async fn exercise_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(1));
    let foreign_tenant = TenantId::from_uuid(uuid(2));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let workspace = WorkspaceId::from_uuid(uuid(3));
    let problem = ProblemId::from_uuid(uuid(4));
    let version = VersionId::from_uuid(uuid(5));
    let second_problem = ProblemId::from_uuid(uuid(6));
    let second_version = VersionId::from_uuid(uuid(7));
    let assignment_id = AssignmentId::from_uuid(uuid(8));
    let course_id = CourseId::from_uuid(uuid(17));
    let course_user = UserId::from_uuid(uuid(18));
    let enrollment_id = EnrollmentId::from_uuid(uuid(9));
    let run_id = RunId::from_uuid(uuid(10));
    let practice_run_id = RunId::from_uuid(uuid(14));
    let draft = DraftRecord {
        tenant,
        question: draft_question(workspace),
        revises: None,
        derived_from: None,
    };
    let publisher = UserId::from_uuid(uuid(16));
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id,
        title: "Molar mass mastery".to_string(),
        items: fixed_items(vec![PublishedVersionRef { problem, version }]),
        selection_groups: Vec::new(),
        policies: policies(),
    };
    let stored_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("conforming draft write should succeed");

    let mut invalid_draft = draft.clone();
    invalid_draft.question.attempt_policy.max_attempts = Some(0);
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, invalid_draft)
            .await,
        Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string()
        ))
    );

    let mut blank_title = draft.clone();
    blank_title.question.metadata.title = " \t\n ".to_string();
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, blank_title)
            .await,
        Err(StoreError::InvalidRecord(
            "question title must not be blank".to_string()
        ))
    );

    let mut invalid_publish = draft.clone();
    invalid_publish.question.metadata.title = "\u{2003}".to_string();
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: invalid_publish,
                    expected_revision: stored_draft.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(
            "question title must not be blank".to_string()
        ))
    );
    assert!(
        store
            .get_published_problem(problem, version)
            .await
            .expect("invalid publication lookup should run")
            .is_none(),
        "invalid publication must not mint or persist a record"
    );

    let mut oversized_title = draft.clone();
    oversized_title.question.metadata.title = "\u{1F9EC}".repeat(513);
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, oversized_title)
            .await,
        Err(StoreError::InvalidRecord(
            "question title must contain at most 512 Unicode scalar values".to_string()
        ))
    );

    let stored_draft_json = serde_json::to_value(&stored_draft.record)
        .expect("stored draft should remain serializable");
    assert!(stored_draft_json["question"].get("problem").is_none());
    assert!(stored_draft_json["question"].get("version").is_none());
    let collaborator = UserId::from_uuid(uuid(19));
    store
        .grant_draft_collaborator(context, publisher, workspace, collaborator)
        .await
        .expect("owner should grant a workspace collaborator");
    assert_eq!(
        store
            .delete_draft(context, collaborator, workspace, stored_draft.revision)
            .await,
        Err(StoreError::Forbidden),
        "a collaborator must not delete an owner workspace"
    );
    assert_eq!(
        store.get_draft(context, collaborator, workspace).await,
        Ok(Some(stored_draft.clone())),
        "a refused deletion must preserve collaborator access"
    );

    let second_workspace = WorkspaceId::from_uuid(uuid(30));
    let paged_draft = DraftRecord {
        tenant,
        question: draft_question(second_workspace),
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, publisher, None, paged_draft)
        .await
        .expect("second private draft should save");
    let first_workspace_page = store
        .list_drafts(
            context,
            publisher,
            PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
        )
        .await
        .expect("tenant workspace list should succeed");
    assert_eq!(first_workspace_page.items.len(), 1);
    assert_eq!(first_workspace_page.items[0].workspace, workspace);
    assert_eq!(first_workspace_page.items[0].title, "Molar mass");
    assert_eq!(
        first_workspace_page.items[0].source_backend,
        QuestionBackend::Native
    );
    let summary_json = serde_json::to_value(&first_workspace_page.items[0])
        .expect("workspace summary should serialize");
    let summary_fields = summary_json
        .as_object()
        .expect("workspace summary should be an object");
    assert_eq!(summary_fields.len(), 3);
    for forbidden in [
        "problem", "version", "source", "grading", "object", "asset", "prompt", "response",
    ] {
        assert!(
            !summary_fields.contains_key(forbidden),
            "workspace summary must not expose {forbidden}"
        );
    }
    let workspace_cursor = first_workspace_page
        .next_cursor
        .clone()
        .expect("bounded first page should continue");
    assert!(
        !workspace_cursor.as_str().contains(&workspace.to_string()),
        "workspace cursor must be opaque rather than a UUID path fragment"
    );
    let second_workspace_page = store
        .list_drafts(
            context,
            publisher,
            PageRequest::after(
                workspace_cursor.clone(),
                PageSize::new(1).expect("one is a valid page size"),
            ),
        )
        .await
        .expect("tenant-bound continuation should resume");
    assert_eq!(second_workspace_page.items.len(), 1);
    assert_eq!(second_workspace_page.items[0].workspace, second_workspace);
    assert!(second_workspace_page.next_cursor.is_none());
    assert!(matches!(
        store
            .list_drafts(
                context,
                publisher,
                PageRequest::after(
                    Cursor::parse(format!("{}x", workspace_cursor.as_str()))
                        .expect("nonempty tampered cursor"),
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .list_drafts(
                foreign_context,
                publisher,
                PageRequest::after(
                    workspace_cursor,
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .list_drafts(
                foreign_context,
                publisher,
                PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
            )
            .await
            .expect("foreign workspace list should run")
            .items
            .is_empty()
    );
    assert_eq!(
        store
            .get_draft(foreign_context, publisher, workspace)
            .await
            .expect("foreign draft lookup should run"),
        None
    );
    assert!(
        !store
            .delete_draft(foreign_context, publisher, workspace, stored_draft.revision,)
            .await
            .expect("foreign deletion should not disclose existence")
    );
    assert!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .expect("foreign deletion must not affect local draft")
            .is_some()
    );
    let second_workspace_before_update = store
        .get_draft(context, publisher, second_workspace)
        .await
        .expect("second workspace lookup should run")
        .expect("second workspace should exist before an update");
    let second_workspace_after_update = store
        .upsert_draft(
            context,
            publisher,
            Some(second_workspace_before_update.revision),
            second_workspace_before_update.record.clone(),
        )
        .await
        .expect("second workspace update should advance its revision");
    assert_eq!(
        store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_before_update.revision,
            )
            .await,
        Err(StoreError::Conflict),
        "a stale delete must preserve the newer workspace and access binding"
    );
    assert_eq!(
        store.get_draft(context, publisher, second_workspace).await,
        Ok(Some(second_workspace_after_update.clone())),
        "a stale delete must not mutate the newer workspace"
    );
    assert!(
        store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_after_update.revision,
            )
            .await
            .expect("current owner revision should delete")
    );
    assert!(
        !store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_after_update.revision,
            )
            .await
            .expect("repeat deletion should be an absence result")
    );
    assert_eq!(
        store
            .get_draft(context, publisher, second_workspace)
            .await
            .expect("deleted draft lookup should run"),
        None
    );

    let published = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: stored_draft.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("conforming publish should succeed");
    assert_eq!(published.problem, problem);
    assert_eq!(published.version, version);
    assert_eq!(published.question.problem, problem);
    assert_eq!(published.question.version, version);
    let deletable_workspace = WorkspaceId::from_uuid(uuid(31));
    let deletable_draft = store
        .upsert_draft(
            context,
            publisher,
            None,
            DraftRecord {
                tenant,
                question: draft_question(deletable_workspace),
                revises: None,
                derived_from: None,
            },
        )
        .await
        .expect("independent draft should save before deletion");
    assert!(
        store
            .delete_draft(
                context,
                publisher,
                deletable_workspace,
                deletable_draft.revision,
            )
            .await
            .expect("independent draft should delete")
    );
    assert!(
        store
            .get_published_problem(problem, version)
            .await
            .expect("published catalog lookup should run after draft deletion")
            .is_some(),
        "deleting a draft must not affect its already-published catalog version"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .expect("published draft lookup"),
        None
    );
    let second_draft = DraftRecord {
        tenant,
        question: draft_question(workspace),
        revises: None,
        derived_from: None,
    };
    let second_draft = store
        .upsert_draft(context, publisher, None, second_draft.clone())
        .await
        .expect("second draft write should succeed");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: second_draft.record,
                expected_revision: second_draft.revision,
                publication: ProblemVersionRef {
                    problem: second_problem,
                    version: second_version,
                },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("second publish should succeed");

    let first_page = store
        .list_published_problems(PageRequest::first(
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("first catalog page should load");
    let second_page = store
        .list_published_problems(PageRequest::after(
            first_page
                .next_cursor
                .clone()
                .expect("first page should carry a cursor"),
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("second catalog page should load");

    store
        .upsert_course(
            context,
            CourseRecord {
                id: course_id,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: course_user,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: UserId::from_uuid(uuid(14)),
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("conforming course write should succeed");
    store
        .create_untimed_assignment(context, assignment.clone())
        .await
        .expect("conforming assignment write should succeed");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment_id,
                tenant,
                assignment: assignment_id,
                user: UserId::from_uuid(uuid(14)),
                student: StudentId::from_uuid(uuid(11)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("conforming enrollment creation should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 1,
                    started_at: ActivityTimestamp::from_unix_millis(100),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Assigned,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("conforming run start should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::RecordQuestionAttempt {
                attempt: Box::new(QuestionAttempt {
                    id: QuestionAttemptId::from_uuid(uuid(12)),
                    tenant,
                    run: run_id,
                    problem,
                    question_version: version,
                    assignment_position: 0,
                    seed: 42,
                    parameter_hash: "parameters-sha256".to_string(),
                    response: Some(StudentResponse::Numeric { value: 18.0 }),
                    status: question_model::AttemptStatus::Submitted,
                    result: Some(AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    }),
                    timer: AttemptTimerRecord {
                        issued_at: ActivityTimestamp::from_unix_millis(110),
                        deadline: None,
                        submitted_at: Some(ActivityTimestamp::from_unix_millis(120)),
                    },
                    provenance: AttemptProvenance {
                        adapter: implementation("native"),
                        renderer: None,
                        generator: Some(generator("molar-mass")),
                        source_artifact: None,
                        asset_objects: vec![ObjectId::from_uuid(uuid(13))],
                        grading: implementation("numeric"),
                        rendered_question_sha256: "render-sha256".to_string(),
                    },
                    issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
                }),
            },
        )
        .await
        .expect("conforming attempt write should succeed");
    let summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: run_id,
                score: 1.0,
                at: ActivityTimestamp::from_unix_millis(130),
            },
        )
        .await
        .expect("conforming completion should succeed");
    let completed_run = store
        .get_run(context, run_id)
        .await
        .expect("run read should succeed")
        .expect("completed run should exist");
    let attempt = store
        .get_question_attempt(context, QuestionAttemptId::from_uuid(uuid(12)))
        .await
        .expect("attempt read should succeed")
        .expect("question attempt should exist");

    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: practice_run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 2,
                    started_at: ActivityTimestamp::from_unix_millis(140),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Practice,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("continued practice should remain available after completion");
    let practice_summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: practice_run_id,
                score: 0.8,
                at: ActivityTimestamp::from_unix_millis(150),
            },
        )
        .await
        .expect("continued-practice completion should succeed");
    let enrollment = store
        .get_enrollment(context, enrollment_id)
        .await
        .expect("enrollment read should succeed")
        .expect("enrollment should exist");
    let persisted_summary = store
        .get_summary(context, enrollment_id)
        .await
        .expect("summary read should succeed")
        .expect("summary should exist");

    let second_student = UserId::from_uuid(uuid(20));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course_id,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: course_user,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: UserId::from_uuid(uuid(14)),
                        role: CourseMembershipRole::Student,
                    },
                    CourseMembership {
                        user: second_student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("course may add another enrolled student");
    let second_enrollment = EnrollmentId::from_uuid(uuid(21));
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: second_enrollment,
                tenant,
                assignment: assignment_id,
                user: second_student,
                student: StudentId::from_uuid(uuid(22)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("second course enrollment should create an empty projection");
    let first_gradebook_page = store
        .list_gradebook_rows(
            context,
            course_id,
            PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
        )
        .await
        .expect("summary-only gradebook page should load");
    let second_gradebook_page = store
        .list_gradebook_rows(
            context,
            course_id,
            PageRequest::after(
                first_gradebook_page
                    .next_cursor
                    .clone()
                    .expect("first gradebook page should carry a cursor"),
                PageSize::new(1).expect("one is a valid page size"),
            ),
        )
        .await
        .expect("gradebook cursor should resume after assignment and enrollment");
    assert_eq!(first_gradebook_page.items.len(), 1);
    assert_eq!(second_gradebook_page.items.len(), 1);
    assert_ne!(
        first_gradebook_page.items[0].enrollment_id, second_gradebook_page.items[0].enrollment_id,
        "gradebook cursor must not duplicate an enrollment"
    );
    let first_gradebook_row = first_gradebook_page
        .items
        .iter()
        .chain(second_gradebook_page.items.iter())
        .find(|row| row.enrollment_id == enrollment_id)
        .expect("completed enrollment should appear in the gradebook");
    assert_eq!(first_gradebook_row.tenant, tenant);
    assert_eq!(first_gradebook_row.course_id, course_id);
    assert_eq!(first_gradebook_row.assignment_id, assignment_id);
    assert_eq!(first_gradebook_row.assignment_title, "Molar mass mastery");
    assert_eq!(first_gradebook_row.summary, persisted_summary);
    assert!(matches!(
        store
            .list_gradebook_rows(
                context,
                course_id,
                PageRequest::after(
                    Cursor::parse("not-a-gradebook-cursor".to_string())
                        .expect("nonempty malformed cursor"),
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(message)) if message == "invalid gradebook cursor"
    ));
    assert_eq!(
        store
            .list_gradebook_rows(
                foreign_context,
                course_id,
                PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
            )
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot discover this course or its summary rows"
    );

    let tenant_mismatch = store
        .upsert_draft(
            foreign_context,
            publisher,
            None,
            DraftRecord {
                tenant,
                question: draft_question(workspace),
                revises: None,
                derived_from: None,
            },
        )
        .await;
    let tenant_assignments = store
        .list_assignments(
            context,
            course_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("assignment list should load");
    let member_courses = store
        .list_courses(
            context,
            CourseListScope::Member(course_user),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("member course list should load");
    let nonmember_courses = store
        .list_courses(
            context,
            CourseListScope::Member(UserId::from_uuid(uuid(19))),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("nonmember course list should load");
    let run_page = store
        .list_runs(
            context,
            enrollment_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("run list should load");

    assert_eq!((first_page.items.len(), second_page.items.len()), (1, 1));
    assert_eq!(
        store.get_draft(context, publisher, workspace).await,
        Ok(None)
    );
    assert_eq!(
        store.get_published_problem(problem, version).await,
        Ok(Some(published))
    );
    assert_eq!(
        store.get_assignment(context, assignment_id).await,
        Ok(Some(assignment))
    );
    assert_eq!(tenant_assignments.items.len(), 1);
    assert_eq!(member_courses.items.len(), 1);
    assert_eq!(
        member_courses.items[0].role,
        CourseMembershipRole::Instructor
    );
    assert!(nonmember_courses.items.is_empty());
    assert_eq!(store.get_course(foreign_context, course_id).await, Ok(None));
    assert_eq!(
        (
            summary.current_score,
            summary.completed_run_count,
            summary.total_question_attempts,
        ),
        (Some(1.0), 1, 1)
    );
    assert_eq!(practice_summary, persisted_summary);
    assert_eq!(
        (
            persisted_summary.current_score,
            persisted_summary.best_score,
            persisted_summary.latest_score,
            persisted_summary.completed_run_count,
        ),
        (Some(1.0), Some(1.0), Some(0.8), 2)
    );
    assert_eq!(
        (
            enrollment.first_completed_at,
            enrollment.current_grade_run,
            enrollment.best_grade_run,
        ),
        (
            Some(ActivityTimestamp::from_unix_millis(130)),
            Some(run_id),
            Some(run_id),
        )
    );
    assert_eq!(
        (
            completed_run.completed_at,
            attempt.problem,
            run_page.items.len()
        ),
        (Some(ActivityTimestamp::from_unix_millis(130)), problem, 2,)
    );
    assert_eq!(tenant_mismatch, Err(StoreError::TenantMismatch));
    assert_eq!(
        store.get_draft(foreign_context, publisher, workspace).await,
        Ok(None)
    );
    assert_eq!(
        store.get_assignment(foreign_context, assignment_id).await,
        Ok(None)
    );
    assert_eq!(
        store.get_enrollment(foreign_context, enrollment_id).await,
        Ok(None)
    );
    assert_eq!(store.get_run(foreign_context, run_id).await, Ok(None));
    assert_eq!(
        store
            .get_question_attempt(foreign_context, QuestionAttemptId::from_uuid(uuid(12)))
            .await,
        Ok(None)
    );
    assert_eq!(
        store.get_summary(foreign_context, enrollment_id).await,
        Ok(None)
    );
}
