//! Deployment-gated entry into ordinary account authentication for seeded demos.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{
    AccountIdentityStore, AccountSessionStore, AuthenticationRateLimitScope, SessionStore,
};
use question_model::UserId;
use serde::{Deserialize, Serialize};

use super::browser_boundary::origin_matches;
use super::passwordless::{
    NETWORK_RATE_LIMIT_ATTEMPTS, PasswordlessRateLimitIssuer, SERVICE_RATE_LIMIT_ATTEMPTS,
    consume_rate_limits, issue_account_session,
};
use super::{ClientAddressPolicy, SessionConfig, clear_session_cookie, no_store, revoke_session};

const SELECTOR_SERVICE_KEY: &[u8] = b"seeded-account-selector-v1";
const SELECTOR_PRINCIPAL_ATTEMPTS: u32 = 24;
const MAX_SELECTOR_BODY_BYTES: usize = 1_024;

/// Closed deployment selector keys. They are not product roles or identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SeededAccountPersona {
    ElenaInstructor,
    MaryStudent,
    JackStudent,
    AveryStudent,
    MorganSysadmin,
}

impl SeededAccountPersona {
    const ALL: [Self; 5] = [
        Self::ElenaInstructor,
        Self::MaryStudent,
        Self::JackStudent,
        Self::AveryStudent,
        Self::MorganSysadmin,
    ];

    const fn config_key(self) -> &'static str {
        match self {
            Self::ElenaInstructor => "elena_instructor",
            Self::MaryStudent => "mary_student",
            Self::JackStudent => "jack_student",
            Self::AveryStudent => "avery_student",
            Self::MorganSysadmin => "morgan_sysadmin",
        }
    }
}

/// Exact five-account mapping enabled only for a live-demo deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededAccountSelectorConfig {
    origin: Arc<str>,
    accounts: [UserId; 5],
}

impl SeededAccountSelectorConfig {
    /// Builds the closed mapping after deployment configuration validation.
    pub fn new(origin: Arc<str>, accounts: [UserId; 5]) -> Result<Self, String> {
        let duplicate_account = accounts
            .iter()
            .enumerate()
            .any(|(index, account)| accounts[index + 1..].contains(account));
        if origin.is_empty() || duplicate_account {
            return Err("live-demo account selector configuration is invalid".to_string());
        }
        Ok(Self { origin, accounts })
    }

    fn account(&self, persona: SeededAccountPersona) -> UserId {
        match persona {
            SeededAccountPersona::ElenaInstructor => self.accounts[0],
            SeededAccountPersona::MaryStudent => self.accounts[1],
            SeededAccountPersona::JackStudent => self.accounts[2],
            SeededAccountPersona::AveryStudent => self.accounts[3],
            SeededAccountPersona::MorganSysadmin => self.accounts[4],
        }
    }

    #[cfg(test)]
    pub(crate) fn contains_user(&self, user: UserId) -> bool {
        self.accounts.contains(&user)
    }
}

struct SelectorState<S> {
    store: Arc<S>,
    config: Option<SeededAccountSelectorConfig>,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
}

impl<S> Clone for SelectorState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            rate_limit_issuer: self.rate_limit_issuer.clone(),
            client_address_policy: self.client_address_policy.clone(),
            session_config: self.session_config,
        }
    }
}

impl<S> SelectorState<S> {
    fn disabled(
        store: Arc<S>,
        rate_limit_issuer: PasswordlessRateLimitIssuer,
        client_address_policy: ClientAddressPolicy,
        session_config: SessionConfig,
    ) -> Self {
        Self {
            store,
            config: None,
            rate_limit_issuer,
            client_address_policy,
            session_config,
        }
    }
}

