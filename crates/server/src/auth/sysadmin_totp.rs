//! Server-side completion of the Sysadmin TOTP second factor.
//!
//! This module intentionally does not provision a seed or create a demo-only
//! authentication mode. A trusted primary ceremony supplies the database
//! derived Account and Product Role; Sysadmin is the only role redirected to
//! this pending-MFA boundary.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{FromRequestParts, Path, Request, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, SessionStore, StoreError,
    SysadminTotpAttestationId, SysadminTotpCounter, SysadminTotpStore,
};
use question_model::ProductRole;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use super::{
    AuthError, IssuedSession, SessionConfig, TOKEN_GENERATION_ATTEMPTS, no_store,
    session_cookie::{
        PendingMfaToken, SessionToken, clear_pending_mfa_cookie, pending_mfa_cookie,
        presented_pending_mfa_token, session_cookie,
    },
};

const TOTP_PERIOD_SECONDS: i64 = 30;
const TOTP_DIGITS: u32 = 6;
const TOTP_ALLOWED_COUNTER_SKEW: i64 = 1;
const PENDING_MFA_LIFETIME_SECONDS: u32 = 5 * 60;
const MAX_TOTP_COMPLETION_BODY_BYTES: usize = 1024;

/// Browser-safe opaque pending-MFA response. It contains no Account, role,
/// seed, verification code, or session credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysadminTotpPendingResponse {
    /// Stable response discriminator used by the browser completion boundary.
    pub pending_mfa: bool,
    /// Opaque pending-attestation identity; the separate HttpOnly cookie binds it.
    pub attestation_id: Uuid,
}

/// Result of a trusted primary authentication ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimaryAuthenticationOutcome {
    /// A Student or Instructor session follows the ordinary session path.
    Authenticated(IssuedSession),
    /// Sysadmin must complete TOTP before any session or session cookie exists.
    PendingSysadminTotp {
        /// Browser-safe completion response.
        response: SysadminTotpPendingResponse,
        /// Host-only Secure HttpOnly binding cookie, sent only via Set-Cookie.
        set_cookie: String,
    },
}

/// Creates the ordinary session for non-Sysadmin primary authentication, or a
/// database-bound pending TOTP attestation for a Sysadmin primary result.
///
/// ASVS 2.2.1--2.2.3 and 2.3.1: only the trusted, database-derived primary
/// result controls this branch. Request data cannot nominate a role or skip
/// the second factor.
pub async fn establish_primary_authentication<S>(
    store: &S,
    primary: AuthenticatedAccount,
    config: SessionConfig,
) -> Result<PrimaryAuthenticationOutcome, AuthError>
where
    S: SessionStore + SysadminTotpStore,
{
    if primary.product_role != ProductRole::Sysadmin {
        return super::issue_session(store, primary.account, config)
            .await
            .map(PrimaryAuthenticationOutcome::Authenticated);
    }
    let binding = PendingMfaToken::generate().map_err(AuthError::Randomness)?;
    let lifetime = AuthenticationCeremonyLifetime::from_seconds(PENDING_MFA_LIFETIME_SECONDS)
        .expect("pending MFA lifetime is within the Store contract");
    let attestation = store
        .create_pending_sysadmin_totp_attestation(primary.account, binding.hash(), lifetime)
        .await
        .map_err(|error| AuthError::Unavailable(error.to_string()))?
        .ok_or_else(|| AuthError::Unavailable("Sysadmin MFA cannot start".to_string()))?;
    Ok(PrimaryAuthenticationOutcome::PendingSysadminTotp {
        response: SysadminTotpPendingResponse {
            pending_mfa: true,
            attestation_id: attestation.as_uuid(),
        },
        set_cookie: pending_mfa_cookie(&binding, config).to_string(),
    })
}

/// Closed, bounded completion request. The seed and code never enter a loggable
/// response or persistence object.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SysadminTotpCompletionRequest {
    /// Exactly six ASCII decimal digits, checked at this trusted boundary.
    pub code: String,
}

struct SysadminTotpRouteState<S> {
    store: Arc<S>,
    config: SessionConfig,
}

