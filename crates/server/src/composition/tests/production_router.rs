//! Production router composition and browser-boundary behavior.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use super::*;

fn injected_production_router() -> Router {
    let router = composed_memory_router_and_store_with_session_config(
        super::super::backend::production_session_config(),
    )
    .0;
    super::super::backend::complete_production_router(
        router,
        crate::auth::ProductionBrowserBoundary::new(Arc::from("https://learn.example.test"))
            .expect("production browser boundary"),
    )
}

#[test]
fn production_session_config_uses_first_party_https() {
    let config = super::super::backend::production_session_config();
    assert_eq!(config.transport(), CookieTransport::FirstPartyHttps);
    assert_eq!(config.lifetime().as_seconds(), 8 * 60 * 60);
}

#[tokio::test]
async fn production_composition_has_passwordless_routes_without_provider_login() {
    let app = composed_memory_router();
    for (method, uri) in [
        ("POST", "/api/auth/passwordless/email/start"),
        ("POST", "/api/course-invitations/redeem"),
        ("POST", "/api/auth/account/course-session"),
        ("POST", "/api/auth/passkeys/authentication/start"),
        ("GET", "/api/auth/session"),
        ("POST", "/api/auth/logout"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::from("{}"))
                    .expect("production-style route request"),
            )
            .await
            .expect("production-style route response");
        assert_ne!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
    let provider_login = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .body(Body::from("{}"))
                .expect("provider login request"),
        )
        .await
        .expect("provider login response");
    assert_eq!(provider_login.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn injected_production_router_enforces_browser_cookie_contract_without_provider_login() {
    let app = injected_production_router();
    let valid_logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("host", "learn.example.test")
                .header("origin", "https://learn.example.test")
                .header("cookie", "__Host-ple_session=trusted")
                .body(Body::empty())
                .expect("same-origin logout request"),
        )
        .await
        .expect("same-origin logout response");
    assert_eq!(valid_logout.status(), StatusCode::OK);

    let cookies = valid_logout
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().expect("ASCII Set-Cookie value"))
        .collect::<Vec<_>>();
    for name in [
        "__Host-ple_session=",
        "__Host-ple_account_session=",
        "__Host-ple_email_binding=",
        "__Host-ple_webauthn_binding=",
    ] {
        let cookie = cookies
            .iter()
            .find(|value| value.starts_with(name))
            .unwrap_or_else(|| panic!("missing {name} deletion cookie"));
        assert!(cookie.contains("Path=/"), "{cookie}");
        assert!(cookie.contains("HttpOnly"), "{cookie}");
        assert!(cookie.contains("Secure"), "{cookie}");
        assert!(cookie.contains("SameSite=Lax"), "{cookie}");
    }

    for (host, origin, expected_status) in [
        (
            "learn.example.test",
            "https://attacker.example.test",
            StatusCode::FORBIDDEN,
        ),
        (
            "attacker.example.test",
            "https://learn.example.test",
            StatusCode::MISDIRECTED_REQUEST,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("host", host)
                    .header("origin", origin)
                    .header("cookie", "__Host-ple_session=trusted")
                    .body(Body::empty())
                    .expect("invalid production logout request"),
            )
            .await
            .expect("invalid production logout response");
        assert_eq!(response.status(), expected_status, "{host} {origin}");
    }

    let provider_login = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("host", "learn.example.test")
                .body(Body::from("{}"))
                .expect("production provider-login request"),
        )
        .await
        .expect("production provider-login response");
    assert_eq!(provider_login.status(), StatusCode::NOT_FOUND);
}