/// Builds the selector routes. A missing configuration exposes no accounts.
pub fn seeded_account_selector_router<S>(
    store: Arc<S>,
    config: Option<SeededAccountSelectorConfig>,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
) -> Router
where
    S: AccountIdentityStore + AccountSessionStore + SessionStore + 'static,
{
    let state = match config {
        Some(config) => SelectorState {
            store,
            config: Some(config),
            rate_limit_issuer,
            client_address_policy,
            session_config,
        },
        None => SelectorState::disabled(
            store,
            rate_limit_issuer,
            client_address_policy,
            session_config,
        ),
    };
    Router::new()
        .route(
            "/api/auth/live-demo/accounts",
            get(list_seeded_accounts::<S>).post(select_seeded_account::<S>),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectSeededAccountRequest {
    persona: SeededAccountPersona,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeededAccountAvailabilityResponse {
    accounts: Vec<SeededAccountAvailability>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeededAccountAvailability {
    persona: SeededAccountPersona,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct SelectedSeededAccountResponse {
    authenticated: bool,
}

async fn list_seeded_accounts<S>(State(state): State<SelectorState<S>>) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + SessionStore + 'static,
{
    let Some(config) = &state.config else {
        return selector_unavailable();
    };
    let mut accounts = Vec::with_capacity(SeededAccountPersona::ALL.len());
    for persona in SeededAccountPersona::ALL {
        let account = match state.store.get_account(config.account(persona)).await {
            Ok(Some(account)) => account,
            Ok(None) | Err(_) => return selector_unavailable(),
        };
        accounts.push(SeededAccountAvailability {
            persona,
            display_name: account.display_name,
        });
    }
    no_store(Json(SeededAccountAvailabilityResponse { accounts }).into_response())
}

async fn select_seeded_account<S>(
    State(state): State<SelectorState<S>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + SessionStore + 'static,
{
    let Some(config) = &state.config else {
        return selector_unavailable();
    };
    let headers = request.headers().clone();
    if !origin_matches(&headers, &config.origin) {
        return selector_same_origin_required();
    }
    let request = match select_seeded_account_request(request).await {
        Some(request) => request,
        None => return selector_unavailable(),
    };
    let Some(network_key) = state.rate_limit_issuer.live_demo_key(
        AuthenticationRateLimitScope::Network,
        &state
            .client_address_policy
            .rate_limit_identity(peer, &headers),
    ) else {
        return selector_unavailable();
    };
    let Some(principal_key) = state.rate_limit_issuer.live_demo_key(
        AuthenticationRateLimitScope::Principal,
        request.persona.config_key().as_bytes(),
    ) else {
        return selector_unavailable();
    };
    let Some(service_key) = state
        .rate_limit_issuer
        .live_demo_key(AuthenticationRateLimitScope::Service, SELECTOR_SERVICE_KEY)
    else {
        return selector_unavailable();
    };
    match consume_rate_limits(
        state.store.as_ref(),
        [
            (
                AuthenticationRateLimitScope::Network,
                network_key,
                NETWORK_RATE_LIMIT_ATTEMPTS,
            ),
            (
                AuthenticationRateLimitScope::Principal,
                principal_key,
                SELECTOR_PRINCIPAL_ATTEMPTS,
            ),
            (
                AuthenticationRateLimitScope::Service,
                service_key,
                SERVICE_RATE_LIMIT_ATTEMPTS,
            ),
        ],
    )
    .await
    {
        Ok(super::passwordless::RateLimitOutcome::Allowed) => {}
        Ok(super::passwordless::RateLimitOutcome::Denied { .. }) => return selector_unavailable(),
        Err(()) => return selector_unavailable(),
    }
    let user = config.account(request.persona);
    match state.store.get_account(user).await {
        Ok(Some(_)) => {}
        Ok(None) | Err(_) => return selector_unavailable(),
    }
    let cookie = match issue_account_session(state.store.as_ref(), user, state.session_config).await
    {
        Ok(cookie) => cookie,
        Err(_) => return selector_unavailable(),
    };
    let Ok(cookie) = HeaderValue::from_str(&cookie) else {
        return selector_unavailable();
    };
    if super::passwordless::revoke_presented_account_session(state.store.as_ref(), &headers)
        .await
        .is_err()
    {
        return selector_unavailable();
    }
    if revoke_session(
        state.store.as_ref(),
        super::joined_cookie_header(&headers).as_deref(),
    )
    .await
    .is_err()
    {
        return selector_unavailable();
    }
    let Ok(clear_session) = HeaderValue::from_str(&clear_session_cookie(state.session_config))
    else {
        return selector_unavailable();
    };
    let mut response = Json(SelectedSeededAccountResponse {
        authenticated: true,
    })
    .into_response();
    response.headers_mut().append(SET_COOKIE, cookie);
    response.headers_mut().append(SET_COOKIE, clear_session);
    no_store(response)
}

/// Decodes the one closed selector request after the route has checked origin.
///
/// This route owns every malformed-input response so all parsing failures keep
/// the same generic JSON body and private cache policy as unavailable state.
async fn select_seeded_account_request(request: Request) -> Option<SelectSeededAccountRequest> {
    if !has_exact_json_content_type(request.headers()) {
        return None;
    }
    let body = to_bytes(request.into_body(), MAX_SELECTOR_BODY_BYTES)
        .await
        .ok()?;
    serde_json::from_slice(&body).ok()
}

fn has_exact_json_content_type(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    matches!(values.next(), Some(value) if value.as_bytes() == b"application/json")
        && values.next().is_none()
}

fn selector_unavailable() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "live demo account selection unavailable" })),
        )
            .into_response(),
    )
}

