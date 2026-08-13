//! Production browser host, cookie, and same-origin enforcement.
//!
//! This module is deliberately separate from credential issuance so browser
//! presentation checks remain a small, auditable trust boundary.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::header::COOKIE;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cookie::Cookie;

use super::{SESSION_COOKIE_NAME, no_store, passwordless, webauthn};

/// Exact public browser identity trusted by the production cookie boundary.
#[derive(Debug, Clone)]
pub(crate) struct ProductionBrowserBoundary {
    pub(super) origin: Arc<str>,
    pub(super) authority: Arc<str>,
}

impl ProductionBrowserBoundary {
    pub(crate) fn new(origin: Arc<str>) -> Result<Self, String> {
        let parsed = url::Url::parse(&origin)
            .map_err(|_| "production browser origin is invalid".to_string())?;
        if parsed.scheme() != "https"
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("production browser origin must be an HTTPS origin without a path".into());
        }
        // Browser Origin values are serialized origins, not arbitrary URL
        // spellings. Normalize the deployment value once so a harmless
        // trailing slash or explicit default port cannot disable CSRF checks.
        let origin = parsed.origin().ascii_serialization();
        let Some(authority) = origin.strip_prefix("https://") else {
            return Err("production browser origin has no authority".into());
        };
        let authority = Arc::<str>::from(authority);
        Ok(Self {
            origin: Arc::from(origin),
            authority,
        })
    }
}

fn internal_cookie_name(name: &str) -> Option<&'static str> {
    match name {
        "__Host-ple_session" => Some(SESSION_COOKIE_NAME),
        "__Host-ple_account_session" => Some(passwordless::ACCOUNT_SESSION_COOKIE),
        "__Host-ple_email_binding" => Some(passwordless::EMAIL_BINDING_COOKIE),
        "__Host-ple_webauthn_binding" => Some(webauthn::WEBAUTHN_BINDING_COOKIE),
        _ => None,
    }
}

fn is_internal_sensitive_cookie(name: &str) -> bool {
    matches!(
        name,
        SESSION_COOKIE_NAME
            | passwordless::ACCOUNT_SESSION_COOKIE
            | passwordless::EMAIL_BINDING_COOKIE
            | webauthn::WEBAUTHN_BINDING_COOKIE
    )
}

/// Enforces production host-only cookie names and exact same-origin mutation.
///
/// The browser-visible `__Host-` names are normalized to internal names only
/// after the prefix and CSRF checks. Unprefixed sensitive cookies are ignored,
/// so a sibling subdomain cannot inject authority or force a logout loop.
pub(crate) async fn production_cookie_boundary(
    State(boundary): State<ProductionBrowserBoundary>,
    mut request: Request,
    next: Next,
) -> Response {
    if request.uri().path() != "/health" && !host_matches(request.headers(), &boundary.authority) {
        return no_store(
            (
                StatusCode::MISDIRECTED_REQUEST,
                Json(serde_json::json!({ "error": "request host is not served" })),
            )
                .into_response(),
        );
    }
    let has_sensitive_cookie = request
        .headers()
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .any(|cookie| internal_cookie_name(cookie.name()).is_some());
    if has_sensitive_cookie
        && is_mutating(request.method())
        && !origin_matches(request.headers(), &boundary.origin)
        && !is_sandboxed_external_activity_post(&request)
    {
        return no_store(
            (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "same-origin request required" })),
            )
                .into_response(),
        );
    }
    normalize_production_cookies(request.headers_mut());
    // HSTS is deliberately owned by the HTTPS browser edge.  This middleware
    // cannot see static documents, CDN-generated errors, redirects, or prove
    // that the request reached the browser over HTTPS; emitting it here would
    // make the application an incomplete and potentially misleading owner.
    next.run(request).await
}

/// A sandboxed provider document has an opaque origin by design.  Its form
/// submission is therefore the sole unsafe request that can legitimately
/// carry `Origin: null`.  Do not generalize this exception: the destination
/// is an exact route shape, it must have one host-only application session and
/// one path-scoped launch capability, and the activity handler subsequently
/// authenticates the encrypted capability against that exact session, tenant,
/// actor, and attempt before it contacts the provider.
///
/// This pre-routing check deliberately validates only the browser presentation
/// shape.  `LaunchStateAead` and the tenant store belong to the external-tool
/// route owner, which performs the cryptographic and durable authorization
/// check after this boundary has admitted the request.
fn is_sandboxed_external_activity_post(request: &Request) -> bool {
    request.method() == Method::POST
        && is_external_activity_path(request.uri().path())
        && origin_is_sandbox_null(request.headers())
        && has_one_unambiguous_sandbox_activity_capability(request.headers())
}

fn is_external_activity_path(path: &str) -> bool {
    let mut segments = path.split('/');
    matches!(
        (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ),
        (
            Some(""),
            Some("api"),
            Some("attempts"),
            Some(attempt),
            Some("external-tool"),
            Some("launch"),
            Some("activity"),
            None,
        ) if uuid::Uuid::parse_str(attempt).is_ok()
    )
}

fn origin_is_sandbox_null(headers: &HeaderMap) -> bool {
    let mut origins = headers.get_all("origin").iter();
    origins
        .next()
        .filter(|_| origins.next().is_none())
        .and_then(|value| value.to_str().ok())
        == Some("null")
}

fn has_one_unambiguous_sandbox_activity_capability(headers: &HeaderMap) -> bool {
    let mut session_count = 0;
    let mut launch_count = 0;

    for header in headers.get_all(COOKIE).iter() {
        let Ok(header) = header.to_str() else {
            return false;
        };
        for parsed in Cookie::split_parse(header) {
            let Ok(cookie) = parsed else {
                return false;
            };
            match cookie.name() {
                "__Host-ple_session" => session_count += 1,
                // This cookie is intentionally host-only (no Domain attribute),
                // Secure, HttpOnly, Strict, and scoped to this launch path.
                // A duplicate is unsafe because cookie ordering is not an
                // authorization decision.
                crate::run::EXTERNAL_LAUNCH_COOKIE => launch_count += 1,
                // Do not allow legacy aliases or a second session class to
                // influence the exception.  Normal requests continue to
                // discard legacy cookies harmlessly during normalization.
                name if is_internal_sensitive_cookie(name)
                    || internal_cookie_name(name).is_some() =>
                {
                    return false;
                }
                _ => {}
            }
        }
    }
    session_count == 1 && launch_count == 1
}

fn is_mutating(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

pub(super) fn origin_matches(headers: &HeaderMap, expected: &str) -> bool {
    let mut origins = headers.get_all("origin").iter();
    origins
        .next()
        .filter(|_| origins.next().is_none())
        .and_then(|value| value.to_str().ok())
        == Some(expected)
}

fn host_matches(headers: &HeaderMap, expected: &str) -> bool {
    let mut hosts = headers.get_all("host").iter();
    hosts
        .next()
        .filter(|_| hosts.next().is_none())
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

pub(super) fn normalize_production_cookies(headers: &mut HeaderMap) {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .filter_map(|cookie| {
            let name = internal_cookie_name(cookie.name()).or_else(|| {
                (!is_internal_sensitive_cookie(cookie.name())).then_some(cookie.name())
            })?;
            Some(Cookie::new(name.to_string(), cookie.value().to_string()).to_string())
        })
        .collect::<Vec<_>>();
    headers.remove(COOKIE);
    if !cookies.is_empty()
        && let Ok(value) = HeaderValue::from_str(&cookies.join("; "))
    {
        headers.insert(COOKIE, value);
    }
}
