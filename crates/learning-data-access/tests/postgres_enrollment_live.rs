#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for passwordless identity and invitation RLS.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AccountIdentityStore, ActivateLocalDevelopmentCourseMember, AuthenticationEmail,
    AuthenticationRateLimitDecision, AuthenticationRateLimitKey, AuthenticationRateLimitPolicy,
    AuthenticationRateLimitScope, BeginWebauthnCeremony, BrowserBindingHash,
    CommitCourseRosterImport, CompletePasskeyAuthenticationAndCreateSession,
    ConsumeAuthenticationRateLimit, CourseInvitationLifetime, CourseInvitationSecretHash,
    CourseRosterId, CourseRosterImportLifetime, CourseRosterImportRowInput,
    CourseRosterMemberSource, CourseRosterStore, CreateCourseInvitation, CredentialIdHash,
    PageRequest, PageSize, PasskeyId, PasskeyRecord, RegisterPasskey, RosterIdempotencyKey,
    RosterImportInvitation, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    StageCourseRosterImport, StoreError, TenantContext, WebauthnCeremonyId, WebauthnCeremonyKind,
    WebauthnCeremonyLifetime, WebauthnState,
};
use objects::Sha256Digest;
use question_model::{CourseId, CourseRole, TenantId, UserId, UserRole};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn database_error_code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned())
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_enrollment_capability_is_locked_unique_and_role_separated() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");

    let tenant = id();
    let course = id();
    let instructor = id();
    let first_invitation = id();
    let second_invitation = id();
    let token_hash = [0x5a_u8; 32];
    let mut transaction = pool.begin().await.expect("begin capability fixture");
    sqlx::query("INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind(course)
        .bind("Disposable enrollment capability")
        .execute(&mut *transaction)
        .await
        .expect("insert fixture course");
    sqlx::query("INSERT INTO course_roster_state (tenant_id, course_id) VALUES ($1, $2)")
        .bind(tenant)
        .bind(course)
        .execute(&mut *transaction)
        .await
        .expect("insert fixture roster state");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("set trusted fixture tenant");
    sqlx::query(
        "INSERT INTO course_invitation \
         (tenant_id, course_id, invitation_id, token_hash, normalized_email, delivery_email, \
          roster_id, invited_by, idempotency_key, expires_at) \
         VALUES ($1, $2, $3, $4, 'learner@example.edu', 'learner@example.edu', \
                 '900000001', $5, 'first-invitation', transaction_timestamp() + interval '1 day')",
    )
    .bind(tenant)
    .bind(course)
    .bind(first_invitation)
    .bind(token_hash.as_slice())
    .bind(instructor)
    .execute(&mut *transaction)
    .await
    .expect("insert first invitation");
    let duplicate = sqlx::query(
        "INSERT INTO course_invitation \
         (tenant_id, course_id, invitation_id, token_hash, normalized_email, delivery_email, \
          roster_id, invited_by, idempotency_key, expires_at) \
         VALUES ($1, $2, $3, $4, 'learner@example.edu', 'learner@example.edu', \
                 '900000002', $5, 'second-invitation', transaction_timestamp() + interval '1 day') \
         ON CONFLICT DO NOTHING",
    )
    .bind(tenant)
    .bind(course)
    .bind(second_invitation)
    .bind([0x6b_u8; 32].as_slice())
    .bind(instructor)
    .execute(&mut *transaction)
    .await
    .expect("duplicate invitation probe");
    assert_eq!(duplicate.rows_affected(), 0);

    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    let row = sqlx::query(
        "SELECT tenant_id, course_id, roster_id, current_setting('ple.tenant_id', true) AS current_tenant \
           FROM public.ple_claim_course_invitation_context($1)",
    )
    .bind(token_hash.as_slice())
    .fetch_one(&mut *transaction)
    .await
    .expect("invitation capability should lock and resolve");
    assert_eq!(row.try_get::<Uuid, _>("tenant_id").unwrap(), tenant);
    assert_eq!(row.try_get::<Uuid, _>("course_id").unwrap(), course);
    assert_eq!(row.try_get::<String, _>("roster_id").unwrap(), "900000001");
    assert_eq!(
        row.try_get::<String, _>("current_tenant").unwrap(),
        tenant.to_string()
    );
    transaction
        .rollback()
        .await
        .expect("rollback capability fixture");

    let mut application_probe = pool.begin().await.expect("begin application role probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *application_probe)
        .await
        .expect("assume application role");
    let application_error = sqlx::query("SELECT user_id FROM ple_account LIMIT 1")
        .execute(&mut *application_probe)
        .await
        .expect_err("educational-record role must not read global accounts");
    assert_eq!(
        database_error_code(&application_error).as_deref(),
        Some("42501")
    );
    application_probe
        .rollback()
        .await
        .expect("rollback application role probe");

    let mut auth_probe = pool.begin().await.expect("begin auth role probe");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *auth_probe)
        .await
        .expect("assume auth role");
    let auth_error = sqlx::query("SELECT course_member_id FROM course_roster_member LIMIT 1")
        .execute(&mut *auth_probe)
        .await
        .expect_err("auth role must not read tenant roster records");
    assert_eq!(database_error_code(&auth_error).as_deref(), Some("42501"));
    auth_probe
        .rollback()
        .await
        .expect("rollback auth role probe");

    let store = PostgresStore::new(pool.clone());
    let command = ConsumeAuthenticationRateLimit {
        scope: AuthenticationRateLimitScope::Email,
        key: AuthenticationRateLimitKey::compute(id().as_bytes()),
        policy: AuthenticationRateLimitPolicy::new(2, 60).expect("bounded live policy"),
    };
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("first durable allowance"),
        AuthenticationRateLimitDecision::Allowed {
            remaining_attempts: 1
        }
    );
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("second durable allowance"),
        AuthenticationRateLimitDecision::Allowed {
            remaining_attempts: 0
        }
    );
    assert!(matches!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("durable denial"),
        AuthenticationRateLimitDecision::Denied {
            retry_after_seconds: 1..=60
        }
    ));

    let managed_tenant = TenantId::from_uuid(id());
    let managed_course = CourseId::from_uuid(id());
    let administrator = UserId::from_uuid(id());
    let administrator_session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            administrator_session,
            SessionSubject::new(
                managed_tenant,
                administrator,
                "Tenant administrator",
                vec![UserRole::Administrator],
            )
            .expect("valid administrator session subject"),
            SessionLifetime::from_seconds(3_600).expect("bounded session lifetime"),
        )
        .await
        .expect("administrator session should persist");
    sqlx::query("INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3)")
        .bind(managed_tenant.as_uuid())
        .bind(managed_course.as_uuid())
        .bind("Tenant-admin managed course")
        .execute(&pool)
        .await
        .expect("insert course without direct administrator membership");

    let managed_context = TenantContext::from_authenticated_session(managed_tenant);
    let invitation = store
        .create_course_invitation(
            managed_context,
            administrator_session,
            CreateCourseInvitation {
                course: managed_course,
                email: AuthenticationEmail::parse("admin-invited@example.edu")
                    .expect("valid invitation email"),
                roster_id: CourseRosterId::parse("900999001").expect("valid roster identifier"),
                token_hash: CourseInvitationSecretHash::compute(b"live-admin-invitation"),
                idempotency_key: RosterIdempotencyKey::parse("live-admin-invitation")
                    .expect("valid invitation idempotency key"),
                lifetime: CourseInvitationLifetime::from_seconds(86_400)
                    .expect("bounded invitation lifetime"),
            },
        )
        .await
        .expect("tenant administrator should manage a course without direct membership");
    assert_eq!(invitation.invited_by, administrator);
    let roster = store
        .list_course_roster(
            managed_context,
            administrator_session,
            managed_course,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("tenant administrator should read the managed roster");
    assert_eq!(roster.entries.items.len(), 1);

    let preview = store
        .stage_course_roster_import(
            managed_context,
            administrator_session,
            StageCourseRosterImport {
                course: managed_course,
                expected_roster_revision: roster.policy.revision,
                normalized_digest: Sha256Digest::compute(b"live-normalized-roster-import"),
                idempotency_key: RosterIdempotencyKey::parse("live-roster-stage")
                    .expect("valid stage key"),
                rows: vec![CourseRosterImportRowInput {
                    row_number: 2,
                    email: Some(
                        AuthenticationEmail::parse("bulk-live@example.edu")
                            .expect("valid bulk invitation email"),
                    ),
                    roster_id: Some(
                        CourseRosterId::parse("900999002").expect("valid bulk roster identifier"),
                    ),
                }],
                lifetime: CourseRosterImportLifetime::from_seconds(3_600)
                    .expect("bounded import lifetime"),
            },
        )
        .await
        .expect("normalized roster preview should persist");
    let commit_command = CommitCourseRosterImport {
        course: managed_course,
        import: preview.id,
        expected_import_revision: preview.revision,
        idempotency_key: RosterIdempotencyKey::parse("live-roster-commit")
            .expect("valid commit key"),
        invitations: vec![RosterImportInvitation {
            row_number: 2,
            token_hash: CourseInvitationSecretHash::compute(b"live-bulk-invitation"),
            idempotency_key: RosterIdempotencyKey::parse("live-roster-row-2")
                .expect("valid row key"),
            lifetime: CourseInvitationLifetime::from_seconds(86_400)
                .expect("bounded invitation lifetime"),
        }],
    };
    let committed = store
        .commit_course_roster_import(
            managed_context,
            administrator_session,
            commit_command.clone(),
        )
        .await
        .expect("ready roster rows should commit atomically");
    assert_eq!(committed.invitations.len(), 1);
    assert_eq!(committed.invitations[0].0, 2);
    assert_eq!(
        store
            .commit_course_roster_import(managed_context, administrator_session, commit_command,)
            .await
            .expect("database commit retry should be idempotent"),
        committed
    );

    let account_user = UserId::from_uuid(id());
    sqlx::query(
        "INSERT INTO ple_account \
         (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, 'passkey-live@example.edu', 'passkey-live@example.edu', 'Passkey live')",
    )
    .bind(account_user.as_uuid())
    .execute(&pool)
    .await
    .expect("insert disposable passwordless account");
    sqlx::query(
        "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
         VALUES ($1, $2, $3, 'instructor')",
    )
    .bind(managed_tenant.as_uuid())
    .bind(managed_course.as_uuid())
    .bind(account_user.as_uuid())
    .execute(&pool)
    .await
    .expect("insert disposable account course relationship");
    let contexts = store
        .list_account_course_contexts(
            account_user,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("account course context page through the narrow broker");
    assert_eq!(contexts.items.len(), 1);
    assert_eq!(contexts.items[0].tenant, managed_tenant);
    assert_eq!(contexts.items[0].course, managed_course);
    assert_eq!(contexts.items[0].role, CourseRole::Instructor);
    assert_eq!(
        store
            .resolve_account_course_context(account_user, managed_course)
            .await
            .expect("account course context lookup")
            .expect("proven membership")
            .tenant,
        managed_tenant
    );
    assert_eq!(
        store
            .resolve_account_course_context(UserId::from_uuid(id()), managed_course)
            .await
            .expect("nonmember lookup"),
        None
    );
    let binding = BrowserBindingHash::compute(b"live-webauthn-browser");
    let ceremony = store
        .begin_webauthn_ceremony(BeginWebauthnCeremony {
            id: WebauthnCeremonyId::from_uuid(id()),
            kind: WebauthnCeremonyKind::Registration { user: account_user },
            browser_binding: binding,
            state: WebauthnState::new(br#"{"registration":"state"}"#.to_vec())
                .expect("bounded ceremony state"),
            lifetime: WebauthnCeremonyLifetime::from_seconds(600)
                .expect("bounded ceremony lifetime"),
        })
        .await
        .expect("database-timed ceremony should persist");
    assert_eq!(
        store
            .take_webauthn_ceremony(ceremony.id, binding)
            .await
            .expect("take ceremony"),
        Some(ceremony)
    );

    let passkey = store
        .insert_passkey(RegisterPasskey {
            id: PasskeyId::from_uuid(id()),
            user: account_user,
            credential_id_hash: CredentialIdHash::compute(b"live-credential-id"),
            label: "Disposable passkey".to_string(),
            credential: WebauthnState::new(br#"{"credential":"initial"}"#.to_vec())
                .expect("bounded credential"),
        })
        .await
        .expect("passkey registration should use database time");
    let completed = store
        .complete_passkey_authentication_and_create_session(
            CompletePasskeyAuthenticationAndCreateSession {
                passkey: PasskeyRecord {
                    credential: WebauthnState::new(br#"{"credential":"updated"}"#.to_vec())
                        .expect("bounded updated credential"),
                    ..passkey
                },
                session_token_hash: learning_data_access::AccountSessionTokenHash::compute(
                    b"live-account-session",
                ),
                session_lifetime: learning_data_access::AccountSessionLifetime::from_seconds(900)
                    .expect("bounded account session"),
            },
        )
        .await
        .expect("passkey update and account session should commit atomically");
    assert_eq!(completed.session.user, account_user);
    assert!(completed.passkey.last_used_at.is_some());
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_local_development_activation_is_atomic_idempotent_and_tenant_scoped() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let manager = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let conflicting_instructor = UserId::from_uuid(id());
    let assignments = [id(), id()];
    let manager_session = SessionTokenHash::compute(id().as_bytes());
    let outsider_session = SessionTokenHash::compute(id().as_bytes());
    let context = TenantContext::from_authenticated_session(tenant);

    for (user, roles, token) in [
        (manager, vec![UserRole::Instructor], manager_session),
        (outsider, vec![UserRole::Instructor], outsider_session),
    ] {
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
    sqlx::query("INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3)")
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
    for user in [manager, conflicting_instructor] {
        sqlx::query(
            "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
             VALUES ($1, $2, $3, 'instructor')",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(user.as_uuid())
        .execute(&mut *fixture)
        .await
        .expect("insert local roster manager fixture");
    }
    for (index, assignment) in assignments.iter().enumerate() {
        sqlx::query(
            "INSERT INTO assignment (tenant_id, assignment_id, course_id, title) \
             VALUES ($1, $2, $3, $4)",
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

    let command = ActivateLocalDevelopmentCourseMember {
        course,
        learner_user: learner,
        learner_display_name: "Local Learner".to_string(),
    };
    let (first, second) = tokio::join!(
        store.activate_local_development_course_member(context, manager_session, command.clone()),
        store.activate_local_development_course_member(context, manager_session, command),
    );
    let first = first.expect("first local activation");
    let second = second.expect("concurrent local activation retry");
    assert_eq!(first, second, "retries must not duplicate a local learner");
    assert_eq!(
        first.member.source,
        CourseRosterMemberSource::LocalDevelopment
    );
    assert_eq!(first.member.roster_email, None);
    assert_eq!(first.member.roster_id, None);

    let roster = store
        .list_course_roster(
            context,
            manager_session,
            course,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("manager local roster read");
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

    let unauthorized = store
        .activate_local_development_course_member(
            context,
            outsider_session,
            ActivateLocalDevelopmentCourseMember {
                course,
                learner_user: UserId::from_uuid(id()),
                learner_display_name: "Unauthorized Learner".to_string(),
            },
        )
        .await;
    assert!(
        matches!(
            unauthorized,
            Err(StoreError::Forbidden | StoreError::NotFound)
        ),
        "a persisted session without course management rights cannot activate local learners: {unauthorized:?}"
    );
    let foreign = store
        .activate_local_development_course_member(
            TenantContext::from_authenticated_session(foreign_tenant),
            manager_session,
            ActivateLocalDevelopmentCourseMember {
                course,
                learner_user: UserId::from_uuid(id()),
                learner_display_name: "Foreign Learner".to_string(),
            },
        )
        .await;
    assert!(
        matches!(foreign, Err(StoreError::Forbidden | StoreError::NotFound)),
        "a foreign tenant context must not activate a local learner: {foreign:?}"
    );

    assert_eq!(
        store
            .activate_local_development_course_member(
                context,
                manager_session,
                ActivateLocalDevelopmentCourseMember {
                    course,
                    learner_user: conflicting_instructor,
                    learner_display_name: "Conflicting Instructor".to_string(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a local learner cannot replace an instructor course membership"
    );
    let stored_conflict = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM course_roster_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3",
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
