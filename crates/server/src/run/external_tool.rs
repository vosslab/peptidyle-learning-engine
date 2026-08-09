//! Protected same-origin routes for contracted external learning tools.
//!
//! This keeps provider-facing launch, activity, and submission behavior out of
//! the ordinary run-route module. The parent owns shared run authorization,
//! issuance, and submission-finalization helpers.

use super::*;

use axum::body::Bytes;
use axum::http::HeaderValue;
use base64::Engine as _;
use cookie::{Cookie, SameSite};

const EXTERNAL_LAUNCH_COOKIE: &str = "ple_external_launch";

/// Separate capability for the protected same-origin external-tool frame.
/// It is intentionally not part of `RunBackend`: native and WeBWorK routers
/// retain their ordinary behavior unless a composition explicitly merges this
/// route group with a contracted backend.
#[async_trait]
pub trait ExternalToolLaunchBackend: Send + Sync {
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError>;

    #[allow(clippy::too_many_arguments)]
    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        session_id: uuid::Uuid,
        token: &learning_data_access::ExternalToolLaunchToken,
        method: adapter_imathas::broker_provider::ProxyMethod,
        body: &[u8],
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError>;
}

/// Per-attempt routing state for a server-brokered external learning tool.
/// It contains only a protected same-origin route, never provider material.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToolLaunch {
    pub(super) launch_url: String,
}

/// Optional route group merged only by a configured contracted iMathAS
/// composition. The normal run router deliberately remains backend-neutral.
pub fn router<S, B>(
    store: Arc<S>,
    backend: Arc<B>,
    aead: Arc<crate::imathas_backend::LaunchStateAead>,
) -> Router
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend
        + crate::imathas_backend::ExternalToolSubmissionBackend
        + RunBackend
        + 'static,
{
    Router::new()
        .route(
            "/api/attempts/{attempt}/external-tool/launch",
            get(external_tool_shell::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/external-tool/launch/activity",
            get(external_tool_activity_get::<S, B>).post(external_tool_activity_post::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/external-tool/launch/submission",
            post(external_tool_submission::<S, B>),
        )
        .layer(DefaultBodyLimit::max(262_144))
        .with_state(ExternalToolRouteState {
            store,
            backend,
            aead,
        })
}

struct ExternalToolRouteState<S, B> {
    store: Arc<S>,
    backend: Arc<B>,
    aead: Arc<crate::imathas_backend::LaunchStateAead>,
}

impl<S, B> Clone for ExternalToolRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backend: Arc::clone(&self.backend),
            aead: Arc::clone(&self.aead),
        }
    }
}

fn external_tool_script_nonce() -> Result<String, RunBackendError> {
    let mut nonce_bytes = [0_u8; 18];
    getrandom::fill(&mut nonce_bytes).map_err(|_| {
        RunBackendError::Unavailable("external-tool launch entropy is unavailable".into())
    })?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce_bytes))
}

async fn external_tool_shell<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    let actor = authenticated.record.subject.user();
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(e) => return store_error_response(e),
    };
    if let Err(response) = owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        return response;
    }
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if !matches!(
        question.response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return error_response(
            StatusCode::NOT_FOUND,
            "external-tool launch is not available",
        );
    }
    let created = match state
        .backend
        .create_external_tool_launch(
            authenticated.tenant_context,
            actor,
            reference,
            &question,
            &attempt,
            state.aead.as_ref(),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return backend_error_response(e),
    };
    let value = match crate::imathas_backend::launch_cookie_value(
        state.aead.as_ref(),
        authenticated.tenant_context,
        actor,
        attempt.id,
        &created,
    ) {
        Ok(v) => v,
        Err(e) => return backend_error_response(e),
    };
    let path = format!("/api/attempts/{attempt_id}/external-tool/launch");
    let cookie = Cookie::build((EXTERNAL_LAUNCH_COOKIE, value))
        .path(path)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();
    let script_nonce = match external_tool_script_nonce() {
        Ok(value) => value,
        Err(error) => return backend_error_response(error),
    };
    let activity_path = format!("/api/attempts/{attempt_id}/external-tool/launch/activity");
    // `attempt_id` is a typed UUID formatted by this server. No provider
    // document, handle, credential, or response becomes shell markup.
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>External activity</title><iframe id=\"ple-external-activity\" title=\"External activity\" src=\"{activity_path}\" sandbox=\"allow-scripts allow-forms\"></iframe><script nonce=\"{script_nonce}\">(function(){{const frame=document.getElementById('ple-external-activity');window.addEventListener('message',function(event){{const value=event.data;if(event.source!==frame.contentWindow||event.origin!=='null'||!value||value.kind!=='ple.externalTool.activityReady'||value.attemptId!=='{attempt_id}')return;parent.postMessage({{kind:'ple.externalTool.ready',attemptId:'{attempt_id}'}},location.origin)}})}})()</script>"
    );
    let csp = format!(
        "default-src 'none'; frame-src 'self'; script-src 'nonce-{script_nonce}'; base-uri 'none'; form-action 'none'"
    );
    let mut response = no_store(
        (
            StatusCode::OK,
            [
                ("content-security-policy", csp.as_str()),
                ("content-type", "text/html; charset=utf-8"),
            ],
            body,
        )
            .into_response(),
    );
    if let Ok(header) = HeaderValue::from_str(&cookie.to_string()) {
        response.headers_mut().append("set-cookie", header);
    }
    response
}

