//! Fast authenticated HTTP contracts for reusable-curriculum routes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, COOKIE};
use axum::http::{Method, Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
    CompleteEmailAuthentication, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeLifetime, EmailChallengeSecretHash, SessionLifetime, SessionSubject,
    TeachingAuthorityStore, TenantContext,
};
use question_model::{TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

const TENANT: u128 = 96_100;
const ELENA: u128 = 96_101;
const MORGAN: u128 = 96_102;
const STUDENT: u128 = 96_103;

struct Fixture {
    app: axum::Router,
    elena_cookie: String,
    student_cookie: String,
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(Uuid::from_u128(TENANT));
    let elena = UserId::from_uuid(Uuid::from_u128(ELENA));
    create_account(store.as_ref(), elena).await;
    let morgan = issue_cookie(
        store.as_ref(),
        tenant,
        UserId::from_uuid(Uuid::from_u128(MORGAN)),
        vec![UserRole::Instructor, UserRole::Sysadmin],
        "Morgan",
    )
    .await;
    store
        .approve_instructor_account(
            TenantContext::from_authenticated_session(tenant),
            ApproveInstructorAccount {
                session: morgan.1,
                target: elena,
                expected_revision: None,
            },
        )
        .await
        .expect("Morgan approves Elena");
    let elena = issue_cookie(
        store.as_ref(),
        tenant,
        elena,
        vec![UserRole::Instructor],
        "Elena",
    )
    .await;
    let student = issue_cookie(
        store.as_ref(),
        tenant,
        UserId::from_uuid(Uuid::from_u128(STUDENT)),
        vec![UserRole::Student],
        "Student",
    )
    .await;
    Fixture {
        app: server_core::reusable_curriculum::router(store),
        elena_cookie: elena.0,
        student_cookie: student.0,
    }
}

async fn create_account(store: &MemoryStore, user: UserId) {
    let token = EmailChallengeSecretHash::compute(b"curriculum-http-account-token");
    let binding = BrowserBindingHash::compute(b"curriculum-http-account-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(Uuid::from_u128(96_110)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"curriculum-http-rate"),
            email: AuthenticationEmail::parse("elena@example.edu").expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("fixture lifetime"),
        })
        .await
        .expect("fixture account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "Elena Instructor".to_owned(),
        })
        .await
        .expect("fixture account");
}

async fn issue_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    display_name: &str,
) -> (String, learning_data_access::SessionTokenHash) {
    let issued = server_core::auth::issue_session(
        store,
        SessionSubject::new(tenant, user, display_name, roles).expect("fixture session"),
        server_core::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("fixture lifetime"),
            server_core::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture session issued");
    (
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_owned(),
        issued.record.token_hash,
    )
}

fn request(method: Method, uri: &str, cookie: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(COOKIE, cookie)
        .body(body.into())
        .expect("request")
}

async fn dispatch(app: &axum::Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("route response")
}

fn assert_no_store(response: &axum::response::Response) {
    assert_eq!(
        response
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
}

#[tokio::test]
async fn approved_instructor_lists_curriculum_with_no_store() {
    let fixture = fixture().await;
    for path in [
        "/api/course-blueprints?pageSize=1",
        "/api/alpha-courses?pageSize=1",
    ] {
        let response = dispatch(
            &fixture.app,
            request(Method::GET, path, &fixture.elena_cookie, Body::empty()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_no_store(&response);
    }
}

#[tokio::test]
async fn role_preflight_precedes_protected_decoding_and_errors_are_not_cacheable() {
    let fixture = fixture().await;
    let cases = [
        request(
            Method::GET,
            "/api/course-blueprints?unknown=value",
            &fixture.student_cookie,
            Body::empty(),
        ),
        request(
            Method::GET,
            "/api/course-blueprints/not-a-blueprint",
            &fixture.student_cookie,
            Body::empty(),
        ),
        request(
            Method::PUT,
            "/api/course-blueprints/BP-1",
            &fixture.student_cookie,
            r#"{"surprise":true}"#,
        ),
        request(
            Method::POST,
            "/api/alpha-courses",
            &fixture.student_cookie,
            r#"{"surprise":true}"#,
        ),
    ];
    for request in cases {
        let response = dispatch(&fixture.app, request).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_no_store(&response);
    }
}

#[tokio::test]
async fn documented_bodies_are_strict() {
    let fixture = fixture().await;
    let strict = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/course-blueprints",
            &fixture.elena_cookie,
            r#"{"surprise":true}"#,
        ),
    )
    .await;
    assert_eq!(strict.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_no_store(&strict);
}
