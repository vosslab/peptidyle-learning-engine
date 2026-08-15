//! Passwordless email bootstrap and authenticated invitation redemption.

#[path = "passwordless/email_change.rs"]
mod email_change;
#[path = "passwordless/support.rs"]
mod support;

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    AccountIdentityStore, AccountPresentationPreference, AccountPresentationStore,
    AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash, AuthenticationEmail,
    AuthenticationRateLimitDecision, AuthenticationRateLimitKey, AuthenticationRateLimitPolicy,
    AuthenticationRateLimitScope, BeginEmailAuthentication, BrowserBindingHash,
    ClaimCourseInvitation, CompleteEmailAuthentication,
    CompleteEmailAuthenticationAndCreateSession, CompleteEmailChangeAndRevokeUserSessions,
    ConsumeAuthenticationRateLimit, CourseInvitationSecretHash, CourseRosterStore, Cursor,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
    NavigationReferenceStore, PageRequest, PageSize, SessionStore, StoreError, TenantContext,
};
use question_model::{CourseId, CourseMembershipRole, UserId, UserRole};
use serde::{Deserialize, Serialize};

use super::{ClientAddressPolicy, SessionConfig, issue_session, no_store, response_with_cookie};

#[cfg(test)]
use super::AuthError;

pub(super) use support::{
    RandomSecret, authenticated_account, authenticated_account_session, authentication_rejected,
    clear_account_authentication_cookies, clear_named_cookie, cookie_secret,
    passwordless_unavailable, revoke_presented_account_session, secret_cookie,
};
use support::{
    accepted_email_response, account_course_not_found, account_email_change_rate_limited,
    course_role_name, invalid_account_course_request, invalid_account_email_request,
    invitation_conflict, invitation_rejected,
};

pub(super) const SECRET_BYTES: usize = 32;
const EMAIL_CHALLENGE_SECONDS: u32 = 10 * 60;
pub(super) const ACCOUNT_SESSION_SECONDS: u32 = 15 * 60;
pub(super) const RATE_LIMIT_WINDOW_SECONDS: u32 = 15 * 60;
const EMAIL_RATE_LIMIT_ATTEMPTS: u32 = 5;
// Network is deliberately a coarse /24 (IPv4) or /56 (IPv6) budget.  A
// campus/NAT should sustain normal class traffic, while one prefix cannot use
// this low-cost endpoint as an unlimited email relay.
pub(super) const NETWORK_RATE_LIMIT_ATTEMPTS: u32 = 600;
const PRINCIPAL_RATE_LIMIT_ATTEMPTS: u32 = 12;
const SERVICE_RATE_LIMIT_ATTEMPTS: u32 = 4_000;
const EMAIL_DELIVERY_SERVICE_KEY: &[u8] = b"email-delivery-v1";
const MAX_PASSWORDLESS_BODY_BYTES: usize = 16 * 1_024;
const DEFAULT_ACCOUNT_COURSE_PAGE_SIZE: u16 = 50;
pub(super) const EMAIL_BINDING_COOKIE: &str = "ple_email_binding";
pub(super) const ACCOUNT_SESSION_COOKIE: &str = "ple_account_session";

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

    pub(super) fn key(
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
            AuthenticationRateLimitScope::Principal => b"principal\0",
            AuthenticationRateLimitScope::Service => b"service\0",
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
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
}

impl<S> Clone for PasswordlessRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            delivery: Arc::clone(&self.delivery),
            rate_limit_issuer: self.rate_limit_issuer.clone(),
            client_address_policy: self.client_address_policy.clone(),
            session_config: self.session_config,
        }
    }
}

