//! Learner-safe assignment and aggregate-progress route projections.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use domain::entitlement::EntitlementDecision;
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, CourseItemAnalysisStore, CourseRecordsAccessStore,
    SessionStore, Store, StoredAssignment,
};
use question_model::AssignmentId;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::run::support::learner_assignment_progress;

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::CourseRouteState;
use super::assignment_summary_items;

/// Reads the deliberately narrow assignment projection used by learner browser
/// surfaces even when an instructor opens a learner-facing route preview.
pub(in crate::course) async fn get_learner_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(assignment): Path<AssignmentId>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let assignment = match state
        .store
        .get_assignment_for_edit(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = require_course_access(
        state.store.as_ref(),
        &authenticated,
        assignment.record.course_id,
        false,
    )
    .await
    {
        return response;
    }
    let member_role = match state
        .store
        .get_current_course_membership(
            authenticated.tenant_context,
            assignment.record.course_id,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) => membership.role,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if member_role == question_model::CourseMembershipRole::Student {
        match state
            .store
            .evaluate_assignment_entitlement(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                assignment.record.course_id,
                assignment.record.id,
            )
            .await
        {
            Ok(EntitlementDecision::Granted(_)) => {}
            Ok(EntitlementDecision::Denied(_)) => {
                return error_response(StatusCode::NOT_FOUND, "assignment not found");
            }
            Err(error) => return store_error_response(error),
        }
    }
    learner_assignment_response(&state, &authenticated, assignment).await
}

pub(in crate::course) async fn get_assignment_summary<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(assignment): Path<AssignmentId>,
) -> Response
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let assignment_record = match state
        .store
        .get_assignment(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment summary not found"),
        Err(error) => return store_error_response(error),
    };
    match state
        .store
        .evaluate_assignment_entitlement(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment_record.course_id,
            assignment,
        )
        .await
    {
        Ok(EntitlementDecision::Granted(_)) => {}
        Ok(EntitlementDecision::Denied(_)) => {
            return error_response(StatusCode::NOT_FOUND, "assignment summary not found");
        }
        Err(error) => return store_error_response(error),
    }
    let enrollment = match state
        .store
        .learner_get_enrollment_for_assignment(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment,
        )
        .await
    {
        Ok(Some(enrollment)) => enrollment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment summary not found"),
        Err(error) => return store_error_response(error),
    };
    match state
        .store
        .learner_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment.id,
        )
        .await
    {
        Ok(Some(summary)) => match learner_assignment_progress(
            state.store.as_ref(),
            &authenticated,
            &enrollment,
            &summary,
        )
        .await
        {
            Ok((summary, _)) => no_store(Json(summary).into_response()),
            Err(response) => response,
        },
        Ok(None) => error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn learner_assignment_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    assignment: StoredAssignment,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let public_id = match state
        .store
        .assignment_reference(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment.record.id,
        )
        .await
    {
        Ok(Some(public_id)) => public_id,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    let (items, selection_groups) =
        match assignment_summary_items(state, authenticated.tenant_context, &assignment.record)
            .await
        {
            Ok(value) => value,
            Err(response) => return response,
        };
    no_store(
        Json(question_model::LearnerAssignmentSummary::from(
            assignment
                .record
                .summary(public_id, items, selection_groups),
        ))
        .into_response(),
    )
}
