//! Provider-neutral authentication and replica-safe sessions (MOD-API-AUTH).
//!
//! A credential provider establishes [`SessionSubject`]. This module then
//! mints a 256-bit opaque cookie credential, persists only its SHA-256 hash,
//! and resolves the tenant from the database row. Request parameters, headers,
//! and bodies never construct [`TenantContext`].

use std::sync::Arc;

use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use cookie::{Cookie, SameSite};
use learning_data_access::{
    AccountSessionStore, SessionLifetime, SessionRecord, SessionStore, SessionSubject, StoreError,
    TenantContext,
};
use question_model::{TenantId, UserId, UserRole};
use serde::Serialize;

#[path = "auth/browser_boundary.rs"]
mod browser_boundary;
#[path = "auth/client_address.rs"]
mod client_address;
#[path = "auth/passwordless.rs"]
mod passwordless;
#[path = "auth/seeded_account_selector.rs"]
mod seeded_account_selector;
#[path = "auth/seeded_sysadmin_ownership.rs"]
mod seeded_sysadmin_ownership;
#[path = "auth/session_cookie.rs"]
mod session_cookie;
#[path = "auth/webauthn.rs"]
mod webauthn;

pub(crate) use browser_boundary::{ProductionBrowserBoundary, production_cookie_boundary};
#[cfg(test)]
use browser_boundary::{normalize_production_cookies, origin_matches};
pub use client_address::ClientAddressPolicy;
pub use passwordless::{
    PasswordlessEmailAction, PasswordlessEmailDelivery, PasswordlessEmailDeliveryError,
    PasswordlessEmailSecret, PasswordlessRateLimitIssuer, UnavailablePasswordlessEmailDelivery,
    passwordless_router,
};
pub use seeded_account_selector::{SeededAccountSelectorConfig, seeded_account_selector_router};
pub use seeded_sysadmin_ownership::{
    SeededSysadminOwnershipConfig, seeded_sysadmin_ownership_router,
};
use session_cookie::{SessionToken, presented_token, session_cookie, wire_cookie_name};
pub use webauthn::{PasswordlessWebauthn, passkey_router};

const SESSION_COOKIE_NAME: &str = "ple_session";
const SESSION_TOKEN_BYTES: usize = 32;
const TOKEN_GENERATION_ATTEMPTS: usize = 3;
/// HTTP setting selected for the application's deployment context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieTransport {
    /// Normal HTTPS navigation, protected against cross-site requests.
    FirstPartyHttps,
}

/// Validated cookie and database-session policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    lifetime: SessionLifetime,
    transport: CookieTransport,
}

impl SessionConfig {
    /// Creates a session policy with a positive lifetime.
    pub fn new(lifetime: SessionLifetime, transport: CookieTransport) -> Self {
        Self {
            lifetime,
            transport,
        }
    }

    /// Database-authoritative lifetime.
    pub fn lifetime(self) -> SessionLifetime {
        self.lifetime
    }

    /// Selected deployment transport.
    pub fn transport(self) -> CookieTransport {
        self.transport
    }

    fn secure(self) -> bool {
        true
    }

    fn same_site(self) -> SameSite {
        SameSite::Lax
    }
}

/// Issued database session and the header value sent to the browser.
#[derive(Clone, PartialEq, Eq)]
pub struct IssuedSession {
    /// Persisted session metadata, containing only the token hash.
    pub record: SessionRecord,
    /// Complete `Set-Cookie` value; the only returned value containing the token.
    pub set_cookie: String,
}

impl std::fmt::Debug for IssuedSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedSession")
            .field("record", &self.record)
            .field("set_cookie", &"[redacted]")
            .finish()
    }
}

/// Authenticated principal and its derived tenant boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    /// Active session metadata resolved from shared storage.
    pub record: SessionRecord,
    /// RLS context derived only from the resolved record.
    pub tenant_context: TenantContext,
    /// Request-derived private session capability carried by trusted server
    /// operations. It never enters a browser DTO.
    pub(crate) session_hash: learning_data_access::SessionTokenHash,
}

