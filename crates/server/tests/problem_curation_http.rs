//! Authenticated HTTP behavior for the D2 problem-curation route group.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, COOKIE, ETAG, IF_MATCH};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
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

const TENANT: u128 = 94_001;
const ELENA: u128 = 94_002;
const MORGAN: u128 = 94_003;
const STUDENT: u128 = 94_004;

struct Fixture {
    app: axum::Router,
    elena_cookie: String,
    morgan_cookie: String,
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
    let morgan_session = morgan.1;
    store
        .approve_instructor_account(
            TenantContext::from_authenticated_session(tenant),
            ApproveInstructorAccount {
                session: morgan_session,
                target: elena,
                expected_revision: None,
            },
        )
        .await
        .expect("Morgan approves Elena for the live curation fixture");
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
        app: server_core::problem_curation::router(store),
        elena_cookie: elena.0,
        morgan_cookie: morgan.0,
        student_cookie: student.0,
    }
}

async fn create_account(store: &MemoryStore, user: UserId) {
    let token = EmailChallengeSecretHash::compute(b"curation-http-account-token");
    let binding = BrowserBindingHash::compute(b"curation-http-account-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(Uuid::from_u128(94_010)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"curation-http-rate"),
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
        SessionSubject::new(tenant, user, display_name, roles).expect("fixture session subject"),
        server_core::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
            server_core::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture session");
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

fn request(
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: impl Into<Body>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    builder.body(body.into()).expect("request")
}

fn revisioned_request(
    method: Method,
    uri: &str,
    cookie: &str,
    revision: &str,
    body: impl Into<Body>,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(COOKIE, cookie)
        .header(IF_MATCH, revision)
        .body(body.into())
        .expect("revisioned request")
}

async fn dispatch(app: &axum::Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.expect("route response")
}

async fn json(response: Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 128 * 1024)
            .await
            .expect("bounded JSON response"),
    )
    .expect("JSON response")
}

fn assert_no_store(response: &Response) {
    assert_eq!(
        response
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
}

#[tokio::test]
async fn favorites_are_idempotent_and_use_a_strong_no_store_etag() {
    let fixture = fixture().await;
    let first = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections/favorites",
            Some(&fixture.elena_cookie),
            Body::empty(),
        ),
    )
    .await;
    let first_etag = first.headers().get(ETAG).cloned().expect("Favorites ETag");
    assert_no_store(&first);
    let first_body = json(first).await;
    let second = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections/favorites",
            Some(&fixture.elena_cookie),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(second.headers().get(ETAG), Some(&first_etag));
    assert_eq!(json(second).await["reference"], first_body["reference"]);
}

