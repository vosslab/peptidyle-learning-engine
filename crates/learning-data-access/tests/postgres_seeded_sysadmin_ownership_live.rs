#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for seeded Sysadmin first ownership.

use learning_data_access::postgres::{
    BaseCourseAccountPlatformRoles, BaseCourseAccountRecipe, PostgresStore,
    acquire_base_course_install_lock, lazy_pool, verify_application_schema,
};
use learning_data_access::{
    AccountIdentityStore, AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationEmail, BeginWebauthnCeremony, BrowserBindingHash,
    CompleteSeededSysadminOwnership, CredentialIdHash, PasskeyId, RegisterPasskey, SessionLifetime,
    SessionStore, SessionSubject, SessionTokenHash, StoreError, WebauthnCeremonyId,
    WebauthnCeremonyKind, WebauthnCeremonyLifetime, WebauthnState, validated_passkey_label,
};
use question_model::{TenantId, UserId, UserRole};
use sqlx::query_scalar;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn provision_seeded_sysadmin(pool: &sqlx::PgPool, user: UserId) {
    let recipe = BaseCourseAccountRecipe::new(
        user,
        AuthenticationEmail::parse(&format!("seeded-{user}@example.edu")).expect("fixture email"),
        "Seeded Sysadmin",
        BaseCourseAccountPlatformRoles::Sysadmin,
    )
    .expect("fixture recipe");
    let mut install = acquire_base_course_install_lock(pool)
        .await
        .expect("installer lock");
    install
        .provision_accounts(&[recipe])
        .await
        .expect("provision seeded Sysadmin");
    install.release().await.expect("release installer lock");
}

