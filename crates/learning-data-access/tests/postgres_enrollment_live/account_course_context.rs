async fn insert_context_course(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    course: Uuid,
    title: &str,
) {
    sqlx::query(
        "INSERT INTO course \
         (tenant_id, course_id, title, term_start_date, term_end_date, time_zone) \
         VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', 'America/Chicago')",
    )
    .bind(tenant)
    .bind(course)
    .bind(title)
    .execute(pool)
    .await
    .expect("insert disposable account-context course");
}

async fn insert_active_student_membership(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    course: Uuid,
    user: Uuid,
    student: Uuid,
) {
    let mut fixture = pool.begin().await.expect("begin Student membership fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("set trusted Student fixture tenant");
    sqlx::query(
        "INSERT INTO course_member \
         (tenant_id, course_id, course_membership_id, user_id, student_id, role, \
          status, roster_id, joined_at, revoked_at) \
         VALUES ($1, $2, gen_random_uuid(), $3, $4, 'student', 'active', \
                 NULL, transaction_timestamp(), NULL)",
    )
    .bind(tenant)
    .bind(course)
    .bind(user)
    .bind(student)
    .execute(&mut *fixture)
    .await
    .expect("insert active Student membership");
    fixture
        .commit()
        .await
        .expect("commit active Student membership fixture");
}

async fn insert_retention(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    course: Uuid,
    generation: i64,
    lifecycle: &str,
) {
    sqlx::query(
        "INSERT INTO course_retention \
         (tenant_id, course_id, ended_at, notify_days, archive_days, delete_days, \
          assignment_disposition, generation, lifecycle) \
         VALUES ($1, $2, transaction_timestamp(), 1, 2, 3, 'retain', $3, $4)",
    )
    .bind(tenant)
    .bind(course)
    .bind(generation)
    .bind(lifecycle)
    .execute(pool)
    .await
    .expect("insert disposable course retention state");
}

async fn insert_started_retention_stage(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    course: Uuid,
    generation: i64,
    stage: &str,
) {
    sqlx::query(
        "INSERT INTO course_retention_stage \
         (tenant_id, course_id, stage, generation, due_at, state) \
         VALUES ($1, $2, $3, $4, transaction_timestamp(), 'started')",
    )
    .bind(tenant)
    .bind(course)
    .bind(stage)
    .bind(generation)
    .execute(pool)
    .await
    .expect("insert started disposable retention stage");
}

async fn student_context_is_hidden(
    store: &PostgresStore,
    user: UserId,
    tenant: TenantId,
    course: CourseId,
) {
    let contexts = store
        .list_account_course_contexts(
            user,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("pre-tenant Student context list");
    assert!(
        !contexts
            .items
            .iter()
            .any(|context| context.tenant == tenant && context.course == course),
        "closed Student course must not appear in account context list"
    );
    assert_eq!(
        store
            .resolve_account_course_context(user, course)
            .await
            .expect("pre-tenant Student context resolve"),
        None,
        "closed Student course must not resolve from account context"
    );
}

async fn verify_student_context_visibility_before_tenant_selection(pool: &sqlx::PgPool) {
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let student_user = UserId::from_uuid(id());
    let student = id();
    let active_course = CourseId::from_uuid(id());
    let revoked_course = CourseId::from_uuid(id());
    let archived_course = CourseId::from_uuid(id());
    let deleted_course = CourseId::from_uuid(id());
    let archive_stage_course = CourseId::from_uuid(id());
    let delete_stage_course = CourseId::from_uuid(id());
    let old_stage_course = CourseId::from_uuid(id());

    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, 'Student context live')",
    )
    .bind(student_user.as_uuid())
    .bind(format!("student-context-{}@example.edu", student_user.as_uuid()))
    .execute(pool)
    .await
    .expect("insert disposable Student account");
    sqlx::query(
        "INSERT INTO tenant_learner_identity (tenant_id, user_id, student_id) \
         VALUES ($1, $2, $3)",
    )
    .bind(tenant.as_uuid())
    .bind(student_user.as_uuid())
    .bind(student)
    .execute(pool)
    .await
    .expect("insert disposable tenant learner identity");

    for (course, title) in [
        (active_course, "Active Student context"),
        (revoked_course, "Revoked Student context"),
        (archived_course, "Archived Student context"),
        (deleted_course, "Deleted Student context"),
        (archive_stage_course, "Archiving Student context"),
        (delete_stage_course, "Deleting Student context"),
        (old_stage_course, "Prior stage Student context"),
    ] {
        insert_context_course(pool, tenant.as_uuid(), course.as_uuid(), title).await;
        insert_active_student_membership(
            pool,
            tenant.as_uuid(),
            course.as_uuid(),
            student_user.as_uuid(),
            student,
        )
        .await;
    }

    let active_contexts = store
        .list_account_course_contexts(
            student_user,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("Student account context list uses normal pre-tenant auth path");
    assert!(
        active_contexts
            .items
            .iter()
            .any(|context| context.tenant == tenant && context.course == active_course),
        "an active Student membership must remain visible before tenant selection"
    );
    assert_eq!(
        store
            .resolve_account_course_context(student_user, active_course)
            .await
            .expect("Student account context resolve")
            .expect("active Student context"),
        learning_data_access::AccountCourseContext {
            tenant,
            course: active_course,
            title: "Active Student context".to_string(),
            role: CourseMembershipRole::Student,
        }
    );

    let mut revoke_fixture = pool.begin().await.expect("begin Student revocation fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *revoke_fixture)
        .await
        .expect("set trusted Student revocation tenant");
    sqlx::query(
        "UPDATE course_member SET status = 'revoked', revoked_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(revoked_course.as_uuid())
    .bind(student_user.as_uuid())
    .execute(&mut *revoke_fixture)
    .await
    .expect("revoke disposable Student membership");
    revoke_fixture
        .commit()
        .await
        .expect("commit Student revocation fixture");
    student_context_is_hidden(&store, student_user, tenant, revoked_course).await;

    insert_retention(
        pool,
        tenant.as_uuid(),
        archived_course.as_uuid(),
        1,
        "archived",
    )
    .await;
    student_context_is_hidden(&store, student_user, tenant, archived_course).await;

    insert_retention(
        pool,
        tenant.as_uuid(),
        deleted_course.as_uuid(),
        1,
        "deleted",
    )
    .await;
    student_context_is_hidden(&store, student_user, tenant, deleted_course).await;

    insert_retention(
        pool,
        tenant.as_uuid(),
        archive_stage_course.as_uuid(),
        1,
        "active",
    )
    .await;
    insert_started_retention_stage(
        pool,
        tenant.as_uuid(),
        archive_stage_course.as_uuid(),
        1,
        "archiveStudentRecords",
    )
    .await;
    student_context_is_hidden(&store, student_user, tenant, archive_stage_course).await;

    insert_retention(
        pool,
        tenant.as_uuid(),
        delete_stage_course.as_uuid(),
        1,
        "active",
    )
    .await;
    insert_started_retention_stage(
        pool,
        tenant.as_uuid(),
        delete_stage_course.as_uuid(),
        1,
        "deleteStudentRecords",
    )
    .await;
    student_context_is_hidden(&store, student_user, tenant, delete_stage_course).await;

    insert_retention(
        pool,
        tenant.as_uuid(),
        old_stage_course.as_uuid(),
        2,
        "active",
    )
    .await;
    insert_started_retention_stage(
        pool,
        tenant.as_uuid(),
        old_stage_course.as_uuid(),
        1,
        "archiveStudentRecords",
    )
    .await;
    assert!(
        store
            .list_account_course_contexts(
                student_user,
                PageRequest::first(PageSize::new(20).expect("bounded page size")),
            )
            .await
            .expect("Student context list after old retention stage")
            .items
            .iter()
            .any(|context| context.tenant == tenant && context.course == old_stage_course),
        "an old-generation retention stage must not close the current Student context"
    );

    let mut auth_probe = pool.begin().await.expect("begin pre-tenant auth probe");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *auth_probe)
        .await
        .expect("assume auth role");
    let current_tenant = sqlx::query_scalar::<_, Option<String>>(
        "SELECT current_setting('ple.tenant_id', true)",
    )
    .fetch_one(&mut *auth_probe)
    .await
    .expect("read unset pre-tenant configuration");
    assert!(
        current_tenant.is_none() || current_tenant.as_deref() == Some(""),
        "account context starts before tenant selection"
    );
    let helper_error = sqlx::query(
        "SELECT public.ple_account_student_context_records_visible($1, $2)",
    )
    .bind(tenant.as_uuid())
    .bind(active_course.as_uuid())
    .execute(&mut *auth_probe)
    .await
    .expect_err("ple_auth must not execute the retention visibility helper directly");
    assert_eq!(database_error_code(&helper_error).as_deref(), Some("42501"));
    auth_probe
        .rollback()
        .await
        .expect("rollback pre-tenant auth probe");
}

async fn verify_instructor_context_order_and_paging(pool: &sqlx::PgPool) {
    let store = PostgresStore::new(pool.clone());
    let user = UserId::from_uuid(id());
    let first_tenant = TenantId::from_uuid(id());
    let second_tenant = TenantId::from_uuid(id());
    let first_course = CourseId::from_uuid(id());
    let second_course = CourseId::from_uuid(id());

    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, 'Instructor context live')",
    )
    .bind(user.as_uuid())
    .bind(format!("instructor-context-{}@example.edu", user.as_uuid()))
    .execute(pool)
    .await
    .expect("insert disposable Instructor account");
    for (tenant, course, title) in [
        (first_tenant, first_course, "First Instructor context"),
        (second_tenant, second_course, "Second Instructor context"),
    ] {
        insert_context_course(pool, tenant.as_uuid(), course.as_uuid(), title).await;
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
    .execute(pool)
        .await
        .expect("insert active Instructor membership");
    }

    let mut expected = [
        (first_tenant, first_course, "First Instructor context"),
        (second_tenant, second_course, "Second Instructor context"),
    ];
    expected.sort_by_key(|(tenant, course, _)| (tenant.as_uuid(), course.as_uuid()));
    let page_size = PageSize::new(1).expect("single-item page size");
    let first_page = store
        .list_account_course_contexts(user, PageRequest::first(page_size))
        .await
        .expect("first Instructor context page");
    assert_eq!(
        (first_page.items[0].tenant, first_page.items[0].course),
        (expected[0].0, expected[0].1),
        "Instructor contexts retain tenant/course key order"
    );
    let second_page = store
        .list_account_course_contexts(
            user,
            PageRequest::after(
                first_page
                    .next_cursor
                    .expect("first Instructor context has continuation"),
                page_size,
            ),
        )
        .await
        .expect("second Instructor context page");
    assert_eq!(
        (second_page.items[0].tenant, second_page.items[0].course),
        (expected[1].0, expected[1].1),
        "Instructor continuation retains the established account-context order"
    );
    for (tenant, course, title) in expected {
        assert_eq!(
            store
                .resolve_account_course_context(user, course)
                .await
                .expect("Instructor context resolve")
                .expect("active Instructor context"),
            learning_data_access::AccountCourseContext {
                tenant,
                course,
                title: title.to_string(),
                role: CourseMembershipRole::Instructor,
            }
        );
    }
}
