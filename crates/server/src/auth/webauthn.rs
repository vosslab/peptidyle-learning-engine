//! Browser-bound passkey registration and discoverable authentication.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::header::{RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use learning_data_access::{
    AccountIdentityStore, AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationRateLimitScope, BeginWebauthnCeremony, BrowserBindingHash,
    CompletePasskeyAuthenticationAndCreateSession, CredentialIdHash, PasskeyId, PasskeyRecord,
    RegisterPasskey, StoreError, WebauthnCeremonyId, WebauthnCeremonyKind,
    WebauthnCeremonyLifetime, WebauthnState, validated_passkey_label,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Webauthn, WebauthnBuilder,
};

use super::passwordless::{
    ACCOUNT_SESSION_COOKIE, ACCOUNT_SESSION_SECONDS, NETWORK_RATE_LIMIT_ATTEMPTS, RandomSecret,
    authenticated_account, authentication_rejected, clear_named_cookie, consume_rate_limit,
    cookie_secret, passwordless_unavailable, secret_cookie,
};
use super::{ClientAddressPolicy, PasswordlessRateLimitIssuer, SessionConfig, no_store};

const WEBAUTHN_CEREMONY_SECONDS: u32 = 10 * 60;
pub(super) const WEBAUTHN_BINDING_COOKIE: &str = "ple_webauthn_binding";
const MAX_PASSKEY_BODY_BYTES: usize = 96 * 1_024;

pub(super) fn clear_binding_cookie(config: SessionConfig) -> String {
    clear_named_cookie(WEBAUTHN_BINDING_COOKIE, config)
}

/// Validated relying-party configuration shared safely across API replicas.
#[derive(Clone)]
pub struct PasswordlessWebauthn {
    pub(super) inner: Arc<Webauthn>,
}

impl PasswordlessWebauthn {
    pub fn new(
        relying_party_id: &str,
        origin: &str,
        relying_party_name: &str,
    ) -> Result<Self, String> {
        let origin = Url::parse(origin).map_err(|_| "WebAuthn origin is invalid".to_string())?;
        if origin.scheme() != "https"
            && !(origin.scheme() == "http" && origin.host_str() == Some("localhost"))
        {
            return Err("WebAuthn origin must use HTTPS or exact local development".to_string());
        }
        let inner = WebauthnBuilder::new(relying_party_id, &origin)
            .map_err(|_| "WebAuthn relying-party configuration is invalid".to_string())?
            .rp_name(relying_party_name)
            .build()
            .map_err(|_| "WebAuthn relying-party configuration is invalid".to_string())?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

impl std::fmt::Debug for PasswordlessWebauthn {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PasswordlessWebauthn([configured])")
    }
}

struct PasskeyRouteState<S> {
    store: Arc<S>,
    webauthn: PasswordlessWebauthn,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
}

impl<S> Clone for PasskeyRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            webauthn: self.webauthn.clone(),
            rate_limit_issuer: self.rate_limit_issuer.clone(),
            client_address_policy: self.client_address_policy.clone(),
            session_config: self.session_config,
        }
    }
}

