#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_course_member_upsert_is_atomic_idempotent_and_tenant_scoped() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let conflicting_instructor = UserId::from_uuid(id());
    let assignments = [id(), id()];
    let instructor_session = SessionTokenHash::compute(id().as_bytes());
    let context = TenantContext::from_authenticated_session(tenant);

    for (user, roles, token) in [(instructor, vec![UserRole::Instructor], instructor_session)] {
        store
            .create_session(
                token,
                SessionSubject::new(tenant, user, "Local roster live fixture", roles)
                    .expect("valid live session"),
                SessionLifetime::from_seconds(3_600).expect("bounded live session"),
            )
            .await
            .expect("persist live session");
    }
    let mut fixture = pool.begin().await.expect("begin local roster fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("set local roster fixture tenant");
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind("Disposable local roster course")
        .execute(&mut *fixture)
        .await
        .expect("insert local roster course");
    sqlx::query("INSERT INTO course_roster_state (tenant_id, course_id) VALUES ($1, $2)")
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *fixture)
        .await
        .expect("insert local roster state");
    for user in [instructor, conflicting_instructor] {
        sqlx::query(
            "INSERT INTO course_member \
             (tenant_id, course_id, course_membership_id, user_id, student_id, role, \
              status, roster_id, joined_at, revoked_at) \
             VALUES ($1, $2, gen_random_uuid(), $3, NULL, 'instructor', 'active', \
                     NULL, transaction_timestamp(), NULL)",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(user.as_uuid())
        .execute(&mut *fixture)
        .await
        .expect("insert local roster instructor fixture");
    }
    for (index, assignment) in assignments.iter().enumerate() {
        sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, audience_kind, title, \
              score_disclosure, per_item_correctness_disclosure, \
              feedback_text_disclosure, solution_disclosure, \
              class_statistics_disclosure) \
             VALUES ($1, $2, $3, 'course_wide', $4, 'after_submit', \
                     'after_submit', 'after_submit', 'after_submit', 'never')",
        )
        .bind(tenant.as_uuid())
        .bind(*assignment)
        .bind(course.as_uuid())
        .bind(format!("Existing local roster assignment {index}"))
        .execute(&mut *fixture)
        .await
        .expect("insert assignment before local activation");
    }
    fixture.commit().await.expect("commit local roster fixture");

    let command = UpsertCourseMember {
        course,
        user: learner,
        display_name: "Canonical Learner".to_string(),
        roster_contact: None,
    };
    let (first, second) = tokio::join!(
        store.upsert_course_member(context, instructor, command.clone()),
        store.upsert_course_member(context, instructor, command),
    );
    let first = first.expect("first canonical roster upsert");
    let second = second.expect("concurrent canonical roster upsert retry");
    assert_eq!(first, second, "retries must not duplicate a course member");
    assert_eq!(first.member.roster_email, None);
    assert_eq!(first.member.roster_id, None);

    let roster = store
        .list_course_roster(
            context,
            instructor_session,
            course,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("instructor roster read");
    assert_eq!(roster.entries.items.len(), 1);
    assert_eq!(roster.policy.revision, first.roster_revision);
    let student_memberships = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'student'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(learner.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count local learner membership");
    assert_eq!(student_memberships, 1);
    let enrollment_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM enrollment \
         WHERE tenant_id = $1 AND user_id = $2 AND student_id = $3 \
           AND assignment_id = ANY($4::uuid[])",
    )
    .bind(tenant.as_uuid())
    .bind(learner.as_uuid())
    .bind(first.member.student.as_uuid())
    .bind(assignments)
    .fetch_one(&pool)
    .await
    .expect("count existing-assignment enrollments");
    assert_eq!(
        enrollment_count, 2,
        "every existing assignment is enrolled once"
    );
    let summary_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM student_assignment_summary summary \
         JOIN enrollment enrollment \
           ON enrollment.tenant_id = summary.tenant_id \
          AND enrollment.enrollment_id = summary.enrollment_id \
         WHERE enrollment.tenant_id = $1 AND enrollment.user_id = $2 \
           AND enrollment.student_id = $3 \
           AND enrollment.assignment_id = ANY($4::uuid[])",
    )
    .bind(tenant.as_uuid())
    .bind(learner.as_uuid())
    .bind(first.member.student.as_uuid())
    .bind(assignments)
    .fetch_one(&pool)
    .await
    .expect("count enrollment summaries");
    assert_eq!(summary_count, 2, "every enrollment has one empty summary");

    let foreign = store
        .upsert_course_member(
            TenantContext::from_authenticated_session(TenantId::from_uuid(id())),
            instructor,
            UpsertCourseMember {
                course,
                user: UserId::from_uuid(id()),
                display_name: "Foreign Learner".to_string(),
                roster_contact: None,
            },
        )
        .await;
    assert!(
        matches!(foreign, Err(StoreError::Forbidden | StoreError::NotFound)),
        "a foreign tenant context must not upsert a course member: {foreign:?}"
    );

    assert_eq!(
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user: conflicting_instructor,
                    display_name: "Conflicting Instructor".to_string(),
                    roster_contact: None,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a course member cannot replace an instructor course membership"
    );
    let stored_conflict = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM course_roster_profile AS profile \
         JOIN course_member AS membership \
           ON membership.tenant_id = profile.tenant_id \
          AND membership.course_id = profile.course_id \
          AND membership.course_membership_id = profile.course_membership_id \
         WHERE membership.tenant_id = $1 AND membership.course_id = $2 \
           AND membership.user_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(conflicting_instructor.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("check rolled back local roster member");
    assert_eq!(
        stored_conflict, 0,
        "conflicting activation must roll back its roster row"
    );
    let conflict_side_effects = sqlx::query_scalar::<_, i64>(
        "SELECT (SELECT count(*) FROM course_member \
                   WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'student') \
              + (SELECT count(*) FROM enrollment WHERE tenant_id = $1 AND user_id = $3)",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(conflicting_instructor.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("check rolled back local activation side effects");
    assert_eq!(
        conflict_side_effects, 0,
        "conflicting activation must be atomic"
    );
}
