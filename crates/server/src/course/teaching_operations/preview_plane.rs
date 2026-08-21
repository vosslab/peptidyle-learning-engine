//! Direct-Instructor, non-mutating WP-PROF-T3 preview-plane transport.
//!
//! The browser supplies only public C-/A-/G-/M- references.  This boundary
//! authenticates, resolves and binds the C-/A- pair before it reads a query or
//! body.  It then creates the accepted qmodel request itself, so an otherwise
//! valid body cannot redirect an evaluation to another assignment.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    CourseRecordsAccessStore, Cursor, NavigationReferenceStore, PageRequest, PageSize,
    PreviewPlaneResult, PreviewPlaneStore, SessionStore, Store, StoreError,
};
use question_model::{
    AssignmentReference, CourseId, CourseMembershipReference, CourseReference,
    DerivedPreviewSubjectRequest, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    SyntheticPreviewModifiers, SyntheticPreviewSubjectRequest, TeachingOperationRevision,
};
use serde::Deserialize;

use super::super::assignments::{AssignmentRevisionHeaderError, required_assignment_revision};
use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, MAX_COURSE_BODY_BYTES};
use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

/// Builds the public-reference preview-plane routes.  `C-` and `A-` mirrors
/// the existing course/assignment nesting while preserving public locators.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + NavigationReferenceStore
        + PreviewPlaneStore
        + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/assignments/{assignment}/preview-schedule",
            get(list_schedule::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/preview-subjects/synthetic",
            post(construct_synthetic::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/preview-subjects/derived",
            post(construct_derived::<S>),
        )
        .with_state(CourseRouteState { store })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SyntheticBody {
    selected_moment: PreviewSelectedMoment,
    groups: PreviewSyntheticGroupReferences,
    modifiers: SyntheticPreviewModifiers,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DerivedBody {
    selected_moment: PreviewSelectedMoment,
    membership: CourseMembershipReference,
}

async fn list_schedule<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + NavigationReferenceStore
        + PreviewPlaneStore
        + 'static,
{
    let bound = match authorize_bound(&state, &course, &assignment, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let revision = match revision(request.headers()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let page = match page_request(request.uri().query()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .list_instructor_preview_schedule(
            bound.auth.tenant_context,
            bound.auth.record.subject.user(),
            bound.course,
            bound.assignment,
            revision,
            page,
        )
        .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => preview_store_error(error),
    }
}

async fn construct_synthetic<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + NavigationReferenceStore
        + PreviewPlaneStore
        + 'static,
{
    let bound = match authorize_bound(&state, &course, &assignment, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let revision = match revision(request.headers()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body::<SyntheticBody>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let result = state
        .store
        .construct_synthetic_preview(
            bound.auth.tenant_context,
            bound.auth.record.subject.user(),
            bound.course,
            SyntheticPreviewSubjectRequest {
                assignment: bound.assignment,
                revision,
                selected_moment: body.selected_moment,
                groups: body.groups,
                modifiers: body.modifiers,
            },
        )
        .await;
    preview_result(result)
}

async fn construct_derived<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + NavigationReferenceStore
        + PreviewPlaneStore
        + 'static,
{
    let bound = match authorize_bound(&state, &course, &assignment, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let revision = match revision(request.headers()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body::<DerivedBody>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let result = state
        .store
        .construct_derived_preview(
            bound.auth.tenant_context,
            bound.auth.record.subject.user(),
            bound.course,
            DerivedPreviewSubjectRequest {
                assignment: bound.assignment,
                revision,
                selected_moment: body.selected_moment,
                membership: body.membership,
            },
        )
        .await;
    preview_result(result)
}

struct BoundPreviewRoute {
    auth: AuthenticatedSession,
    course: CourseId,
    assignment: AssignmentReference,
}

/// Resolves only C-/A- after authentication and performs Instructor access
/// plus exact-course binding before body, query, G-, or M- parsing.
async fn authorize_bound<S>(
    state: &CourseRouteState<S>,
    course_raw: &str,
    assignment_raw: &str,
    headers: &HeaderMap,
) -> Result<BoundPreviewRoute, Response>
where
    S: Store + CourseRecordsAccessStore + SessionStore + NavigationReferenceStore + 'static,
{
    let auth = resolve_request_session(state.store.as_ref(), headers)
        .await
        .map_err(auth_error_response)?;
    let course_reference = course_raw
        .parse::<CourseReference>()
        .map_err(|_| concealed_route_response())?;
    let course = state
        .store
        .resolve_course_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course_reference,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(concealed_route_response)?;
    if require_course_access(state.store.as_ref(), &auth, course, true)
        .await
        .is_err()
    {
        return Err(concealed_route_response());
    }
    let assignment = assignment_raw
        .parse::<AssignmentReference>()
        .map_err(|_| concealed_route_response())?;
    let identity = state
        .store
        .resolve_assignment_reference(auth.tenant_context, auth.record.subject.user(), assignment)
        .await
        .map_err(store_error_response)?
        .filter(|identity| identity.course == course)
        .ok_or_else(concealed_route_response)?;
    let _ = identity;
    Ok(BoundPreviewRoute {
        auth,
        course,
        assignment,
    })
}

#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn revision(headers: &HeaderMap) -> Result<TeachingOperationRevision, Response> {
    required_assignment_revision(headers)
        .map(|value| {
            TeachingOperationRevision::new(value.value())
                .expect("stored assignment revision is within qmodel bounds")
        })
        .map_err(|error| match error {
            AssignmentRevisionHeaderError::Missing => error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match assignment revision is required",
            ),
            AssignmentRevisionHeaderError::Malformed => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match assignment revision is invalid",
            ),
        })
}

#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn page_request(raw_query: Option<&str>) -> Result<PageRequest, Response> {
    let mut after = None;
    let mut size = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "after" => &mut after,
            "size" => &mut size,
            _ => return Err(invalid_page_response()),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(invalid_page_response());
        }
    }
    let size = match size {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .and_then(|value| PageSize::new(value).ok())
            .ok_or_else(invalid_page_response)?,
        None => PageSize::new(50).expect("default preview page size is valid"),
    };
    match after {
        Some(value) => Cursor::parse(value)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|_| invalid_page_response()),
        None => Ok(PageRequest::first(size)),
    }
}

async fn json_body<T>(request: Request) -> Result<T, Response>
where
    T: serde::de::DeserializeOwned,
{
    if !request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|v| v.trim() == "application/json")
        })
    {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request must be JSON",
        ));
    }
    let bytes = to_bytes(request.into_body(), MAX_COURSE_BODY_BYTES + 1)
        .await
        .map_err(|_| error_response(StatusCode::PAYLOAD_TOO_LARGE, "request is too large"))?;
    if bytes.len() > MAX_COURSE_BODY_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request is too large",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| error_response(StatusCode::UNPROCESSABLE_ENTITY, "request is invalid"))
}

fn preview_result(result: Result<PreviewPlaneResult, StoreError>) -> Response {
    match result {
        Ok(value) => no_store(
            Json(question_model::PreviewPlaneResponse {
                evaluation: value.evaluation,
                accommodation: value.accommodation,
            })
            .into_response(),
        ),
        Err(error) => preview_store_error(error),
    }
}

fn preview_store_error(error: StoreError) -> Response {
    match error {
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch => {
            concealed_route_response()
        }
        error => store_error_response(error),
    }
}

fn invalid_page_response() -> Response {
    error_response(StatusCode::BAD_REQUEST, "preview schedule page is invalid")
}

fn concealed_route_response() -> Response {
    error_response(StatusCode::NOT_FOUND, "preview target not found")
}

#[cfg(test)]
#[path = "preview_plane/tests.rs"]
mod tests;