impl<S> Clone for SysadminTotpRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            config: self.config,
        }
    }
}

/// Mounts only the second-factor completion route. The primary-authentication
/// owner invokes [`establish_primary_authentication`] and adds its returned
/// pending response/cookie; it never issues a Sysadmin session directly.
pub fn sysadmin_totp_router<S>(store: Arc<S>, config: SessionConfig) -> Router
where
    S: SessionStore + SysadminTotpStore + 'static,
{
    Router::new()
        .route(
            "/api/auth/sysadmin-totp/complete/{attestation_id}",
            post(complete_handler::<S>),
        )
        .with_state(SysadminTotpRouteState { store, config })
}

async fn complete_handler<S>(
    State(state): State<SysadminTotpRouteState<S>>,
    request: Request,
) -> Response
where
    S: SessionStore + SysadminTotpStore + 'static,
{
    let (mut parts, body) = request.into_parts();
    let Some(binding) =
        presented_pending_mfa_token(joined_cookie_header(&parts.headers).as_deref())
    else {
        return completion_denied(state.config, true);
    };
    let attestation = match Path::<Uuid>::from_request_parts(&mut parts, &state).await {
        Ok(Path(attestation)) => SysadminTotpAttestationId::from_uuid(attestation),
        Err(_) => return completion_denied(state.config, true),
    };
    // ASVS 2.3.1 and 2.4.1: a valid browser-bound pending ceremony consumes
    // its shared, database-authoritative attempt budget before this handler
    // inspects even malformed JSON or a malformed code.
    let reservation = match state
        .store
        .reserve_sysadmin_totp_verification_attempt(attestation, binding.hash())
        .await
    {
        Ok(Some(reservation)) => reservation,
        Ok(None) => return completion_denied(state.config, true),
        Err(_) => return completion_unavailable(),
    };
    let request = match to_bytes(body, MAX_TOTP_COMPLETION_BODY_BYTES)
        .await
        .ok()
        .and_then(|body| serde_json::from_slice::<SysadminTotpCompletionRequest>(&body).ok())
    {
        Some(request) => request,
        None => return completion_denied(state.config, false),
    };
    let Ok(code) = TotpCode::parse(&request.code) else {
        return completion_denied(state.config, false);
    };
    let now_counter = match current_counter() {
        Some(counter) => counter,
        None => return completion_unavailable(),
    };
    let result = complete_reserved_sysadmin_totp(
        state.store.as_ref(),
        attestation,
        binding.hash(),
        reservation.account,
        code,
        now_counter,
        state.config,
    )
    .await;
    match result {
        Ok(issued) => {
            let Ok(cookie) = HeaderValue::from_str(&issued.set_cookie) else {
                return completion_unavailable();
            };
            let mut response = Json(serde_json::json!({ "authenticated": true })).into_response();
            response.headers_mut().append(SET_COOKIE, cookie);
            let Ok(clear_cookie) =
                HeaderValue::from_str(&clear_pending_mfa_cookie(state.config).to_string())
            else {
                return completion_unavailable();
            };
            response.headers_mut().append(SET_COOKIE, clear_cookie);
            no_store(response)
        }
        Err(TotpCompletionError::InvalidCode) => completion_denied(state.config, false),
        Err(TotpCompletionError::TerminalDenied) => completion_denied(state.config, true),
        Err(TotpCompletionError::Unavailable) => completion_unavailable(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TotpCompletionError {
    InvalidCode,
    TerminalDenied,
    Unavailable,
}

async fn complete_reserved_sysadmin_totp<S>(
    store: &S,
    attestation: SysadminTotpAttestationId,
    binding_hash: learning_data_access::AuthenticationSecretHash,
    reserved_account: question_model::AccountId,
    code: TotpCode,
    current: SysadminTotpCounter,
    config: SessionConfig,
) -> Result<IssuedSession, TotpCompletionError>
where
    S: SessionStore + SysadminTotpStore + ?Sized,
{
    let pending = store
        .load_pending_sysadmin_totp_attestation(attestation, binding_hash)
        .await
        .map_err(|_| TotpCompletionError::Unavailable)?
        .ok_or(TotpCompletionError::TerminalDenied)?;
    if pending.account != reserved_account {
        return Err(TotpCompletionError::Unavailable);
    }
    let counter =
        matching_counter(&pending.seed, code, current).ok_or(TotpCompletionError::InvalidCode)?;
    for _ in 0..TOKEN_GENERATION_ATTEMPTS {
        let token = SessionToken::generate().map_err(|_| TotpCompletionError::Unavailable)?;
        match store
            .consume_sysadmin_totp_attestation_into_session(
                attestation,
                binding_hash,
                counter,
                token.hash(),
                config.lifetime(),
            )
            .await
        {
            Ok(Some(record)) => {
                if record.product_role != ProductRole::Sysadmin || record.account != pending.account
                {
                    return Err(TotpCompletionError::Unavailable);
                }
                return Ok(IssuedSession {
                    record,
                    set_cookie: session_cookie(&token, config).to_string(),
                });
            }
            Ok(None) => return Err(TotpCompletionError::TerminalDenied),
            Err(StoreError::AlreadyExists) => continue,
            Err(_) => return Err(TotpCompletionError::Unavailable),
        }
    }
    Err(TotpCompletionError::Unavailable)
}

#[derive(Clone, Copy)]
struct TotpCode([u8; TOTP_DIGITS as usize]);

impl TotpCode {
    /// ASVS 2.2.1 and 2.2.2: accept only the fixed TOTP presentation at the
    /// trusted server boundary, before any persistence call.
    fn parse(value: &str) -> Result<Self, ()> {
        let bytes = value.as_bytes();
        let digits: [u8; TOTP_DIGITS as usize] = bytes.try_into().map_err(|_| ())?;
        if digits.iter().all(u8::is_ascii_digit) {
            Ok(Self(digits))
        } else {
            Err(())
        }
    }
}

fn current_counter() -> Option<SysadminTotpCounter> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let seconds = i64::try_from(seconds).ok()?;
    SysadminTotpCounter::from_server_counter(seconds / TOTP_PERIOD_SECONDS)
}

fn matching_counter(
    seed: &learning_data_access::SysadminTotpSeed,
    provided: TotpCode,
    current: SysadminTotpCounter,
) -> Option<SysadminTotpCounter> {
    let current = current.as_i64();
    let mut matched = None;
    // Evaluate every permitted counter before selecting one, so an otherwise
    // valid code does not make the server exit early (ASVS 2.2.1).
    for offset in -TOTP_ALLOWED_COUNTER_SKEW..=TOTP_ALLOWED_COUNTER_SKEW {
        let Some(candidate) = SysadminTotpCounter::from_server_counter(current + offset) else {
            continue;
        };
        let expected = hotp(seed.as_bytes(), candidate.as_i64());
        let is_match = provided.0.ct_eq(&expected).unwrap_u8() == 1;
        if is_match && matched.is_none() {
            matched = Some(candidate);
        }
    }
    matched
}

fn hotp(seed: &[u8], counter: i64) -> [u8; TOTP_DIGITS as usize] {
    type HmacSha1 = Hmac<Sha1>;
    let mut mac = HmacSha1::new_from_slice(seed).expect("TOTP seed length is bounded and valid");
    mac.update(&(counter as u64).to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[digest.len() - 1] & 0x0f);
    let binary = (u32::from(digest[offset] & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);
    let value = binary % 10_u32.pow(TOTP_DIGITS);
    let mut output = [b'0'; TOTP_DIGITS as usize];
    let mut divisor = 100_000;
    for digit in &mut output {
        *digit = b'0' + ((value / divisor) % 10) as u8;
        divisor /= 10;
    }
    output
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn completion_denied(config: SessionConfig, clear_pending: bool) -> Response {
    let mut response = no_store(
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "MFA verification failed" })),
        )
            .into_response(),
    );
    if clear_pending
        && let Ok(cookie) = HeaderValue::from_str(&clear_pending_mfa_cookie(config).to_string())
    {
        response.headers_mut().append(SET_COOKIE, cookie);
    }
    response
}

fn completion_unavailable() -> Response {
    no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "MFA unavailable" })),
        )
            .into_response(),
    )
}
