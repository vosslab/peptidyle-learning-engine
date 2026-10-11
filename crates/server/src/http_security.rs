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
    // A ready immutable public Question Image redirect owns its explicit long-lived
    // cache policy. Every ordinary dynamic response remains no-store.
    if !headers.contains_key("cache-control") {
        headers.insert("cache-control", HeaderValue::from_static("no-store"));
    }
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
    // The reviewed author-content runtime pair is deliberately public to an
    // opaque-origin sandbox and owns its narrower `cross-origin` exception.
    if !headers.contains_key("cross-origin-resource-policy") {
        headers.insert(
            "cross-origin-resource-policy",
            HeaderValue::from_static("same-origin"),
        );
    }
    response
}
