//! Browser-bound passwordless email-code sign-in.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    AuthenticationCeremonyLifetime, AuthenticationCeremonyStore, AuthenticationEmail,
    EmailAuthenticationChallengeId, EmailAuthenticationCode, EmailAuthenticationStart,
    PasswordlessLoginMethod, SessionStore,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{SessionConfig, issue_passwordless_login, no_store},
    instructor_setup_email_delivery::InstructorSetupEmailDelivery,
};

use super::session_cookie::{
    PendingMfaToken, clear_pending_email_cookie, pending_email_cookie,
    presented_pending_email_token,
};

const EMAIL_CODE_LIFETIME_SECONDS: u32 = 10 * 60;
const MAX_EMAIL_CODE_BODY_BYTES: usize = 1024;

struct EmailCodeRouteState<C, S> {
    ceremonies: Arc<C>,
    sessions: Arc<S>,
    delivery: Arc<dyn InstructorSetupEmailDelivery>,
    config: SessionConfig,
}

impl<C, S> Clone for EmailCodeRouteState<C, S> {
    fn clone(&self) -> Self {
        Self {
            ceremonies: Arc::clone(&self.ceremonies),
            sessions: Arc::clone(&self.sessions),
            delivery: Arc::clone(&self.delivery),
            config: self.config,
        }
    }
}

/// Mounts the normal Student and Instructor email-code sign-in flow.
pub fn email_code_router<C, S>(
    ceremonies: Arc<C>,
    sessions: Arc<S>,
    delivery: Arc<dyn InstructorSetupEmailDelivery>,
    config: SessionConfig,
) -> Router
where
    C: AuthenticationCeremonyStore + 'static,
    S: SessionStore + 'static,
{
    Router::new()
        .route("/api/auth/email-code/start", post(start_handler::<C, S>))
        .route(
            "/api/auth/email-code/complete/{challenge_id}",
            post(complete_handler::<C, S>),
        )
        .with_state(EmailCodeRouteState {
            ceremonies,
            sessions,
            delivery,
            config,
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartRequest {
    email: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartedResponse {
    email_code_requested: bool,
    challenge_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteRequest {
    code: String,
}

async fn start_handler<C, S>(
    State(state): State<EmailCodeRouteState<C, S>>,
    request: Request,
) -> Response
where
    C: AuthenticationCeremonyStore + 'static,
    S: SessionStore + 'static,
{
    let Ok(body) = to_bytes(request.into_body(), MAX_EMAIL_CODE_BODY_BYTES).await else {
        return start_denied();
    };
    let Ok(request) = serde_json::from_slice::<StartRequest>(&body) else {
        return start_denied();
    };
    let Ok(email) = AuthenticationEmail::parse(&request.email) else {
        return start_denied();
    };
    let Ok(code) = EmailAuthenticationCode::generate() else {
        return unavailable();
    };
    let Ok(binding) = PendingMfaToken::generate() else {
        return unavailable();
    };
    let lifetime = AuthenticationCeremonyLifetime::from_seconds(EMAIL_CODE_LIFETIME_SECONDS)
        .expect("email-code lifetime is bounded by the Store contract");
    let started = match state
        .ceremonies
        .start_email_authentication_challenge(email.clone(), code.clone(), binding.hash(), lifetime)
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    // ASVS 6.2.1, 6.2.2, and 6.3.3: known and unknown addresses take the
    // same provider path and public result. A cover code is never persisted,
    // so only Issued can later create a session.
    let challenge_id = match started {
        EmailAuthenticationStart::Eligible(prepared) => {
            if state
                .delivery
                .send_email_authentication_code(&prepared.destination, &code)
                .await
                .is_err()
            {
                return unavailable();
            }
            let challenge = EmailAuthenticationChallengeId::from_uuid(Uuid::now_v7());
            match state
                .ceremonies
                .commit_email_authentication_challenge(
                    challenge,
                    email,
                    code.hash(),
                    binding.hash(),
                    lifetime,
                )
                .await
            {
                Ok(true) => challenge.as_uuid(),
                Ok(false) | Err(_) => return unavailable(),
            }
        }
        EmailAuthenticationStart::Covered(prepared) => {
            if state
                .delivery
                .send_email_authentication_code(&prepared.destination, &code)
                .await
                .is_err()
            {
                return unavailable();
            }
            Uuid::now_v7()
        }
        EmailAuthenticationStart::RateLimited => Uuid::now_v7(),
    };
    let Ok(cookie) =
        HeaderValue::from_str(&pending_email_cookie(&binding, state.config).to_string())
    else {
        return unavailable();
    };
    let mut response = Json(StartedResponse {
        email_code_requested: true,
        challenge_id,
    })
    .into_response();
    response.headers_mut().append(SET_COOKIE, cookie);
    no_store(response)
}

async fn complete_handler<C, S>(
    State(state): State<EmailCodeRouteState<C, S>>,
    request: Request,
) -> Response
where
    C: AuthenticationCeremonyStore + 'static,
    S: SessionStore + 'static,
{
    let (parts, body) = request.into_parts();
    let Some(binding) =
        presented_pending_email_token(joined_cookie_header(&parts.headers).as_deref())
    else {
        return completion_denied(state.config, true);
    };
    let Ok(challenge_id) = parts
        .uri
        .path()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .parse::<Uuid>()
    else {
        return completion_denied(state.config, true);
    };
    let Ok(body) = to_bytes(body, MAX_EMAIL_CODE_BODY_BYTES).await else {
        return completion_denied(state.config, false);
    };
    let Some(code) = serde_json::from_slice::<CompleteRequest>(&body)
        .ok()
        .and_then(|request| EmailAuthenticationCode::parse(&request.code))
    else {
        return completion_denied(state.config, false);
    };
    let primary = match state
        .ceremonies
        .consume_email_authentication_challenge(
            EmailAuthenticationChallengeId::from_uuid(challenge_id),
            code.hash(),
            binding.hash(),
        )
        .await
    {
        Ok(Some(primary)) => primary,
        Ok(None) => return completion_denied(state.config, true),
        Err(_) => return unavailable(),
    };
    let issued = match issue_passwordless_login(
        state.sessions.as_ref(),
        primary.account,
        primary.user_role,
        PasswordlessLoginMethod::EmailCode,
        state.config,
    )
    .await
    {
        Ok(Some(issued)) => issued,
        Ok(None) => return completion_denied(state.config, true),
        Err(_) => return unavailable(),
    };
    let (mut response, clear) = (
        Json(serde_json::json!({ "authenticated": true })).into_response(),
        clear_pending_email_cookie(state.config),
    );
    let (Ok(session), Ok(clear)) = (
        HeaderValue::from_str(&issued.set_cookie),
        HeaderValue::from_str(&clear.to_string()),
    ) else {
        return unavailable();
    };
    response.headers_mut().append(SET_COOKIE, session);
    response.headers_mut().append(SET_COOKIE, clear);
    no_store(response)
}

fn start_denied() -> Response {
    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"error":"email is invalid"})),
        )
            .into_response(),
    )
}

fn completion_denied(config: SessionConfig, clear: bool) -> Response {
    let mut response = no_store(
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"email code is invalid"})),
        )
            .into_response(),
    );
    if clear
        && let Ok(cookie) = HeaderValue::from_str(&clear_pending_email_cookie(config).to_string())
    {
        response.headers_mut().append(SET_COOKIE, cookie);
    }
    response
}

fn unavailable() -> Response {
    no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error":"authentication unavailable"})),
        )
            .into_response(),
    )
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}
