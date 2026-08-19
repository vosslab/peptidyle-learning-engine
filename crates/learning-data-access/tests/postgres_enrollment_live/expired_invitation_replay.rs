#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_expired_invitation_replay_materializes_terminal_delivery_before_conflict() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let sysadmin = UserId::from_uuid(id());
    let session = SessionTokenHash::compute(id().as_bytes());
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                sysadmin,
                "Expired replay Sysadmin",
                vec![UserRole::Sysadmin],
            )
            .expect("valid Sysadmin session"),
            SessionLifetime::from_seconds(3_600).expect("bounded session lifetime"),
        )
        .await
        .expect("persist Sysadmin session");
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind("Expired replay course")
        .execute(&pool)
        .await
        .expect("insert disposable course");
    let command = CreateCourseInvitation {
        course,
        email: AuthenticationEmail::parse("expired-replay@example.edu").expect("valid email"),
        roster_id: CourseRosterId::parse("900998001").expect("valid roster ID"),
        token_hash: CourseInvitationSecretHash::compute(b"expired-replay-token"),
        idempotency_key: RosterIdempotencyKey::parse("expired-replay-key")
            .expect("valid idempotency key"),
        lifetime: CourseInvitationLifetime::from_seconds(86_400).expect("bounded lifetime"),
    };
    let invitation = store
        .create_course_invitation(context, session, command.clone())
        .await
        .expect("create invitation before expiring its fixture clock");
    sqlx::query(
        "UPDATE course_invitation SET created_at = transaction_timestamp() - interval '2 days', \
             expires_at = transaction_timestamp() - interval '1 day' \
         WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(invitation.id.as_uuid())
    .execute(&pool)
    .await
    .expect("make the durable pending invitation expired without bypassing its lifecycle route");
    assert_eq!(
        store
            .create_course_invitation(context, session, command)
            .await,
        Err(StoreError::Conflict),
        "an exact replay of a now-expired invitation cannot revive it"
    );
    let terminal = sqlx::query_as::<_, (String, String, String)>(
        "SELECT invitation.status, delivery.state, delivery.outcome_code \
         FROM course_invitation AS invitation \
         JOIN course_invitation_delivery AS delivery \
           USING (tenant_id, course_id, invitation_id) \
         WHERE invitation.tenant_id = $1 AND invitation.course_id = $2 \
           AND invitation.invitation_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read terminal invitation and delivery state");
    assert_eq!(
        terminal,
        (
            "expired".to_string(),
            "cancelled".to_string(),
            "cancelled".to_string()
        ),
        "the idempotency replay durably materializes expiry and its delivery cancellation"
    );
    assert_eq!(
        store
            .revoke_course_invitation(
                context,
                session,
                RevokeCourseInvitation {
                    course,
                    invitation: invitation.id,
                    expected_revision: learning_data_access::RosterRevision::from_stored(2)
                        .expect("initial invitation revision"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "revoke after expiry must reject instead of replacing the terminal state"
    );
    let after_revoke = sqlx::query_as::<_, (String, String, String)>(
        "SELECT invitation.status, delivery.state, delivery.outcome_code \
         FROM course_invitation AS invitation \
         JOIN course_invitation_delivery AS delivery \
           USING (tenant_id, course_id, invitation_id) \
         WHERE invitation.tenant_id = $1 AND invitation.course_id = $2 \
           AND invitation.invitation_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read terminal state after rejected revoke");
    assert_eq!(after_revoke, terminal);
    let audits = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_event WHERE tenant_id = $1 AND course_id = $2 \
           AND actor_id = $3 AND action = 'sysadmin.rosterSupport'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(sysadmin.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count support audits");
    assert_eq!(
        audits, 1,
        "expiry maintenance alone creates no support audit"
    );
}
