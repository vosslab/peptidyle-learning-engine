use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::middleware;
use axum::routing::post;
use learning_data_access::in_memory::MemoryStore;
use question_model::{TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

fn subject() -> SessionSubject {
    SessionSubject::new(
        TenantId::from_uuid(Uuid::from_u128(1)),
        UserId::from_uuid(Uuid::from_u128(2)),
        "Fixture Student",
        vec![UserRole::Student],
    )
    .expect("fixture subject")
}

fn config(transport: CookieTransport) -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
        transport,
    )
}

fn cookie_request_header(set_cookie: &str) -> &str {
    set_cookie
        .split(';')
        .next()
        .expect("Set-Cookie should begin with a cookie pair")
}

#[tokio::test]
async fn revocation_on_one_replica_takes_effect_on_another() {
    let issuer = MemoryStore::default();
    let next_replica = issuer.clone();
    let issued = issue_session(&issuer, subject(), config(CookieTransport::FirstPartyHttps))
        .await
        .expect("session should issue");
    let header = cookie_request_header(&issued.set_cookie);

    revoke_session(&next_replica, Some(header))
        .await
        .expect("session should revoke");
    assert!(matches!(
        resolve_session(&issuer, Some(header)).await,
        Err(AuthError::Unauthenticated)
    ));
}

#[tokio::test]
async fn issued_session_debug_redacts_the_set_cookie_credential() {
    let issued = issue_session(
        &MemoryStore::default(),
        subject(),
        config(CookieTransport::FirstPartyHttps),
    )
    .await
    .expect("session should issue");
    let debug = format!("{issued:?}");
    assert!(!debug.contains(&issued.set_cookie));
    assert!(debug.contains("[redacted]"));
}

#[test]
fn cookie_attributes_match_first_party_https() {
    let token = SessionToken([7; SESSION_TOKEN_BYTES]);
    let first_party = session_cookie(&token, config(CookieTransport::FirstPartyHttps));
    assert_eq!(first_party.http_only(), Some(true));
    assert_eq!(first_party.secure(), Some(true));
    assert_eq!(first_party.same_site(), Some(SameSite::Lax));
    assert_eq!(first_party.path(), Some("/"));
    assert_eq!(first_party.domain(), None);
    assert_eq!(first_party.max_age(), None);
    assert_eq!(first_party.expires(), None);
    assert!(!first_party.value().contains('='));
    assert_eq!(first_party.name(), "__Host-ple_session");
}

#[test]
fn production_cookie_normalization_ignores_unprefixed_sensitive_injection() {
    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_static(
            "ple_session=attacker; __Host-ple_session=trusted; theme=contrast",
        ),
    );
    assert!(normalize_production_cookies(&mut headers));
    assert_eq!(
        headers.get(COOKIE).and_then(|value| value.to_str().ok()),
        Some("ple_session=trusted; theme=contrast")
    );
}

#[test]
fn production_origin_must_be_one_exact_value() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "origin",
        HeaderValue::from_static("https://learn.example.edu"),
    );
    assert!(origin_matches(&headers, "https://learn.example.edu"));
    assert!(!origin_matches(&headers, "https://other.example.edu"));
    headers.append(
        "origin",
        HeaderValue::from_static("https://learn.example.edu"),
    );
    assert!(!origin_matches(&headers, "https://learn.example.edu"));
}

#[test]
fn production_browser_boundary_normalizes_a_serialized_origin() {
    let boundary = ProductionBrowserBoundary::new(Arc::from("https://learn.example.edu/"))
        .expect("a root HTTPS URL is an origin");
    assert_eq!(boundary.origin.as_ref(), "https://learn.example.edu");
    assert_eq!(boundary.authority.as_ref(), "learn.example.edu");
    assert!(ProductionBrowserBoundary::new(Arc::from("https://user@learn.example.edu/")).is_err());
}

