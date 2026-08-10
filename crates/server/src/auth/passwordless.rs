//! Passwordless email bootstrap and authenticated invitation redemption.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Query, State};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cookie::{Cookie, SameSite};
use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    AccountIdentityStore, AccountRecord, AccountSessionLifetime, AccountSessionStore,
    AccountSessionTokenHash, AuthenticationEmail, AuthenticationRateLimitDecision,
    AuthenticationRateLimitKey, AuthenticationRateLimitPolicy, AuthenticationRateLimitScope,
    BeginEmailAuthentication, BrowserBindingHash, ClaimCourseInvitation,
    CompleteEmailAuthentication, CompleteEmailAuthenticationAndCreateSession,
    ConsumeAuthenticationRateLimit, CourseInvitationSecretHash, CourseRosterStore, Cursor,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
    PageRequest, PageSize, SessionStore, StoreError,
};
use question_model::{CourseId, CourseRole, UserId, UserRole};
use serde::{Deserialize, Serialize};

use super::{
    AuthError, CookieTransport, SessionConfig, issue_session, no_store, response_with_cookie,
};

const SECRET_BYTES: usize = 32;
const EMAIL_CHALLENGE_SECONDS: u32 = 10 * 60;
pub(super) const ACCOUNT_SESSION_SECONDS: u32 = 15 * 60;
const RATE_LIMIT_WINDOW_SECONDS: u32 = 15 * 60;
const EMAIL_RATE_LIMIT_ATTEMPTS: u32 = 5;
const NETWORK_RATE_LIMIT_ATTEMPTS: u32 = 120;
const MAX_PASSWORDLESS_BODY_BYTES: usize = 16 * 1_024;
const DEFAULT_ACCOUNT_COURSE_PAGE_SIZE: u16 = 50;
const EMAIL_BINDING_COOKIE: &str = "ple_email_binding";
pub(super) const ACCOUNT_SESSION_COOKIE: &str = "ple_account_session";
const CLIENT_IP_HEADER: &str = "x-ple-client-ip";

/// Derives non-reversible database rate-limit keys from protected request data.
#[derive(Clone)]
pub struct PasswordlessRateLimitIssuer(Option<[u8; 32]>);

impl PasswordlessRateLimitIssuer {
    pub fn from_server_secret(secret: [u8; 32]) -> Self {
        Self(Some(secret))
    }

    pub fn unavailable() -> Self {
        Self(None)
    }

    fn key(
        &self,
        scope: AuthenticationRateLimitScope,
        value: &[u8],
    ) -> Option<AuthenticationRateLimitKey> {
        let secret = self.0?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .expect("HMAC-SHA256 accepts a 32-byte server secret");
        mac.update(b"ple-passwordless-rate-limit-v1\0");
        mac.update(match scope {
            AuthenticationRateLimitScope::Email => b"email\0",
            AuthenticationRateLimitScope::Network => b"network\0",
        });
        mac.update(&(value.len() as u64).to_be_bytes());
        mac.update(value);
        Some(AuthenticationRateLimitKey::from_bytes(
            mac.finalize().into_bytes().into(),
        ))
    }
}

impl std::fmt::Debug for PasswordlessRateLimitIssuer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PasswordlessRateLimitIssuer([redacted])")
    }
}

/// Redacted raw email-authentication secret used only by the mailer.
pub struct PasswordlessEmailSecret([u8; SECRET_BYTES]);

impl PasswordlessEmailSecret {
    pub fn encoded(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    fn hash(&self) -> EmailChallengeSecretHash {
        EmailChallengeSecretHash::compute(&self.0)
    }
}

impl std::fmt::Debug for PasswordlessEmailSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PasswordlessEmailSecret([redacted])")
    }
}

/// Coarse mail failure without recipient or provider detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordlessEmailDeliveryError {
    Unavailable,
}

/// Browser landing path selected by the server-owned challenge purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordlessEmailAction {
    SignIn,
    ChangeEmail,
}

/// Email-authentication delivery boundary shared by sign-in and email change.
#[async_trait]
pub trait PasswordlessEmailDelivery: Send + Sync {
    fn is_configured(&self) -> bool;

    async fn send_email_authentication(
        &self,
        email: &AuthenticationEmail,
        secret: &PasswordlessEmailSecret,
        action: PasswordlessEmailAction,
    ) -> Result<(), PasswordlessEmailDeliveryError>;
}

#[derive(Debug, Clone, Copy)]
pub struct UnavailablePasswordlessEmailDelivery;

