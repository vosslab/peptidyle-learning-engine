//! Provider-neutral authentication and replica-safe sessions (MOD-API-AUTH).
//!
//! A credential provider establishes [`SessionSubject`]. This module then
//! mints a 256-bit opaque cookie credential, persists only its SHA-256 hash,
//! and resolves the tenant from the database row. Request parameters, headers,
//! and bodies never construct [`TenantContext`].

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::header::{CACHE_CONTROL, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cookie::{Cookie, SameSite};
use question_model::{TenantId, UserId, UserRole};
use serde::Serialize;
use serde::de::DeserializeOwned;
use store::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash, StoreError,
    TenantContext,
};

const SESSION_COOKIE_NAME: &str = "ple_session";
const SESSION_TOKEN_BYTES: usize = 32;
const TOKEN_GENERATION_ATTEMPTS: usize = 3;
const MAX_AUTH_PRESENTATION_BYTES: usize = 64 * 1_024;

/// HTTP setting selected for the application's deployment context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieTransport {
    /// Normal HTTPS navigation, protected against cross-site requests.
    FirstPartyHttps,
    /// HTTPS embedding needed by a future configured LTI launch.
    EmbeddedHttps,
    /// Explicit opt-out used only by local plain-HTTP development.
    LocalHttp,
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

    fn secure(self) -> bool {
        !matches!(self.transport, CookieTransport::LocalHttp)
    }

    fn same_site(self) -> SameSite {
        match self.transport {
            CookieTransport::FirstPartyHttps | CookieTransport::LocalHttp => SameSite::Lax,
            CookieTransport::EmbeddedHttps => SameSite::None,
        }
    }
}

/// Provider-specific credential verification kept outside session mechanics.
///
/// OIDC, institutional SSO, LTI, or a local development provider can implement
/// this boundary without changing cookie handling or persistence.
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Credential presentation type owned by that provider.
    type Presentation: Send + Sync + ?Sized;

    /// Verifies credentials and returns a trusted application identity.
    async fn verify(
        &self,
        presentation: &Self::Presentation,
    ) -> Result<SessionSubject, IdentityProviderError>;
}

/// Credential-provider failure without exposing provider secrets to callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityProviderError {
    /// Credentials are absent, invalid, or no longer authorized.
    Rejected,
    /// The provider could not complete a verification request.
    Unavailable(String),
}

/// Issued database session and the header value sent to the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedSession {
    /// Persisted session metadata, containing only the token hash.
    pub record: SessionRecord,
    /// Complete `Set-Cookie` value; the only returned value containing the token.
    pub set_cookie: String,
}

/// Authenticated principal and its derived tenant boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    /// Active session metadata resolved from shared storage.
    pub record: SessionRecord,
    /// RLS context derived only from the resolved record.
    pub tenant_context: TenantContext,
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
    /// The configured credential provider rejected a login.
    ProviderRejected,
    /// A dependency needed for authentication was unavailable.
    Unavailable(String),
    /// The operating system could not supply cryptographic randomness.
    Randomness(getrandom::Error),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthenticated | Self::ProviderRejected => {
                formatter.write_str("authentication required")
            }
            Self::Unavailable(message) => {
                write!(formatter, "authentication unavailable: {message}")
            }
            Self::Randomness(error) => write!(formatter, "secure randomness unavailable: {error}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Builds the provider-backed `/api/auth` route group.
///
/// The provider owns the typed credential presentation and its anti-replay or
/// anti-CSRF validation. Session and cookie policy remain provider-neutral.
pub fn router<P, S>(provider: Arc<P>, sessions: Arc<S>, config: SessionConfig) -> Router
where
    P: IdentityProvider + 'static,
    P::Presentation: DeserializeOwned + Send + Sync + 'static,
    S: SessionStore + 'static,
{
    let state = AuthRouteState {
        provider,
        sessions,
        config,
    };
    Router::new()
        .route("/api/auth/login", post(login_handler::<P, S>))
        .route("/api/auth/session", get(session_handler::<P, S>))
        .route("/api/auth/logout", post(logout_handler::<P, S>))
        .layer(DefaultBodyLimit::max(MAX_AUTH_PRESENTATION_BYTES))
        .with_state(state)
}

/// Verifies provider credentials and establishes a database session.
pub async fn authenticate_with_provider<P: IdentityProvider>(
    provider: &P,
    presentation: &P::Presentation,
    sessions: &dyn SessionStore,
    config: SessionConfig,
) -> Result<IssuedSession, AuthError> {
    let subject = provider
        .verify(presentation)
        .await
        .map_err(|error| match error {
            IdentityProviderError::Rejected => AuthError::ProviderRejected,
            IdentityProviderError::Unavailable(message) => AuthError::Unavailable(message),
        })?;
    issue_session(sessions, subject, config).await
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
    Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(config.secure())
        .same_site(config.same_site())
        .max_age(cookie::time::Duration::ZERO)
        .build()
        .to_string()
}

struct AuthRouteState<P, S> {
    provider: Arc<P>,
    sessions: Arc<S>,
    config: SessionConfig,
}

impl<P, S> Clone for AuthRouteState<P, S> {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            sessions: Arc::clone(&self.sessions),
            config: self.config,
        }
    }
}

