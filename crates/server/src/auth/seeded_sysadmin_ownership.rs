//! Deployment-gated first passkey setup for the seeded live-demo Sysadmin.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use learning_data_access::{
    AccountIdentityStore, AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationRateLimitScope, BrowserBindingHash, CompleteSeededSysadminOwnership,
    CredentialIdHash, LiveDemoInstallationStore, PasskeyId, RegisterPasskey, SessionStore,
    StoreError, WebauthnCeremonyId, WebauthnCeremonyKind, validated_passkey_label,
};
use question_model::UserId;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PasskeyRegistration, RegisterPublicKeyCredential,
};

use super::browser_boundary::origin_matches;
use super::passwordless::{
    ACCOUNT_SESSION_COOKIE, ACCOUNT_SESSION_SECONDS, NETWORK_RATE_LIMIT_ATTEMPTS,
    PasswordlessRateLimitIssuer, RandomSecret, SERVICE_RATE_LIMIT_ATTEMPTS, clear_named_cookie,
    secret_cookie,
};
use super::webauthn::{
    PasswordlessWebauthn, WEBAUTHN_BINDING_COOKIE, decode_state, encode_state, persist_ceremony,
    require_discoverable_credential,
};
use super::{ClientAddressPolicy, SessionConfig, clear_session_cookie, no_store};

const OWNERSHIP_SERVICE_KEY: &[u8] = b"seeded-sysadmin-ownership-v1";
const OWNERSHIP_PRINCIPAL_ATTEMPTS: u32 = 12;
// A persisted WebAuthn state/credential is capped at 64 KiB. Base64url
// transport plus the fixed registration envelope fits within 96 KiB while
// retaining a hard handler-owned request bound.
const MAX_OWNERSHIP_BODY_BYTES: usize = 96 * 1024;

/// Closed deployment configuration for the one seeded Sysadmin ownership path.
#[derive(Clone)]
pub struct SeededSysadminOwnershipConfig {
    origin: Arc<str>,
    installation_generation: Uuid,
    user: UserId,
    ownership_proof: [u8; 32],
}

impl SeededSysadminOwnershipConfig {
    /// Validates a canonical installer-owned claim context.
    pub fn new(
        origin: Arc<str>,
        installation_generation: Uuid,
        user: UserId,
        ownership_proof: [u8; 32],
    ) -> Result<Self, String> {
        if origin.is_empty() || installation_generation.is_nil() || user.as_uuid().is_nil() {
            return Err("seeded Sysadmin ownership configuration is invalid".to_string());
        }
        Ok(Self {
            origin,
            installation_generation,
            user,
            ownership_proof,
        })
    }

    pub(crate) fn user(&self) -> UserId {
        self.user
    }
}

impl std::fmt::Debug for SeededSysadminOwnershipConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SeededSysadminOwnershipConfig")
            .field("origin", &self.origin)
            .field("installation_generation", &"[redacted]")
            .field("user", &"[redacted]")
            .field("ownership_proof", &"[redacted]")
            .finish()
    }
}

struct OwnershipState<S> {
    store: Arc<S>,
    config: Option<SeededSysadminOwnershipConfig>,
    webauthn: PasswordlessWebauthn,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
}

impl<S> Clone for OwnershipState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            webauthn: self.webauthn.clone(),
            rate_limit_issuer: self.rate_limit_issuer.clone(),
            client_address_policy: self.client_address_policy.clone(),
            session_config: self.session_config,
        }
    }
}

/// Builds only the first-ownership routes; ordinary passkey authentication stays unchanged.
pub fn seeded_sysadmin_ownership_router<S>(
    store: Arc<S>,
    config: Option<SeededSysadminOwnershipConfig>,
    webauthn: PasswordlessWebauthn,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
) -> Router
where
    S: AccountIdentityStore
        + AccountSessionStore
        + LiveDemoInstallationStore
        + SessionStore
        + 'static,
{
    Router::new()
        .route(
            "/api/auth/live-demo/sysadmin-ownership",
            get(ownership_status::<S>).post(start_ownership::<S>),
        )
        .route(
            "/api/auth/live-demo/sysadmin-ownership/complete",
            post(complete_ownership::<S>),
        )
        .with_state(OwnershipState {
            store,
            config,
            webauthn,
            rate_limit_issuer,
            client_address_policy,
            session_config,
        })
}