#[async_trait]
impl PasswordlessEmailDelivery for UnavailablePasswordlessEmailDelivery {
    fn is_configured(&self) -> bool {
        false
    }

    async fn send_email_authentication(
        &self,
        _email: &AuthenticationEmail,
        _secret: &PasswordlessEmailSecret,
        _action: PasswordlessEmailAction,
    ) -> Result<(), PasswordlessEmailDeliveryError> {
        Err(PasswordlessEmailDeliveryError::Unavailable)
    }
}

struct PasswordlessRouteState<S> {
    store: Arc<S>,
    delivery: Arc<dyn PasswordlessEmailDelivery>,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    session_config: SessionConfig,
}

impl<S> Clone for PasswordlessRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            delivery: Arc::clone(&self.delivery),
            rate_limit_issuer: self.rate_limit_issuer.clone(),
            session_config: self.session_config,
        }
    }
}

pub fn passwordless_router<S>(
    store: Arc<S>,
    delivery: Arc<dyn PasswordlessEmailDelivery>,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    session_config: SessionConfig,
) -> Router
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/auth/passwordless/email/start",
            post(start_email_authentication::<S>),
        )
        .route(
            "/api/auth/passwordless/email/complete",
            post(complete_email_authentication::<S>),
        )
        .route(
            "/api/auth/account/email/start",
            post(start_account_email_change::<S>),
        )
        .route(
            "/api/auth/account/email/complete",
            post(complete_account_email_change::<S>),
        )
        .route(
            "/api/course-invitations/redeem",
            post(redeem_course_invitation::<S>),
        )
        .route("/api/auth/account/courses", get(list_account_courses::<S>))
        .route(
            "/api/auth/account/course-session",
            post(select_account_course::<S>),
        )
        .layer(axum::extract::DefaultBodyLimit::max(
            MAX_PASSWORDLESS_BODY_BYTES,
        ))
        .with_state(PasswordlessRouteState {
            store,
            delivery,
            rate_limit_issuer,
            session_config,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartEmailRequest {
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteEmailRequest {
    token: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteEmailChangeRequest {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RedeemInvitationRequest {
    invitation_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AccountCourseQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectAccountCourseRequest {
    course_id: CourseId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcceptedEmailResponse {
    accepted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountAuthenticatedResponse {
    authenticated: bool,
    passkey_enrollment_suggested: bool,
}

#[derive(Debug, Serialize)]
struct AccountEmailChangedResponse {
    changed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimedInvitationResponse {
    course_id: question_model::CourseId,
    membership_status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountCoursePageResponse {
    courses: Vec<AccountCourseResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountCourseResponse {
    course_id: CourseId,
    title: String,
    role: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedCourseSessionResponse {
    authenticated: bool,
    course_id: CourseId,
    role: &'static str,
}

async fn start_email_authentication<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<StartEmailRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    if !state.delivery.is_configured() {
        return passwordless_unavailable();
    }
    let Some(network_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Network,
        &network_rate_limit_identity(&headers),
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
        Ok(false) => return accepted_email_response(None, state.session_config),
        Err(()) => return passwordless_unavailable(),
    }
    let email = match AuthenticationEmail::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return accepted_email_response(None, state.session_config),
    };
    let Some(email_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Email,
        email.normalized().as_bytes(),
    ) else {
        return passwordless_unavailable();
    };
    match consume_rate_limit(
        state.store.as_ref(),
        AuthenticationRateLimitScope::Email,
        email_key,
        EMAIL_RATE_LIMIT_ATTEMPTS,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => return accepted_email_response(None, state.session_config),
        Err(()) => return passwordless_unavailable(),
    }
    let email_secret = match RandomSecret::generate() {
        Ok(secret) => PasswordlessEmailSecret(secret.0),
        Err(error) => return super::auth_error_response(error),
    };
    let browser_binding = match RandomSecret::generate() {
        Ok(secret) => secret,
        Err(error) => return super::auth_error_response(error),
    };
    let lifetime = EmailChallengeLifetime::from_seconds(EMAIL_CHALLENGE_SECONDS)
        .expect("ten minutes is the email challenge bound");
    let command = BeginEmailAuthentication {
        id: match EmailChallengeId::generate() {
            Ok(id) => id,
            Err(_) => return passwordless_unavailable(),
        },
        token_hash: email_secret.hash(),
        browser_binding: BrowserBindingHash::compute(&browser_binding.0),
        email: email.clone(),
        purpose: EmailAuthenticationPurpose::SignInOrRegister,
        lifetime,
    };
    if state
        .store
        .begin_email_authentication(command)
        .await
        .is_err()
    {
        return passwordless_unavailable();
    }
    if state
        .delivery
        .send_email_authentication(&email, &email_secret, PasswordlessEmailAction::SignIn)
        .await
        .is_err()
    {
        return passwordless_unavailable();
    }
    accepted_email_response(Some(browser_binding), state.session_config)
}

async fn start_account_email_change<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<StartEmailRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    if !state.delivery.is_configured() {
        return passwordless_unavailable();
    }
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let email = match AuthenticationEmail::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return invalid_account_email_request(),
    };
    let Some(email_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Email,
        email.normalized().as_bytes(),
    ) else {
        return passwordless_unavailable();
    };
    match consume_rate_limit(
        state.store.as_ref(),
        AuthenticationRateLimitScope::Email,
        email_key,
        EMAIL_RATE_LIMIT_ATTEMPTS,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => return accepted_email_response(None, state.session_config),
        Err(()) => return passwordless_unavailable(),
    }
    let email_secret = match RandomSecret::generate() {
        Ok(secret) => PasswordlessEmailSecret(secret.0),
        Err(error) => return super::auth_error_response(error),
    };
    let browser_binding = match RandomSecret::generate() {
        Ok(secret) => secret,
        Err(error) => return super::auth_error_response(error),
    };
    let command = BeginEmailAuthentication {
        id: match EmailChallengeId::generate() {
            Ok(id) => id,
            Err(_) => return passwordless_unavailable(),
        },
        token_hash: email_secret.hash(),
        browser_binding: BrowserBindingHash::compute(&browser_binding.0),
        email: email.clone(),
        purpose: EmailAuthenticationPurpose::ChangeEmail { user: account.user },
        lifetime: EmailChallengeLifetime::from_seconds(EMAIL_CHALLENGE_SECONDS)
            .expect("ten minutes is the email challenge bound"),
    };
    if state
        .store
        .begin_email_authentication(command)
        .await
        .is_err()
        || state
            .delivery
            .send_email_authentication(&email, &email_secret, PasswordlessEmailAction::ChangeEmail)
            .await
            .is_err()
    {
        return passwordless_unavailable();
    }
    accepted_email_response(Some(browser_binding), state.session_config)
}

async fn complete_account_email_change<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CompleteEmailChangeRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let Some(token) = RandomSecret::decode(&request.token) else {
        return authentication_rejected();
    };
    let Some(browser_binding) = cookie_secret(&headers, EMAIL_BINDING_COOKIE) else {
        return authentication_rejected();
    };
    match state
        .store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: EmailChallengeSecretHash::compute(&token.0),
            browser_binding: BrowserBindingHash::compute(&browser_binding.0),
            proposed_user: account.user,
            proposed_display_name: account.display_name,
        })
        .await
    {
        Ok(_) => {
            let mut response = Json(AccountEmailChangedResponse { changed: true }).into_response();
            let Ok(clear_binding) = HeaderValue::from_str(&clear_named_cookie(
                EMAIL_BINDING_COOKIE,
                state.session_config,
            )) else {
                return passwordless_unavailable();
            };
            response.headers_mut().append(SET_COOKIE, clear_binding);
            no_store(response)
        }
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::Conflict) => {
            authentication_rejected()
        }
        Err(_) => passwordless_unavailable(),
    }
}

async fn consume_rate_limit<S>(
    store: &S,
    scope: AuthenticationRateLimitScope,
    key: AuthenticationRateLimitKey,
    maximum_attempts: u32,
) -> Result<bool, ()>
where
    S: AccountIdentityStore,
{
    let policy = AuthenticationRateLimitPolicy::new(maximum_attempts, RATE_LIMIT_WINDOW_SECONDS)
        .expect("fixed passwordless rate-limit policy is bounded");
    store
        .consume_authentication_rate_limit(ConsumeAuthenticationRateLimit { scope, key, policy })
        .await
        .map(|decision| matches!(decision, AuthenticationRateLimitDecision::Allowed { .. }))
        .map_err(|_| ())
}

fn network_rate_limit_identity(headers: &HeaderMap) -> Vec<u8> {
    let mut values = headers.get_all(CLIENT_IP_HEADER).iter();
    let address = values
        .next()
        .filter(|_| values.next().is_none())
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<std::net::IpAddr>().ok());
    match address {
        Some(address) => address.to_string().into_bytes(),
        None => b"unknown-client-network".to_vec(),
    }
}

async fn complete_email_authentication<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CompleteEmailRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let token = match RandomSecret::decode(&request.token) {
        Some(token) => token,
        None => return authentication_rejected(),
    };
    let browser_binding = match cookie_secret(&headers, EMAIL_BINDING_COOKIE) {
        Some(binding) => binding,
        None => return authentication_rejected(),
    };
    let account_token = match RandomSecret::generate() {
        Ok(token) => token,
        Err(error) => return super::auth_error_response(error),
    };
    let account_lifetime = AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
        .expect("fifteen minutes is the account-session bound");
    let completed = match state
        .store
        .complete_email_authentication_and_create_session(
            CompleteEmailAuthenticationAndCreateSession {
                authentication: CompleteEmailAuthentication {
                    token_hash: EmailChallengeSecretHash::compute(&token.0),
                    browser_binding: BrowserBindingHash::compute(&browser_binding.0),
                    proposed_user: UserId::generate(),
                    proposed_display_name: request.display_name,
                },
                session_token_hash: AccountSessionTokenHash::compute(&account_token.0),
                session_lifetime: account_lifetime,
            },
        )
        .await
    {
        Ok(completed) => completed,
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::Conflict) => {
            return authentication_rejected();
        }
        Err(_) => return passwordless_unavailable(),
    };
    let passkey_enrollment_suggested = match state
        .store
        .list_active_passkeys(completed.authentication.account.user)
        .await
    {
        Ok(passkeys) => passkeys.is_empty(),
        Err(_) => return passwordless_unavailable(),
    };
    let mut response = Json(AccountAuthenticatedResponse {
        authenticated: true,
        passkey_enrollment_suggested,
    })
    .into_response();
    let account_cookie = secret_cookie(
        ACCOUNT_SESSION_COOKIE,
        &account_token,
        ACCOUNT_SESSION_SECONDS,
        state.session_config,
    );
    let clear_binding = clear_named_cookie(EMAIL_BINDING_COOKIE, state.session_config);
    let Ok(account_cookie) = HeaderValue::from_str(&account_cookie) else {
        return passwordless_unavailable();
    };
    let Ok(clear_binding) = HeaderValue::from_str(&clear_binding) else {
        return passwordless_unavailable();
    };
    response.headers_mut().append(SET_COOKIE, account_cookie);
    response.headers_mut().append(SET_COOKIE, clear_binding);
    no_store(response)
}

async fn redeem_course_invitation<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<RedeemInvitationRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let invitation_token = match RandomSecret::decode(&request.invitation_token) {
        Some(token) => token,
        None => return invitation_rejected(),
    };
    let claimed = match state
        .store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash: CourseInvitationSecretHash::compute(&invitation_token.0),
            user: account.user,
            verified_email: account.email,
            display_name: account.display_name.clone(),
        })
        .await
    {
        Ok(claimed) => claimed,
        Err(StoreError::NotFound | StoreError::Forbidden) => return invitation_rejected(),
        Err(StoreError::Conflict | StoreError::AlreadyExists) => {
            return invitation_conflict();
        }
        Err(_) => return passwordless_unavailable(),
    };
    let subject = match learning_data_access::SessionSubject::new(
        claimed.tenant,
        account.user,
        account.display_name,
        vec![UserRole::Student],
    ) {
        Ok(subject) => subject,
        Err(_) => return passwordless_unavailable(),
    };
    let issued = match issue_session(state.store.as_ref(), subject, state.session_config).await {
        Ok(issued) => issued,
        Err(error) => return super::auth_error_response(error),
    };
    response_with_cookie(
        StatusCode::OK,
        issued.set_cookie,
        ClaimedInvitationResponse {
            course_id: claimed.course,
            membership_status: "active",
        },
    )
}

