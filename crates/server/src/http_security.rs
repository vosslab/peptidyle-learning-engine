//! Uniform security headers for the dynamic API surface.

use axum::Router;
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::{self, Next};
use axum::response::Response;

/// Applies the API policy after all route groups have been merged so a newly
/// added route cannot accidentally sit outside the response boundary.
pub(crate) fn apply_api_security_headers(router: Router) -> Router {
    router.layer(middleware::from_fn(api_security_headers))
}

/// Makes dynamic responses non-cacheable and constrains browser interpretation.
pub(crate) async fn api_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static(
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
        ),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    // Some HTML routes have a deliberately narrower, nonce-bearing CSP. A
    // second CSP would intersect with it (and can therefore disable its nonce),
    // while overwriting it would discard its route-specific script policy.
    // Route handlers own those policies and must include the shared base,
    // object, and framing restrictions themselves.
    if !headers.contains_key("content-security-policy") {
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static(
        "default-src 'none'; base-uri 'none'; object-src 'none'; form-action 'none'; frame-ancestors 'self'",
            ),
        );
    }
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    response
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use axum::middleware;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn every_dynamic_status_receives_non_cacheable_security_headers() {
        let app = Router::new()
            .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
            .layer(middleware::from_fn(api_security_headers));
        let response = app
            .oneshot(Request::get("/missing").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");
        assert_eq!(
            response.headers()["content-security-policy"],
            "default-src 'none'; base-uri 'none'; object-src 'none'; form-action 'none'; frame-ancestors 'self'"
        );
        assert_eq!(
            response.headers()["cross-origin-resource-policy"],
            "same-origin"
        );
    }

    #[tokio::test]
    async fn route_owned_nonce_csp_survives_unchanged() {
        const CSP: &str = "default-src 'none'; script-src 'nonce-test-nonce'; base-uri 'none'; object-src 'none'; frame-ancestors 'self'";
        let app = Router::new()
            .route(
                "/external-shell",
                get(|| async move {
                    let mut response = StatusCode::OK.into_response();
                    response
                        .headers_mut()
                        .insert("content-security-policy", HeaderValue::from_static(CSP));
                    response
                }),
            )
            .layer(middleware::from_fn(api_security_headers));
        let response = app
            .oneshot(Request::get("/external-shell").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.headers()["content-security-policy"], CSP);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");
        assert_eq!(
            response.headers()["cross-origin-resource-policy"],
            "same-origin"
        );
    }
}
