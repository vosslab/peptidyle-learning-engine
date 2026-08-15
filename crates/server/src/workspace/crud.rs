use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{CatalogStore, DraftRecord, SessionStore, Store};
use question_model::WorkspaceId;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::BackendRegistry;

use super::state::WorkspaceRouteState;
use super::support::{
    RequiredRevisionError, WorkspaceQuery, draft_response, error_response, expected_revision,
    may_author_workspaces, page_request, required_revision, store_error_response,
    strict_draft_definition,
};

pub(super) async fn list_workspaces<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Query(query): Query<WorkspaceQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_drafts(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            page,
        )
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn get_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft_response(draft),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn save_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let question = match strict_draft_definition(value) {
        Ok(question) => question,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "workspace draft body is invalid",
            );
        }
    };
    if question.workspace != workspace {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workspace path does not match the draft body",
        );
    }
    let expected_revision = match expected_revision(&headers) {
        Ok(revision) => revision,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    // The browser never supplies provenance. A refresh retains the optional
    // descriptive attribution while the next publication receives fresh IDs.
    let existing = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(existing) => existing,
        Err(error) => return store_error_response(error),
    };
    let draft = DraftRecord {
        tenant: authenticated.tenant_context.tenant_id(),
        question: question.clone(),
        derived_from: existing.and_then(|draft| draft.record.derived_from),
    };
    match state
        .store
        .upsert_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            expected_revision,
            draft,
        )
        .await
    {
        Ok(saved) => draft_response(saved),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn delete_workspace<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RequiredRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to delete a workspace",
            );
        }
        Err(RequiredRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    match state
        .store
        .delete_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
            expected_revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        // A foreign tenant intentionally has the same result as an absent row.
        Ok(false) => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => store_error_response(error),
    }
}
