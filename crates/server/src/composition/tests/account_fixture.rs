use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, AuthenticationEmail, AuthenticationRateLimitKey,
    BeginEmailAuthentication, BrowserBindingHash, CompleteEmailAuthentication,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
};
use question_model::UserId;

pub(super) async fn provision_account(store: &MemoryStore, user: UserId, display_name: &str) {
    let challenge = EmailChallengeSecretHash::compute(b"composition-account");
    let binding = BrowserBindingHash::compute(b"composition-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid::Uuid::from_u128(0x937)),
            token_hash: challenge,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"composition-limit"),
            email: AuthenticationEmail::parse(&format!("composition-{user}@example.edu"))
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("fixture lifetime"),
        })
        .await
        .expect("account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: challenge,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: display_name.to_string(),
        })
        .await
        .expect("fixture account");
}
