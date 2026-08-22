use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{CourseRecord, CreateCourseCommand, Store, TenantContext};
use question_model::{TenantId, UserId};
use tower::ServiceExt;

use super::*;
use crate::auth::{CookieTransport, clear_session_cookie};

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
        CookieTransport::FirstPartyHttps,
    );
    assert!(clear_session_cookie(config).starts_with("__Host-ple_session="));
    assert!(
        clear_named_cookie(ACCOUNT_SESSION_COOKIE, config)
            .starts_with("__Host-ple_account_session=")
    );
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
    let principal = issuer
        .key(
            AuthenticationRateLimitScope::Principal,
            b"student@example.edu",
        )
        .expect("configured issuer");
    let service = issuer
        .key(
            AuthenticationRateLimitScope::Service,
            b"student@example.edu",
        )
        .expect("configured issuer");
    assert_ne!(email, network);
    assert_ne!(email, principal);
    assert_ne!(network, service);
    assert_eq!(
        format!("{email:?}"),
        "AuthenticationRateLimitKey([redacted])"
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
        ClientAddressPolicy::direct(),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(60).expect("session lifetime"),
            CookieTransport::FirstPartyHttps,
        ),
    )
    .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 55], 443))));
    let mut throttled_without_delivery = false;
    for _ in 0..100 {
        let delivered_before = delivery.count();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/passwordless/email/start")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "198.51.100.200")
                    .body(Body::from(r#"{"email":"learner@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        if delivery.count() == delivered_before {
            throttled_without_delivery = true;
            break;
        }
    }
    assert!(
        throttled_without_delivery,
        "the uniform endpoint must eventually stop delivering email"
    );
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
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"account-context-limit"),
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
        .create_course(
            TenantContext::from_authenticated_session(tenant),
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: user,
            },
        )
        .await
        .expect("course");
    let session_config = SessionConfig::new(
        learning_data_access::SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        CookieTransport::FirstPartyHttps,
    );
    let app = passwordless_router(
        Arc::clone(&store),
        Arc::new(RecordingEmailDelivery::default()),
        PasswordlessRateLimitIssuer::from_server_secret([0x55; 32]),
        ClientAddressPolicy::direct(),
        session_config,
    )
    .merge(crate::auth::session_router(
        Arc::clone(&store),
        session_config,
    ));
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
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/course-session")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(
                    serde_json::json!({ "courseId": course }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("course selection");
    assert_eq!(selected.status(), StatusCode::OK);
    let tenant_cookie = selected
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("__Host-ple_session="))
        .and_then(|value| value.split(';').next())
        .expect("course selection should issue tenant session")
        .to_string();
    let browser_cookies = format!("{cookie}; {tenant_cookie}");

    let logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("cookie", &browser_cookies)
                .body(Body::empty())
                .expect("logout request"),
        )
        .await
        .expect("logout response");
    assert_eq!(logout.status(), StatusCode::OK);
    let cleared = logout
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>();
    for name in [
        "__Host-ple_session=",
        "__Host-ple_account_session=",
        "__Host-ple_email_binding=",
        "__Host-ple_webauthn_binding=",
    ] {
        assert!(
            cleared
                .iter()
                .any(|value| value.starts_with(name) && value.contains("Max-Age=0")),
            "logout must clear {name}"
        );
    }
    assert_eq!(
        store
            .resolve_account_session(AccountSessionTokenHash::compute(&account_secret.0))
            .await
            .expect("account-session lookup"),
        None
    );
    assert!(matches!(
        crate::auth::resolve_session(store.as_ref(), Some(&tenant_cookie)).await,
        Err(AuthError::Unauthenticated)
    ));

    let replacement = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/course-session")
                .header("content-type", "application/json")
                .header("cookie", browser_cookies)
                .body(Body::from(
                    serde_json::json!({ "courseId": course }).to_string(),
                ))
                .expect("replacement request"),
        )
        .await
        .expect("replacement response");
    assert_eq!(replacement.status(), StatusCode::UNAUTHORIZED);
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
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"original-email-limit"),
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
    let route_delivery: Arc<dyn PasswordlessEmailDelivery> = delivery.clone();
    let app = passwordless_router(
        Arc::clone(&store),
        route_delivery,
        PasswordlessRateLimitIssuer::from_server_secret([0x62; 32]),
        ClientAddressPolicy::direct(),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            CookieTransport::FirstPartyHttps,
        ),
    )
    .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 61], 443))));
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
            value.starts_with("__Host-ple_email_binding=").then(|| {
                value
                    .split(';')
                    .next()
                    .expect("cookie pair")
                    .replacen("__Host-", "", 1)
            })
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
    let replacement_account_cookie = complete
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value.starts_with("__Host-ple_account_session=").then(|| {
                value
                    .split(';')
                    .next()
                    .expect("cookie pair")
                    .replacen("__Host-", "", 1)
            })
        })
        .expect("replacement account cookie");
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

    let stale_session = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/email/start")
                .header("content-type", "application/json")
                .header("cookie", &account_cookie)
                .body(Body::from(r#"{"email":"stale@example.edu"}"#))
                .expect("stale-session request"),
        )
        .await
        .expect("stale-session response");
    assert_eq!(stale_session.status(), StatusCode::UNAUTHORIZED);
    let replacement_session = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/account/email/start")
                .header("content-type", "application/json")
                .header("cookie", &replacement_account_cookie)
                .body(Body::from(r#"{"email":"replacement@example.edu"}"#))
                .expect("replacement-session request"),
        )
        .await
        .expect("replacement-session response");
    assert_eq!(replacement_session.status(), StatusCode::ACCEPTED);

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

#[tokio::test]
async fn account_email_change_has_principal_network_and_recoverable_retry_budget() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(3_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid::Uuid::from_u128(60_100));
    let token_hash = EmailChallengeSecretHash::compute(b"principal-limit-token");
    let browser_binding = BrowserBindingHash::compute(b"principal-limit-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid::Uuid::from_u128(60_101)),
            token_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"principal-limit-email"),
            email: AuthenticationEmail::parse("principal@example.edu").expect("email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash,
            browser_binding,
            proposed_user: user,
            proposed_display_name: "Principal Learner".to_string(),
        })
        .await
        .expect("account");
    let account_secret = RandomSecret([0x63; SECRET_BYTES]);
    store
        .create_account_session(
            AccountSessionTokenHash::compute(&account_secret.0),
            user,
            AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS).expect("lifetime"),
        )
        .await
        .expect("account session");
    let delivery = Arc::new(RecordingEmailDelivery::default());
    let route_delivery: Arc<dyn PasswordlessEmailDelivery> = delivery.clone();
    let app = passwordless_router(
        Arc::clone(&store),
        route_delivery,
        PasswordlessRateLimitIssuer::from_server_secret([0x64; 32]),
        ClientAddressPolicy::direct(),
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            CookieTransport::FirstPartyHttps,
        ),
    )
    .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 63], 443))));
    let account_cookie = format!("{ACCOUNT_SESSION_COOKIE}={}", account_secret.encoded());
    let mut accepted = 0;
    let mut throttled = false;
    for attempt in 0..100 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/account/email/start")
                    .header("content-type", "application/json")
                    .header("cookie", &account_cookie)
                    .body(Body::from(
                        serde_json::json!({ "email": format!("new-{attempt}@example.edu") })
                            .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        match response.status() {
            StatusCode::ACCEPTED => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u32>().ok());
                assert!(retry_after.is_some_and(|seconds| seconds > 0));
                throttled = true;
                break;
            }
            status => panic!("unexpected email-change response: {status}"),
        }
    }
    assert!(
        throttled,
        "principal quota must bound cross-address retries"
    );
    assert_eq!(delivery.count(), accepted);
}
