//! Retention route mutations and read orchestration.

use axum::extract::{Path, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use learning_data_access::{RetentionApiStore, RetentionStore, SessionStore, Store};
use question_model::CourseId;

use crate::auth::{auth_error_response, resolve_request_session};

use super::RetentionRouteState;
use super::access::require_course_retention_authority;
use super::parsing::{
    ArchiveRequest, ExtendRequest, IfMatchError, is_application_json_content_type,
    parse_strict_json, read_body, required_if_match_revision,
};
use super::projection::{
    error_response, retention_action_response, retention_response, route_store_error,
};

pub(super) async fn get_retention<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_retention_authority(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    let retention = match state
        .store
        .retention_view(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(Some(retention)) => retention,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course retention not found"),
        Err(error) => return route_store_error(error),
    };
    match state
        .store
        .retention_notification(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(notification) => retention_response(StatusCode::OK, retention, notification),
        Err(error) => route_store_error(error),
    }
}

pub(super) async fn end_course_retention<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_retention_authority(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "end request body must be empty",
        );
    }
    match state
        .store
        .end_course_retention(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(record) => match record.safe_view() {
            Ok(view) => retention_response(StatusCode::OK, view, None),
            Err(error) => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("retention record is invalid: {error}"),
            ),
        },
        Err(error) => route_store_error(error),
    }
}

pub(super) async fn request_archive<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_retention_authority(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    let expected_revision = match expected_revision(&headers) {
        Ok(revision) => revision,
        Err(error) => return expected_revision_error(error),
    };
    if !is_application_json_content_type(request.headers().get(CONTENT_TYPE)) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request content type must be application/json",
        );
    }
    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let request = match parse_strict_json::<ArchiveRequest>(body) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "retention archive request is invalid",
            );
        }
    };
    match state
        .store
        .request_retention_archive_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
            request.disposition(),
        )
        .await
    {
        Ok(result) => retention_action_response(result),
        Err(error) => route_store_error(error),
    }
}

pub(super) async fn request_delete<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_retention_authority(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    let expected_revision = match expected_revision(&headers) {
        Ok(revision) => revision,
        Err(error) => return expected_revision_error(error),
    };
    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "delete request body must be empty",
        );
    }
    match state
        .store
        .request_retention_delete_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
        )
        .await
    {
        Ok(result) => retention_action_response(result),
        Err(error) => route_store_error(error),
    }
}

pub(super) async fn request_extend<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let access = match require_course_retention_authority(
        state.store.as_ref(),
        &authenticated,
        course,
    )
    .await
    {
        Ok(access) => access,
        Err(response) => return response,
    };
    if !access.is_sysadmin {
        return error_response(
            StatusCode::FORBIDDEN,
            "retention extension is sysadmin-only",
        );
    }
    let expected_revision = match expected_revision(&headers) {
        Ok(revision) => revision,
        Err(error) => return expected_revision_error(error),
    };
    if !is_application_json_content_type(request.headers().get(CONTENT_TYPE)) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request content type must be application/json",
        );
    }
    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let request = match parse_strict_json::<ExtendRequest>(body) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "retention extension request is invalid",
            );
        }
    };
    let additional_days = match request.additional_days() {
        Ok(days) => days,
        Err(message) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    match state
        .store
        .extend_retention_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
            additional_days,
        )
        .await
    {
        Ok(retention) => retention_response(StatusCode::OK, retention, None),
        Err(error) => route_store_error(error),
    }
}

fn expected_revision(
    headers: &HeaderMap,
) -> Result<learning_data_access::RetentionRevision, IfMatchError> {
    required_if_match_revision(headers)
}

fn expected_revision_error(error: IfMatchError) -> Response {
    match error {
        IfMatchError::Missing => error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match retention revision is required",
        ),
        IfMatchError::Malformed => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "If-Match retention revision is invalid",
        ),
    }
}