pub fn passkey_router<S>(
    store: Arc<S>,
    webauthn: PasswordlessWebauthn,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
) -> Router
where
    S: AccountIdentityStore + AccountSessionStore + 'static,
{
    Router::new()
        .route(
            "/api/auth/passkeys/registration/start",
            post(start_registration::<S>),
        )
        .route(
            "/api/auth/passkeys/registration/complete",
            post(complete_registration::<S>),
        )
        .route(
            "/api/auth/passkeys/authentication/start",
            post(start_authentication::<S>),
        )
        .route(
            "/api/auth/passkeys/authentication/complete",
            post(complete_authentication::<S>),
        )
        .route("/api/auth/passkeys", get(list_passkeys::<S>))
        .route("/api/auth/passkeys/{passkey}", delete(revoke_passkey::<S>))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_PASSKEY_BODY_BYTES))
        .with_state(PasskeyRouteState {
            store,
            webauthn,
            rate_limit_issuer,
            client_address_policy,
            session_config,
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistrationStartResponse {
    ceremony_id: Uuid,
    options: CreationChallengeResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticationStartResponse {
    ceremony_id: Uuid,
    options: RequestChallengeResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteRegistrationRequest {
    ceremony_id: Uuid,
    label: String,
    credential: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteAuthenticationRequest {
    ceremony_id: Uuid,
    credential: PublicKeyCredential,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PasskeyResponse {
    id: Uuid,
    label: String,
    created_at_millis: i64,
    last_used_at_millis: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PasskeyAuthenticatedResponse {
    authenticated: bool,
}

async fn start_registration<S>(
    State(state): State<PasskeyRouteState<S>>,
    headers: HeaderMap,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let existing = match state.store.list_active_passkeys(account.user).await {
        Ok(existing) => existing,
        Err(_) => return passwordless_unavailable(),
    };
    let exclude_credentials = match existing
        .iter()
        .map(decode_passkey)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(passkeys) => Some(
            passkeys
                .into_iter()
                .map(|passkey| passkey.cred_id().clone())
                .collect(),
        ),
        Err(()) => return passwordless_unavailable(),
    };
    let (options, registration) = match state.webauthn.inner.start_passkey_registration(
        account.user.as_uuid(),
        account.email.normalized(),
        &account.display_name,
        exclude_credentials,
    ) {
        Ok(value) => value,
        Err(_) => return passwordless_unavailable(),
    };
    let binding = match RandomSecret::generate() {
        Ok(binding) => binding,
        Err(error) => return super::auth_error_response(error),
    };
    let ceremony_id = match WebauthnCeremonyId::generate() {
        Ok(id) => id,
        Err(_) => return passwordless_unavailable(),
    };
    if persist_ceremony(
        state.store.as_ref(),
        ceremony_id,
        WebauthnCeremonyKind::Registration { user: account.user },
        &binding,
        &registration,
    )
    .await
    .is_err()
    {
        return passwordless_unavailable();
    }
    challenge_response(
        RegistrationStartResponse {
            ceremony_id: ceremony_id.as_uuid(),
            options,
        },
        binding,
        state.session_config,
    )
}

async fn complete_registration<S>(
    State(state): State<PasskeyRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CompleteRegistrationRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let binding = match cookie_secret(&headers, WEBAUTHN_BINDING_COOKIE) {
        Some(binding) => binding,
        None => return authentication_rejected(),
    };
    let ceremony = match state
        .store
        .take_webauthn_ceremony(
            WebauthnCeremonyId::from_uuid(request.ceremony_id),
            BrowserBindingHash::compute(&binding.0),
        )
        .await
    {
        Ok(Some(ceremony)) => ceremony,
        Ok(None) => return authentication_rejected(),
        Err(_) => return passwordless_unavailable(),
    };
    if ceremony.kind != (WebauthnCeremonyKind::Registration { user: account.user }) {
        return authentication_rejected();
    }
    let registration: PasskeyRegistration = match decode_state(&ceremony.state) {
        Ok(state) => state,
        Err(()) => return passwordless_unavailable(),
    };
    let passkey = match state
        .webauthn
        .inner
        .finish_passkey_registration(&request.credential, &registration)
    {
        Ok(passkey) => passkey,
        Err(_) => return authentication_rejected(),
    };
    let label = match validated_passkey_label(&request.label) {
        Ok(label) => label,
        Err(_) => return invalid_passkey_request(),
    };
    let credential_id_hash = CredentialIdHash::compute(passkey.cred_id().as_ref());
    match state
        .store
        .get_active_passkey_by_credential_id_hash(credential_id_hash)
        .await
    {
        Ok(None) => {}
        Ok(Some(_)) => return passkey_conflict(),
        Err(_) => return passwordless_unavailable(),
    }
    let credential = match encode_state(&passkey) {
        Ok(credential) => credential,
        Err(()) => return passwordless_unavailable(),
    };
    let record = match state
        .store
        .insert_passkey(RegisterPasskey {
            id: match PasskeyId::generate() {
                Ok(id) => id,
                Err(_) => return passwordless_unavailable(),
            },
            user: account.user,
            credential_id_hash,
            label,
            credential,
        })
        .await
    {
        Ok(record) => record,
        Err(StoreError::AlreadyExists | StoreError::Conflict) => return passkey_conflict(),
        Err(_) => return passwordless_unavailable(),
    };
    passkey_result_response(record, state.session_config)
}

async fn start_authentication<S>(
    State(state): State<PasskeyRouteState<S>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let Some(network_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Network,
        &state
            .client_address_policy
            .rate_limit_identity(peer, &headers),
    ) else {
        return passwordless_unavailable();
    };
    match consume_rate_limit(
        state.store.as_ref(),
        AuthenticationRateLimitScope::Network,
        network_key,
        NETWORK_RATE_LIMIT_ATTEMPTS,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => return passkey_rate_limited(),
        Err(()) => return passwordless_unavailable(),
    }
    let (options, authentication) = match state.webauthn.inner.start_discoverable_authentication() {
        Ok(value) => value,
        Err(_) => return passwordless_unavailable(),
    };
    let binding = match RandomSecret::generate() {
        Ok(binding) => binding,
        Err(error) => return super::auth_error_response(error),
    };
    let ceremony_id = match WebauthnCeremonyId::generate() {
        Ok(id) => id,
        Err(_) => return passwordless_unavailable(),
    };
    if persist_ceremony(
        state.store.as_ref(),
        ceremony_id,
        WebauthnCeremonyKind::Authentication { user: None },
        &binding,
        &authentication,
    )
    .await
    .is_err()
    {
        return passwordless_unavailable();
    }
    challenge_response(
        AuthenticationStartResponse {
            ceremony_id: ceremony_id.as_uuid(),
            options,
        },
        binding,
        state.session_config,
    )
}

fn passkey_rate_limited() -> Response {
    #[derive(Serialize)]
    struct ErrorBody<'a> {
        error: &'a str,
        message: &'a str,
    }

    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorBody {
            error: "too_many_requests",
            message: "Too many sign-in attempts. Try again later.",
        }),
    )
        .into_response();
    response
        .headers_mut()
        .insert(RETRY_AFTER, HeaderValue::from_static("900"));
    no_store(response)
}

async fn complete_authentication<S>(
    State(state): State<PasskeyRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CompleteAuthenticationRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let binding = match cookie_secret(&headers, WEBAUTHN_BINDING_COOKIE) {
        Some(binding) => binding,
        None => return authentication_rejected(),
    };
    let ceremony = match state
        .store
        .take_webauthn_ceremony(
            WebauthnCeremonyId::from_uuid(request.ceremony_id),
            BrowserBindingHash::compute(&binding.0),
        )
        .await
    {
        Ok(Some(ceremony)) => ceremony,
        Ok(None) => return authentication_rejected(),
        Err(_) => return passwordless_unavailable(),
    };
    if ceremony.kind != (WebauthnCeremonyKind::Authentication { user: None }) {
        return authentication_rejected();
    }
    let authentication: DiscoverableAuthentication = match decode_state(&ceremony.state) {
        Ok(state) => state,
        Err(()) => return passwordless_unavailable(),
    };
    let (user, credential_id) = match state
        .webauthn
        .inner
        .identify_discoverable_authentication(&request.credential)
    {
        Ok(value) => value,
        Err(_) => return authentication_rejected(),
    };
    let record = match state
        .store
        .get_active_passkey_by_credential_id_hash(CredentialIdHash::compute(credential_id))
        .await
    {
        Ok(Some(record)) if record.user.as_uuid() == user => record,
        Ok(_) => return authentication_rejected(),
        Err(_) => return passwordless_unavailable(),
    };
    let mut passkey = match decode_passkey(&record) {
        Ok(passkey) => passkey,
        Err(()) => return passwordless_unavailable(),
    };
    let result = match state.webauthn.inner.finish_discoverable_authentication(
        &request.credential,
        authentication,
        &[DiscoverableKey::from(&passkey)],
    ) {
        Ok(result) if result.user_verified() => result,
        Ok(_) | Err(_) => return authentication_rejected(),
    };
    if passkey.update_credential(&result).is_none() {
        return authentication_rejected();
    }
    let credential = match encode_state(&passkey) {
        Ok(credential) => credential,
        Err(()) => return passwordless_unavailable(),
    };
    let account_token = match RandomSecret::generate() {
        Ok(token) => token,
        Err(error) => return super::auth_error_response(error),
    };
    let completed = match state
        .store
        .complete_passkey_authentication_and_create_session(
            CompletePasskeyAuthenticationAndCreateSession {
                passkey: PasskeyRecord {
                    credential,
                    ..record
                },
                session_token_hash: AccountSessionTokenHash::compute(&account_token.0),
                session_lifetime: AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
                    .expect("account-session lifetime is bounded"),
            },
        )
        .await
    {
        Ok(completed) => completed,
        Err(StoreError::NotFound | StoreError::Conflict) => return authentication_rejected(),
        Err(_) => return passwordless_unavailable(),
    };
    let mut response = Json(PasskeyAuthenticatedResponse {
        authenticated: true,
    })
    .into_response();
    let account_cookie = secret_cookie(
        ACCOUNT_SESSION_COOKIE,
        &account_token,
        ACCOUNT_SESSION_SECONDS,
        state.session_config,
    );
    let clear_binding = clear_named_cookie(WEBAUTHN_BINDING_COOKIE, state.session_config);
    let Ok(account_cookie) = HeaderValue::from_str(&account_cookie) else {
        return passwordless_unavailable();
    };
    let Ok(clear_binding) = HeaderValue::from_str(&clear_binding) else {
        return passwordless_unavailable();
    };
    response.headers_mut().append(SET_COOKIE, account_cookie);
    response.headers_mut().append(SET_COOKIE, clear_binding);
    let _ = completed;
    no_store(response)
}

async fn list_passkeys<S>(State(state): State<PasskeyRouteState<S>>, headers: HeaderMap) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    match state.store.list_active_passkeys(account.user).await {
        Ok(passkeys) => no_store(
            Json(
                passkeys
                    .into_iter()
                    .map(project_passkey)
                    .collect::<Vec<_>>(),
            )
            .into_response(),
        ),
        Err(_) => passwordless_unavailable(),
    }
}

async fn revoke_passkey<S>(
    State(state): State<PasskeyRouteState<S>>,
    headers: HeaderMap,
    Path(passkey): Path<Uuid>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    match state
        .store
        .revoke_passkey(account.user, PasskeyId::from_uuid(passkey))
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(StoreError::NotFound) => passkey_not_found(),
        Err(_) => passwordless_unavailable(),
    }
}

pub(super) async fn persist_ceremony<S, T: Serialize>(
    store: &S,
    id: WebauthnCeremonyId,
    kind: WebauthnCeremonyKind,
    binding: &RandomSecret,
    state: &T,
) -> Result<(), ()>
where
    S: AccountIdentityStore,
{
    store
        .begin_webauthn_ceremony(BeginWebauthnCeremony {
            id,
            kind,
            browser_binding: BrowserBindingHash::compute(&binding.0),
            state: encode_state(state)?,
            lifetime: WebauthnCeremonyLifetime::from_seconds(WEBAUTHN_CEREMONY_SECONDS)
                .expect("WebAuthn ceremony lifetime is bounded"),
        })
        .await
        .map(|_| ())
        .map_err(|_| ())
}

pub(super) fn encode_state<T: Serialize>(state: &T) -> Result<WebauthnState, ()> {
    serde_json::to_vec(state)
        .map_err(|_| ())
        .and_then(|bytes| WebauthnState::new(bytes).map_err(|_| ()))
}

pub(super) fn decode_state<T: for<'de> Deserialize<'de>>(state: &WebauthnState) -> Result<T, ()> {
    serde_json::from_slice(state.as_bytes()).map_err(|_| ())
}

fn decode_passkey(record: &PasskeyRecord) -> Result<Passkey, ()> {
    decode_state(&record.credential)
}

fn challenge_response<T: Serialize>(
    payload: T,
    binding: RandomSecret,
    config: SessionConfig,
) -> Response {
    let mut response = Json(payload).into_response();
    let cookie = secret_cookie(
        WEBAUTHN_BINDING_COOKIE,
        &binding,
        WEBAUTHN_CEREMONY_SECONDS,
        config,
    );
    let Ok(cookie) = HeaderValue::from_str(&cookie) else {
        return passwordless_unavailable();
    };
    response.headers_mut().append(SET_COOKIE, cookie);
    no_store(response)
}

fn passkey_result_response(record: PasskeyRecord, config: SessionConfig) -> Response {
    let mut response = Json(project_passkey(record)).into_response();
    let clear_binding = clear_named_cookie(WEBAUTHN_BINDING_COOKIE, config);
    let Ok(clear_binding) = HeaderValue::from_str(&clear_binding) else {
        return passwordless_unavailable();
    };
    response.headers_mut().append(SET_COOKIE, clear_binding);
    no_store(response)
}

fn project_passkey(record: PasskeyRecord) -> PasskeyResponse {
    PasskeyResponse {
        id: record.id.as_uuid(),
        label: record.label,
        created_at_millis: record.created_at.as_unix_millis(),
        last_used_at_millis: record.last_used_at.map(|value| value.as_unix_millis()),
    }
}

fn invalid_passkey_request() -> Response {
    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": "passkey request is invalid" })),
        )
            .into_response(),
    )
}

