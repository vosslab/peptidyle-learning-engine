use super::*;
use crate::{
    AccountIdentityStore, AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationEmail, AuthenticationRateLimitKey, BeginEmailAuthentication,
    BeginWebauthnCeremony, BrowserBindingHash, CompleteEmailAuthentication,
    CompleteSeededSysadminOwnership, CredentialIdHash, EmailAuthenticationPurpose,
    EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash, PasskeyId, RegisterPasskey,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, TenantId, WebauthnCeremonyId,
    WebauthnCeremonyKind, WebauthnCeremonyLifetime, WebauthnState, validated_passkey_label,
};
use question_model::UserRole;
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn seeded_account(store: &MemoryStore, user: UserId, sysadmin: bool) {
    let binding = BrowserBindingHash::compute(user.as_uuid().as_bytes());
    let token = EmailChallengeSecretHash::compute(user.as_uuid().as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(user.as_uuid().as_u128() + 1)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(user.as_uuid().as_bytes()),
            email: AuthenticationEmail::parse(&format!("{user}@example.edu"))
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("seed account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "Seeded Account".to_string(),
        })
        .await
        .expect("seed account");
    if sysadmin {
        store
            .write_state()
            .expect("fixture state")
            .accounts
            .get_mut(&user)
            .expect("seeded account")
            .platform_roles = vec![UserRole::Sysadmin];
    }
}

async fn ownership_command(
    store: &MemoryStore,
    user: UserId,
    id: u128,
) -> CompleteSeededSysadminOwnership {
    let binding = BrowserBindingHash::compute(&id.to_be_bytes());
    let ceremony_id = WebauthnCeremonyId::from_uuid(uuid(id));
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
            id: PasskeyId::from_uuid(uuid(id + 1)),
            user,
            credential_id_hash: CredentialIdHash::compute(&(id + 2).to_be_bytes()),
            label: validated_passkey_label("First passkey").expect("label"),
            credential: WebauthnState::new(br#"{"credential":"verified"}"#.to_vec())
                .expect("credential"),
        },
        session_token_hash: AccountSessionTokenHash::compute(&(id + 3).to_be_bytes()),
        session_lifetime: AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        presented_account_session: None,
        presented_tenant_session: None,
    }
}

#[tokio::test]
async fn seeded_sysadmin_ownership_is_atomic_and_single_use() {
    let store = MemoryStore::default();
    let user = UserId::from_uuid(uuid(0x90_001));
    let ordinary = UserId::from_uuid(uuid(0x90_002));
    assert_eq!(
        store.seeded_sysadmin_ownership_available(user).await,
        Err(StoreError::NotFound)
    );
    seeded_account(&store, user, true).await;
    seeded_account(&store, ordinary, false).await;
    assert_eq!(
        store.seeded_sysadmin_ownership_available(ordinary).await,
        Err(StoreError::Forbidden)
    );
    assert!(
        store
            .seeded_sysadmin_ownership_available(user)
            .await
            .expect("unclaimed Sysadmin is available")
    );
    let command = ownership_command(&store, user, 0x90_100).await;
    let completed = store
        .complete_seeded_sysadmin_ownership(command.clone())
        .await
        .expect("first ownership completion");
    assert_eq!(completed.passkey.user, user);
    assert_eq!(completed.session.user, user);
    assert_eq!(
        store
            .resolve_account_session(command.session_token_hash)
            .await
            .expect("account-session lookup"),
        Some(completed.session),
    );
    assert_eq!(
        store.complete_seeded_sysadmin_ownership(command).await,
        Err(StoreError::Conflict),
        "a historical passkey keeps ownership permanently claimed"
    );
}