#[tokio::test]
async fn collection_revision_and_member_reads_share_one_current_etag() {
    let fixture = fixture().await;
    let created = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Exam candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    let created_body = json(created).await;
    let reference = created_body["reference"]
        .as_str()
        .expect("collection reference");
    let get_uri = format!("/api/problem-collections/{reference}");
    let current = dispatch(
        &fixture.app,
        request(
            Method::GET,
            &get_uri,
            Some(&fixture.elena_cookie),
            Body::empty(),
        ),
    )
    .await;
    let etag = current
        .headers()
        .get(ETAG)
        .cloned()
        .expect("collection ETag");
    assert_no_store(&current);
    assert_eq!(current.status(), StatusCode::OK);
    let members = dispatch(
        &fixture.app,
        request(
            Method::GET,
            &format!("{get_uri}/members"),
            Some(&fixture.elena_cookie),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(members.headers().get(ETAG), Some(&etag));
    assert!(
        !String::from_utf8_lossy(
            &to_bytes(members.into_body(), 128 * 1024)
                .await
                .expect("members body")
        )
        .contains("correct")
    );
    let missing = dispatch(
        &fixture.app,
        request(
            Method::PUT,
            &get_uri,
            Some(&fixture.elena_cookie),
            r#"{"title":"Exam candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
}

#[tokio::test]
async fn collection_replacement_rejects_stale_revision_and_accepts_the_current_revision() {
    let fixture = fixture().await;
    let created = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Revision candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    let current_etag = created
        .headers()
        .get(ETAG)
        .cloned()
        .expect("collection ETag");
    let reference = json(created).await["reference"]
        .as_str()
        .expect("collection reference")
        .to_owned();
    let uri = format!("/api/problem-collections/{reference}");
    let stale = dispatch(
        &fixture.app,
        revisioned_request(
            Method::PUT,
            &uri,
            &fixture.elena_cookie,
            "\"99\"",
            r#"{"title":"Revision candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
    assert_no_store(&stale);
    let updated = dispatch(
        &fixture.app,
        revisioned_request(
            Method::PUT,
            &uri,
            &fixture.elena_cookie,
            current_etag.to_str().expect("strong ETag"),
            r#"{"title":"Current candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_ne!(updated.headers().get(ETAG), Some(&current_etag));
}

#[tokio::test]
async fn private_and_institution_collections_apply_the_curation_role_matrix() {
    let fixture = fixture().await;
    let created = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Shared candidates","visibility":"institution","questionIds":[]}"#,
        ),
    )
    .await;
    let reference = json(created).await["reference"]
        .as_str()
        .expect("collection reference")
        .to_owned();
    let collection_uri = format!("/api/problem-collections/{reference}");
    let readable = dispatch(
        &fixture.app,
        request(
            Method::GET,
            &collection_uri,
            Some(&fixture.morgan_cookie),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(readable.status(), StatusCode::OK);
    let denied = dispatch(
        &fixture.app,
        revisioned_request(
            Method::PUT,
            &collection_uri,
            &fixture.morgan_cookie,
            "\"1\"",
            r#"{"title":"Changed","visibility":"institution","questionIds":[]}"#,
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let private = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Private candidates","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    let private_reference = json(private).await["reference"]
        .as_str()
        .expect("private collection reference")
        .to_owned();
    let concealed = dispatch(
        &fixture.app,
        request(
            Method::GET,
            &format!("/api/problem-collections/{private_reference}"),
            Some(&fixture.morgan_cookie),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(concealed.status(), StatusCode::NOT_FOUND);
    assert_no_store(&concealed);
}

#[tokio::test]
async fn saved_searches_normalize_and_require_one_current_revision() {
    let fixture = fixture().await;
    let create = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/saved-problem-searches",
            Some(&fixture.elena_cookie),
            r#"{"title":"Protein search","filter":{"text":"  kinase  ","bylines":[],"backends":[],"tags":[],"responseFamilies":[],"taxonomy":[],"capabilities":[],"licenses":[],"publicationScopes":[],"evidence":"any","usedInMyCourses":"any","authorship":"any"}}"#,
        ),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let etag = create.headers().get(ETAG).cloned().expect("search ETag");
    let value = json(create).await;
    assert_eq!(value["filter"]["text"], "kinase");
    let uri = format!(
        "/api/saved-problem-searches/{}",
        value["reference"].as_str().expect("search reference")
    );
    let missing = dispatch(
        &fixture.app,
        request(
            Method::PUT,
            &uri,
            Some(&fixture.elena_cookie),
            r#"{"title":"Protein search","filter":{"text":"kinase","bylines":[],"backends":[],"tags":[],"responseFamilies":[],"taxonomy":[],"capabilities":[],"licenses":[],"publicationScopes":[],"evidence":"any","usedInMyCourses":"any","authorship":"any"}}"#,
        ),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
    let update = Request::builder()
        .method(Method::PUT)
        .uri(&uri)
        .header(COOKIE, &fixture.elena_cookie)
        .header(IF_MATCH, etag)
        .body(Body::from(r#"{"title":"Kinase search","filter":{"text":"kinase","bylines":[],"backends":[],"tags":[],"responseFamilies":[],"taxonomy":[],"capabilities":[],"licenses":[],"publicationScopes":[],"evidence":"any","usedInMyCourses":"any","authorship":"any"}}"#))
        .expect("update request");
    let updated = dispatch(&fixture.app, update).await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_no_store(&updated);
    let current = dispatch(
        &fixture.app,
        request(
            Method::GET,
            &uri,
            Some(&fixture.elena_cookie),
            Body::empty(),
        ),
    )
    .await;
    let delete_etag = current
        .headers()
        .get(ETAG)
        .cloned()
        .expect("current search ETag");
    assert_eq!(current.status(), StatusCode::OK);
    let deleted = dispatch(
        &fixture.app,
        revisioned_request(
            Method::DELETE,
            &uri,
            &fixture.elena_cookie,
            delete_etag.to_str().expect("strong ETag"),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert_no_store(&deleted);
}

#[tokio::test]
async fn authentication_precedes_malformed_inputs_and_strict_body_errors_are_no_store() {
    let fixture = fixture().await;
    let unauthenticated = dispatch(
        &fixture.app,
        request(
            Method::PUT,
            "/api/problem-collections/PC-not-a-number",
            None,
            r#"{"surprise":true}"#,
        ),
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let strict = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Strict","visibility":"private","questionIds":[],"surprise":true}"#,
        ),
    )
    .await;
    assert_eq!(strict.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_no_store(&strict);
    let unauthenticated_saved_search = dispatch(
        &fixture.app,
        request(
            Method::PUT,
            "/api/saved-problem-searches/PS-not-a-number",
            None,
            r#"{"surprise":true}"#,
        ),
    )
    .await;
    assert_eq!(
        unauthenticated_saved_search.status(),
        StatusCode::UNAUTHORIZED
    );
    let oversized = format!(
        r#"{{"title":"Too large","visibility":"private","questionIds":[],"padding":"{}"}}"#,
        "x".repeat(65 * 1024)
    );
    let bounded = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            oversized,
        ),
    )
    .await;
    assert_eq!(bounded.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_no_store(&bounded);
}

#[tokio::test]
async fn role_preflight_precedes_protected_path_query_revision_and_body_parsing() {
    let fixture = fixture().await;

    for (method, uri, body) in [
        (
            Method::GET,
            "/api/problem-collections/PC-not-a-number",
            Body::empty(),
        ),
        (
            Method::GET,
            "/api/problem-collections?pageSize=not-a-number",
            Body::empty(),
        ),
        (Method::POST, "/api/problem-collections", Body::from("{")),
    ] {
        let denied = dispatch(
            &fixture.app,
            request(method, uri, Some(&fixture.student_cookie), body),
        )
        .await;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert_no_store(&denied);
    }

    let created = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            r#"{"title":"Preflight target","visibility":"private","questionIds":[]}"#,
        ),
    )
    .await;
    let reference = json(created).await["reference"]
        .as_str()
        .expect("collection reference")
        .to_owned();
    let malformed_revision = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/problem-collections/{reference}"))
        .header(COOKIE, &fixture.morgan_cookie)
        .header(IF_MATCH, "not-an-etag")
        .body(Body::from("{"))
        .expect("mutation-ineligible request");
    let denied = dispatch(&fixture.app, malformed_revision).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_no_store(&denied);

    let sysadmin_query = dispatch(
        &fixture.app,
        request(
            Method::GET,
            "/api/problem-collections?pageSize=not-a-number",
            Some(&fixture.morgan_cookie),
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(sysadmin_query.status(), StatusCode::BAD_REQUEST);

    let instructor_body = dispatch(
        &fixture.app,
        request(
            Method::POST,
            "/api/problem-collections",
            Some(&fixture.elena_cookie),
            Body::from("{"),
        ),
    )
    .await;
    assert_eq!(instructor_body.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