async fn list_account_courses<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Query(query): Query<AccountCourseQuery>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let size = match PageSize::new(query.page_size.unwrap_or(DEFAULT_ACCOUNT_COURSE_PAGE_SIZE)) {
        Ok(size) => size,
        Err(_) => return invalid_account_course_request(),
    };
    let page = match query.cursor {
        Some(cursor) => match Cursor::parse(cursor) {
            Ok(cursor) => PageRequest::after(cursor, size),
            Err(_) => return invalid_account_course_request(),
        },
        None => PageRequest::first(size),
    };
    let page = match state
        .store
        .list_account_course_contexts(account.user, page)
        .await
    {
        Ok(page) => page,
        Err(StoreError::InvalidRecord(_)) => return invalid_account_course_request(),
        Err(_) => return passwordless_unavailable(),
    };
    no_store(
        Json(AccountCoursePageResponse {
            courses: page
                .items
                .into_iter()
                .map(|context| AccountCourseResponse {
                    course_id: context.course,
                    title: context.title,
                    role: course_role_name(context.role),
                })
                .collect(),
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_string()),
        })
        .into_response(),
    )
}

async fn select_account_course<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<SelectAccountCourseRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response,
    };
    let context = match state
        .store
        .resolve_account_course_context(account.user, request.course_id)
        .await
    {
        Ok(Some(context)) => context,
        Ok(None) => return account_course_not_found(),
        Err(_) => return passwordless_unavailable(),
    };
    let user_role = match context.role {
        CourseRole::Student => UserRole::Student,
        CourseRole::Instructor => UserRole::Instructor,
        CourseRole::Administrator => UserRole::Administrator,
    };
    let subject = match learning_data_access::SessionSubject::new(
        context.tenant,
        account.user,
        account.display_name,
        vec![user_role],
    ) {
        Ok(subject) => subject,
        Err(_) => return passwordless_unavailable(),
    };
    let issued = match issue_session(state.store.as_ref(), subject, state.session_config).await {
        Ok(issued) => issued,
        Err(error) => return super::auth_error_response(error),
    };
    response_with_cookie(
        StatusCode::OK,
        issued.set_cookie,
        SelectedCourseSessionResponse {
            authenticated: true,
            course_id: context.course,
            role: course_role_name(context.role),
        },
    )
}