/// Browser-safe signed-in identity response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSessionResponse {
    /// Literal true for this authenticated response shape.
    pub authenticated: bool,
    /// Tenant established by the session row.
    pub tenant: TenantId,
    /// Browser-safe identity with coarse roles and no credential.
    pub user: AuthUserResponse,
}

/// Browser-safe user projection nested in [`AuthSessionResponse`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserResponse {
    /// Authenticated person, not an assignment enrollment identifier.
    pub id: UserId,
    /// Provider-established display label.
    pub display_name: String,
    /// Coarse route-authorization roles.
    pub roles: Vec<UserRole>,
}

impl AuthenticatedSession {
    /// Builds the browser-safe current-session response.
    pub fn response(&self) -> AuthSessionResponse {
        session_response(&self.record)
    }
}

/// Browser-safe response returned after successful sign-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SignedOutResponse {
    /// Literal false after the browser credential is cleared.
    pub authenticated: bool,
}

/// Authentication failure mapped by the future HTTP route layer.
#[derive(Debug)]
pub enum AuthError {
    /// Missing, malformed, expired, revoked, or unknown cookie.
    Unauthenticated,
    /// A dependency needed for authentication was unavailable.
    Unavailable(String),
    /// The operating system could not supply cryptographic randomness.
    Randomness(getrandom::Error),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthenticated => formatter.write_str("authentication required"),
            Self::Unavailable(message) => {
                write!(formatter, "authentication unavailable: {message}")
            }
            Self::Randomness(error) => write!(formatter, "secure randomness unavailable: {error}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Builds the provider-neutral session resolution and complete sign-out routes.
pub fn session_router<S>(sessions: Arc<S>, config: SessionConfig) -> Router
where
    S: SessionStore + AccountSessionStore + 'static,
{
    let state = SessionRouteState { sessions, config };
    Router::new()
        .route("/api/auth/session", get(session_handler::<S>))
        .route("/api/auth/logout", post(logout_handler::<S>))
        .with_state(state)
}

/// Issues a session after a trusted provider has established the subject.
pub async fn issue_session(
    sessions: &dyn SessionStore,
    subject: SessionSubject,
    config: SessionConfig,
) -> Result<IssuedSession, AuthError> {
    for _ in 0..TOKEN_GENERATION_ATTEMPTS {
        let token = SessionToken::generate().map_err(AuthError::Randomness)?;
        let token_hash = token.hash();
        match sessions
            .create_session(token_hash, subject.clone(), config.lifetime())
            .await
        {
            Ok(record) => {
                return Ok(IssuedSession {
                    record,
                    set_cookie: session_cookie(&token, config).to_string(),
                });
            }
            Err(StoreError::AlreadyExists) => continue,
            Err(error) => return Err(AuthError::Unavailable(error.to_string())),
        }
    }
    Err(AuthError::Unavailable(
        "repeated session-token collision".to_string(),
    ))
}

/// Resolves a cookie against shared storage and derives tenant context.
pub async fn resolve_session(
    sessions: &dyn SessionStore,
    cookie_header: Option<&str>,
) -> Result<AuthenticatedSession, AuthError> {
    let token = presented_token(cookie_header).ok_or(AuthError::Unauthenticated)?;
    let record = sessions
        .resolve_session(token.hash())
        .await
        .map_err(|error| AuthError::Unavailable(error.to_string()))?
        .ok_or(AuthError::Unauthenticated)?;
    let tenant_context = TenantContext::from_authenticated_session(record.subject.tenant());
    Ok(AuthenticatedSession {
        record,
        tenant_context,
        session_hash: token.hash(),
    })
}

/// Resolves the authentication cookie carried by an HTTP request.
pub(crate) async fn resolve_request_session(
    sessions: &dyn SessionStore,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AuthError> {
    let cookie_header = joined_cookie_header(headers);
    resolve_session(sessions, cookie_header.as_deref()).await
}

/// Revokes the presented session. Missing or malformed cookies are idempotent.
pub async fn revoke_session(
    sessions: &dyn SessionStore,
    cookie_header: Option<&str>,
) -> Result<(), AuthError> {
    if let Some(token) = presented_token(cookie_header) {
        sessions
            .revoke_session(token.hash())
            .await
            .map_err(|error| AuthError::Unavailable(error.to_string()))?;
    }
    Ok(())
}

/// Builds the deletion cookie returned after sign-out.
pub fn clear_session_cookie(config: SessionConfig) -> String {
    Cookie::build((wire_cookie_name(SESSION_COOKIE_NAME, config), ""))
        .path("/")
        .http_only(true)
        .secure(config.secure())
        .same_site(config.same_site())
        .max_age(cookie::time::Duration::ZERO)
        .build()
        .to_string()
}

struct SessionRouteState<S> {
    sessions: Arc<S>,
    config: SessionConfig,
}

impl<S> Clone for SessionRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            config: self.config,
        }
    }
}