fn selector_same_origin_required() -> Response {
    no_store(
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "same-origin request required" })),
        )
            .into_response(),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Request, StatusCode};
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        AccountIdentityStore, AccountSessionStore, AccountSessionTokenHash, AuthenticationEmail,
        AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
        CompleteEmailAuthentication, EmailAuthenticationPurpose, EmailChallengeId,
        EmailChallengeLifetime, EmailChallengeSecretHash,
    };
    use question_model::UserId;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    const ORIGIN: &str = "https://demo.example.test";

    fn user(value: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(value))
    }

    fn selector_config() -> SeededAccountSelectorConfig {
        SeededAccountSelectorConfig::new(
            Arc::from(ORIGIN),
            [user(1), user(2), user(3), user(4), user(5)],
        )
        .expect("closed selector configuration")
    }

    fn session_config() -> SessionConfig {
        SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(60).expect("positive lifetime"),
            super::super::CookieTransport::FirstPartyHttps,
        )
    }

    async fn provision(store: &MemoryStore, user: UserId, display_name: &str) {
        let secret = [user.as_uuid().as_u128() as u8; 32];
        store
            .begin_email_authentication(BeginEmailAuthentication {
                id: EmailChallengeId::from_uuid(user.as_uuid()),
                token_hash: EmailChallengeSecretHash::compute(&secret),
                browser_binding: BrowserBindingHash::compute(b"test binding"),
                email_rate_limit_key: AuthenticationRateLimitKey::compute(b"test email"),
                email: AuthenticationEmail::parse(&format!("{user}@example.test"))
                    .expect("test email"),
                purpose: EmailAuthenticationPurpose::SignInOrRegister,
                lifetime: EmailChallengeLifetime::from_seconds(60).expect("positive lifetime"),
            })
            .await
            .expect("begin account provisioning");
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                token_hash: EmailChallengeSecretHash::compute(&secret),
                browser_binding: BrowserBindingHash::compute(b"test binding"),
                proposed_user: user,
                proposed_display_name: display_name.to_string(),
            })
            .await
            .expect("complete account provisioning");
    }

    async fn provision_selector_accounts(store: &MemoryStore) {
        for (user, name) in [
            (user(1), "Elena Instructor"),
            (user(2), "Mary Student"),
            (user(3), "Jack Student"),
            (user(4), "Avery Student"),
            (user(5), "Morgan Sysadmin"),
        ] {
            provision(store, user, name).await;
        }
    }

    fn app(store: Arc<MemoryStore>, config: Option<SeededAccountSelectorConfig>) -> Router {
        seeded_account_selector_router(
            store,
            config,
            PasswordlessRateLimitIssuer::from_server_secret([7; 32]),
            ClientAddressPolicy::direct(),
            session_config(),
        )
        .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 8], 443))))
    }

    fn select_request() -> axum::http::request::Builder {
        Request::builder()
            .method("POST")
            .uri("/api/auth/live-demo/accounts")
            .header("content-type", "application/json")
            .header("origin", ORIGIN)
    }

    fn account_cookie_pair(response: &Response) -> String {
        response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.starts_with("__Host-ple_account_session="))
            .and_then(|value| value.split(';').next())
            .expect("account session cookie")
            .replacen("__Host-", "", 1)
    }

    fn account_token_hash(cookie_pair: &str) -> AccountSessionTokenHash {
        let encoded = cookie_pair
            .split_once('=')
            .map(|(_, value)| value)
            .expect("account cookie pair");
        let secret = super::super::passwordless::RandomSecret::decode(encoded)
            .expect("account cookie secret");
        AccountSessionTokenHash::compute(&secret.0)
    }

    async fn assert_selector_unavailable(response: Response) {
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
        let body = to_bytes(response.into_body(), 8_192)
            .await
            .expect("error body");
        assert_eq!(
            body.as_ref(),
            br#"{"error":"live demo account selection unavailable"}"#
        );
    }

    #[tokio::test]
    async fn disabled_and_drifted_selector_do_not_expose_accounts() {
        let store = Arc::new(MemoryStore::default());
        let disabled = app(Arc::clone(&store), None)
            .oneshot(
                Request::builder()
                    .uri("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

        let drifted = app(store, Some(selector_config()))
            .oneshot(
                Request::builder()
                    .uri("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(drifted.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn availability_exposes_only_safe_persisted_labels() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(Arc::clone(&store), Some(selector_config()))
            .oneshot(
                Request::builder()
                    .uri("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let body = String::from_utf8(
            to_bytes(response.into_body(), 8_192)
                .await
                .expect("body")
                .to_vec(),
        )
        .expect("utf8");
        assert!(
            body.contains("Elena Instructor")
                && body.contains("Morgan Sysadmin")
                && body.contains("morganSysadmin")
                && !body.contains("role")
                && !body.contains("tenant")
        );
    }

    #[tokio::test]
    async fn selector_requires_exact_origin_and_closed_persona_decode() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let missing_origin = app(Arc::clone(&store), Some(selector_config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"persona":"morganSysadmin"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing_origin.status(), StatusCode::FORBIDDEN);

        let invalid = app(store, Some(selector_config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .header("origin", ORIGIN)
                    .body(Body::from(r#"{"persona":"sysadmin"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_selector_unavailable(invalid).await;
    }

    #[test]
    fn selector_config_rejects_non_adjacent_duplicate_accounts() {
        let result = SeededAccountSelectorConfig::new(
            Arc::from(ORIGIN),
            [user(1), user(2), user(1), user(4), user(5)],
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn selector_returns_generic_no_store_json_for_malformed_body() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(store, Some(selector_config()))
            .oneshot(
                select_request()
                    .body(Body::from("{"))
                    .expect("malformed selector request"),
            )
            .await
            .expect("response");
        assert_selector_unavailable(response).await;
    }

    #[tokio::test]
    async fn selector_returns_generic_no_store_json_for_unknown_field() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(store, Some(selector_config()))
            .oneshot(
                select_request()
                    .body(Body::from(r#"{"persona":"maryStudent","extra":true}"#))
                    .expect("unknown field request"),
            )
            .await
            .expect("response");
        assert_selector_unavailable(response).await;
    }

    #[tokio::test]
    async fn selector_returns_generic_no_store_json_for_wrong_content_type() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(store, Some(selector_config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/accounts")
                    .header("content-type", "text/plain")
                    .header("origin", ORIGIN)
                    .body(Body::from(r#"{"persona":"morganSysadmin"}"#))
                    .expect("wrong content type request"),
            )
            .await
            .expect("response");
        assert_selector_unavailable(response).await;
    }

    #[tokio::test]
    async fn selector_returns_generic_no_store_json_for_oversize_body() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(store, Some(selector_config()))
            .oneshot(
                select_request()
                    .body(Body::from(vec![b' '; MAX_SELECTOR_BODY_BYTES + 1]))
                    .expect("oversize selector request"),
            )
            .await
            .expect("response");
        assert_selector_unavailable(response).await;
    }

    #[tokio::test]
    async fn unavailable_rate_limit_service_fails_closed() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = seeded_account_selector_router(
            store,
            Some(selector_config()),
            PasswordlessRateLimitIssuer::unavailable(),
            ClientAddressPolicy::direct(),
            session_config(),
        )
        .layer(MockConnectInfo(SocketAddr::from(([192, 0, 2, 8], 443))))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/live-demo/accounts")
                .header("content-type", "application/json")
                .header("origin", ORIGIN)
                .body(Body::from(r#"{"persona":"maryStudent"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn selected_account_gets_only_account_proof_before_course_selection() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let response = app(Arc::clone(&store), Some(selector_config()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .header("origin", ORIGIN)
                    .body(Body::from(r#"{"persona":"morganSysadmin"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        let cookies = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|value| value.to_str().expect("cookie header"))
            .collect::<Vec<_>>();
        assert!(
            cookies
                .iter()
                .any(|value| value.starts_with("__Host-ple_account_session="))
                && cookies
                    .iter()
                    .any(|value| value.starts_with("__Host-ple_session=;"))
        );
        let account = store
            .resolve_account_session(account_token_hash(&account_cookie_pair(&response)))
            .await
            .expect("account session lookup");
        assert_eq!(account.map(|session| session.user), Some(user(5)));
    }

    #[tokio::test]
    async fn selection_replaces_only_the_presented_browser_account_proof() {
        let store = Arc::new(MemoryStore::default());
        provision_selector_accounts(store.as_ref()).await;
        let app = app(Arc::clone(&store), Some(selector_config()));
        let first = app
            .clone()
            .oneshot(
                select_request()
                    .body(Body::from(r#"{"persona":"maryStudent"}"#))
                    .expect("first selection"),
            )
            .await
            .expect("first response");
        let first_cookie = account_cookie_pair(&first);
        let first_hash = account_token_hash(&first_cookie);
        let other = app
            .clone()
            .oneshot(
                select_request()
                    .body(Body::from(r#"{"persona":"jackStudent"}"#))
                    .expect("other selection"),
            )
            .await
            .expect("other response");
        let other_hash = account_token_hash(&account_cookie_pair(&other));
        let replacement = app
            .oneshot(
                select_request()
                    .header("cookie", &first_cookie)
                    .body(Body::from(r#"{"persona":"elenaInstructor"}"#))
                    .expect("replacement selection"),
            )
            .await
            .expect("replacement response");
        let replacement_hash = account_token_hash(&account_cookie_pair(&replacement));
        assert_eq!(
            store
                .resolve_account_session(first_hash)
                .await
                .expect("first account lookup"),
            None
        );
        assert_eq!(
            store
                .resolve_account_session(replacement_hash)
                .await
                .expect("replacement account lookup")
                .map(|session| session.user),
            Some(user(1))
        );
        assert_eq!(
            store
                .resolve_account_session(other_hash)
                .await
                .expect("other account lookup")
                .map(|session| session.user),
            Some(user(3))
        );
    }
}
