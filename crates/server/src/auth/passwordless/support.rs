//! Shared credential, cookie, and uniform-response helpers for passwordless routes.

use axum::Json;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cookie::{Cookie, SameSite};
use learning_data_access::{
    AccountIdentityStore, AccountRecord, AccountSessionStore, AccountSessionTokenHash,
};
use question_model::CourseMembershipRole;

use super::super::{AuthError, SessionConfig, no_store};
use super::{
    ACCOUNT_SESSION_COOKIE, AcceptedEmailResponse, EMAIL_BINDING_COOKIE, EMAIL_CHALLENGE_SECONDS,
    SECRET_BYTES,
};

pub(in crate::auth) async fn authenticated_account<S>(
    store: &S,
    headers: &HeaderMap,
) -> Result<AccountRecord, Response>
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let token =
        cookie_secret(headers, ACCOUNT_SESSION_COOKIE).ok_or_else(authentication_rejected)?;
    let session = store
        .resolve_account_session(AccountSessionTokenHash::compute(&token.0))
        .await
        .map_err(|_| passwordless_unavailable())?
        .ok_or_else(authentication_rejected)?;
    store
        .get_account(session.user)
        .await
        .map_err(|_| passwordless_unavailable())?
        .ok_or_else(authentication_rejected)
}

/// Revokes the short-lived account credential presented by this browser.
///
/// Missing or malformed cookies are idempotent so sign-out does not become a
/// credential-validity oracle.
pub(in crate::auth) async fn revoke_presented_account_session<S>(
    store: &S,
    headers: &HeaderMap,
) -> Result<(), AuthError>
where
    S: AccountSessionStore,
{
    if let Some(token) = cookie_secret(headers, ACCOUNT_SESSION_COOKIE) {
        store
            .revoke_account_session(AccountSessionTokenHash::compute(&token.0))
            .await
            .map_err(|error| AuthError::Unavailable(error.to_string()))?;
    }
    Ok(())
}

pub(super) fn course_role_name(role: CourseMembershipRole) -> &'static str {
    match role {
        CourseMembershipRole::Student => "student",
        CourseMembershipRole::Instructor => "instructor",
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::auth) struct RandomSecret(pub(in crate::auth) [u8; SECRET_BYTES]);

impl RandomSecret {
    pub(in crate::auth) fn generate() -> Result<Self, AuthError> {
        let mut value = [0_u8; SECRET_BYTES];
        getrandom::fill(&mut value).map_err(AuthError::Randomness)?;
        Ok(Self(value))
    }

    pub(in crate::auth) fn decode(value: &str) -> Option<Self> {
        if value.len() != 43 {
            return None;
        }
        let decoded: [u8; SECRET_BYTES] = URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()?;
        (URL_SAFE_NO_PAD.encode(decoded) == value).then_some(Self(decoded))
    }

    pub(super) fn encoded(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl std::fmt::Debug for RandomSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RandomSecret([redacted])")
    }
}

pub(in crate::auth) fn cookie_secret(headers: &HeaderMap, name: &str) -> Option<RandomSecret> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    let joined = values.join("; ");
    let mut matches = Cookie::split_parse(&joined)
        .filter_map(Result::ok)
        .filter(|cookie| cookie.name() == name)
        .filter_map(|cookie| RandomSecret::decode(cookie.value()));
    let value = matches.next()?;
    matches.next().is_none().then_some(value)
}

pub(in crate::auth) fn secret_cookie(
    name: &'static str,
    secret: &RandomSecret,
    seconds: u32,
    config: SessionConfig,
) -> String {
    Cookie::build((
        super::super::session_cookie::wire_cookie_name(name, config),
        secret.encoded(),
    ))
    .path("/")
    .http_only(true)
    .secure(config.secure())
    .same_site(SameSite::Lax)
    .max_age(cookie::time::Duration::seconds(i64::from(seconds)))
    .build()
    .to_string()
}

pub(in crate::auth) fn clear_named_cookie(name: &'static str, config: SessionConfig) -> String {
    Cookie::build((
        super::super::session_cookie::wire_cookie_name(name, config),
        "",
    ))
    .path("/")
    .http_only(true)
    .secure(config.secure())
    .same_site(SameSite::Lax)
    .max_age(cookie::time::Duration::ZERO)
    .build()
    .to_string()
}

pub(in crate::auth) fn clear_account_authentication_cookies(config: SessionConfig) -> [String; 2] {
    [
        clear_named_cookie(ACCOUNT_SESSION_COOKIE, config),
        clear_named_cookie(EMAIL_BINDING_COOKIE, config),
    ]
}

pub(super) fn accepted_email_response(
    binding: Option<RandomSecret>,
    config: SessionConfig,
) -> Response {
    let mut response = (
        StatusCode::ACCEPTED,
        Json(AcceptedEmailResponse { accepted: true }),
    )
        .into_response();
    if let Some(binding) = binding {
        let cookie = secret_cookie(
            EMAIL_BINDING_COOKIE,
            &binding,
            EMAIL_CHALLENGE_SECONDS,
            config,
        );
        let Ok(cookie) = HeaderValue::from_str(&cookie) else {
            return passwordless_unavailable();
        };
        response.headers_mut().insert(SET_COOKIE, cookie);
    }
    no_store(response)
}

/// An authenticated account may be told when its own sensitive action can be
/// retried. Anonymous sign-in deliberately remains a uniform accepted reply.
pub(super) fn account_email_change_rate_limited(retry_after_seconds: u32) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({ "error": "try this email change again later" })),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
        response.headers_mut().insert("retry-after", value);
    }
    no_store(response)
}

pub(in crate::auth) fn authentication_rejected() -> Response {
    no_store(
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "authentication required" })),
        )
            .into_response(),
    )
}

pub(super) fn invitation_rejected() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "course invitation not found" })),
        )
            .into_response(),
    )
}

pub(super) fn invitation_conflict() -> Response {
    no_store(
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "course invitation cannot be claimed" })),
        )
            .into_response(),
    )
}

pub(super) fn invalid_account_course_request() -> Response {
    no_store(
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "account course request is invalid" })),
        )
            .into_response(),
    )
}

pub(super) fn invalid_account_email_request() -> Response {
    no_store(
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "account email request is invalid" })),
        )
            .into_response(),
    )
}

pub(super) fn account_course_not_found() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "course not found" })),
        )
            .into_response(),
    )
}

pub(in crate::auth) fn passwordless_unavailable() -> Response {
    no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "passwordless authentication unavailable" })),
        )
            .into_response(),
    )
}
