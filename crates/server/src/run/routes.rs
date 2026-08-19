//! Route assembly and lifecycle commands for authenticated assignment runs.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, CourseAppearanceStore, CourseItemAnalysisStore,
    ManualGradingStore, SessionStore, Store,
};
use question_model::RunId;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::contracts::RunBackend;
use super::manual_grading;
use super::prefetch::{ensure_active_questions, prefetch_next_question};
use super::queries::{
    all_attempts, get_attempt, get_attempt_question, get_enrollment, get_run, get_run_summary,
    get_summary, list_attempts, list_runs, release_attempt_feedback,
};
use super::submission::submit_response;
use super::support::{
    MAX_SUBMISSION_BODY_BYTES, RunRouteState, StartRunRequest, no_store_response,
    store_error_response,
};

/// Builds the authenticated run route group around a shared store and backend registry.
pub fn router<S, B>(store: Arc<S>, backend: Arc<B>) -> Router
where
    S: Store
        + CatalogStore
        + CourseAppearanceStore
        + CourseItemAnalysisStore
        + ManualGradingStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    B: RunBackend + 'static,
{
    Router::new()
        .route("/api/runs", post(start_run::<S, B>))
        .route("/api/runs/{run}", get(get_run::<S, B>))
        .route("/api/runs/{run}/summary", get(get_run_summary::<S, B>))
        .route("/api/runs/{run}/attempts", get(list_attempts::<S, B>))
        .route("/api/attempts/{attempt}", get(get_attempt::<S, B>))
        .route(
            "/api/attempts/{attempt}/question",
            get(get_attempt_question::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/prefetch-next",
            post(prefetch_next_question::<S, B>),
        )
        .route("/api/submissions/{attempt}", post(submit_response::<S, B>))
        .route(
            "/api/attempts/{attempt}/manual-grade",
            get(manual_grading::get_manual_grade::<S, B>)
                .put(manual_grading::put_manual_grade::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/feedback-release",
            post(release_attempt_feedback::<S, B>),
        )
        .route(
            "/api/grading/summaries/{enrollment}",
            get(get_summary::<S, B>),
        )
        .route("/api/enrollments/{enrollment}", get(get_enrollment::<S, B>))
        .route("/api/enrollments/{enrollment}/runs", get(list_runs::<S, B>))
        .layer(axum::extract::DefaultBodyLimit::max(
            MAX_SUBMISSION_BODY_BYTES,
        ))
        .layer(middleware::map_response(no_store_response))
        .with_state(RunRouteState { store, backend })
}

async fn start_run<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Json(request): Json<StartRunRequest>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let actor = authenticated.record.subject.user();
    let run = match state
        .store
        .start_or_resume_run(
            authenticated.tenant_context,
            actor,
            request.assignment_id,
            RunId::generate(),
        )
        .await
    {
        Ok(run) => run,
        Err(error) => return store_error_response(error),
    };
    let attempts = match all_attempts(state.store.as_ref(), &authenticated, run.id).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let predecessor = if attempts.is_empty() {
        None
    } else {
        match state
            .store
            .learner_pending_submission_for_run(authenticated.tenant_context, actor, run.id)
            .await
        {
            Ok(value) => value,
            Err(error) => return store_error_response(error),
        }
    };
    if (attempts.is_empty() || predecessor.is_some())
        && let Err(response) = ensure_active_questions(
            state.store.as_ref(),
            state.backend.as_ref(),
            &authenticated,
            &run,
            predecessor,
        )
        .await
    {
        return response;
    }
    no_store((StatusCode::CREATED, Json(run)).into_response())
}
