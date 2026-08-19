#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_passwordless_challenge_consumption_is_binding_atomic() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool);
    let browser_binding = BrowserBindingHash::compute(b"durable issuing browser");
    let foreign_binding = BrowserBindingHash::compute(b"durable forwarded browser");

    let user = UserId::from_uuid(id());
    let token_hash = EmailChallengeSecretHash::compute(id().as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id()),
            token_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(id().as_bytes()),
            email: AuthenticationEmail::parse(&format!("binding-{}@example.edu", id()))
                .expect("valid unique email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("persist durable email challenge");
    let command = CompleteEmailAuthentication {
        token_hash,
        browser_binding,
        proposed_user: user,
        proposed_display_name: "Binding atomic learner".to_string(),
    };
    assert_eq!(
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                browser_binding: foreign_binding,
                ..command.clone()
            })
            .await,
        Err(StoreError::NotFound),
        "a mismatched browser binding must leave the durable email challenge usable"
    );
    store
        .complete_email_authentication(command)
        .await
        .expect("the issuing browser can still consume the email challenge");

    let concurrent_token_hash = EmailChallengeSecretHash::compute(id().as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id()),
            token_hash: concurrent_token_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(id().as_bytes()),
            email: AuthenticationEmail::parse(&format!("concurrent-{}@example.edu", id()))
                .expect("valid unique email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("persist concurrent durable email challenge");
    let concurrent_command = CompleteEmailAuthentication {
        token_hash: concurrent_token_hash,
        browser_binding,
        proposed_user: UserId::from_uuid(id()),
        proposed_display_name: "Concurrent learner".to_string(),
    };
    let first_store = store.clone();
    let second_store = store.clone();
    let (first, second) = tokio::join!(
        first_store.complete_email_authentication(concurrent_command.clone()),
        second_store.complete_email_authentication(concurrent_command),
    );
    assert_eq!(
        [first, second]
            .into_iter()
            .filter(|result| result.is_ok())
            .count(),
        1,
        "exactly one concurrent valid email completion may consume the challenge"
    );

    let ceremony = store
        .begin_webauthn_ceremony(BeginWebauthnCeremony {
            id: WebauthnCeremonyId::from_uuid(id()),
            kind: WebauthnCeremonyKind::Registration { user },
            browser_binding,
            state: WebauthnState::new(br#"{"binding":"atomic"}"#.to_vec())
                .expect("valid ceremony state"),
            lifetime: WebauthnCeremonyLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("persist durable WebAuthn ceremony");
    assert_eq!(
        store
            .take_webauthn_ceremony(ceremony.id, foreign_binding)
            .await
            .expect("mismatched durable ceremony take"),
        None,
        "a mismatched browser binding must leave the WebAuthn ceremony usable"
    );
    assert_eq!(
        store
            .take_webauthn_ceremony(ceremony.id, browser_binding)
            .await
            .expect("issuing browser takes ceremony"),
        Some(ceremony),
    );

    let concurrent_ceremony = store
        .begin_webauthn_ceremony(BeginWebauthnCeremony {
            id: WebauthnCeremonyId::from_uuid(id()),
            kind: WebauthnCeremonyKind::Registration { user },
            browser_binding,
            state: WebauthnState::new(br#"{"binding":"concurrent"}"#.to_vec())
                .expect("valid ceremony state"),
            lifetime: WebauthnCeremonyLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("persist concurrent durable WebAuthn ceremony");
    let first_store = store.clone();
    let second_store = store.clone();
    let (first, second) = tokio::join!(
        first_store.take_webauthn_ceremony(concurrent_ceremony.id, browser_binding),
        second_store.take_webauthn_ceremony(concurrent_ceremony.id, browser_binding),
    );
    assert_eq!(
        [first, second]
            .into_iter()
            .filter(|result| matches!(result, Ok(Some(_))))
            .count(),
        1,
        "exactly one concurrent valid WebAuthn ceremony take may succeed"
    );
}