async fn login_handler<P, S>(
    State(state): State<AuthRouteState<P, S>>,
    Json(presentation): Json<P::Presentation>,
) -> Response
where
    P: IdentityProvider + 'static,
    P::Presentation: DeserializeOwned + Send + Sync + 'static,
    S: SessionStore + 'static,
{
    match authenticate_with_provider(
        state.provider.as_ref(),
        &presentation,
        state.sessions.as_ref(),
        state.config,
    )
    .await
    {
        Ok(issued) => response_with_cookie(
            StatusCode::OK,
            issued.set_cookie,
            session_response(&issued.record),
        ),
        Err(error) => auth_error_response(error),
    }
}

async fn session_handler<P, S>(
    State(state): State<AuthRouteState<P, S>>,
    headers: HeaderMap,
) -> Response
where
    P: IdentityProvider + 'static,
    S: SessionStore + 'static,
{
    let cookie_header = joined_cookie_header(&headers);
    match resolve_session(state.sessions.as_ref(), cookie_header.as_deref()).await {
        Ok(authenticated) => no_store(Json(authenticated.response()).into_response()),
        Err(error) => auth_error_response(error),
    }
}

async fn logout_handler<P, S>(
    State(state): State<AuthRouteState<P, S>>,
    headers: HeaderMap,
) -> Response
where
    P: IdentityProvider + 'static,
    S: SessionStore + 'static,
{
    let cookie_header = joined_cookie_header(&headers);
    let mut response = match revoke_session(state.sessions.as_ref(), cookie_header.as_deref()).await
    {
        Ok(()) => Json(SignedOutResponse {
            authenticated: false,
        })
        .into_response(),
        Err(error) => auth_error_response(error),
    };
    if let Ok(value) = HeaderValue::from_str(&clear_session_cookie(state.config)) {
        response.headers_mut().insert(SET_COOKIE, value);
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
    let mut response = (status, Json(body)).into_response();
    match HeaderValue::from_str(&set_cookie) {
        Ok(value) => {
            response.headers_mut().insert(SET_COOKIE, value);
            no_store(response)
        }
        Err(_) => auth_error_response(AuthError::Unavailable(
            "generated cookie was not a valid HTTP header".to_string(),
        )),
    }
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
        AuthError::Unauthenticated | AuthError::ProviderRejected => {
            (StatusCode::UNAUTHORIZED, "authentication required")
        }
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

#[derive(Clone, PartialEq, Eq)]
struct SessionToken([u8; SESSION_TOKEN_BYTES]);

impl SessionToken {
    fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; SESSION_TOKEN_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    fn decode(value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        let bytes: [u8; SESSION_TOKEN_BYTES] = bytes.try_into().ok()?;
        Some(Self(bytes))
    }

    fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    fn hash(&self) -> SessionTokenHash {
        SessionTokenHash::compute(&self.0)
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionToken([redacted])")
    }
}

fn session_cookie(token: &SessionToken, config: SessionConfig) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, token.encode()))
        .path("/")
        .http_only(true)
        .secure(config.secure())
        .same_site(config.same_site())
        .build()
}

