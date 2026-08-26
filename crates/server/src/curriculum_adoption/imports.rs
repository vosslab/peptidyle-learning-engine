//! Controlled import update, inspection, and evidence reconciliation routes.

use axum::Json;
use axum::extract::{Path, Request, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use learning_data_access::{CurriculumAdoptionStore, SessionStore};
use question_model::{
    AssignmentFastForwardCommand, AssignmentFastForwardPreviewRequest,
    AssignmentFastForwardPreviewView, CourseReference, CreateSourceDerivedAssignmentCommand,
    ReconcileCurriculumAdoptionCommand, SourceDerivedAssignmentPreviewRequest,
    SourceDerivedAssignmentPreviewView,
};

use super::{
    ApplyBody, CurriculumAdoptionRouteState, authenticate_and_preflight, binding_refused,
    command_refused, error_response, parse_course_assignment, parse_reference, response_from_store,
    store_error, strict_json_body,
};
use crate::auth::no_store;

pub(super) async fn preview_assignment_fast_forward<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path((raw_course, raw_assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let (course, assignment) = match parse_course_assignment(&raw_course, &raw_assignment) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<AssignmentFastForwardPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.course != course || body.assignment.assignment != assignment {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_assignment_fast_forward(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_assignment_fast_forward<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path((raw_course, raw_assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let (course, assignment) = match parse_course_assignment(&raw_course, &raw_assignment) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ApplyBody<AssignmentFastForwardPreviewView>>(request).await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.preview.course != course || body.preview.assignment.assignment != assignment {
        return binding_refused();
    }
    let command =
        match AssignmentFastForwardCommand::from_preview(&body.preview, body.idempotency_key) {
            Ok(value) => value,
            Err(error) => return command_refused(error),
        };
    response_from_store(
        state
            .store
            .apply_assignment_fast_forward(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}

pub(super) async fn preview_source_derived_assignment<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_course): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let course = match parse_reference::<CourseReference>(&raw_course) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<SourceDerivedAssignmentPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.course != course {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_source_derived_assignment(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn create_source_derived_assignment<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_course): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let course = match parse_reference::<CourseReference>(&raw_course) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body =
        match strict_json_body::<ApplyBody<SourceDerivedAssignmentPreviewView>>(request).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    if body.preview.course != course {
        return binding_refused();
    }
    let command = match CreateSourceDerivedAssignmentCommand::from_preview(
        &body.preview,
        body.idempotency_key,
    ) {
        Ok(value) => value,
        Err(error) => return command_refused(error),
    };
    response_from_store(
        state
            .store
            .create_source_derived_assignment(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}

pub(super) async fn inspect_curriculum_imports<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_course): Path<String>,
    headers: HeaderMap,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let course = match parse_reference::<CourseReference>(&raw_course) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .inspect_curriculum_imports(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(Some(value)) if value.witness.course == course => no_store(Json(value).into_response()),
        Ok(Some(_)) => binding_refused(),
        Ok(None) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "curriculum imports not found",
        ),
        Err(error) => store_error(error),
    }
}

pub(super) async fn reconcile_curriculum_adoption<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ReconcileCurriculumAdoptionCommand>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    response_from_store(
        state
            .store
            .reconcile_curriculum_adoption(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}