async fn session_handler<S>(
    State(state): State<SessionRouteState<S>>,
    headers: HeaderMap,
) -> Response
where
    S: SessionStore + AccountSessionStore + 'static,
{
    let cookie_header = joined_cookie_header(&headers);
    match resolve_session(state.sessions.as_ref(), cookie_header.as_deref()).await {
        Ok(authenticated) => no_store(Json(authenticated.response()).into_response()),
        Err(error) => auth_error_response(error),
    }
}

async fn logout_handler<S>(
    State(state): State<SessionRouteState<S>>,
    headers: HeaderMap,
) -> Response
where
    S: SessionStore + AccountSessionStore + 'static,
{
    let cookie_header = joined_cookie_header(&headers);
    let tenant_result = revoke_session(state.sessions.as_ref(), cookie_header.as_deref()).await;
    let account_result =
        passwordless::revoke_presented_account_session(state.sessions.as_ref(), &headers).await;
    let (mut response, revoked) = match (tenant_result, account_result) {
        (Ok(()), Ok(())) => (
            Json(SignedOutResponse {
                authenticated: false,
            })
            .into_response(),
            true,
        ),
        (Err(error), _) | (_, Err(error)) => (auth_error_response(error), false),
    };
    if revoked {
        if let Ok(value) = HeaderValue::from_str(&clear_session_cookie(state.config)) {
            response.headers_mut().append(SET_COOKIE, value);
        }
        for cookie in passwordless::clear_account_authentication_cookies(state.config) {
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                response.headers_mut().append(SET_COOKIE, value);
            }
        }
        if let Ok(value) = HeaderValue::from_str(&webauthn::clear_binding_cookie(state.config)) {
            response.headers_mut().append(SET_COOKIE, value);
        }
    }
    no_store(response)
}

fn session_response(record: &SessionRecord) -> AuthSessionResponse {
    let subject = &record.subject;
    AuthSessionResponse {
        authenticated: true,
        tenant: subject.tenant(),
        user: AuthUserResponse {
            id: subject.user(),
            display_name: subject.display_name().to_string(),
            roles: subject.roles().to_vec(),
        },
    }
}

fn response_with_cookie<T: Serialize>(status: StatusCode, set_cookie: String, body: T) -> Response {
    response_with_cookies(status, [set_cookie], body)
}

fn response_with_cookies<T, I>(status: StatusCode, set_cookies: I, body: T) -> Response
where
    T: Serialize,
    I: IntoIterator<Item = String>,
{
    let set_cookies = match set_cookies
        .into_iter()
        .map(|cookie| HeaderValue::from_str(&cookie))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(set_cookies) => set_cookies,
        Err(_) => {
            return auth_error_response(AuthError::Unavailable(
                "generated cookie was not a valid HTTP header".to_string(),
            ));
        }
    };
    let mut response = (status, Json(body)).into_response();
    for set_cookie in set_cookies {
        response.headers_mut().append(SET_COOKIE, set_cookie);
    }
    no_store(response)
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

pub(crate) fn auth_error_response(error: AuthError) -> Response {
    let (status, message) = match error {
        AuthError::Unauthenticated => (StatusCode::UNAUTHORIZED, "authentication required"),
        AuthError::Unavailable(_) | AuthError::Randomness(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "authentication unavailable",
        ),
    };
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

pub(crate) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
#[path = "auth/tests.rs"]
mod tests;