#[derive(Serialize)]
struct OwnershipStatusResponse {
    available: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartOwnershipRequest {
    ownership_proof: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnershipStartResponse {
    ceremony_id: Uuid,
    options: CreationChallengeResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteOwnershipRequest {
    ownership_proof: String,
    ceremony_id: Uuid,
    label: String,
    credential: RegisterPublicKeyCredential,
}

#[derive(Serialize)]
struct OwnershipCompleteResponse {
    authenticated: bool,
}

async fn ownership_status<S>(State(state): State<OwnershipState<S>>) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + LiveDemoInstallationStore + SessionStore,
{
    let Some(config) = state.config else {
        return ownership_unavailable();
    };
    match configured_live_demo_generation(state.store.as_ref(), &config).await {
        Ok(()) => {}
        Err(()) => return ownership_unavailable(),
    }
    match state
        .store
        .seeded_sysadmin_ownership_available(config.user)
        .await
    {
        Ok(available) => no_store(Json(OwnershipStatusResponse { available }).into_response()),
        Err(_) => ownership_unavailable(),
    }
}

async fn start_ownership<S>(
    State(state): State<OwnershipState<S>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + LiveDemoInstallationStore + SessionStore,
{
    let Some(config) = &state.config else {
        return ownership_unavailable();
    };
    let headers = request.headers().clone();
    if !origin_matches(&headers, &config.origin) {
        return ownership_rejected();
    }
    let Some(request) = ownership_request::<StartOwnershipRequest>(request).await else {
        return ownership_unavailable();
    };
    if consume_ownership_limits(&state, peer, &headers, config)
        .await
        .is_err()
    {
        return ownership_unavailable();
    }
    if !proof_matches(&request.ownership_proof, config) {
        return ownership_rejected();
    }
    if configured_live_demo_generation(state.store.as_ref(), config)
        .await
        .is_err()
    {
        return ownership_unavailable();
    }
    if !state
        .store
        .seeded_sysadmin_ownership_available(config.user)
        .await
        .unwrap_or(false)
    {
        return ownership_unavailable();
    }
    let account = match state.store.get_account(config.user).await {
        Ok(Some(account)) => account,
        Ok(None) | Err(_) => return ownership_unavailable(),
    };
    let (options, registration) = match state.webauthn.inner.start_passkey_registration(
        config.user.as_uuid(),
        account.email.normalized(),
        &account.display_name,
        None,
    ) {
        Ok(value) => value,
        Err(_) => return ownership_unavailable(),
    };
    let options = match require_discoverable_credential(options) {
        Ok(options) => options,
        Err(()) => return ownership_unavailable(),
    };
    let binding = match RandomSecret::generate() {
        Ok(value) => value,
        Err(error) => return super::auth_error_response(error),
    };
    let ceremony_id = match WebauthnCeremonyId::generate() {
        Ok(value) => value,
        Err(_) => return ownership_unavailable(),
    };
    if persist_ceremony(
        state.store.as_ref(),
        ceremony_id,
        WebauthnCeremonyKind::Registration { user: config.user },
        &binding,
        &registration,
    )
    .await
    .is_err()
    {
        return ownership_unavailable();
    }
    ownership_challenge_response(
        OwnershipStartResponse {
            ceremony_id: ceremony_id.as_uuid(),
            options,
        },
        binding,
        state.session_config,
    )
}

async fn complete_ownership<S>(
    State(state): State<OwnershipState<S>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + LiveDemoInstallationStore + SessionStore,
{
    let Some(config) = &state.config else {
        return ownership_unavailable();
    };
    let headers = request.headers().clone();
    if !origin_matches(&headers, &config.origin) {
        return ownership_rejected();
    }
    let Some(request) = ownership_request::<CompleteOwnershipRequest>(request).await else {
        return ownership_unavailable();
    };
    if consume_ownership_limits(&state, peer, &headers, config)
        .await
        .is_err()
    {
        return ownership_unavailable();
    }
    if !proof_matches(&request.ownership_proof, config) {
        return ownership_rejected();
    }
    if configured_live_demo_generation(state.store.as_ref(), config)
        .await
        .is_err()
    {
        return ownership_unavailable();
    }
    if !state
        .store
        .seeded_sysadmin_ownership_available(config.user)
        .await
        .unwrap_or(false)
    {
        return ownership_unavailable();
    }
    let Some(binding) = super::passwordless::cookie_secret(&headers, WEBAUTHN_BINDING_COOKIE)
    else {
        return ownership_rejected();
    };
    // The Store reads this ceremony without consuming it. Its atomic ownership
    // command consumes the exact bound row only after WebAuthn verification.
    let ceremony = match state
        .store
        .get_webauthn_ceremony(
            WebauthnCeremonyId::from_uuid(request.ceremony_id),
            BrowserBindingHash::compute(&binding.0),
        )
        .await
    {
        Ok(Some(ceremony)) => ceremony,
        Ok(None) => return ownership_rejected(),
        Err(_) => return ownership_unavailable(),
    };
    if ceremony.kind != (WebauthnCeremonyKind::Registration { user: config.user }) {
        return ownership_rejected();
    }
    let registration: PasskeyRegistration = match decode_state(&ceremony.state) {
        Ok(value) => value,
        Err(()) => return ownership_unavailable(),
    };
    let passkey = match state
        .webauthn
        .inner
        .finish_passkey_registration(&request.credential, &registration)
    {
        Ok(value) => value,
        Err(_) => return ownership_rejected(),
    };
    let label = match validated_passkey_label(&request.label) {
        Ok(value) => value,
        Err(_) => return ownership_rejected(),
    };
    let credential = match encode_state(&passkey) {
        Ok(value) => value,
        Err(()) => return ownership_unavailable(),
    };
    let account_token = match RandomSecret::generate() {
        Ok(value) => value,
        Err(error) => return super::auth_error_response(error),
    };
    let passkey_id = match PasskeyId::generate() {
        Ok(value) => value,
        Err(_) => return ownership_unavailable(),
    };
    let account_cookie = secret_cookie(
        ACCOUNT_SESSION_COOKIE,
        &account_token,
        ACCOUNT_SESSION_SECONDS,
        state.session_config,
    );
    let clear_binding = clear_named_cookie(WEBAUTHN_BINDING_COOKIE, state.session_config);
    let clear_account = clear_named_cookie(ACCOUNT_SESSION_COOKIE, state.session_config);
    let clear_tenant = clear_session_cookie(state.session_config);
    let (Ok(account_cookie), Ok(clear_binding), Ok(clear_account), Ok(clear_tenant)) = (
        HeaderValue::from_str(&account_cookie),
        HeaderValue::from_str(&clear_binding),
        HeaderValue::from_str(&clear_account),
        HeaderValue::from_str(&clear_tenant),
    ) else {
        return ownership_unavailable();
    };
    let presented_account_session =
        super::passwordless::cookie_secret(&headers, ACCOUNT_SESSION_COOKIE)
            .map(|token| AccountSessionTokenHash::compute(&token.0));
    let presented_tenant_session =
        super::session_cookie::presented_token(super::joined_cookie_header(&headers).as_deref())
            .map(|token| token.hash());
    match state
        .store
        .complete_seeded_sysadmin_ownership(CompleteSeededSysadminOwnership {
            target: config.user,
            ceremony_id: WebauthnCeremonyId::from_uuid(request.ceremony_id),
            browser_binding: BrowserBindingHash::compute(&binding.0),
            passkey: RegisterPasskey {
                id: passkey_id,
                user: config.user,
                credential_id_hash: CredentialIdHash::compute(passkey.cred_id().as_ref()),
                label,
                credential,
            },
            session_token_hash: AccountSessionTokenHash::compute(&account_token.0),
            session_lifetime: AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
                .expect("account-session lifetime is bounded"),
            presented_account_session,
            presented_tenant_session,
        })
        .await
    {
        Ok(_) => {}
        Err(StoreError::NotFound | StoreError::Conflict | StoreError::Forbidden) => {
            return ownership_rejected();
        }
        Err(_) => return ownership_unavailable(),
    }
    let mut response = Json(OwnershipCompleteResponse {
        authenticated: true,
    })
    .into_response();
    response.headers_mut().append(SET_COOKIE, clear_account);
    response.headers_mut().append(SET_COOKIE, clear_binding);
    response.headers_mut().append(SET_COOKIE, clear_tenant);
    response.headers_mut().append(SET_COOKIE, account_cookie);
    no_store(response)
}

async fn ownership_request<T>(request: Request) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    if !has_exact_json_content_type(request.headers()) {
        return None;
    }
    let body = to_bytes(request.into_body(), MAX_OWNERSHIP_BODY_BYTES)
        .await
        .ok()?;
    serde_json::from_slice(&body).ok()
}

fn has_exact_json_content_type(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    matches!(values.next(), Some(value) if value.as_bytes() == b"application/json")
        && values.next().is_none()
}

async fn configured_live_demo_generation<S>(
    store: &S,
    config: &SeededSysadminOwnershipConfig,
) -> Result<(), ()>
where
    S: LiveDemoInstallationStore + ?Sized,
{
    match store.completed_live_demo_installation_generation().await {
        Ok(Some(generation)) if generation == config.installation_generation => Ok(()),
        Ok(None) | Ok(Some(_)) | Err(_) => Err(()),
    }
}

async fn consume_ownership_limits<S>(
    state: &OwnershipState<S>,
    peer: SocketAddr,
    headers: &HeaderMap,
    config: &SeededSysadminOwnershipConfig,
) -> Result<(), ()>
where
    S: AccountIdentityStore + AccountSessionStore + LiveDemoInstallationStore + SessionStore,
{
    let network = state.rate_limit_issuer.seeded_sysadmin_ownership_key(
        AuthenticationRateLimitScope::Network,
        &state
            .client_address_policy
            .rate_limit_identity(peer, headers),
    );
    let principal = state.rate_limit_issuer.seeded_sysadmin_ownership_key(
        AuthenticationRateLimitScope::Principal,
        config.user.as_uuid().as_bytes(),
    );
    let service = state.rate_limit_issuer.seeded_sysadmin_ownership_key(
        AuthenticationRateLimitScope::Service,
        OWNERSHIP_SERVICE_KEY,
    );
    let (Some(network), Some(principal), Some(service)) = (network, principal, service) else {
        return Err(());
    };
    match super::passwordless::consume_rate_limits(
        state.store.as_ref(),
        [
            (
                AuthenticationRateLimitScope::Network,
                network,
                NETWORK_RATE_LIMIT_ATTEMPTS,
            ),
            (
                AuthenticationRateLimitScope::Principal,
                principal,
                OWNERSHIP_PRINCIPAL_ATTEMPTS,
            ),
            (
                AuthenticationRateLimitScope::Service,
                service,
                SERVICE_RATE_LIMIT_ATTEMPTS,
            ),
        ],
    )
    .await
    {
        Ok(super::passwordless::RateLimitOutcome::Allowed) => Ok(()),
        Ok(super::passwordless::RateLimitOutcome::Denied { .. }) | Err(()) => Err(()),
    }
}

fn proof_matches(value: &str, config: &SeededSysadminOwnershipConfig) -> bool {
    let Ok(decoded) = URL_SAFE_NO_PAD.decode(value.as_bytes()) else {
        return false;
    };
    let Ok(proof) = <[u8; 32]>::try_from(decoded.as_slice()) else {
        return false;
    };
    URL_SAFE_NO_PAD.encode(proof) == value && bool::from(proof.ct_eq(&config.ownership_proof))
}

fn ownership_challenge_response<T: Serialize>(
    payload: T,
    binding: RandomSecret,
    session_config: SessionConfig,
) -> Response {
    let cookie = super::passwordless::secret_cookie(
        WEBAUTHN_BINDING_COOKIE,
        &binding,
        10 * 60,
        session_config,
    );
    let Ok(cookie) = HeaderValue::from_str(&cookie) else {
        return ownership_unavailable();
    };
    let mut response = Json(payload).into_response();
    response.headers_mut().append(SET_COOKIE, cookie);
    no_store(response)
}

fn ownership_unavailable() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "unavailable" })),
        )
            .into_response(),
    )
}