async fn command(
    store: &PostgresStore,
    user: UserId,
    ceremony_id: WebauthnCeremonyId,
    binding: BrowserBindingHash,
) -> CompleteSeededSysadminOwnership {
    store
        .begin_webauthn_ceremony(BeginWebauthnCeremony {
            id: ceremony_id,
            kind: WebauthnCeremonyKind::Registration { user },
            browser_binding: binding,
            state: WebauthnState::new(br#"{"registration":"verified"}"#.to_vec())
                .expect("ceremony state"),
            lifetime: WebauthnCeremonyLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("registration ceremony");
    CompleteSeededSysadminOwnership {
        target: user,
        ceremony_id,
        browser_binding: binding,
        passkey: RegisterPasskey {
            id: PasskeyId::from_uuid(id()),
            user,
            credential_id_hash: CredentialIdHash::compute(id().as_bytes()),
            label: validated_passkey_label("Initial passkey").expect("label"),
            credential: WebauthnState::new(br#"{"credential":"verified"}"#.to_vec())
                .expect("credential"),
        },
        session_token_hash: AccountSessionTokenHash::compute(id().as_bytes()),
        session_lifetime: AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        presented_account_session: None,
        presented_tenant_session: None,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_seeded_sysadmin_ownership_is_atomic_and_irreversible() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let user = UserId::from_uuid(id());
    let another_sysadmin = UserId::from_uuid(id());
    let tenant = TenantId::from_uuid(id());
    provision_seeded_sysadmin(&pool, user).await;
    provision_seeded_sysadmin(&pool, another_sysadmin).await;

    let mismatched_binding = BrowserBindingHash::compute(id().as_bytes());
    let mut mismatched = command(
        &store,
        another_sysadmin,
        WebauthnCeremonyId::from_uuid(id()),
        mismatched_binding,
    )
    .await;
    mismatched.target = user;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(mismatched.clone())
            .await,
        Err(StoreError::Forbidden),
        "a second passkey-free Sysadmin cannot claim the configured target"
    );
    assert!(
        store
            .get_webauthn_ceremony(mismatched.ceremony_id, mismatched.browser_binding)
            .await
            .expect("mismatched ceremony lookup")
            .is_some(),
        "a target mismatch leaves the verifier ceremony available"
    );

    let first = command(
        &store,
        user,
        WebauthnCeremonyId::from_uuid(id()),
        BrowserBindingHash::compute(id().as_bytes()),
    )
    .await;
    let second = command(
        &store,
        user,
        WebauthnCeremonyId::from_uuid(id()),
        BrowserBindingHash::compute(id().as_bytes()),
    )
    .await;
    let presented_account = AccountSessionTokenHash::compute(id().as_bytes());
    let unrelated_account = AccountSessionTokenHash::compute(id().as_bytes());
    let presented_tenant = SessionTokenHash::compute(id().as_bytes());
    let unrelated_tenant = SessionTokenHash::compute(id().as_bytes());
    let account_lifetime = AccountSessionLifetime::from_seconds(900).expect("lifetime");
    let tenant_lifetime = SessionLifetime::from_seconds(900).expect("lifetime");
    store
        .create_account_session(presented_account, user, account_lifetime)
        .await
        .expect("presented account session");
    store
        .create_account_session(unrelated_account, another_sysadmin, account_lifetime)
        .await
        .expect("unrelated account session");
    for (token, session_user, display_name) in [
        (presented_tenant, user, "Presented browser"),
        (unrelated_tenant, another_sysadmin, "Unrelated browser"),
    ] {
        store
            .create_session(
                token,
                SessionSubject::new(tenant, session_user, display_name, vec![UserRole::Student])
                    .expect("fixture tenant session"),
                tenant_lifetime,
            )
            .await
            .expect("tenant session persists");
    }
    let mut first = first;
    first.presented_account_session = Some(presented_account);
    first.presented_tenant_session = Some(presented_tenant);
    let mut second = second;
    second.presented_account_session = Some(presented_account);
    second.presented_tenant_session = Some(presented_tenant);
    let persisted = store
        .get_webauthn_ceremony(first.ceremony_id, first.browser_binding)
        .await
        .expect("verifier ceremony lookup")
        .expect("matching current ceremony");
    assert_eq!(persisted.kind, WebauthnCeremonyKind::Registration { user });

    let first_store = store.clone();
    let second_store = store.clone();
    let (first_result, second_result) = tokio::join!(
        first_store.complete_seeded_sysadmin_ownership(first.clone()),
        second_store.complete_seeded_sysadmin_ownership(second.clone()),
    );
    let outcomes = [first_result, second_result];
    assert_eq!(
        outcomes.iter().filter(|result| result.is_ok()).count(),
        1,
        "exactly one simultaneous valid claim may complete"
    );
    assert!(
        outcomes.contains(&Err(StoreError::Conflict)),
        "the losing valid claim receives a safe already-claimed conflict"
    );
    let completed = outcomes
        .into_iter()
        .find_map(Result::ok)
        .expect("one completed ownership claim");
    assert_eq!(completed.passkey.user, user);
    assert_eq!(completed.session.user, user);
    assert_eq!(
        store
            .resolve_account_session(completed.session.token_hash)
            .await
            .expect("ordinary account-session lookup"),
        Some(completed.session),
    );
    assert_eq!(
        store
            .resolve_account_session(presented_account)
            .await
            .expect("presented account-session lookup"),
        None,
        "the completing browser's presented account session is replaced"
    );
    assert_eq!(
        store
            .resolve_session(presented_tenant)
            .await
            .expect("presented tenant-session lookup"),
        None,
        "the completing browser's presented tenant session is revoked"
    );
    assert!(
        store
            .resolve_account_session(unrelated_account)
            .await
            .expect("unrelated account-session lookup")
            .is_some(),
        "ownership completion does not revoke an unrelated account session"
    );
    assert!(
        store
            .resolve_session(unrelated_tenant)
            .await
            .expect("unrelated tenant-session lookup")
            .is_some(),
        "ownership completion does not revoke an unrelated tenant session"
    );
    assert_eq!(
        store.complete_seeded_sysadmin_ownership(first).await,
        Err(StoreError::Conflict),
        "replaying a presented claim remains permanently refused"
    );

    let passkey_count: i64 =
        query_scalar("SELECT count(*) FROM public.account_passkey WHERE user_id = $1")
            .bind(user.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("count exact target passkeys");
    let account_session_count: i64 = query_scalar(
        "SELECT count(*) FROM public.account_authentication_session WHERE user_id = $1",
    )
    .bind(user.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count exact target account sessions");
    let course_membership_count: i64 =
        query_scalar("SELECT count(*) FROM public.course_member WHERE user_id = $1")
            .bind(user.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("count target course memberships");
    let tenant_session_count: i64 = query_scalar(
        "SELECT count(*) FROM public.auth_session WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count target tenant sessions");
    let platform_roles: String =
        query_scalar("SELECT platform_roles::text FROM public.ple_account WHERE user_id = $1")
            .bind(user.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("read exact target platform role");
    assert_eq!(passkey_count, 1, "exactly one historical passkey persists");
    assert_eq!(
        account_session_count, 1,
        "exactly one account session persists"
    );
    assert_eq!(
        course_membership_count, 0,
        "claim creates no course membership"
    );
    assert_eq!(tenant_session_count, 0, "claim creates no tenant session");
    assert_eq!(platform_roles, r#"["sysadmin"]"#, "claim changes no roles");

    store
        .revoke_passkey(user, completed.passkey.id)
        .await
        .expect("revoke the first passkey");
    let binding = BrowserBindingHash::compute(id().as_bytes());
    let after_revocation =
        command(&store, user, WebauthnCeremonyId::from_uuid(id()), binding).await;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(after_revocation.clone())
            .await,
        Err(StoreError::Conflict),
        "any historical passkey keeps the seeded account claimed"
    );
    assert!(
        store
            .take_webauthn_ceremony(after_revocation.ceremony_id, binding)
            .await
            .expect("refused ceremony lookup")
            .is_some(),
        "a refused ownership completion rolls back ceremony consumption"
    );
}
