use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{CourseRecord, Store, TenantContext};
use question_model::{CourseMembership, CourseMembershipRole, TenantId, UserId};
use tower::ServiceExt;

use super::*;
use crate::auth::clear_session_cookie;

#[derive(Default)]
struct RecordingEmailDelivery {
    count: AtomicUsize,
    deliveries: Mutex<Vec<(PasswordlessEmailAction, String)>>,
}

impl RecordingEmailDelivery {
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    fn take_last(&self) -> (PasswordlessEmailAction, String) {
        self.deliveries
            .lock()
            .expect("recording delivery lock")
            .pop()
            .expect("recorded email delivery")
    }
}

#[async_trait]
impl PasswordlessEmailDelivery for RecordingEmailDelivery {
    fn is_configured(&self) -> bool {
        true
    }

    async fn send_email_authentication(
        &self,
        _email: &AuthenticationEmail,
        secret: &PasswordlessEmailSecret,
        action: PasswordlessEmailAction,
    ) -> Result<(), PasswordlessEmailDeliveryError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.deliveries
            .lock()
            .expect("recording delivery lock")
            .push((action, secret.encoded()));
        Ok(())
    }
}

#[test]
fn random_secret_accepts_only_canonical_256_bit_values() {
    let secret = RandomSecret([0x42; 32]);
    assert_eq!(RandomSecret::decode(&secret.encoded()), Some(secret));
    assert_eq!(RandomSecret::decode("not-a-secret"), None);
}

#[test]
fn clear_session_cookie_remains_separate_from_account_proof() {
    let config = SessionConfig::new(
        learning_data_access::SessionLifetime::from_seconds(60).expect("lifetime"),
        CookieTransport::LocalHttp,
    );
    assert!(clear_session_cookie(config).starts_with("ple_session="));
    assert!(clear_named_cookie(ACCOUNT_SESSION_COOKIE, config).starts_with("ple_account_session="));
}

#[test]
fn rate_limit_keys_are_domain_separated_and_never_contain_input() {
    let issuer = PasswordlessRateLimitIssuer::from_server_secret([0x44; 32]);
    let email = issuer
        .key(AuthenticationRateLimitScope::Email, b"student@example.edu")
        .expect("configured issuer");
    let network = issuer
        .key(
            AuthenticationRateLimitScope::Network,
            b"student@example.edu",
        )
        .expect("configured issuer");
    assert_ne!(email, network);
    assert_eq!(
        format!("{email:?}"),
        "AuthenticationRateLimitKey([redacted])"
    );
}

#[test]
fn client_network_header_is_one_exact_ip_or_the_shared_unknown_bucket() {
    let mut headers = HeaderMap::new();
    headers.insert(CLIENT_IP_HEADER, HeaderValue::from_static("192.0.2.44"));
    assert_eq!(network_rate_limit_identity(&headers), b"192.0.2.44");
    headers.append(CLIENT_IP_HEADER, HeaderValue::from_static("192.0.2.45"));
    assert_eq!(
        network_rate_limit_identity(&headers),
        b"unknown-client-network"
    );
}