fn presented_token(cookie_header: Option<&str>) -> Option<SessionToken> {
    let mut tokens = Cookie::split_parse(cookie_header?)
        .filter_map(Result::ok)
        .filter_map(|cookie| {
            (cookie.name() == SESSION_COOKIE_NAME)
                .then(|| SessionToken::decode(cookie.value()))
                .flatten()
        });
    let token = tokens.next()?;
    if tokens.next().is_some() {
        return None;
    }
    Some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use question_model::{TenantId, UserId, UserRole};
    use store::memory::MemoryStore;
    use tower::ServiceExt;
    use uuid::Uuid;

    struct FixtureProvider {
        subject: SessionSubject,
    }

    #[derive(serde::Deserialize)]
    struct FixturePresentation {
        assertion: String,
    }

    #[async_trait]
    impl IdentityProvider for FixtureProvider {
        type Presentation = FixturePresentation;

        async fn verify(
            &self,
            presentation: &Self::Presentation,
        ) -> Result<SessionSubject, IdentityProviderError> {
            if presentation.assertion == "valid fixture assertion" {
                Ok(self.subject.clone())
            } else {
                Err(IdentityProviderError::Rejected)
            }
        }
    }

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
    async fn provider_login_on_one_replica_resolves_on_another() {
        let issuer = MemoryStore::default();
        let next_replica = issuer.clone();
        let provider = FixtureProvider { subject: subject() };
        let issued = authenticate_with_provider(
            &provider,
            &FixturePresentation {
                assertion: "valid fixture assertion".to_string(),
            },
            &issuer,
            config(CookieTransport::FirstPartyHttps),
        )
        .await
        .expect("provider login should issue a session");
        let authenticated = resolve_session(
            &next_replica,
            Some(cookie_request_header(&issued.set_cookie)),
        )
        .await
        .expect("another replica should resolve the database session");

        assert_eq!(authenticated.record, issued.record);
        assert_eq!(authenticated.tenant_context.tenant_id(), subject().tenant());
        assert_eq!(
            serde_json::to_value(authenticated.response()).expect("response should serialize"),
            serde_json::json!({
                "authenticated": true,
                "tenant": subject().tenant(),
                "user": {
                    "id": subject().user(),
                    "displayName": "Fixture Student",
                    "roles": ["student"]
                }
            })
        );
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

    #[test]
    fn cookie_attributes_match_the_selected_transport() {
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

        let embedded = session_cookie(&token, config(CookieTransport::EmbeddedHttps));
        assert_eq!(embedded.secure(), Some(true));
        assert_eq!(embedded.same_site(), Some(SameSite::None));

        let local = session_cookie(&token, config(CookieTransport::LocalHttp));
        assert_eq!(local.secure(), Some(false));
        assert_eq!(local.same_site(), Some(SameSite::Lax));
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

    #[tokio::test]
    async fn rejected_provider_credentials_create_no_session() {
        let store = MemoryStore::default();
        let provider = FixtureProvider { subject: subject() };
        assert!(matches!(
            authenticate_with_provider(
                &provider,
                &FixturePresentation {
                    assertion: "wrong fixture assertion".to_string(),
                },
                &store,
                config(CookieTransport::FirstPartyHttps),
            )
            .await,
            Err(AuthError::ProviderRejected)
        ));
    }

    #[tokio::test]
    async fn auth_http_routes_preserve_the_replica_boundary() {
        let issuer = Arc::new(MemoryStore::default());
        let next_replica = Arc::new(issuer.as_ref().clone());
        let provider = Arc::new(FixtureProvider { subject: subject() });
        let issuer_app = router(
            Arc::clone(&provider),
            issuer,
            config(CookieTransport::LocalHttp),
        );
        let replica_app = router(provider, next_replica, config(CookieTransport::LocalHttp));
        let login = issuer_app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "assertion": "valid fixture assertion" }).to_string(),
                    ))
                    .expect("login request"),
            )
            .await
            .expect("login response");
        assert_eq!(login.status(), StatusCode::OK);
        assert_eq!(
            login.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let cookie = login
            .headers()
            .get(SET_COOKIE)
            .expect("login should set the opaque cookie")
            .to_str()
            .expect("cookie header should be text")
            .to_string();
        let cookie = cookie_request_header(&cookie).to_string();

        let resumed = replica_app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/session")
                    .header(COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("session request"),
            )
            .await
            .expect("session response");
        assert_eq!(resumed.status(), StatusCode::OK);
        assert_eq!(
            resumed.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );

        let logout = replica_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header(COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("logout request"),
            )
            .await
            .expect("logout response");
        assert_eq!(logout.status(), StatusCode::OK);
        assert!(
            logout
                .headers()
                .get(SET_COOKIE)
                .expect("logout should clear the cookie")
                .to_str()
                .expect("clear-cookie header should be text")
                .contains("Max-Age=0")
        );

        let revoked = issuer_app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/session")
                    .header(COOKIE, cookie)
                    .body(Body::empty())
                    .expect("revoked session request"),
            )
            .await
            .expect("revoked session response");
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    }
}