fn production_boundary_test_router() -> Router {
    Router::new()
        .route(
            "/write",
            post(|headers: HeaderMap| async move {
                headers
                    .get(COOKIE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("no-cookie")
                    .to_string()
            }),
        )
        .layer(middleware::from_fn_with_state(
            ProductionBrowserBoundary::new(Arc::from("https://learn.example.edu"))
                .expect("fixture origin"),
            production_cookie_boundary,
        ))
}

#[tokio::test]
async fn production_boundary_requires_exact_host_and_origin_for_cookie_mutations() {
    let app = production_boundary_test_router();
    let valid = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "learn.example.edu")
                .header("origin", "https://learn.example.edu")
                .header(COOKIE, "__Host-ple_session=trusted")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("valid response");
    assert_eq!(valid.status(), StatusCode::OK);
    assert!(
        !valid.headers().contains_key("strict-transport-security"),
        "the API boundary must not emit HSTS; the HTTPS edge owns it"
    );
    assert_eq!(
        to_bytes(valid.into_body(), 1024).await.expect("body"),
        "ple_session=trusted"
    );

    let cross_origin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "learn.example.edu")
                .header("origin", "https://attacker.example")
                .header(COOKIE, "__Host-ple_session=trusted")
                .body(Body::empty())
                .expect("cross-origin request"),
        )
        .await
        .expect("cross-origin response");
    assert_eq!(cross_origin.status(), StatusCode::FORBIDDEN);
    assert_eq!(cross_origin.headers()[CACHE_CONTROL], "no-store");

    let wrong_host = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "attacker.example")
                .header("origin", "https://learn.example.edu")
                .header(COOKIE, "__Host-ple_session=trusted")
                .body(Body::empty())
                .expect("wrong-host request"),
        )
        .await
        .expect("wrong-host response");
    assert_eq!(wrong_host.status(), StatusCode::MISDIRECTED_REQUEST);
}

#[tokio::test]
async fn production_boundary_drops_legacy_sensitive_cookie_aliases() {
    let response = production_boundary_test_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "learn.example.edu")
                .header(COOKIE, "ple_session=attacker; theme=contrast")
                .body(Body::empty())
                .expect("legacy-cookie request"),
        )
        .await
        .expect("legacy-cookie response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.expect("body"),
        "theme=contrast"
    );
}

#[tokio::test]
async fn production_boundary_normalizes_each_host_cookie_once_and_rejects_duplicates() {
    let app = production_boundary_test_router();
    let normalized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "learn.example.edu")
                .header("origin", "https://learn.example.edu")
                .header(
                    COOKIE,
                    "__Host-ple_session=tenant; __Host-ple_account_session=account; \
                     __Host-ple_email_binding=email; __Host-ple_webauthn_binding=webauthn",
                )
                .body(Body::empty())
                .expect("host-only cookie request"),
        )
        .await
        .expect("normalized response");
    assert_eq!(normalized.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(normalized.into_body(), 1024)
            .await
            .expect("normalized body"),
        "ple_session=tenant; ple_account_session=account; ple_email_binding=email; ple_webauthn_binding=webauthn"
    );

    let duplicate = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/write")
                .header("host", "learn.example.edu")
                .header("origin", "https://learn.example.edu")
                .header(
                    COOKIE,
                    "__Host-ple_account_session=first; __Host-ple_account_session=second",
                )
                .body(Body::empty())
                .expect("duplicate host-only cookie request"),
        )
        .await
        .expect("duplicate response");
    assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);
    assert_eq!(duplicate.headers()[CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn malformed_unknown_and_duplicate_cookies_share_one_failure() {
    let store = MemoryStore::default();
    for header in [
        None,
        Some("ple_session=not-base64"),
        Some("ple_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        Some(
            "ple_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA; ple_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
    ] {
        assert!(matches!(
            resolve_session(&store, header).await,
            Err(AuthError::Unauthenticated)
        ));
    }
}
