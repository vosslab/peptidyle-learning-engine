use axum::Json;
use axum::body::to_bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CatalogStore, CourseListScope, CourseRecord, CourseRecordsAccessStore, CreateCourseCommand,
    Cursor, PageRequest, PageSize, PaginationError, SessionStore, Store,
};
use question_model::{CourseId, CourseMembershipRole};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::assignments::assignment_summary_items;
use super::policy::{course_records_are_visible, may_create_course, require_course_access};
use super::projection::{error_response, store_error_response};
use super::routing::{
    CourseQuery, CourseRouteState, CreateCourseDecodeError, DEFAULT_PAGE_SIZE,
    MAX_COURSE_BODY_BYTES, decode_course_create_request,
};

pub(super) async fn list_courses<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let scope = CourseListScope::Member(authenticated.record.subject.user());
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
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_create_course(&authenticated) {
        return error_response(StatusCode::FORBIDDEN, "course creation is not authorized");
    }
    if !headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "course request must use application/json",
        );
    }
    let body = match to_bytes(request.into_body(), MAX_COURSE_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "course request is invalid",
            );
        }
    };
    let value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "course request is invalid",
            );
        }
    };
    let request = match decode_course_create_request(value) {
        Ok(request) => request,
        Err(CreateCourseDecodeError::Invalid) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "course request is invalid",
            );
        }
        Err(CreateCourseDecodeError::Term(failure)) => {
            return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(failure)).into_response());
        }
    };
    let course = CourseRecord {
        id: CourseId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        title: request.title,
        term: request.term,
    };
    match state
        .store
        .create_course(
            authenticated.tenant_context,
            CreateCourseCommand {
                course: course.clone(),
                initial_instructor: authenticated.record.subject.user(),
            },
        )
        .await
    {
        Ok(()) => match state
            .store
            .course_reference(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                course.id,
            )
            .await
        {
            Ok(Some(public_id)) => no_store(
                (
                    StatusCode::CREATED,
                    Json(course.summary(CourseMembershipRole::Instructor, public_id)),
                )
                    .into_response(),
            ),
            Ok(None) | Err(_) => error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "course navigation reference is unavailable",
            ),
        },
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
    let role = match state
        .store
        .get_current_course_membership(
            authenticated.tenant_context,
            course,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) => membership.role,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    if role == CourseMembershipRole::Student {
        match course_records_are_visible(state.store.as_ref(), &authenticated, course).await {
            Ok(true) => {}
            Ok(false) => return error_response(StatusCode::NOT_FOUND, "course not found"),
            Err(response) => return response,
        }
    }
    let public_id = match state
        .store
        .course_reference(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            course,
        )
        .await
    {
        Ok(Some(public_id)) => public_id,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    no_store(Json(record.summary(role, public_id)).into_response())
}

pub(super) async fn list_assignments<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<CourseQuery>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
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
    let member_role = match state
        .store
        .get_current_course_membership(
            authenticated.tenant_context,
            course,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) => membership.role,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let assignments = match member_role {
        CourseMembershipRole::Student => {
            state
                .store
                .list_learner_entitled_assignments(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    course,
                    page,
                )
                .await
        }
        CourseMembershipRole::Instructor => {
            state
                .store
                .list_assignments(authenticated.tenant_context, course, page)
                .await
        }
    };
    match assignments {
        Ok(page) => {
            let mut summaries = Vec::with_capacity(page.items.len());
            for assignment in page.items {
                let public_id = match state
                    .store
                    .assignment_reference(
                        authenticated.tenant_context,
                        authenticated.record.subject.user(),
                        assignment.id,
                    )
                    .await
                {
                    Ok(Some(public_id)) => public_id,
                    Ok(None) => {
                        return error_response(StatusCode::NOT_FOUND, "assignment not found");
                    }
                    Err(error) => return store_error_response(error),
                };
                let (items, selection_groups) = match assignment_summary_items(
                    &state,
                    authenticated.tenant_context,
                    &assignment,
                )
                .await
                {
                    Ok(value) => value,
                    Err(response) => return response,
                };
                summaries.push(assignment.summary(public_id, items, selection_groups));
            }
            no_store(
                Json(learning_data_access::Page {
                    items: summaries,
                    next_cursor: page.next_cursor,
                })
                .into_response(),
            )
        }
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