#[tokio::test]
async fn repeated_email_starts_keep_uniform_response_and_stop_delivery_at_limit() {
    let store = Arc::new(MemoryStore::default());
    let delivery = Arc::new(RecordingEmailDelivery::default());
    let app = passwordless_router(
        store,
        delivery.clone(),
        PasswordlessRateLimitIssuer::from_server_secret([0x45; 32]),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(60).expect("session lifetime"),
            CookieTransport::LocalHttp,
        ),
    );
    for _ in 0..=EMAIL_RATE_LIMIT_ATTEMPTS {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/passwordless/email/start")
                    .header("content-type", "application/json")
                    .header(CLIENT_IP_HEADER, "192.0.2.55")
                    .body(Body::from(r#"{"email":"learner@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
    assert_eq!(delivery.count(), EMAIL_RATE_LIMIT_ATTEMPTS as usize);
}

#[tokio::test]
async fn account_course_selection_derives_tenant_and_role_from_store_membership() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid::Uuid::from_u128(50_001));
    let email_token = EmailChallengeSecretHash::compute(b"account-context-email-token");
    let email_binding = BrowserBindingHash::compute(b"account-context-email-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid::Uuid::from_u128(50_002)),
            token_hash: email_token,
            browser_binding: email_binding,
            email: AuthenticationEmail::parse("teacher@example.edu").expect("email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("email challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: email_token,
            browser_binding: email_binding,
            proposed_user: user,
            proposed_display_name: "Course Teacher".to_string(),
        })
        .await
        .expect("account");
    let account_secret = RandomSecret([0x5a; SECRET_BYTES]);
    store
        .create_account_session(
            AccountSessionTokenHash::compute(&account_secret.0),
            user,
            AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
                .expect("account lifetime"),
        )
        .await
        .expect("account session");
    let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(50_003));
    let course = CourseId::from_uuid(uuid::Uuid::from_u128(50_004));
    store
        .upsert_course(
            TenantContext::from_authenticated_session(tenant),
            CourseRecord {
                id: course,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![CourseMembership {
                    user,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course");
    let app = passwordless_router(
        Arc::clone(&store),
        Arc::new(RecordingEmailDelivery::default()),
        PasswordlessRateLimitIssuer::from_server_secret([0x55; 32]),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            CookieTransport::LocalHttp,
        ),
    );
    let cookie = format!("{ACCOUNT_SESSION_COOKIE}={}", account_secret.encoded());

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/account/courses")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("course list");
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = to_bytes(list.into_body(), 16 * 1_024)
        .await
        .expect("bounded list body");
    let list_json: serde_json::Value = serde_json::from_slice(&list_body).expect("list JSON");
    assert_eq!(list_json["courses"][0]["courseId"], course.to_string());
    assert_eq!(list_json["courses"][0]["role"], "instructor");
    assert!(list_json.to_string().find("tenant").is_none());
    assert!(list_json.to_string().find("email").is_none());

    let selected = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/course-session")
                .header("content-type", "application/json")
                .header("cookie", cookie)
                .body(Body::from(
                    serde_json::json!({ "courseId": course }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("course selection");
    assert_eq!(selected.status(), StatusCode::OK);
    assert!(
        selected
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| value.starts_with("ple_session="))
    );
}

#[tokio::test]
async fn signed_in_account_changes_email_only_after_new_address_verification() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(2_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid::Uuid::from_u128(60_001));
    let original_token = EmailChallengeSecretHash::compute(b"original-email-token");
    let original_binding = BrowserBindingHash::compute(b"original-email-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid::Uuid::from_u128(60_002)),
            token_hash: original_token,
            browser_binding: original_binding,
            email: AuthenticationEmail::parse("learner@example.edu").expect("original email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("original email challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: original_token,
            browser_binding: original_binding,
            proposed_user: user,
            proposed_display_name: "Course Learner".to_string(),
        })
        .await
        .expect("account");
    let account_secret = RandomSecret([0x61; SECRET_BYTES]);
    store
        .create_account_session(
            AccountSessionTokenHash::compute(&account_secret.0),
            user,
            AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
                .expect("account lifetime"),
        )
        .await
        .expect("account session");
    let delivery = Arc::new(RecordingEmailDelivery::default());
    let app = passwordless_router(
        Arc::clone(&store),
        delivery.clone(),
        PasswordlessRateLimitIssuer::from_server_secret([0x62; 32]),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            CookieTransport::LocalHttp,
        ),
    );
    let account_cookie = format!("{ACCOUNT_SESSION_COOKIE}={}", account_secret.encoded());
    let started = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/email/start")
                .header("content-type", "application/json")
                .header("cookie", &account_cookie)
                .body(Body::from(r#"{"email":"new.learner@example.edu"}"#))
                .expect("start request"),
        )
        .await
        .expect("start response");
    assert_eq!(started.status(), StatusCode::ACCEPTED);
    let binding_cookie = started
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value
                .starts_with(EMAIL_BINDING_COOKIE)
                .then(|| value.split(';').next().expect("cookie pair").to_string())
        })
        .expect("browser binding cookie");
    let (action, token) = delivery.take_last();
    assert_eq!(action, PasswordlessEmailAction::ChangeEmail);

    let complete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/email/complete")
                .header("content-type", "application/json")
                .header("cookie", format!("{account_cookie}; {binding_cookie}"))
                .body(Body::from(
                    serde_json::json!({ "token": token }).to_string(),
                ))
                .expect("complete request"),
        )
        .await
        .expect("complete response");
    assert_eq!(complete.status(), StatusCode::OK);
    assert_eq!(
        store
            .get_account(user)
            .await
            .expect("account lookup")
            .expect("account")
            .email
            .normalized(),
        "new.learner@example.edu"
    );

    let replay = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/email/complete")
                .header("content-type", "application/json")
                .header("cookie", format!("{account_cookie}; {binding_cookie}"))
                .body(Body::from(
                    serde_json::json!({ "token": token }).to_string(),
                ))
                .expect("replay request"),
        )
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
}
