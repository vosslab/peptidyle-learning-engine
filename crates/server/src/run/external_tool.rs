//! Protected same-origin routes for contracted external learning tools.
//!
//! This keeps provider-facing launch, activity, and submission behavior out of
//! the ordinary run-route module. It composes the sibling authorization,
//! issuance, and submission-finalization capabilities without widening the
//! ordinary run facade.

use async_trait::async_trait;
use axum::Router;
use axum::body::Bytes;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::routing::{get, post};
use base64::Engine as _;
use cookie::{Cookie, SameSite};
use learning_data_access::ExternalToolLaunchSessionStore;

use super::contracts::{RunBackend, RunBackendError, SubmissionDisposition};
use super::submission::{SuccessorIssuance, finish_submission};
use super::support::*;

/// A host-only, strict, path-scoped browser presentation of an encrypted
/// launch capability.  It is deliberately separate from the host-wide session
/// cookie because provider activity may use it only beneath one attempt path.
pub(crate) const EXTERNAL_LAUNCH_COOKIE: &str = "ple_external_launch";

/// Separate capability for the protected same-origin external-tool frame.
/// It is intentionally not part of `RunBackend`: native and WeBWorK routers
/// retain their ordinary behavior unless a composition explicitly merges this
/// route group with a contracted backend.
#[async_trait]
pub trait ExternalToolLaunchBackend: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create_external_tool_launch(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
        attempt: &QuestionAttempt,
        aead: &crate::imathas_backend::LaunchStateAead,
    ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError>;

    #[allow(clippy::too_many_arguments)]
    async fn proxy_external_tool_activity(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        learner_work_binding: LearnerWorkRoutingBinding,
        issued_question_snapshot: &learning_data_access::IssuedQuestionSnapshotV1,
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
    S: Store
        + CatalogStore
        + ExternalToolLaunchSessionStore
        + ManualGradingStore
        + SessionStore
        + 'static,
    B: ExternalToolLaunchBackend
        + crate::imathas_backend::ExternalToolSubmissionBackend
        + RunBackend
        + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch",
            get(external_tool_shell::<S, B>).post(begin_external_tool_launch::<S, B>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
            get(external_tool_activity_get::<S, B>).post(external_tool_activity_post::<S, B>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/submission",
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
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
) -> Response
where
    S: Store
        + CatalogStore
        + ExternalToolLaunchSessionStore
        + ManualGradingStore
        + SessionStore
        + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    let actor = authenticated.record.subject.user();
    let learner_work_binding = LearnerWorkRoutingBinding::new(course, assignment);
    let prepared = match state
        .store
        .prepare_external_tool_attempt(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            attempt_id,
        )
        .await
    {
        Ok(value) => value,
        Err(e) => return store_error_response(e),
    };
    let attempt = prepared.attempt;
    // A GET only renders the sandbox shell for a launch that was already
    // created by the same-origin POST below. This is deliberately separate
    // from session issuance: Lax cookies accompany top-level cross-site GET
    // navigations, so creation on this route would be GET-CSRFable.
    if external_launch_proof(
        state.aead.as_ref(),
        &headers,
        authenticated.tenant_context,
        actor,
        learner_work_binding,
        attempt.id,
    )
    .is_none()
    {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    }
    let script_nonce = match external_tool_script_nonce() {
        Ok(value) => value,
        Err(error) => return backend_error_response(error),
    };
    let activity_path = format!(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt_id}/external-tool/launch/activity"
    );
    // `attempt_id` is a typed UUID formatted by this server. No provider
    // document, handle, credential, or response becomes shell markup.
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>External activity</title><iframe id=\"ple-external-activity\" title=\"External activity\" src=\"{activity_path}\" sandbox=\"allow-scripts allow-forms\"></iframe><script nonce=\"{script_nonce}\">(function(){{const frame=document.getElementById('ple-external-activity');window.addEventListener('message',function(event){{const value=event.data;if(event.source!==frame.contentWindow||event.origin!=='null'||!value||value.kind!=='ple.externalTool.activityReady'||value.attemptId!=='{attempt_id}')return;parent.postMessage({{kind:'ple.externalTool.ready',attemptId:'{attempt_id}'}},location.origin)}})}})()</script>"
    );
    let csp = format!(
        "default-src 'none'; frame-src 'self'; script-src 'nonce-{script_nonce}'; base-uri 'none'; object-src 'none'; form-action 'none'; frame-ancestors 'self'"
    );
    no_store(
        (
            StatusCode::OK,
            [
                ("content-security-policy", csp.as_str()),
                ("content-type", "text/html; charset=utf-8"),
            ],
            body,
        )
            .into_response(),
    )
}

/// Creates a one-attempt external-tool session after a same-origin API POST.
///
/// The returned route is inert on GET: it only renders a sandbox shell when
/// this response's Strict, HttpOnly binding cookie is present. Keeping session
/// creation here prevents top-level GET navigation from becoming a CSRF path.
async fn begin_external_tool_launch<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
) -> Response
where
    S: Store
        + CatalogStore
        + ExternalToolLaunchSessionStore
        + ManualGradingStore
        + SessionStore
        + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    let actor = authenticated.record.subject.user();
    let learner_work_binding = LearnerWorkRoutingBinding::new(course, assignment);
    let prepared = match state
        .store
        .prepare_external_tool_attempt(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            attempt_id,
        )
        .await
    {
        Ok(value) => value,
        Err(e) => return store_error_response(e),
    };
    let issued_question_snapshot = prepared.issued_question_snapshot;
    let attempt = prepared.attempt;
    if !matches!(
        issued_question_snapshot.question().response,
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
            learner_work_binding,
            &issued_question_snapshot,
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
        learner_work_binding,
        attempt.id,
        &created,
    ) {
        Ok(v) => v,
        Err(e) => return backend_error_response(e),
    };
    let path = format!(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt_id}/external-tool/launch"
    );
    let cookie = Cookie::build((EXTERNAL_LAUNCH_COOKIE, value))
        .path(path.clone())
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();
    let mut response = no_store(Json(ExternalToolLaunch { launch_url: path }).into_response());
    if let Ok(header) = HeaderValue::from_str(&cookie.to_string()) {
        response.headers_mut().append("set-cookie", header);
    }
    response
}

