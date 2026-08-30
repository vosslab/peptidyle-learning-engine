//! Exact-course Instructor active-Student picker representation.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{
    CourseRecordsAccessStore, Cursor, PageRequest, PageSize, SessionStore, Store, StoreError,
    TeachingAuthorityReferenceStore,
};
use question_model::teaching_operations::{
    CourseStudentMembershipsPage, TeachingDisplayLabel, TeachingMembershipRole,
    TeachingMembershipStatus,
};
use question_model::{CourseId, CourseMembershipRole, TeachingPageSize};

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::CourseRouteState;
use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::{HttpRefusal, HttpResult};

/// Builds the active Student picker route group.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store + CourseRecordsAccessStore + SessionStore + TeachingAuthorityReferenceStore + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/student-targets",
            get(list_student_targets::<S>),
        )
        .with_state(CourseRouteState { store })
}

async fn list_student_targets<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + TeachingAuthorityReferenceStore + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, request.headers()).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let page = match page_request(request.uri().query()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    match state
        .store
        .list_course_active_student_membership_reference_views(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            page,
        )
        .await
    {
        Ok(page) => no_store(
            Json(CourseStudentMembershipsPage {
                students: page.items.into_iter().map(student).collect(),
                next_cursor: page.next_cursor.map(|value| value.as_str().to_owned()),
            })
            .into_response(),
        ),
        Err(error) => student_target_error(error),
    }
}

async fn instructor<S>(
    store: &S,
    course: CourseId,
    headers: &HeaderMap,
) -> HttpResult<AuthenticatedSession>
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let auth = resolve_request_session(store, headers)
        .await
        .map_err(auth_error_response)?;
    require_course_access(store, &auth, course, true).await?;
    Ok(auth)
}

/// Parses after exact-course authorization so malformed input cannot enumerate
/// a course to a Student or outsider.
fn page_request(raw_query: Option<&str>) -> HttpResult<PageRequest> {
    let mut after = None;
    let mut size = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "after" => &mut after,
            "size" => &mut size,
            _ => return Err(invalid_page_response().into()),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(invalid_page_response().into());
        }
    }
    let size = match size {
        Some(value) => value
            .parse::<u32>()
            .ok()
            .and_then(|value| TeachingPageSize::try_from(value).ok())
            .ok_or_else(invalid_page_response)?,
        None => TeachingPageSize::try_from(50).expect("default teaching page size is valid"),
    };
    let size = PageSize::new(size.get() as u16).expect("teaching page size is bounded");
    match after {
        Some(value) => Cursor::parse(value)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|_| HttpRefusal::from(invalid_page_response())),
        None => Ok(PageRequest::first(size)),
    }
}

fn invalid_page_response() -> Response {
    no_store(error_response(
        StatusCode::BAD_REQUEST,
        "student target page is invalid",
    ))
}

fn student(
    value: learning_data_access::CourseMembershipReferenceView,
) -> question_model::CourseGroupMemberView {
    debug_assert_eq!(value.role, CourseMembershipRole::Student);
    question_model::CourseGroupMemberView {
        reference: value.reference,
        display: TeachingDisplayLabel::try_from(value.display_name)
            .expect("stored student display is valid"),
        role: TeachingMembershipRole::Student,
        status: match value.status {
            learning_data_access::CourseMemberStatus::Active => TeachingMembershipStatus::Active,
            learning_data_access::CourseMemberStatus::Revoked => TeachingMembershipStatus::Revoked,
        },
    }
}

fn student_target_error(error: StoreError) -> Response {
    match error {
        StoreError::Forbidden | StoreError::OwnershipMismatch | StoreError::NotFound => {
            error_response(StatusCode::NOT_FOUND, "student targets not found")
        }
        other => store_error_response(other),
    }
}

#[cfg(test)]
mod tests;
