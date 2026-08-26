//! Teaching-course rollover and whole-course term-shift routes.

use axum::extract::{Path, Request, State};
use axum::response::Response;
use learning_data_access::{CurriculumAdoptionStore, SessionStore};
use question_model::{
    CourseReference, CourseRolloverCommand, CourseRolloverPreviewRequest,
    CourseRolloverPreviewView, CourseTermShiftCommand, CourseTermShiftPreviewOutcome,
    CourseTermShiftPreviewRequest,
};

use super::{
    ApplyBody, CurriculumAdoptionRouteState, authenticate_and_preflight, binding_refused,
    command_refused, outcome_course, parse_reference, response_from_store, strict_json_body,
};

pub(super) async fn preview_course_rollover<S>(
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
    let body = match strict_json_body::<CourseRolloverPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.witness.course != course {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_course_rollover(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_course_rollover<S>(
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
    let body = match strict_json_body::<ApplyBody<CourseRolloverPreviewView>>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.preview.witness.course != course {
        return binding_refused();
    }
    let command = match CourseRolloverCommand::from_preview(&body.preview, body.idempotency_key) {
        Ok(value) => value,
        Err(error) => return command_refused(error),
    };
    response_from_store(
        state
            .store
            .apply_course_rollover(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}

pub(super) async fn preview_course_term_shift<S>(
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
    let body = match strict_json_body::<CourseTermShiftPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.witness.course != course {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_course_term_shift(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_course_term_shift<S>(
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
    let body = match strict_json_body::<ApplyBody<CourseTermShiftPreviewOutcome>>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if outcome_course(&body.preview) != course {
        return binding_refused();
    }
    let command = match CourseTermShiftCommand::from_preview(&body.preview, body.idempotency_key) {
        Ok(value) => value,
        Err(error) => return command_refused(error),
    };
    response_from_store(
        state
            .store
            .apply_course_term_shift(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}