#[tokio::test]
async fn seeded_sysadmin_ownership_replaces_only_presented_browser_sessions_atomically() {
    let store = MemoryStore::default();
    let user = UserId::from_uuid(uuid(0x90_101));
    let visitor = UserId::from_uuid(uuid(0x90_102));
    let tenant = TenantId::from_uuid(uuid(0x90_103));
    seeded_account(&store, user, true).await;
    seeded_account(&store, visitor, false).await;

    let old_account = AccountSessionTokenHash::compute(b"presented-account");
    let other_account = AccountSessionTokenHash::compute(b"other-account");
    let old_tenant = SessionTokenHash::compute(b"presented-tenant");
    let other_tenant = SessionTokenHash::compute(b"other-tenant");
    let account_lifetime = AccountSessionLifetime::from_seconds(900).expect("lifetime");
    let tenant_lifetime = SessionLifetime::from_seconds(900).expect("lifetime");
    store
        .create_account_session(old_account, user, account_lifetime)
        .await
        .expect("presented account session");
    store
        .create_account_session(other_account, visitor, account_lifetime)
        .await
        .expect("other account session");
    for (token, session_user, display_name) in [
        (old_tenant, user, "Presented Browser"),
        (other_tenant, visitor, "Other Browser"),
    ] {
        store
            .create_session(
                token,
                SessionSubject::new(tenant, session_user, display_name, vec![UserRole::Student])
                    .expect("session subject"),
                tenant_lifetime,
            )
            .await
            .expect("tenant session");
    }

    let mut command = ownership_command(&store, user, 0x90_110).await;
    command.presented_account_session = Some(old_account);
    command.presented_tenant_session = Some(old_tenant);
    let replacement = store
        .complete_seeded_sysadmin_ownership(command.clone())
        .await
        .expect("ownership completion");

    assert_eq!(
        store.resolve_account_session(old_account).await.unwrap(),
        None
    );
    assert_eq!(store.resolve_session(old_tenant).await.unwrap(), None);
    assert!(
        store
            .resolve_account_session(other_account)
            .await
            .unwrap()
            .is_some()
    );
    assert!(store.resolve_session(other_tenant).await.unwrap().is_some());
    assert_eq!(
        store
            .resolve_account_session(command.session_token_hash)
            .await
            .unwrap(),
        Some(replacement.session)
    );
    assert!(
        !store
            .seeded_sysadmin_ownership_available(user)
            .await
            .expect("claimed is unavailable")
    );

    let another = UserId::from_uuid(uuid(0x90_120));
    seeded_account(&store, another, true).await;
    let stale = AccountSessionTokenHash::compute(b"failed-presented-account");
    let collision = AccountSessionTokenHash::compute(b"collision");
    store
        .create_account_session(stale, another, account_lifetime)
        .await
        .expect("old account before failure");
    store
        .create_account_session(collision, visitor, account_lifetime)
        .await
        .expect("conflicting replacement token");
    let mut rejected = ownership_command(&store, another, 0x90_130).await;
    rejected.presented_account_session = Some(stale);
    rejected.session_token_hash = collision;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(rejected.clone())
            .await,
        Err(StoreError::AlreadyExists)
    );
    assert!(
        store
            .resolve_account_session(stale)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .get_webauthn_ceremony(rejected.ceremony_id, rejected.browser_binding)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .list_active_passkeys(another)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn seeded_sysadmin_ownership_rejects_ordinary_and_revoked_history_without_consuming() {
    let store = MemoryStore::default();
    let ordinary = UserId::from_uuid(uuid(0x90_201));
    seeded_account(&store, ordinary, false).await;
    let ordinary_command = ownership_command(&store, ordinary, 0x90_300).await;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(ordinary_command.clone())
            .await,
        Err(StoreError::Forbidden)
    );
    assert!(
        store
            .take_webauthn_ceremony(
                ordinary_command.ceremony_id,
                ordinary_command.browser_binding
            )
            .await
            .expect("ordinary ceremony remains")
            .is_some()
    );

    let sysadmin = UserId::from_uuid(uuid(0x90_202));
    seeded_account(&store, sysadmin, true).await;
    let prior = store
        .insert_passkey(RegisterPasskey {
            id: PasskeyId::from_uuid(uuid(0x90_401)),
            user: sysadmin,
            credential_id_hash: CredentialIdHash::compute(b"historical-passkey"),
            label: validated_passkey_label("Old passkey").expect("label"),
            credential: WebauthnState::new(br#"{"credential":"old"}"#.to_vec())
                .expect("credential"),
        })
        .await
        .expect("historical passkey");
    store
        .revoke_passkey(sysadmin, prior.id)
        .await
        .expect("revoke historical passkey");
    assert!(
        !store
            .seeded_sysadmin_ownership_available(sysadmin)
            .await
            .expect("revoked credential permanently closes ownership")
    );
    let command = ownership_command(&store, sysadmin, 0x90_500).await;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(command.clone())
            .await,
        Err(StoreError::Conflict),
        "revoked historical credentials also mark the account claimed"
    );
    assert!(
        store
            .take_webauthn_ceremony(command.ceremony_id, command.browser_binding)
            .await
            .expect("rejected claim preserves ceremony")
            .is_some()
    );
}

#[tokio::test]
async fn seeded_sysadmin_ownership_requires_the_configured_target_everywhere() {
    let store = MemoryStore::default();
    let morgan = UserId::from_uuid(uuid(0x90_601));
    let another_sysadmin = UserId::from_uuid(uuid(0x90_602));
    seeded_account(&store, morgan, true).await;
    seeded_account(&store, another_sysadmin, true).await;

    let mut mismatched = ownership_command(&store, another_sysadmin, 0x90_700).await;
    mismatched.target = morgan;
    assert_eq!(
        store
            .complete_seeded_sysadmin_ownership(mismatched.clone())
            .await,
        Err(StoreError::Forbidden),
        "a second passkey-free Sysadmin cannot claim Morgan's configured target"
    );
    assert!(
        store
            .get_webauthn_ceremony(mismatched.ceremony_id, mismatched.browser_binding)
            .await
            .expect("mismatched claim lookup")
            .is_some(),
        "a rejected target mismatch preserves the verified-credential ceremony"
    );

    let command = ownership_command(&store, morgan, 0x90_800).await;
    let found = store
        .get_webauthn_ceremony(command.ceremony_id, command.browser_binding)
        .await
        .expect("matching ceremony lookup")
        .expect("matching ceremony");
    assert_eq!(
        found.kind,
        WebauthnCeremonyKind::Registration { user: morgan }
    );
    assert_eq!(
        store
            .get_webauthn_ceremony(
                command.ceremony_id,
                BrowserBindingHash::compute(b"wrong browser")
            )
            .await
            .expect("wrong binding lookup"),
        None,
        "a ceremony state read is browser-bound"
    );
    assert!(
        store
            .get_webauthn_ceremony(command.ceremony_id, command.browser_binding)
            .await
            .expect("non-consuming ceremony lookup")
            .is_some(),
        "the verifier can reread its matching ceremony before atomic completion"
    );
}