async fn external_tool_activity_get<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
) -> Response
where
    S: Store
        + CatalogStore
        + ExternalToolLaunchSessionStore
        + ManualGradingStore
        + SessionStore
        + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    external_tool_activity(
        state,
        headers,
        LearnerWorkRoutingBinding::new(course, assignment),
        attempt_id,
        adapter_imathas::broker_provider::ProxyMethod::Get,
        &[],
    )
    .await
}

async fn external_tool_activity_post<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
    body: Bytes,
) -> Response
where
    S: Store + CatalogStore + ExternalToolLaunchSessionStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    external_tool_activity(
        state,
        headers,
        LearnerWorkRoutingBinding::new(course, assignment),
        attempt_id,
        adapter_imathas::broker_provider::ProxyMethod::Post,
        &body,
    )
    .await
}

async fn external_tool_activity<S, B>(
    state: ExternalToolRouteState<S, B>,
    headers: HeaderMap,
    learner_work_binding: LearnerWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
    method: adapter_imathas::broker_provider::ProxyMethod,
    body: &[u8],
) -> Response
where
    S: Store + CatalogStore + ExternalToolLaunchSessionStore + SessionStore + 'static,
    B: ExternalToolLaunchBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    let actor = authenticated.record.subject.user();
    let prepared = match state
        .store
        .prepare_external_tool_attempt(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            attempt_id,
        )
        .await
    {
        Ok(value) => value,
        Err(e) => return store_error_response(e),
    };
    let issued_question_snapshot = prepared.issued_question_snapshot;
    let attempt = prepared.attempt;
    let Some(cookie) = external_launch_cookie(&headers) else {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    };
    let (session_id, token) = match state.aead.open_cookie(
        &cookie,
        &crate::imathas_backend::launch_cookie_aad(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            attempt.id,
        ),
    ) {
        Ok(v) => v,
        Err(_) => {
            return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
        }
    };
    let response = match state
        .backend
        .proxy_external_tool_activity(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            &issued_question_snapshot,
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
            "default-src 'none'; script-src 'nonce-{script_nonce}'; form-action 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'self'"
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
    learner_work_binding: LearnerWorkRoutingBinding,
    attempt: QuestionAttemptId,
) -> Option<learning_data_access::ExternalToolLaunchProof> {
    let cookie = external_launch_cookie(headers)?;
    aead.open_cookie(
        &cookie,
        &crate::imathas_backend::launch_cookie_aad(context, actor, learner_work_binding, attempt),
    )
    .map(|(session_id, token)| learning_data_access::ExternalToolLaunchProof { session_id, token })
    .ok()
}

/// Returns the sole launch capability presentation.  A domain cookie from a
/// sibling host can share this name, and RFC cookie ordering is not authority;
/// duplicate or malformed presentations must therefore fail closed instead of
/// allowing the first parsed value to win.
fn external_launch_cookie(headers: &HeaderMap) -> Option<String> {
    let mut launch = None;
    for value in headers.get_all("cookie").iter() {
        let value = value.to_str().ok()?;
        for parsed in Cookie::split_parse(value) {
            let cookie = parsed.ok()?;
            if cookie.name() == EXTERNAL_LAUNCH_COOKIE {
                if launch.is_some() {
                    return None;
                }
                launch = Some(cookie.value().to_owned());
            }
        }
    }
    launch
}

async fn external_tool_submission<S, B>(
    State(state): State<ExternalToolRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
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
    let learner_work_binding = LearnerWorkRoutingBinding::new(course, assignment);
    let prepared = match state
        .store
        .prepare_question_submission(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            attempt_id,
            &request.response,
            &idempotency_key,
        )
        .await
    {
        Ok(learning_data_access::SubmissionPreparation::Replay(record)) => {
            return finish_submission(
                state.store.as_ref(),
                state.backend.as_ref(),
                &authenticated,
                learner_work_binding,
                *record,
                SuccessorIssuance::Deferred,
            )
            .await;
        }
        Ok(learning_data_access::SubmissionPreparation::AcceptedPending(pending)) => {
            return accepted_pending_response(pending.attempt());
        }
        Ok(learning_data_access::SubmissionPreparation::FirstEffect(prepared)) => *prepared,
        Err(error) => return store_error_response(error),
    };
    let issued_question_snapshot = prepared.issued_question_snapshot;
    let attempt = prepared.attempt;
    let Some(proof) = external_launch_proof(
        state.aead.as_ref(),
        &headers,
        authenticated.tenant_context,
        actor,
        learner_work_binding,
        attempt.id,
    ) else {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    };
    if !matches!(
        issued_question_snapshot.question().response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return error_response(StatusCode::NOT_FOUND, "external-tool launch is unavailable");
    }
    let record = match state
        .backend
        .submit_external_tool(
            authenticated.tenant_context,
            actor,
            learner_work_binding,
            &issued_question_snapshot,
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
        learner_work_binding,
        record,
        SuccessorIssuance::Deferred,
    )
    .await
}