fn passkey_conflict() -> Response {
    no_store(
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "passkey is already registered" })),
        )
            .into_response(),
    )
}

fn passkey_not_found() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "passkey not found" })),
        )
            .into_response(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Request;
    use learning_data_access::in_memory::MemoryStore;
    use tower::ServiceExt;

    #[test]
    fn configuration_requires_https_except_exact_localhost_development() {
        assert!(
            PasswordlessWebauthn::new("example.edu", "https://learn.example.edu", "PLE",).is_ok()
        );
        assert!(
            PasswordlessWebauthn::new("localhost", "http://localhost:4173", "PLE local",).is_ok()
        );
        assert!(PasswordlessWebauthn::new("example.edu", "http://example.edu", "PLE",).is_err());
    }

    #[test]
    fn discoverable_authentication_challenge_has_no_allowed_credential_list() {
        let configured =
            PasswordlessWebauthn::new("localhost", "http://localhost:4173", "PLE local")
                .expect("local configuration");
        let (challenge, _) = configured
            .inner
            .start_discoverable_authentication()
            .expect("discoverable challenge");
        assert!(challenge.public_key.allow_credentials.is_empty());
    }

    #[tokio::test]
    async fn authentication_start_is_browser_bound_no_store_and_usernameless() {
        let app = passkey_router(
            Arc::new(MemoryStore::default()),
            PasswordlessWebauthn::new("localhost", "http://localhost:4173", "PLE local")
                .expect("local configuration"),
            PasswordlessRateLimitIssuer::from_server_secret([0x91; 32]),
            ClientAddressPolicy::direct(),
            SessionConfig::new(
                learning_data_access::SessionLifetime::from_seconds(60).expect("session lifetime"),
                super::super::CookieTransport::LocalHttp,
            ),
        )
        .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 91], 443))));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/passkeys/authentication/start")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        assert!(response.headers().get_all(SET_COOKIE).iter().any(|value| {
            value.to_str().is_ok_and(|value| {
                value.starts_with("ple_webauthn_binding=") && value.contains("HttpOnly")
            })
        }));
        let body = to_bytes(response.into_body(), MAX_PASSKEY_BODY_BYTES)
            .await
            .expect("response body");
        let body: serde_json::Value = serde_json::from_slice(&body).expect("challenge JSON");
        assert_eq!(
            body.pointer("/options/publicKey/allowCredentials")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(0)
        );
    }
}
