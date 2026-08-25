#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_email_change_conflict_rolls_back_without_revoking_prior_sessions() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool);
    let owner = UserId::from_uuid(id());
    let conflicting_owner = UserId::from_uuid(id());
    for (user, email, token, binding) in [
        (
            owner,
            format!("owner-{}@example.edu", id()),
            EmailChallengeSecretHash::compute(id().as_bytes()),
            BrowserBindingHash::compute(id().as_bytes()),
        ),
        (
            conflicting_owner,
            format!("conflict-{}@example.edu", id()),
            EmailChallengeSecretHash::compute(id().as_bytes()),
            BrowserBindingHash::compute(id().as_bytes()),
        ),
    ] {
        store
            .begin_email_authentication(BeginEmailAuthentication {
                id: EmailChallengeId::from_uuid(id()),
                token_hash: token,
                browser_binding: binding,
                email_rate_limit_key: AuthenticationRateLimitKey::compute(id().as_bytes()),
                email: AuthenticationEmail::parse(&email).expect("valid unique email"),
                purpose: EmailAuthenticationPurpose::SignInOrRegister,
                lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
            })
            .await
            .expect("persist account challenge");
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                token_hash: token,
                browser_binding: binding,
                proposed_user: user,
                proposed_display_name: "Rollback Learner".to_string(),
            })
            .await
            .expect("persist account");
    }
    let stale_account = AccountSessionTokenHash::compute(id().as_bytes());
    let stale_tenant = SessionTokenHash::compute(id().as_bytes());
    store
        .create_account_session(
            stale_account,
            owner,
            AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        )
        .await
        .expect("persist account proof");
    store
        .create_session(
            stale_tenant,
            SessionSubject::new(
                TenantId::from_uuid(id()),
                owner,
                "Rollback Learner",
                vec![UserRole::Student],
            )
            .expect("valid tenant subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("persist tenant proof");

    let change_token = EmailChallengeSecretHash::compute(id().as_bytes());
    let change_binding = BrowserBindingHash::compute(id().as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id()),
            token_hash: change_token,
            browser_binding: change_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(id().as_bytes()),
            email: store
                .get_account(conflicting_owner)
                .await
                .expect("conflicting account lookup")
                .expect("conflicting account")
                .email,
            purpose: EmailAuthenticationPurpose::ChangeEmail { user: owner },
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("persist conflicting replacement challenge");
    assert_eq!(
        store
            .complete_email_change_and_revoke_user_sessions(
                CompleteEmailChangeAndRevokeUserSessions {
                    authentication: CompleteEmailAuthentication {
                        token_hash: change_token,
                        browser_binding: change_binding,
                        proposed_user: owner,
                        proposed_display_name: "Rollback Learner".to_string(),
                    },
                    session_token_hash: AccountSessionTokenHash::compute(id().as_bytes()),
                    session_lifetime: AccountSessionLifetime::from_seconds(900).expect("lifetime"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "the unique-email failure must abort the entire Store transaction"
    );
    assert!(
        store
            .resolve_account_session(stale_account)
            .await
            .expect("stale account lookup")
            .is_some()
            && store
                .resolve_session(stale_tenant)
                .await
                .expect("stale tenant lookup")
                .is_some(),
        "rollback must preserve all prior bearer proofs"
    );
}
