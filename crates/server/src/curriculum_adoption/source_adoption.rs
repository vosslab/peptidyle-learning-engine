//! Alpha fork and reusable-source instantiation routes.

use axum::extract::{Path, Request, State};
use axum::response::Response;
use learning_data_access::{CurriculumAdoptionStore, SessionStore};
use question_model::{
    AlphaCourseReference, AlphaInstantiationCommand, AlphaInstantiationPreviewRequest,
    AlphaInstantiationPreviewView, BlueprintInstantiationCommand,
    BlueprintInstantiationPreviewRequest, BlueprintInstantiationPreviewView, BlueprintReference,
    ForkAlphaCommand, ForkAlphaPreviewRequest, ForkAlphaPreviewView,
};

use super::{
    ApplyBody, CurriculumAdoptionRouteState, authenticate_and_preflight, binding_refused,
    command_refused, parse_reference, response_from_store, strict_json_body,
};

pub(super) async fn preview_fork_alpha<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_alpha): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let alpha = match parse_reference::<AlphaCourseReference>(&raw_alpha) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ForkAlphaPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.source.reference != alpha {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_fork_alpha(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_fork_alpha<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_alpha): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let alpha = match parse_reference::<AlphaCourseReference>(&raw_alpha) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ApplyBody<ForkAlphaPreviewView>>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.preview.source.reference != alpha {
        return binding_refused();
    }
    let command = match ForkAlphaCommand::from_preview(&body.preview, body.idempotency_key) {
        Ok(value) => value,
        Err(error) => return command_refused(error),
    };
    response_from_store(
        state
            .store
            .apply_fork_alpha(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}

pub(super) async fn preview_blueprint_instantiation<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_blueprint): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let blueprint = match parse_reference::<BlueprintReference>(&raw_blueprint) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<BlueprintInstantiationPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.source.reference != blueprint {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_blueprint_instantiation(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_blueprint_instantiation<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_blueprint): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let blueprint = match parse_reference::<BlueprintReference>(&raw_blueprint) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ApplyBody<BlueprintInstantiationPreviewView>>(request).await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.preview.source.reference != blueprint {
        return binding_refused();
    }
    let command =
        match BlueprintInstantiationCommand::from_preview(&body.preview, body.idempotency_key) {
            Ok(value) => value,
            Err(error) => return command_refused(error),
        };
    response_from_store(
        state
            .store
            .apply_blueprint_instantiation(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}

pub(super) async fn preview_alpha_instantiation<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_alpha): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let alpha = match parse_reference::<AlphaCourseReference>(&raw_alpha) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<AlphaInstantiationPreviewRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.source.reference != alpha {
        return binding_refused();
    }
    response_from_store(
        state
            .store
            .preview_alpha_instantiation(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                body,
            )
            .await,
    )
}

pub(super) async fn apply_alpha_instantiation<S>(
    State(state): State<CurriculumAdoptionRouteState<S>>,
    Path(raw_alpha): Path<String>,
    request: Request,
) -> Response
where
    S: CurriculumAdoptionStore + SessionStore + 'static,
{
    let authenticated = match authenticate_and_preflight(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let alpha = match parse_reference::<AlphaCourseReference>(&raw_alpha) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match strict_json_body::<ApplyBody<AlphaInstantiationPreviewView>>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.preview.source.reference != alpha {
        return binding_refused();
    }
    let command = match AlphaInstantiationCommand::from_preview(&body.preview, body.idempotency_key)
    {
        Ok(value) => value,
        Err(error) => return command_refused(error),
    };
    response_from_store(
        state
            .store
            .apply_alpha_instantiation(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                command,
            )
            .await,
    )
}
