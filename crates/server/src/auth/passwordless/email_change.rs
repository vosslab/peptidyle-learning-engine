//! Verified account-email replacement routes and session-proof rotation.

use super::*;

pub(super) async fn start_account_email_change<S>(
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
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response.into_response(),
    };
    let email = match AuthenticationEmail::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return invalid_account_email_request(),
    };
    let Some(network_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Network,
        &state
            .client_address_policy
            .rate_limit_identity(peer, &headers),
    ) else {
        return passwordless_unavailable();
    };
    let Some(principal_key) = state.rate_limit_issuer.key(
        AuthenticationRateLimitScope::Principal,
        account.user.as_uuid().as_bytes(),
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
                AuthenticationRateLimitScope::Principal,
                principal_key,
                PRINCIPAL_RATE_LIMIT_ATTEMPTS,
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
        Ok(RateLimitOutcome::Denied {
            retry_after_seconds,
        }) => {
            return account_email_change_rate_limited(retry_after_seconds);
        }
        Err(()) => return passwordless_unavailable(),
    }
    let email_secret = match RandomSecret::generate() {
        Ok(secret) => PasswordlessEmailSecret(secret.0),
        Err(error) => return super::super::auth_error_response(error),
    };
    let browser_binding = match RandomSecret::generate() {
        Ok(secret) => secret,
        Err(error) => return super::super::auth_error_response(error),
    };
    let command = BeginEmailAuthentication {
        id: match EmailChallengeId::generate() {
            Ok(id) => id,
            Err(_) => return passwordless_unavailable(),
        },
        token_hash: email_secret.hash(),
        browser_binding: BrowserBindingHash::compute(&browser_binding.0),
        email_rate_limit_key: email_key,
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

pub(super) async fn complete_account_email_change<S>(
    State(state): State<PasswordlessRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CompleteEmailChangeRequest>,
) -> Response
where
    S: AccountIdentityStore + AccountSessionStore + CourseRosterStore + SessionStore + 'static,
{
    let account = match authenticated_account(state.store.as_ref(), &headers).await {
        Ok(account) => account,
        Err(response) => return response.into_response(),
    };
    let Some(token) = RandomSecret::decode(&request.token) else {
        return authentication_rejected();
    };
    let Some(browser_binding) = cookie_secret(&headers, EMAIL_BINDING_COOKIE) else {
        return authentication_rejected();
    };
    let account_token = match RandomSecret::generate() {
        Ok(token) => token,
        Err(error) => return super::super::auth_error_response(error),
    };
    let account_lifetime = AccountSessionLifetime::from_seconds(ACCOUNT_SESSION_SECONDS)
        .expect("fifteen minutes is the account-session bound");
    match state
        .store
        .complete_email_change_and_revoke_user_sessions(CompleteEmailChangeAndRevokeUserSessions {
            authentication: CompleteEmailAuthentication {
                token_hash: EmailChallengeSecretHash::compute(&token.0),
                browser_binding: BrowserBindingHash::compute(&browser_binding.0),
                proposed_user: account.user,
                proposed_display_name: account.display_name,
            },
            session_token_hash: AccountSessionTokenHash::compute(&account_token.0),
            session_lifetime: account_lifetime,
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
            let Ok(account_cookie) = HeaderValue::from_str(&secret_cookie(
                ACCOUNT_SESSION_COOKIE,
                &account_token,
                ACCOUNT_SESSION_SECONDS,
                state.session_config,
            )) else {
                return passwordless_unavailable();
            };
            response.headers_mut().append(SET_COOKIE, account_cookie);
            no_store(response)
        }
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::Conflict) => {
            authentication_rejected()
        }
        Err(_) => passwordless_unavailable(),
    }
}
