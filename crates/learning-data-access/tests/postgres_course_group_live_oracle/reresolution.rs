use super::*;
use super::{fixture::*, published_assignment};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_course_group_edit_reresolves_zero_one_and_multiple_assignments() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x74; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());

    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T4 course-group re-resolution".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "T4 learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("member");
    let membership = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("member read")
        .expect("member");
    let reference = publish(&store, context, tenant, instructor).await;
    let base_policy = BaseAssignmentPolicy {
        due_at: Some(ActivityTimestamp::from_unix_millis(1_797_465_600_000)),
        ..BaseAssignmentPolicy::default()
    };

    let create_assignment = |assignment| {
        let store = &store;
        async move {
            published_assignment::create_published_assignment(
                store,
                context,
                instructor,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "T4 affected assignment".into(),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience: question_model::AssignmentAudience::CourseWide,
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id()),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: vec![],
                    disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                    policies: policies(),
                },
                base_policy,
            )
            .await
            .expect("assignment");
        }
    };

    let zero_group = CourseGroupId::from_uuid(id());
    let zero_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: zero_group,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "T4 zero affected".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("zero group");
    let mut zero_record = zero_view.record.clone();
    zero_record.title = "T4 zero affected renamed".into();
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(zero_view.revision),
                record: zero_record,
            },
        )
        .await
        .expect("zero-affected group edit");

    let single_group = CourseGroupId::from_uuid(id());
    let single_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: single_group,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "T4 one affected".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("single group");
    let multiple_group = CourseGroupId::from_uuid(id());
    let multiple_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: multiple_group,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "T4 multiple affected".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("multiple group");

    let single_assignment = AssignmentId::from_uuid(id());
    let first_multiple_assignment = AssignmentId::from_uuid(id());
    let second_multiple_assignment = AssignmentId::from_uuid(id());
    create_assignment(single_assignment).await;
    create_assignment(first_multiple_assignment).await;
    create_assignment(second_multiple_assignment).await;

    for (assignment, group) in [
        (single_assignment, single_group),
        (first_multiple_assignment, multiple_group),
        (second_multiple_assignment, multiple_group),
    ] {
        store
            .put_group_schedule_offset(
                context,
                PutGroupScheduleOffsetCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: revision(&store, context, assignment).await,
                    offset: GroupScheduleOffset {
                        group,
                        offset_seconds: ScheduleOffsetSeconds::try_new(60).expect("offset"),
                    },
                },
            )
            .await
            .expect("schedule offset");
    }

    let issue_active_attempt = |assignment| {
        let store = &store;
        async move {
            let run = store
                .start_or_resume_run(
                    context,
                    learner,
                    StudentWorkRoutingBinding::new(course, assignment),
                    RunId::from_uuid(id()),
                )
                .await
                .expect("run");
            store
                .issue_or_resume_question_attempt(
                    context,
                    issue(
                        learner,
                        run.id,
                        QuestionAttemptId::from_uuid(id()),
                        reference,
                        course,
                        assignment,
                    ),
                )
                .await
                .expect("active attempt")
        }
    };
    let single_attempt = issue_active_attempt(single_assignment).await;
    let first_multiple_attempt = issue_active_attempt(first_multiple_assignment).await;
    let second_multiple_attempt = issue_active_attempt(second_multiple_assignment).await;
    let single_before = current(&store, context, single_attempt.id).await;
    let first_multiple_before = current(&store, context, first_multiple_attempt.id).await;
    let second_multiple_before = current(&store, context, second_multiple_attempt.id).await;

    let mut single_record = single_view.record.clone();
    single_record.members.clear();
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(single_view.revision),
                record: single_record,
            },
        )
        .await
        .expect("one-affected group edit");
    let single_after = current(&store, context, single_attempt.id).await;
    assert!(single_after.generation > single_before.generation);
    assert_eq!(single_after.policy.due_at.source, PolicySource::Base);

    let mut multiple_record = multiple_view.record.clone();
    multiple_record.members.clear();
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(multiple_view.revision),
                record: multiple_record,
            },
        )
        .await
        .expect("multiple-affected group edit");
    let first_multiple_after = current(&store, context, first_multiple_attempt.id).await;
    let second_multiple_after = current(&store, context, second_multiple_attempt.id).await;
    assert!(first_multiple_after.generation > first_multiple_before.generation);
    assert!(second_multiple_after.generation > second_multiple_before.generation);
    assert_eq!(
        first_multiple_after.policy.due_at.source,
        PolicySource::Base
    );
    assert_eq!(
        second_multiple_after.policy.due_at.source,
        PolicySource::Base
    );

    let has_assignment_update: bool =
        sqlx::query_scalar("SELECT has_table_privilege('ple_app', 'public.assignment', 'UPDATE')")
            .fetch_one(&pool)
            .await
            .expect("assignment ACL probe");
    assert!(
        !has_assignment_update,
        "ple_app must not update assignment rows"
    );

    // Shared closed-record validation rejects duplicate members before either
    // backend writes the group. The unchanged read below proves the operation
    // remains all-or-nothing without a production fault-injection hook.
    let before_failed = store
        .get_course_group(context, zero_group)
        .await
        .expect("zero group read")
        .expect("zero group");
    let mut invalid_record = before_failed.record.clone();
    invalid_record.members = vec![membership.id, membership.id];
    assert!(matches!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(before_failed.revision),
                    record: invalid_record,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_course_group(context, zero_group)
            .await
            .expect("zero group read after failure")
            .expect("zero group"),
        before_failed,
        "failed group edit rolls back the group state"
    );
}
