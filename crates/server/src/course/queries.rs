use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CourseListScope, CourseRecord, CourseRecordsAccessStore, Cursor, PageRequest, PageSize,
    PaginationError, SessionStore, Store,
};
use question_model::{CourseId, CourseMembership, CourseMembershipRole, CourseRole};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::policy::{
    course_records_are_visible, is_tenant_administrator, may_create_course, require_course_access,
};
use super::projection::{assignment_page, error_response, store_error_response};
use super::routing::{CourseQuery, CourseRouteState, CreateCourseRequest, DEFAULT_PAGE_SIZE};

pub(super) async fn list_courses<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let scope = if is_tenant_administrator(&authenticated) {
        CourseListScope::TenantAdministrator
    } else {
        CourseListScope::Member(authenticated.record.subject.user())
    };
    match state
        .store
        .list_courses(authenticated.tenant_context, scope, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn create_course<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<CreateCourseRequest>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_create_course(&authenticated) {
        return error_response(StatusCode::FORBIDDEN, "course creation is not authorized");
    }
    let course = CourseRecord {
        id: CourseId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        title: request.title,
        members: vec![CourseMembership {
            user: authenticated.record.subject.user(),
            role: CourseMembershipRole::Instructor,
        }],
    };
    match state
        .store
        .upsert_course(authenticated.tenant_context, course.clone())
        .await
    {
        Ok(()) => no_store(
            (
                StatusCode::CREATED,
                Json(course.summary(CourseRole::Instructor)),
            )
                .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn get_course<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let record = match state
        .store
        .get_course(authenticated.tenant_context, course)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let role = if is_tenant_administrator(&authenticated) {
        CourseRole::Administrator
    } else if let Some(role) = record.role_for(authenticated.record.subject.user()) {
        role
    } else {
        return error_response(StatusCode::NOT_FOUND, "course not found");
    };
    if role == CourseRole::Student {
        match course_records_are_visible(state.store.as_ref(), &authenticated, course).await {
            Ok(true) => {}
            Ok(false) => return error_response(StatusCode::NOT_FOUND, "course not found"),
            Err(response) => return response,
        }
    }
    no_store(Json(record.summary(role)).into_response())
}

pub(super) async fn list_assignments<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, false).await
    {
        return response;
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_assignments(authenticated.tenant_context, course, page)
        .await
    {
        Ok(page) => no_store(Json(assignment_page(page)).into_response()),
        Err(error) => store_error_response(error),
    }
}

/// Lists the compact, browser-safe gradebook projection for one managed course.
///
/// The store owns the bounded assignment/enrollment/summary join. This route
/// intentionally neither loads historical runs nor accepts student or tenant
/// identifiers as authority inputs.
pub(super) async fn list_gradebook<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_gradebook_rows(authenticated.tenant_context, course, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

fn page_request(query: CourseQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}
