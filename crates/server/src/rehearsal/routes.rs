//! Route binding shared by live rehearsal operations.

use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use learning_data_access::{
    CourseRecordsAccessStore, NavigationReferenceStore, SessionStore, Store,
};
use question_model::{AssignmentReference, CourseId, CourseMembershipRole, CourseReference};

use crate::auth::{AuthenticatedSession, auth_error_response, resolve_request_session};

use super::error_response;

/// Authenticated C-/A- binding established before any rehearsal body, RH-
/// reference, request key, or submission response is decoded.
pub(super) struct BoundRehearsalRoute {
    pub(super) authenticated: AuthenticatedSession,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentReference,
}

/// Resolves public route references and requires the actor's direct Instructor
/// membership.  Concealed failures deliberately share one 404 result: no
/// unauthorised request can distinguish a malformed reference, foreign course,
/// removed assignment, or a non-Instructor membership.
pub(super) async fn authorize_bound<S>(
    store: &Arc<S>,
    course_raw: &str,
    assignment_raw: &str,
    headers: &HeaderMap,
) -> Result<BoundRehearsalRoute, Response>
where
    S: Store + CourseRecordsAccessStore + SessionStore + NavigationReferenceStore + 'static,
{
    let authenticated = resolve_request_session(store.as_ref(), headers)
        .await
        .map_err(auth_error_response)?;
    let course_reference = course_raw
        .parse::<CourseReference>()
        .map_err(|_| concealed_route_response())?;
    let actor = authenticated.record.subject.user();
    let course = store
        .resolve_course_reference(authenticated.tenant_context, actor, course_reference)
        .await
        .map_err(|_| concealed_route_response())?
        .ok_or_else(concealed_route_response)?;
    let membership = store
        .get_current_course_membership(authenticated.tenant_context, course, actor)
        .await
        .map_err(|_| concealed_route_response())?
        .ok_or_else(concealed_route_response)?;
    if membership.role != CourseMembershipRole::Instructor {
        return Err(concealed_route_response());
    }
    let assignment = assignment_raw
        .parse::<AssignmentReference>()
        .map_err(|_| concealed_route_response())?;
    let assignment_identity = store
        .resolve_assignment_reference(authenticated.tenant_context, actor, assignment)
        .await
        .map_err(|_| concealed_route_response())?
        .filter(|identity| identity.course == course)
        .ok_or_else(concealed_route_response)?;
    let _ = assignment_identity;
    Ok(BoundRehearsalRoute {
        authenticated,
        course,
        assignment,
    })
}

pub(super) fn concealed_route_response() -> Response {
    error_response(StatusCode::NOT_FOUND, "rehearsal target not found")
}