pub fn passwordless_router<S>(
    store: Arc<S>,
    delivery: Arc<dyn PasswordlessEmailDelivery>,
    rate_limit_issuer: PasswordlessRateLimitIssuer,
    client_address_policy: ClientAddressPolicy,
    session_config: SessionConfig,
) -> Router
where
    S: AccountIdentityStore
        + AccountSessionStore
        + CourseRosterStore
        + NavigationReferenceStore
        + AccountPresentationStore
        + SessionStore
        + 'static,
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
            post(email_change::start_account_email_change::<S>),
        )
        .route(
            "/api/auth/account/email/complete",
            post(email_change::complete_account_email_change::<S>),
        )
        .route(
            "/api/course-invitations/redeem",
            post(redeem_course_invitation::<S>),
        )
        .route("/api/auth/account/courses", get(list_account_courses::<S>))
        .route(
            "/api/auth/account/presentation",
            get(get_account_presentation::<S>).put(save_account_presentation::<S>),
        )
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
            client_address_policy,
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
    course_public_id: question_model::CoursePublicId,
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
    course_public_id: question_model::CoursePublicId,
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<StartEmailRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    if !state.delivery.is_configured() {
        return passwordless_unavailable();
    }
    let email = match AuthenticationEmail::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return accepted_email_response(None, state.session_config),
    };
    let Some(network_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Network,
        &state
            .client_address_policy
            .rate_limit_identity(peer, &headers),
    ) else {
        return passwordless_unavailable();
    };
    let Some(email_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Email,
        email.normalized().as_bytes(),
    ) else {
        return passwordless_unavailable();
    };
    let Some(service_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Service,
        EMAIL_DELIVERY_SERVICE_KEY,
    ) else {
        return passwordless_unavailable();
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
                AuthenticationRateLimitScope::Email,
                email_key,
                EMAIL_RATE_LIMIT_ATTEMPTS,
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
        Ok(RateLimitOutcome::Allowed) => {}
        Ok(RateLimitOutcome::Denied { .. }) => {
            return accepted_email_response(None, state.session_config);
        }
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
        email_rate_limit_key: email_key,
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

pub(super) async fn consume_rate_limit<S>(
    store: &S,
    scope: AuthenticationRateLimitScope,
    key: AuthenticationRateLimitKey,
    maximum_attempts: u32,
) -> Result<bool, ()>
where
    S: AccountIdentityStore,
{
    Ok(matches!(
        consume_rate_limit_outcome(store, scope, key, maximum_attempts).await?,
        RateLimitOutcome::Allowed
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitOutcome {
    Allowed,
    Denied { retry_after_seconds: u32 },
}

async fn consume_rate_limits<S, const N: usize>(
    store: &S,
    limits: [(
        AuthenticationRateLimitScope,
        AuthenticationRateLimitKey,
        u32,
    ); N],
) -> Result<RateLimitOutcome, ()>
where
    S: AccountIdentityStore,
{
    for (scope, key, maximum_attempts) in limits {
        let outcome = consume_rate_limit_outcome(store, scope, key, maximum_attempts).await?;
        if !matches!(outcome, RateLimitOutcome::Allowed) {
            return Ok(outcome);
        }
    }
    Ok(RateLimitOutcome::Allowed)
}

async fn consume_rate_limit_outcome<S>(
    store: &S,
    scope: AuthenticationRateLimitScope,
    key: AuthenticationRateLimitKey,
    maximum_attempts: u32,
) -> Result<RateLimitOutcome, ()>
where
    S: AccountIdentityStore,
{
    let policy = AuthenticationRateLimitPolicy::new(maximum_attempts, RATE_LIMIT_WINDOW_SECONDS)
        .expect("fixed passwordless rate-limit policy is bounded");
    store
        .consume_authentication_rate_limit(ConsumeAuthenticationRateLimit { scope, key, policy })
        .await
        .map(|decision| match decision {
            AuthenticationRateLimitDecision::Allowed { .. } => RateLimitOutcome::Allowed,
            AuthenticationRateLimitDecision::Denied {
                retry_after_seconds,
            } => RateLimitOutcome::Denied {
                retry_after_seconds,
            },
        })
        .map_err(|_| ())
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
    S: AccountIdentityStore
        + AccountSessionStore
        + CourseRosterStore
        + NavigationReferenceStore
        + SessionStore
        + 'static,
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
    let mut roles = vec![UserRole::Student];
    roles.extend(account.platform_roles.iter().copied());
    let subject = match learning_data_access::SessionSubject::new(
        claimed.tenant,
        account.user,
        account.display_name,
        roles,
    ) {
        Ok(subject) => subject,
        Err(_) => return passwordless_unavailable(),
    };
    let issued = match issue_session(state.store.as_ref(), subject, state.session_config).await {
        Ok(issued) => issued,
        Err(error) => return super::auth_error_response(error),
    };
    let course_public_id = match state
        .store
        .course_public_id(
            TenantContext::from_authenticated_session(claimed.tenant),
            account.user,
            claimed.course,
        )
        .await
    {
        Ok(Some(public_id)) => public_id,
        Ok(None) | Err(_) => return passwordless_unavailable(),
    };
    response_with_cookie(
        StatusCode::OK,
        issued.set_cookie,
        ClaimedInvitationResponse {
            course_id: claimed.course,
            course_public_id,
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
    S: AccountIdentityStore
        + AccountSessionStore
        + CourseRosterStore
        + NavigationReferenceStore
        + SessionStore
        + 'static,
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
    let mut courses = Vec::with_capacity(page.items.len());
    for context in page.items {
        let public_id = match state
            .store
            .course_public_id(
                TenantContext::from_authenticated_session(context.tenant),
                account.user,
                context.course,
            )
            .await
        {
            Ok(Some(public_id)) => public_id,
            Ok(None) | Err(_) => return passwordless_unavailable(),
        };
        courses.push(AccountCourseResponse {
            course_id: context.course,
            course_public_id: public_id,
            title: context.title,
            role: course_role_name(context.role),
        });
    }
    no_store(
        Json(AccountCoursePageResponse {
            courses,
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_string()),
        })
        .into_response(),
    )
}

async fn get_account_presentation<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
) -> Response
where
    S: AccountIdentityStore
        + AccountPresentationStore
        + AccountSessionStore
        + CourseRosterStore
        + SessionStore
        + 'static,
{
    let account_session = match authenticated_account_session(state.store.as_ref(), &headers).await
    {
        Ok(account_session) => account_session,
        Err(response) => return response,
    };
    match state
        .store
        .account_presentation(account_session.token_hash)
        .await
    {
        Ok(preference) => no_store(Json(preference).into_response()),
        Err(_) => passwordless_unavailable(),
    }
}

async fn save_account_presentation<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(preference): Json<AccountPresentationPreference>,
) -> Response
where
    S: AccountIdentityStore
        + AccountPresentationStore
        + AccountSessionStore
        + CourseRosterStore
        + SessionStore
        + 'static,
{
    let account_session = match authenticated_account_session(state.store.as_ref(), &headers).await
    {
        Ok(account_session) => account_session,
        Err(response) => return response,
    };
    match state
        .store
        .save_account_presentation(account_session.token_hash, preference)
        .await
    {
        Ok(saved) => no_store(Json(saved).into_response()),
        Err(_) => passwordless_unavailable(),
    }
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
        CourseMembershipRole::Student => UserRole::Student,
        CourseMembershipRole::Instructor => UserRole::Instructor,
    };
    let mut roles = vec![user_role];
    roles.extend(account.platform_roles.iter().copied());
    let subject = match learning_data_access::SessionSubject::new(
        context.tenant,
        account.user,
        account.display_name,
        roles,
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

#[cfg(test)]
#[path = "passwordless/tests.rs"]
mod tests;
