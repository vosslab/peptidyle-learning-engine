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

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use learning_data_access::{
        AuthenticatedAccount, AuthenticationSecretHash, EmailAuthenticationChallengeId,
        EmailAuthenticationStart, PreparedEmailAuthentication, SessionId, SessionLifetime,
        SessionRecord, SessionTokenHash, StoreError,
    };
    use question_model::{AccountId, Timestamp, UserRole};
    use tower::ServiceExt;

    use super::*;

    fn account() -> AccountId {
        AccountId::from_debug_serial(63)
    }

    #[derive(Default)]
    struct CeremonyMemory {
        record: Mutex<
            Option<(
                EmailAuthenticationChallengeId,
                AuthenticationSecretHash,
                AuthenticationSecretHash,
            )>,
        >,
        expired: bool,
    }

    #[async_trait]
    impl AuthenticationCeremonyStore for CeremonyMemory {
        async fn start_email_authentication_challenge(
            &self,
            email: AuthenticationEmail,
            code: EmailAuthenticationCode,
            binding: AuthenticationSecretHash,
            _: AuthenticationCeremonyLifetime,
        ) -> Result<EmailAuthenticationStart, StoreError> {
            let _ = (code, binding);
            Ok(EmailAuthenticationStart::Eligible(
                PreparedEmailAuthentication { destination: email },
            ))
        }

        async fn commit_email_authentication_challenge(
            &self,
            challenge: EmailAuthenticationChallengeId,
            _: AuthenticationEmail,
            code: AuthenticationSecretHash,
            binding: AuthenticationSecretHash,
            _: AuthenticationCeremonyLifetime,
        ) -> Result<bool, StoreError> {
            *self.record.lock().expect("record") = Some((challenge, code, binding));
            Ok(true)
        }

        async fn consume_email_authentication_challenge(
            &self,
            challenge: EmailAuthenticationChallengeId,
            code: AuthenticationSecretHash,
            binding: AuthenticationSecretHash,
        ) -> Result<Option<AuthenticatedAccount>, StoreError> {
            if self.expired {
                return Ok(None);
            }
            let mut record = self.record.lock().expect("record");
            let matches = record.as_ref().is_some_and(|stored| {
                stored.0 == challenge && stored.1 == code && stored.2 == binding
            });
            if !matches {
                return Ok(None);
            }
            *record = None;
            Ok(Some(AuthenticatedAccount {
                account: account(),
                user_role: UserRole::Instructor,
            }))
        }

        async fn authenticate_passkey(
            &self,
            _: learning_data_access::PasskeyCeremonyId,
            _: AuthenticationSecretHash,
            _: AuthenticationSecretHash,
        ) -> Result<Option<AuthenticatedAccount>, StoreError> {
            Ok(None)
        }
    }

    #[derive(Default)]
    struct SessionMemory(Mutex<BTreeMap<SessionTokenHash, SessionRecord>>);

    #[async_trait]
    impl SessionStore for SessionMemory {
        async fn create_session(
            &self,
            token_hash: SessionTokenHash,
            account: AccountId,
            lifetime: SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            let record = SessionRecord {
                id: SessionId::generate()?,
                token_hash,
                account,
                user_role: UserRole::Instructor,
                created_at: Timestamp::from_unix_millis(0),
                expires_at: Timestamp::from_unix_millis(i64::from(lifetime.as_seconds()) * 1000),
            };
            self.0
                .lock()
                .expect("sessions")
                .insert(token_hash, record.clone());
            Ok(record)
        }
        async fn resolve_session(
            &self,
            token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            Ok(self.0.lock().expect("sessions").get(&token_hash).cloned())
        }
        async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
            self.0.lock().expect("sessions").remove(&token_hash);
            Ok(())
        }
    }

    #[derive(Default)]
    struct DeliveryMemory {
        code: Mutex<Option<EmailAuthenticationCode>>,
        destinations: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait]
    impl InstructorSetupEmailDelivery for DeliveryMemory {
        async fn send_setup_email(
            &self,
            _: &AuthenticationEmail,
        ) -> Result<(), crate::instructor_setup_email_delivery::InstructorSetupEmailFailure>
        {
            Ok(())
        }
        async fn send_email_authentication_code(
            &self,
            destination: &AuthenticationEmail,
            code: &EmailAuthenticationCode,
        ) -> Result<(), crate::instructor_setup_email_delivery::InstructorSetupEmailFailure>
        {
            if self.fail {
                return Err(crate::instructor_setup_email_delivery::InstructorSetupEmailFailure::NotConfigured);
            }
            self.destinations
                .lock()
                .expect("destinations")
                .push(destination.normalized().to_string());
            *self.code.lock().expect("code") = Some(code.clone());
            Ok(())
        }
    }

    fn config() -> SessionConfig {
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        )
    }

    #[tokio::test]
    async fn email_code_requires_its_browser_binding_then_issues_one_instructor_session() {
        let ceremonies = Arc::new(CeremonyMemory::default());
        let sessions = Arc::new(SessionMemory::default());
        let delivery = Arc::new(DeliveryMemory::default());
        let router = email_code_router(
            Arc::clone(&ceremonies),
            Arc::clone(&sessions),
            Arc::clone(&delivery) as Arc<dyn InstructorSetupEmailDelivery>,
            config(),
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/email-code/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"instructor@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("start");
        let cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("cookie")
            .to_str()
            .expect("text")
            .split(';')
            .next()
            .expect("pair")
            .to_string();
        let body = to_bytes(response.into_body(), 1024).await.expect("body");
        let challenge: Uuid =
            serde_json::from_slice::<serde_json::Value>(&body).expect("json")["challengeId"]
                .as_str()
                .expect("id")
                .parse()
                .expect("UUID");
        let code = delivery
            .code
            .lock()
            .expect("code")
            .clone()
            .expect("sent code")
            .as_text();
        assert_eq!(
            delivery
                .destinations
                .lock()
                .expect("destinations")
                .as_slice(),
            ["instructor@example.edu"]
        );
        let denied = complete_request(&router, &challenge.to_string(), &code, None).await;
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let wrong_code = complete_request(
            &router,
            &challenge.to_string(),
            "not-a-valid-code",
            Some(&cookie),
        )
        .await;
        assert_eq!(wrong_code.status(), StatusCode::UNAUTHORIZED);
        let complete =
            complete_request(&router, &challenge.to_string(), &code, Some(&cookie)).await;
        assert_eq!(complete.status(), StatusCode::OK);
        assert!(complete.headers().get_all(SET_COOKIE).iter().any(|value| {
            value
                .to_str()
                .is_ok_and(|value| value.starts_with("__Host-ple_session="))
        }));
        assert_eq!(sessions.0.lock().expect("sessions").len(), 1);
        let reused = complete_request(&router, &challenge.to_string(), &code, Some(&cookie)).await;
        assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_code_and_missing_provider_never_claim_sign_in_started() {
        let expired = Arc::new(CeremonyMemory {
            record: Mutex::new(None),
            expired: true,
        });
        let sessions = Arc::new(SessionMemory::default());
        let delivery = Arc::new(DeliveryMemory::default());
        let router = email_code_router(
            Arc::clone(&expired),
            sessions,
            Arc::clone(&delivery) as Arc<dyn InstructorSetupEmailDelivery>,
            config(),
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/email-code/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"instructor@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("start");
        let cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("cookie")
            .to_str()
            .expect("text")
            .split(';')
            .next()
            .expect("pair")
            .to_string();
        let body = to_bytes(response.into_body(), 1024).await.expect("body");
        let challenge =
            serde_json::from_slice::<serde_json::Value>(&body).expect("json")["challengeId"]
                .as_str()
                .expect("id")
                .to_owned();
        let code = delivery
            .code
            .lock()
            .expect("code")
            .clone()
            .expect("code")
            .as_text();
        let expired_response = complete_request(&router, &challenge, &code, Some(&cookie)).await;
        assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);
        let unavailable_router = email_code_router(
            Arc::new(CeremonyMemory::default()),
            Arc::new(SessionMemory::default()),
            Arc::new(DeliveryMemory {
                code: Mutex::new(None),
                destinations: Mutex::new(Vec::new()),
                fail: true,
            }) as Arc<dyn InstructorSetupEmailDelivery>,
            config(),
        );
        let unavailable_response = unavailable_router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/email-code/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"instructor@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("unavailable");
        assert_eq!(
            unavailable_response.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn provider_failure_preserves_the_previous_usable_code() {
        let ceremonies = Arc::new(CeremonyMemory::default());
        let sessions = Arc::new(SessionMemory::default());
        let working_delivery = Arc::new(DeliveryMemory::default());
        let working_router = email_code_router(
            Arc::clone(&ceremonies),
            Arc::clone(&sessions),
            Arc::clone(&working_delivery) as Arc<dyn InstructorSetupEmailDelivery>,
            config(),
        );
        let first = working_router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/email-code/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"instructor@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("first response");
        let first_cookie = first
            .headers()
            .get(SET_COOKIE)
            .expect("cookie")
            .to_str()
            .expect("text")
            .split(';')
            .next()
            .expect("pair")
            .to_string();
        let first_challenge = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(first.into_body(), 1024).await.expect("body"),
        )
        .expect("json")["challengeId"]
            .as_str()
            .expect("id")
            .to_string();
        let first_code = working_delivery
            .code
            .lock()
            .expect("code")
            .clone()
            .expect("code")
            .as_text();

        let failing_router = email_code_router(
            Arc::clone(&ceremonies),
            sessions,
            Arc::new(DeliveryMemory {
                code: Mutex::new(None),
                destinations: Mutex::new(Vec::new()),
                fail: true,
            }) as Arc<dyn InstructorSetupEmailDelivery>,
            config(),
        );
        let failed = failing_router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/email-code/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"instructor@example.edu"}"#))
                    .expect("request"),
            )
            .await
            .expect("failed response");
        assert_eq!(failed.status(), StatusCode::SERVICE_UNAVAILABLE);
        let completed = complete_request(
            &email_code_router(
                ceremonies,
                Arc::new(SessionMemory::default()),
                working_delivery as Arc<dyn InstructorSetupEmailDelivery>,
                config(),
            ),
            &first_challenge,
            &first_code,
            Some(&first_cookie),
        )
        .await;
        assert_eq!(completed.status(), StatusCode::OK);
    }

    async fn complete_request(
        router: &Router,
        challenge: &str,
        code: &str,
        cookie: Option<&str>,
    ) -> Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri(format!("/api/auth/email-code/complete/{challenge}"))
            .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            builder = builder.header(COOKIE, cookie);
        }
        router
            .clone()
            .oneshot(
                builder
                    .body(Body::from(serde_json::json!({ "code": code }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response")
    }
}
