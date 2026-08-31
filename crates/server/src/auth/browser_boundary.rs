//! First-party HTTPS boundary for cookie-authenticated requests.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::header::{COOKIE, HOST, ORIGIN};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use url::Url;

use super::no_store;

/// One validated first-party HTTPS browser origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionBrowserBoundary {
    pub(crate) origin: Arc<str>,
    pub(crate) authority: Arc<str>,
}

impl ProductionBrowserBoundary {
    /// Validates one root HTTPS origin for the cookie-authenticated application.
    pub fn new(value: Arc<str>) -> Result<Self, String> {
        let parsed = Url::parse(&value).map_err(|_| "browser origin must be a URL".to_string())?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err("browser origin must be a root HTTPS origin".to_string());
        }
        let origin = parsed.origin().ascii_serialization();
        let authority = parsed
            .host_str()
            .map(|host| match parsed.port() {
                Some(port) => format!("{host}:{port}"),
                None => host.to_string(),
            })
            .expect("validated host");
        Ok(Self {
            origin: Arc::from(origin),
            authority: Arc::from(authority),
        })
    }
}

/// Accepts exactly one Origin header with the configured first-party value.
/// ASVS 3.5.1: state-changing cookie requests originate at the application.
pub(crate) fn origin_matches(headers: &HeaderMap, expected: &str) -> bool {
    one_header_value(headers, ORIGIN).is_some_and(|value| value == expected)
}

/// Drops unprefixed sensitive-cookie aliases and rejects duplicate host cookies.
/// ASVS 3.3.1, 3.3.3, 3.3.4: only the host-only HttpOnly credential reaches
/// the session resolver.
pub(crate) fn normalize_production_cookies(headers: &mut HeaderMap) -> bool {
    let Some(raw) = one_header_value(headers, COOKIE) else {
        return true;
    };
    let mut normalized = Vec::new();
    let mut session = None;
    for cookie in Cookie::split_parse(raw).filter_map(Result::ok) {
        match cookie.name() {
            "__Host-ple_session" => {
                if session.replace(cookie.value().to_string()).is_some() {
                    return false;
                }
            }
            "ple_session" | "__Secure-ple_session" | "__Host-ple_account_session" => {}
            _ => normalized.push(format!("{}={}", cookie.name(), cookie.value())),
        }
    }
    if let Some(value) = session {
        normalized.insert(0, format!("ple_session={value}"));
    }
    let value = normalized.join("; ");
    if value.is_empty() {
        headers.remove(COOKIE);
    } else if let Ok(value) = HeaderValue::from_str(&value) {
        headers.insert(COOKIE, value);
    } else {
        return false;
    }
    true
}

/// Enforces the first-party browser boundary before a cookie-authenticated route.
pub(crate) async fn production_cookie_boundary(
    axum::extract::State(boundary): axum::extract::State<ProductionBrowserBoundary>,
    mut request: Request,
    next: Next,
) -> Response {
    if !normalize_production_cookies(request.headers_mut()) {
        return no_store((StatusCode::BAD_REQUEST, "invalid cookie header").into_response());
    }
    let is_write = !request.method().is_safe();
    if is_write && !origin_matches(request.headers(), &boundary.origin) {
        return no_store((StatusCode::FORBIDDEN, "first-party origin required").into_response());
    }
    if !one_header_value(request.headers(), HOST)
        .is_some_and(|host| host == boundary.authority.as_ref())
    {
        return no_store(
            (StatusCode::MISDIRECTED_REQUEST, "canonical host required").into_response(),
        );
    }
    next.run(request).await
}

fn one_header_value(headers: &HeaderMap, name: axum::http::header::HeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}
