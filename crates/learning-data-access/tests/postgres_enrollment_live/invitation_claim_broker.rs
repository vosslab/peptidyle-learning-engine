#[derive(Clone, Copy)]
enum DeliveryFixture {
    Pending,
    ProviderAccepted,
}

struct ClaimFixture {
    tenant: TenantId,
    course: CourseId,
    invitation: Uuid,
    token: CourseInvitationSecretHash,
    email: AuthenticationEmail,
}

async fn seed_claim_fixture(
    pool: &sqlx::PgPool,
    delivery: DeliveryFixture,
    expired: bool,
) -> ClaimFixture {
    let fixture = ClaimFixture {
        tenant: TenantId::from_uuid(id()),
        course: CourseId::from_uuid(id()),
        invitation: id(),
        token: CourseInvitationSecretHash::compute(id().as_bytes()),
        email: AuthenticationEmail::parse("claimed-learner@example.edu").expect("fixture email"),
    };
    let mut transaction = pool.begin().await.expect("claim fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("trusted fixture tenant");
    sqlx::query(
        "INSERT INTO public.course(tenant_id,course_id,title,term_start_date,term_end_date,time_zone) \
         VALUES($1,$2,'Claim witness fixture',DATE '2026-01-01',DATE '2026-12-31','America/Chicago')",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect("fixture course");
    sqlx::query("INSERT INTO public.course_roster_state(tenant_id,course_id) VALUES($1,$2)")
        .bind(fixture.tenant.as_uuid())
        .bind(fixture.course.as_uuid())
        .execute(&mut *transaction)
        .await
        .expect("fixture roster state");
    sqlx::query(
        "INSERT INTO public.course_invitation(tenant_id,course_id,invitation_id,token_hash, \
         normalized_email,delivery_email,roster_id,invited_by,idempotency_key,created_at,expires_at) \
         VALUES($1,$2,$3,$4,$5,$5,'claim-fixture',$6,$7, \
                transaction_timestamp()-CASE WHEN $8 THEN interval '2 days' ELSE interval '0 seconds' END, \
                transaction_timestamp()+CASE WHEN $8 THEN interval '-1 second' ELSE interval '1 day' END)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.invitation)
    .bind(fixture.token.as_bytes().to_vec())
    .bind(fixture.email.delivery())
    .bind(id())
    .bind(format!("claim-fixture-{}", fixture.invitation))
    .bind(expired)
    .execute(&mut *transaction)
    .await
    .expect("fixture invitation");
    let (state, outcome, accepted_at) = match delivery {
        DeliveryFixture::Pending => ("pending", Option::<&str>::None, false),
        DeliveryFixture::ProviderAccepted => ("accepted_by_provider", Some("accepted"), true),
    };
    sqlx::query(
        "INSERT INTO public.course_invitation_delivery(tenant_id,course_id,invitation_id,delivery_id, \
         state,outcome_code,terminal_at,accepted_at) \
         VALUES($1,$2,$3,$4,$5,$6,CASE WHEN $7 THEN transaction_timestamp() ELSE NULL END,CASE WHEN $7 THEN transaction_timestamp() ELSE NULL END)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.invitation)
    .bind(id())
    .bind(state)
    .bind(outcome)
    .bind(accepted_at)
    .execute(&mut *transaction)
    .await
    .expect("fixture delivery");
    transaction.commit().await.expect("claim fixture commit");
    fixture
}

fn claim_command(
    fixture: &ClaimFixture,
    user: UserId,
    email: AuthenticationEmail,
) -> ClaimCourseInvitation {
    ClaimCourseInvitation {
        token_hash: fixture.token,
        user,
        verified_email: email,
        display_name: "Claimed Learner".to_string(),
    }
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn postgres_invitation_claim_broker_returns_one_checked_membership_and_truthful_delivery_terminal()
 {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated PostgreSQL schema");
    let store = PostgresStore::new(pool.clone());
    let fixture = seed_claim_fixture(&pool, DeliveryFixture::Pending, false).await;
    let learner = UserId::from_uuid(id());
    let first = store
        .claim_course_invitation(claim_command(&fixture, learner, fixture.email.clone()))
        .await
        .expect("first invitation claim");
    assert_eq!(first.tenant, fixture.tenant);
    assert_eq!(first.course, fixture.course);
    assert_eq!(first.member.user, learner);
    assert_eq!(first.member.roster_email.as_ref(), Some(&fixture.email));
    assert_eq!(first.roster_revision.value(), 2);
    assert_eq!(
        store
            .claim_course_invitation(claim_command(&fixture, learner, fixture.email.clone()))
            .await
            .expect("same learner claim replay"),
        first
    );
    let revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM public.course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("one roster revision advance");
    assert_eq!(revision, 2);
    let delivery: (String, Option<String>, Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT state,outcome_code, \
                floor(extract(epoch FROM terminal_at)*1000)::bigint, \
                floor(extract(epoch FROM accepted_at)*1000)::bigint \
         FROM public.course_invitation_delivery WHERE tenant_id=$1 AND course_id=$2 AND invitation_id=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.invitation)
    .fetch_one(&pool)
    .await
    .expect("pending delivery closes on claim");
    assert_eq!(delivery.0, "cancelled");
    assert_eq!(delivery.1.as_deref(), Some("cancelled"));
    assert!(delivery.2.is_some() && delivery.3.is_none());
    assert_eq!(
        store
            .claim_course_invitation(claim_command(
                &fixture,
                UserId::from_uuid(id()),
                fixture.email.clone()
            ))
            .await,
        Err(StoreError::AlreadyExists)
    );
    assert_eq!(
        store
            .claim_course_invitation(claim_command(
                &fixture,
                learner,
                AuthenticationEmail::parse("wrong@example.edu").expect("wrong fixture email"),
            ))
            .await,
        Err(StoreError::Forbidden)
    );

    let accepted = seed_claim_fixture(&pool, DeliveryFixture::ProviderAccepted, false).await;
    store
        .claim_course_invitation(claim_command(
            &accepted,
            UserId::from_uuid(id()),
            accepted.email.clone(),
        ))
        .await
        .expect("provider-accepted delivery remains claimable");
    let accepted_delivery: (String, Option<String>, Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT state,outcome_code, \
                floor(extract(epoch FROM terminal_at)*1000)::bigint, \
                floor(extract(epoch FROM accepted_at)*1000)::bigint \
         FROM public.course_invitation_delivery WHERE tenant_id=$1 AND course_id=$2 AND invitation_id=$3",
    )
    .bind(accepted.tenant.as_uuid())
    .bind(accepted.course.as_uuid())
    .bind(accepted.invitation)
    .fetch_one(&pool)
    .await
    .expect("provider accepted delivery remains truthful");
    assert_eq!(accepted_delivery.0, "accepted_by_provider");
    assert_eq!(accepted_delivery.1.as_deref(), Some("accepted"));
    assert!(accepted_delivery.2.is_some() && accepted_delivery.3.is_some());

    let expired = seed_claim_fixture(&pool, DeliveryFixture::Pending, true).await;
    assert_eq!(
        store
            .claim_course_invitation(claim_command(
                &expired,
                UserId::from_uuid(id()),
                expired.email.clone(),
            ))
            .await,
        Err(StoreError::NotFound)
    );
}