fn ownership_rejected() -> Response {
    no_store(
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "rejected" })),
        )
            .into_response(),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Request;
    use learning_data_access::in_memory::{MemoryLiveDemoInstallationState, MemoryStore};
    use tower::ServiceExt;

    use super::*;

    const ORIGIN: &str = "http://localhost:3000";

    fn config() -> SeededSysadminOwnershipConfig {
        SeededSysadminOwnershipConfig::new(
            Arc::from(ORIGIN),
            Uuid::from_u128(1),
            UserId::from_uuid(Uuid::from_u128(2)),
            [3; 32],
        )
        .expect("valid closed configuration")
    }

    fn proof() -> String {
        URL_SAFE_NO_PAD.encode([3; 32])
    }

    fn app(config: Option<SeededSysadminOwnershipConfig>) -> Router {
        let store = Arc::new(MemoryStore::default());
        if config.is_some() {
            store
                .set_live_demo_installation_state_for_test(
                    MemoryLiveDemoInstallationState::Complete {
                        generation: Uuid::from_u128(1),
                    },
                )
                .expect("test lifecycle state");
        }
        seeded_sysadmin_ownership_router(
            store,
            config,
            PasswordlessWebauthn::new("localhost", ORIGIN, "PLE test")
                .expect("local WebAuthn configuration"),
            PasswordlessRateLimitIssuer::from_server_secret([4; 32]),
            ClientAddressPolicy::direct(),
            SessionConfig::new(
                learning_data_access::SessionLifetime::from_seconds(60).expect("lifetime"),
                super::super::CookieTransport::FirstPartyHttps,
            ),
        )
        .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 9], 443))))
    }

    fn app_with_lifecycle(state: MemoryLiveDemoInstallationState) -> Router {
        let store = Arc::new(MemoryStore::default());
        store
            .set_live_demo_installation_state_for_test(state)
            .expect("test lifecycle state");
        seeded_sysadmin_ownership_router(
            store,
            Some(config()),
            PasswordlessWebauthn::new("localhost", ORIGIN, "PLE test")
                .expect("local WebAuthn configuration"),
            PasswordlessRateLimitIssuer::from_server_secret([4; 32]),
            ClientAddressPolicy::direct(),
            SessionConfig::new(
                learning_data_access::SessionLifetime::from_seconds(60).expect("lifetime"),
                super::super::CookieTransport::FirstPartyHttps,
            ),
        )
        .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 9], 443))))
    }

    #[tokio::test]
    async fn status_is_a_small_disabled_or_generic_availability_surface() {
        let disabled = app(None)
            .oneshot(
                Request::builder()
                    .uri("/api/auth/live-demo/sysadmin-ownership")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

        let response = app(Some(config()))
            .oneshot(
                Request::builder()
                    .uri("/api/auth/live-demo/sysadmin-ownership")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), 1_024).await.expect("body");
        let body = String::from_utf8(body.to_vec()).expect("UTF-8 response");
        assert_eq!(body, r#"{"error":"unavailable"}"#);
        assert!(!body.contains("ownershipProof") && !body.contains("installationGeneration"));
    }

    #[tokio::test]
    async fn start_requires_exact_origin_and_constant_time_proof_before_cookies() {
        let missing_origin = app(Some(config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/sysadmin-ownership")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ownershipProof":"{}"}}"#, proof())))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing_origin.status(), StatusCode::FORBIDDEN);
        assert!(
            missing_origin
                .headers()
                .get_all(SET_COOKIE)
                .iter()
                .next()
                .is_none()
        );

        let invalid_proof = app(Some(config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/sysadmin-ownership")
                    .header("content-type", "application/json")
                    .header("origin", ORIGIN)
                    .body(Body::from(r#"{"ownershipProof":"wrong"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(invalid_proof.status(), StatusCode::FORBIDDEN);
        assert!(
            invalid_proof
                .headers()
                .get_all(SET_COOKIE)
                .iter()
                .next()
                .is_none()
        );
    }

    #[tokio::test]
    async fn stale_or_incomplete_generation_rejects_before_ownership_effects() {
        for lifecycle in [
            MemoryLiveDemoInstallationState::Missing,
            MemoryLiveDemoInstallationState::Installing {
                generation: Uuid::from_u128(1),
            },
            MemoryLiveDemoInstallationState::Complete {
                generation: Uuid::from_u128(9),
            },
        ] {
            let response = app_with_lifecycle(lifecycle)
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/auth/live-demo/sysadmin-ownership")
                        .header("content-type", "application/json")
                        .header("origin", ORIGIN)
                        .body(Body::from(format!(r#"{{"ownershipProof":"{}"}}"#, proof())))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(
                response
                    .headers()
                    .get_all(SET_COOKIE)
                    .iter()
                    .next()
                    .is_none()
            );
        }
    }

    #[tokio::test]
    async fn post_parser_failures_are_one_generic_no_store_cookie_free_surface() {
        let oversized = "{".repeat(MAX_OWNERSHIP_BODY_BYTES + 1);
        for (path, content_type, body) in [
            (
                "/api/auth/live-demo/sysadmin-ownership",
                "text/plain",
                r#"{"ownershipProof":"x"}"#.to_string(),
            ),
            (
                "/api/auth/live-demo/sysadmin-ownership",
                "application/json",
                r#"{"ownershipProof":"#.to_string(),
            ),
            (
                "/api/auth/live-demo/sysadmin-ownership",
                "application/json",
                r#"{"ownershipProof":"x","extra":true}"#.to_string(),
            ),
            (
                "/api/auth/live-demo/sysadmin-ownership/complete",
                "application/json",
                r#"{"ownershipProof":"x"}"#.to_string(),
            ),
            (
                "/api/auth/live-demo/sysadmin-ownership/complete",
                "application/json",
                r#"{"ownershipProof":"x","extra":true}"#.to_string(),
            ),
            (
                "/api/auth/live-demo/sysadmin-ownership/complete",
                "application/json",
                oversized,
            ),
        ] {
            let response = app(Some(config()))
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("origin", ORIGIN)
                        .header("content-type", content_type)
                        .body(Body::from(body))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
            assert!(
                response
                    .headers()
                    .get_all(SET_COOKIE)
                    .iter()
                    .next()
                    .is_none()
            );
            assert_eq!(
                to_bytes(response.into_body(), 1_024)
                    .await
                    .unwrap()
                    .as_ref(),
                br#"{"error":"unavailable"}"#
            );
        }
    }

    #[test]
    fn proof_requires_canonical_32_byte_base64url() {
        let config = config();
        assert!(proof_matches(&proof(), &config));
        assert!(!proof_matches("Aw", &config));
        assert!(!proof_matches(&format!("{}=", proof()), &config));
    }
}