async fn external_tool_activity_get<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    external_tool_activity(
        state,
        headers,
        attempt_id,
        adapter_imathas::broker_provider::ProxyMethod::Get,
        &[],
    )
    .await
}

async fn external_tool_activity_post<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
    body: Bytes,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    external_tool_activity(
        state,
        headers,
        attempt_id,
        adapter_imathas::broker_provider::ProxyMethod::Post,
        &body,
    )
    .await
}

async fn external_tool_activity<S, B>(
    state: ExternalToolRouteState<S, B>,
    headers: HeaderMap,
    attempt_id: QuestionAttemptId,
    method: adapter_imathas::broker_provider::ProxyMethod,
    body: &[u8],
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    let actor = authenticated.record.subject.user();
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(e) => return store_error_response(e),
    };
    if let Err(response) = owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        return response;
    }
    let cookie = match Cookie::split_parse(
        headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    )
    .filter_map(Result::ok)
    .find(|c| c.name() == EXTERNAL_LAUNCH_COOKIE)
    {
        Some(v) => v.value().to_owned(),
        None => {
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
    };
    let (session_id, token) = match state.aead.open_cookie(
        &cookie,
        &crate::imathas_backend::launch_cookie_aad(authenticated.tenant_context, actor, attempt.id),
    ) {
        Ok(v) => v,
        Err(_) => {
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
    };
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let response = match state
        .backend
        .proxy_external_tool_activity(
            authenticated.tenant_context,
            actor,
            reference,
            &question,
            &attempt,
            session_id,
            &token,
            method,
            body,
            state.aead.as_ref(),
        )
        .await
    {
        Ok(v) => v,
        // A restored launch session has no browser-visible distinction between
        // expiry, revocation, copied state, stale source binding, or invalid
        // encrypted state. Returning one opaque 404 prevents this activity
        // route from becoming a session/binding oracle.
        Err(RunBackendError::Invalid(_)) => {
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
        Err(e) => return backend_error_response(e),
    };
    let script_nonce = match external_tool_script_nonce() {
        Ok(value) => value,
        Err(error) => return backend_error_response(error),
    };
    let readiness = format!(
        "<script nonce=\"{script_nonce}\">parent.postMessage({{kind:'ple.externalTool.activityReady',attemptId:'{attempt_id}'}},'*')</script>"
    );
    let mut body = response.html().to_vec();
    body.extend_from_slice(readiness.as_bytes());
    let mut out = no_store(body.into_response());
    out.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    out.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_str(&format!(
            "default-src 'none'; script-src 'nonce-{script_nonce}'; form-action 'self'; base-uri 'none'"
        ))
        .expect("CSP nonce uses base64url"),
    );
    out
}

fn external_launch_proof(
    aead: &crate::imathas_backend::LaunchStateAead,
    headers: &HeaderMap,
    context: TenantContext,
    actor: question_model::UserId,
    attempt: QuestionAttemptId,
) -> Option<learning_data_access::ExternalToolLaunchProof> {
    let cookie = Cookie::split_parse(
        headers
            .get("cookie")
            .and_then(|value| value.to_str().ok())
            .unwrap_or(""),
    )
    .filter_map(Result::ok)
    .find(|cookie| cookie.name() == EXTERNAL_LAUNCH_COOKIE)
    .map(|cookie| cookie.value().to_owned())?;
    aead.open_cookie(
        &cookie,
        &crate::imathas_backend::launch_cookie_aad(context, actor, attempt),
    )
    .map(|(session_id, token)| learning_data_access::ExternalToolLaunchProof { session_id, token })
    .ok()
}

async fn external_tool_submission<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
    Json(request): Json<SubmitResponseRequest>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend
        + crate::imathas_backend::ExternalToolSubmissionBackend
        + RunBackend
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    let idempotency_key = match submission_key(&headers) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    if !matches!(request.response, StudentResponse::ExternalTool {}) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "external-tool response is required",
        );
    }
    let actor = authenticated.record.subject.user();
    match state
        .store
        .replay_submission(
            authenticated.tenant_context,
            actor,
            attempt_id,
            &request.response,
            &idempotency_key,
        )
        .await
    {
        Ok(Some(record)) => {
            return finish_submission(
                state.store.as_ref(),
                state.backend.as_ref(),
                &authenticated,
                record,
            )
            .await;
        }
        Ok(None) => {}
        Err(error) => return store_error_response(error),
    }
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if run.completed_at.is_some() {
        return error_response(StatusCode::CONFLICT, "run is already complete");
    }
    let Some(proof) = external_launch_proof(
        state.aead.as_ref(),
        &headers,
        authenticated.tenant_context,
        actor,
        attempt.id,
    ) else {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    };
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !matches!(
        question.response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    }
    let record = match state
        .backend
        .submit_external_tool(
            authenticated.tenant_context,
            actor,
            reference,
            &question,
            &attempt,
            idempotency_key,
            proof,
            state.aead.as_ref(),
        )
        .await
    {
        Ok(SubmissionDisposition::Committed(record)) => *record,
        Ok(SubmissionDisposition::Grade(_)) | Ok(SubmissionDisposition::NeedsManualGrading) => {
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
        Err(RunBackendError::Invalid(_)) => {
            // Session and encrypted-state failures intentionally share a 404.
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
        Err(error) => return backend_error_response(error),
    };
    finish_submission(
        state.store.as_ref(),
        state.backend.as_ref(),
        &authenticated,
        record,
    )
    .await
}
