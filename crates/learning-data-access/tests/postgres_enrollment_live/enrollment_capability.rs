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
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
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
    let auth_error = sqlx::query("SELECT course_membership_id FROM course_member LIMIT 1")
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
    let sysadmin = UserId::from_uuid(id());
    let sysadmin_session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            sysadmin_session,
            SessionSubject::new(
                managed_tenant,
                sysadmin,
                "Sysadmin",
                vec![UserRole::Sysadmin],
            )
            .expect("valid sysadmin session subject"),
            SessionLifetime::from_seconds(3_600).expect("bounded session lifetime"),
        )
        .await
        .expect("sysadmin session should persist");
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
        .bind(managed_tenant.as_uuid())
        .bind(managed_course.as_uuid())
        .bind("Sysadmin-created course")
        .execute(&pool)
        .await
        .expect("insert course without direct sysadmin membership");

    let managed_context = TenantContext::from_authenticated_session(managed_tenant);
    let invitation_command = CreateCourseInvitation {
        course: managed_course,
        email: AuthenticationEmail::parse("sysadmin-invited@example.edu")
            .expect("valid invitation email"),
        roster_id: CourseRosterId::parse("900999001").expect("valid roster identifier"),
        token_hash: CourseInvitationSecretHash::compute(b"live-sysadmin-invitation"),
        idempotency_key: RosterIdempotencyKey::parse("live-sysadmin-invitation")
            .expect("valid invitation idempotency key"),
        lifetime: CourseInvitationLifetime::from_seconds(86_400)
            .expect("bounded invitation lifetime"),
    };
    let invitation = store
        .create_course_invitation(
            managed_context,
            sysadmin_session,
            invitation_command.clone(),
        )
        .await
        .expect("sysadmin roster-support capability should authorize the narrow mutation");
    assert_eq!(invitation.invited_by, sysadmin);
    assert_eq!(
        store
            .create_course_invitation(
                managed_context,
                sysadmin_session,
                invitation_command.clone()
            )
            .await
            .expect("identical invitation replay"),
        invitation
    );
    assert_eq!(
        store
            .create_course_invitation(
                managed_context,
                sysadmin_session,
                CreateCourseInvitation {
                    token_hash: CourseInvitationSecretHash::compute(id().as_bytes()),
                    ..invitation_command.clone()
                },
            )
            .await,
        Err(StoreError::Conflict),
        "one idempotency key cannot change its invitation payload"
    );
    assert_eq!(
        store
            .course_invitation_delivery_state(
                managed_context,
                sysadmin_session,
                managed_course,
                invitation.id,
            )
            .await
            .expect("single invitation delivery projection"),
        Some(CourseInvitationDeliveryState::Pending)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM course_invitation_delivery WHERE invitation_id = $1",
        )
        .bind(invitation.id.as_uuid())
        .fetch_one(&pool)
        .await
        .expect("one delivery row probe"),
        1,
        "identical invitation replay creates no second durable delivery"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT revision FROM course_roster_state WHERE tenant_id = $1 AND course_id = $2",
        )
        .bind(managed_tenant.as_uuid())
        .bind(managed_course.as_uuid())
        .fetch_one(&pool)
        .await
        .expect("read invitation replay roster revision"),
        2,
        "identical invitation replay creates no second roster revision"
    );
    let roster = store
        .list_course_roster(
            managed_context,
            sysadmin_session,
            managed_course,
            PageRequest::first(PageSize::new(20).expect("bounded page size")),
        )
        .await
        .expect("sysadmin roster-support capability should authorize the narrow disclosure");
    assert_eq!(roster.entries.items.len(), 1);
    let support_actions = sqlx::query_scalar::<_, String>(
        "SELECT payload ->> 'supportAction' FROM audit_event \
         WHERE tenant_id = $1 AND course_id = $2 AND actor_id = $3 \
           AND action = 'sysadmin.rosterSupport' ORDER BY payload ->> 'supportAction'",
    )
    .bind(managed_tenant.as_uuid())
    .bind(managed_course.as_uuid())
    .bind(sysadmin.as_uuid())
    .fetch_all(&pool)
    .await
    .expect("read roster-support audit evidence");
    assert_eq!(
        support_actions,
        vec![
            "createInvitation".to_string(),
            "createInvitation".to_string(),
            "listRoster".to_string(),
        ],
        "each successful Sysadmin invitation request, including its exact replay, commits one audit event without duplicating delivery"
    );

    let preview = store
        .stage_course_roster_import(
            managed_context,
            sysadmin_session,
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
    assert_eq!(
        store
            .stage_course_roster_import(
                managed_context,
                sysadmin_session,
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
                            CourseRosterId::parse("900999002")
                                .expect("valid bulk roster identifier"),
                        ),
                    }],
                    lifetime: CourseRosterImportLifetime::from_seconds(3_600)
                        .expect("bounded preview lifetime"),
                },
            )
            .await
            .expect("stage replay should disclose the same preview with a fresh support audit"),
        preview
    );
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
        .commit_course_roster_import(managed_context, sysadmin_session, commit_command.clone())
        .await
        .expect("ready roster rows should commit atomically");
    assert_eq!(committed.invitations.len(), 1);
    assert_eq!(committed.invitations[0].0, 2);
    assert_eq!(
        store
            .commit_course_roster_import(managed_context, sysadmin_session, commit_command.clone(),)
            .await
            .expect("database commit retry should be idempotent"),
        committed
    );
    assert_eq!(
        store
            .commit_course_roster_import(
                managed_context,
                sysadmin_session,
                CommitCourseRosterImport {
                    idempotency_key: RosterIdempotencyKey::parse("live-roster-commit-changed")
                        .expect("valid changed commit key"),
                    ..commit_command.clone()
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a committed import refuses a second commit identity"
    );
    assert_eq!(
        store
            .course_invitation_delivery_state(
                managed_context,
                sysadmin_session,
                managed_course,
                committed.invitations[0].1.id,
            )
            .await
            .expect("bulk invitation delivery projection"),
        Some(CourseInvitationDeliveryState::Pending)
    );
    let replay_support_actions = sqlx::query_scalar::<_, String>(
        "SELECT payload ->> 'supportAction' FROM audit_event \
         WHERE tenant_id = $1 AND course_id = $2 AND actor_id = $3 \
           AND action = 'sysadmin.rosterSupport' ORDER BY payload ->> 'supportAction', audit_event_id",
    )
    .bind(managed_tenant.as_uuid())
    .bind(managed_course.as_uuid())
    .bind(sysadmin.as_uuid())
    .fetch_all(&pool)
    .await
    .expect("read Sysadmin replay audit evidence");
    assert_eq!(
        replay_support_actions,
        vec![
            "commitImport".to_string(),
            "commitImport".to_string(),
            "createInvitation".to_string(),
            "createInvitation".to_string(),
            "listRoster".to_string(),
            "stageImport".to_string(),
            "stageImport".to_string(),
        ],
        "each successful replay creates one support audit while invitation and import effects remain idempotent"
    );
    let unchanged_policy = store
        .replace_course_enrollment_policy(
            managed_context,
            sysadmin_session,
            ReplaceCourseEnrollmentPolicy {
                course: managed_course,
                expected_revision: committed.roster_revision,
                allowed_domains: Default::default(),
                signup_posture: CourseSignupPosture::InvitationOnly,
            },
        )
        .await
        .expect("Sysadmin policy no-op returns its protected current policy");
    assert_eq!(unchanged_policy.revision, committed.roster_revision);

    let revoked_invitation_revision = store
        .revoke_course_invitation(
            managed_context,
            sysadmin_session,
            RevokeCourseInvitation {
                course: managed_course,
                invitation: committed.invitations[0].1.id,
                expected_revision: committed.roster_revision,
            },
        )
        .await
        .expect("revoke pending invitation");
    assert_eq!(
        store
            .revoke_course_invitation(
                managed_context,
                sysadmin_session,
                RevokeCourseInvitation {
                    course: managed_course,
                    invitation: committed.invitations[0].1.id,
                    expected_revision: revoked_invitation_revision,
                },
            )
            .await
            .expect("Sysadmin invitation revoke replay returns the unchanged revision"),
        revoked_invitation_revision
    );

    let member = store
        .upsert_course_member(
            managed_context,
            UpsertCourseMember {
                course: managed_course,
                user: UserId::from_uuid(id()),
                display_name: "Disposable audited member".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create a disposable canonical member for revoke replay");
    let revoked_member_revision = store
        .revoke_course_member(
            managed_context,
            sysadmin_session,
            RevokeCourseMember {
                course: managed_course,
                member: member.member.id,
                expected_revision: member.roster_revision,
            },
        )
        .await
        .expect("revoke active member");
    assert_eq!(
        store
            .revoke_course_member(
                managed_context,
                sysadmin_session,
                RevokeCourseMember {
                    course: managed_course,
                    member: member.member.id,
                    expected_revision: revoked_member_revision,
                },
            )
            .await
            .expect("Sysadmin member revoke replay returns the unchanged revision"),
        revoked_member_revision
    );
    let action_counts = sqlx::query_as::<_, (String, i64)>(
        "SELECT payload ->> 'supportAction', count(*) \
         FROM audit_event WHERE tenant_id = $1 AND course_id = $2 AND actor_id = $3 \
           AND action = 'sysadmin.rosterSupport' \
         GROUP BY payload ->> 'supportAction' ORDER BY payload ->> 'supportAction'",
    )
    .bind(managed_tenant.as_uuid())
    .bind(managed_course.as_uuid())
    .bind(sysadmin.as_uuid())
    .fetch_all(&pool)
    .await
    .expect("count every successful Sysadmin roster action");
    assert_eq!(
        action_counts,
        vec![
            ("commitImport".to_string(), 2),
            ("createInvitation".to_string(), 2),
            ("listRoster".to_string(), 1),
            ("replaceEnrollmentPolicy".to_string(), 1),
            ("revokeInvitation".to_string(), 2),
            ("revokeMember".to_string(), 2),
            ("stageImport".to_string(), 2),
        ],
        "every successful Sysadmin response, including a no-op or replay, commits one support audit without duplicate domain effects"
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
        "INSERT INTO course_member \
         (tenant_id, course_id, course_membership_id, user_id, student_id, role, \
          status, roster_id, joined_at, revoked_at) \
         VALUES ($1, $2, gen_random_uuid(), $3, NULL, 'instructor', 'active', \
                 NULL, transaction_timestamp(), NULL)",
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
    assert_eq!(contexts.items[0].role, CourseMembershipRole::Instructor);
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

    verify_student_context_visibility_before_tenant_selection(&pool).await;
    verify_instructor_context_order_and_paging(&pool).await;
}