pub(super) async fn authenticated_account<S>(
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

fn course_role_name(role: CourseRole) -> &'static str {
    match role {
        CourseRole::Student => "student",
        CourseRole::Instructor => "instructor",
        CourseRole::Administrator => "administrator",
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct RandomSecret(pub(super) [u8; SECRET_BYTES]);

impl RandomSecret {
    pub(super) fn generate() -> Result<Self, AuthError> {
        let mut value = [0_u8; SECRET_BYTES];
        getrandom::fill(&mut value).map_err(AuthError::Randomness)?;
        Ok(Self(value))
    }

    fn decode(value: &str) -> Option<Self> {
        if value.len() != 43 {
            return None;
        }
        let decoded: [u8; SECRET_BYTES] = URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()?;
        (URL_SAFE_NO_PAD.encode(decoded) == value).then_some(Self(decoded))
    }

    fn encoded(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl std::fmt::Debug for RandomSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RandomSecret([redacted])")
    }
}

pub(super) fn cookie_secret(headers: &HeaderMap, name: &str) -> Option<RandomSecret> {
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

pub(super) fn secret_cookie(
    name: &'static str,
    secret: &RandomSecret,
    seconds: u32,
    config: SessionConfig,
) -> String {
    Cookie::build((name, secret.encoded()))
        .path("/")
        .http_only(true)
        .secure(config.secure())
        .same_site(match config.transport() {
            CookieTransport::EmbeddedHttps => SameSite::None,
            CookieTransport::FirstPartyHttps | CookieTransport::LocalHttp => SameSite::Lax,
        })
        .max_age(cookie::time::Duration::seconds(i64::from(seconds)))
        .build()
        .to_string()
}

pub(super) fn clear_named_cookie(name: &'static str, config: SessionConfig) -> String {
    Cookie::build((name, ""))
        .path("/")
        .http_only(true)
        .secure(config.secure())
        .same_site(match config.transport() {
            CookieTransport::EmbeddedHttps => SameSite::None,
            CookieTransport::FirstPartyHttps | CookieTransport::LocalHttp => SameSite::Lax,
        })
        .max_age(cookie::time::Duration::ZERO)
        .build()
        .to_string()
}

fn accepted_email_response(binding: Option<RandomSecret>, config: SessionConfig) -> Response {
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

pub(super) fn authentication_rejected() -> Response {
    no_store(
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "authentication required" })),
        )
            .into_response(),
    )
}

fn invitation_rejected() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "course invitation not found" })),
        )
            .into_response(),
    )
}

fn invitation_conflict() -> Response {
    no_store(
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "course invitation cannot be claimed" })),
        )
            .into_response(),
    )
}

fn invalid_account_course_request() -> Response {
    no_store(
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "account course request is invalid" })),
        )
            .into_response(),
    )
}

fn invalid_account_email_request() -> Response {
    no_store(
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "account email request is invalid" })),
        )
            .into_response(),
    )
}

fn account_course_not_found() -> Response {
    no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "course not found" })),
        )
            .into_response(),
    )
}

pub(super) fn passwordless_unavailable() -> Response {
    no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "passwordless authentication unavailable" })),
        )
            .into_response(),
    )
}

#[cfg(test)]
#[path = "passwordless/tests.rs"]
mod tests;
