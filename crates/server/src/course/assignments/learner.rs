//! Learner-safe assignment and aggregate-progress route projections.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use domain::entitlement::EntitlementDecision;
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, CourseItemAnalysisStore, CourseRecordsAccessStore,
    LearnerAssignmentSummarySnapshot, ResolveEffectivePolicyCommand, SessionStore, Store,
    StoreError, StoredAssignment,
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
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
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
        let entitlement = match state
            .store
            .evaluate_assignment_entitlement(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                assignment.record.course_id,
                assignment.record.id,
            )
            .await
        {
            Ok(EntitlementDecision::Granted(grant)) => EntitlementDecision::Granted(grant),
            Ok(EntitlementDecision::Denied(_)) => {
                return error_response(StatusCode::NOT_FOUND, "assignment not found");
            }
            Err(error) => return store_error_response(error),
        };
        let now = match state
            .store
            .authoritative_time(authenticated.tenant_context)
            .await
        {
            Ok(now) => now,
            Err(error) => return store_error_response(error),
        };
        // The start verdict needs the number of prior completed runs, not a
        // route-local guess.  The compact maintained summary is bounded and
        // also treats an active run as resumable rather than as an exhausted
        // attempt; do not replace this with a run-history scan.
        let prior_run_count = match state
            .store
            .learner_get_enrollment_for_assignment(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                assignment.record.id,
            )
            .await
        {
            Ok(Some(enrollment)) => match state
                .store
                .learner_get_summary(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    enrollment.id,
                )
                .await
            {
                Ok(Some(summary)) => summary.summary.completed_run_count,
                Ok(None) => 0,
                Err(error) => return store_error_response(error),
            },
            Ok(None) => 0,
            Err(error) => return store_error_response(error),
        };
        let resolved = match state
            .store
            .resolve_effective_policy(
                authenticated.tenant_context,
                ResolveEffectivePolicyCommand {
                    assignment: assignment.record.id,
                    entitlement,
                    authorization:
                        domain::effective_assignment_policy::AuthorizationGate::Authorized,
                    now,
                    prior_run_count,
                },
            )
            .await
        {
            Ok(Some(resolved)) => resolved,
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
            Err(error) => return store_error_response(error),
        };
        let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed { policy, start } =
            resolved.decision
        else {
            return error_response(StatusCode::NOT_FOUND, "assignment not found");
        };
        let domain::effective_assignment_policy::StartVerdict::MayStart { late } = start else {
            return error_response(StatusCode::NOT_FOUND, "assignment not found");
        };
        let course = match state
            .store
            .get_course(authenticated.tenant_context, assignment.record.course_id)
            .await
        {
            Ok(Some(course)) => course,
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
            Err(error) => return store_error_response(error),
        };
        return learner_assignment_detail_response(
            &state,
            &authenticated,
            assignment,
            course.term.time_zone().clone(),
            question_model::course::LearnerAssignmentDelivery {
                available_at: policy.available_at.value,
                due_at: policy.due_at.value,
                closes_at: policy.closes_at.value,
                time_limit_seconds: policy.time_limit_seconds.value,
                attempt_limit: policy.attempt_limit.value,
                late_submission: policy.late_submission.value,
                deadline_behavior: policy.deadline_behavior.value,
                late_status: match late {
                    domain::effective_assignment_policy::LateVerdict::OnTime => {
                        question_model::course::LearnerLateStatus::OnTime
                    }
                    domain::effective_assignment_policy::LateVerdict::AcceptedLate => {
                        question_model::course::LearnerLateStatus::AcceptedLate
                    }
                    domain::effective_assignment_policy::LateVerdict::MarkedLate => {
                        question_model::course::LearnerLateStatus::MarkedLate
                    }
                    domain::effective_assignment_policy::LateVerdict::RejectedLate => {
                        return error_response(StatusCode::NOT_FOUND, "assignment not found");
                    }
                },
            },
        )
        .await;
    }
    let course = match state
        .store
        .get_course(authenticated.tenant_context, assignment.record.course_id)
        .await
    {
        Ok(Some(course)) => course,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    let base_policy = assignment.base_policy;
    learner_assignment_detail_response(
        &state,
        &authenticated,
        assignment,
        course.term.time_zone().clone(),
        question_model::course::LearnerAssignmentDelivery {
            available_at: base_policy.available_at,
            due_at: base_policy.due_at,
            closes_at: base_policy.closes_at,
            time_limit_seconds: base_policy.time_limit_seconds,
            attempt_limit: base_policy.attempt_limit,
            late_submission: base_policy.late_submission,
            deadline_behavior: base_policy.deadline_behavior,
            late_status: question_model::course::LearnerLateStatus::OnTime,
        },
    )
    .await
}

async fn learner_assignment_detail_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    assignment: StoredAssignment,
    time_zone: question_model::IanaTimeZone,
    delivery: question_model::course::LearnerAssignmentDelivery,
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
        Json(
            question_model::course::LearnerAssignmentDetail::from_summary(
                assignment
                    .record
                    .summary(public_id, items, selection_groups),
                assignment.record.instructions,
                time_zone,
                delivery,
            ),
        )
        .into_response(),
    )
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
        Ok(None) => {
            return match learner_assignment_progress(
                state.store.as_ref(),
                &authenticated,
                assignment,
                None,
            )
            .await
            {
                Ok((summary, _)) => no_store(Json(summary).into_response()),
                Err(response) => response,
            };
        }
        Err(error) => return store_error_response(error),
    };
    let summary = match required_materialized_summary(
        state
            .store
            .learner_get_summary(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                enrollment.id,
            )
            .await,
    ) {
        MaterializedSummaryLookup::Found(summary) => summary,
        MaterializedSummaryLookup::Failed(response) => return response,
    };
    match learner_assignment_progress(
        state.store.as_ref(),
        &authenticated,
        assignment,
        Some(&summary),
    )
    .await
    {
        Ok((summary, _)) => no_store(Json(summary).into_response()),
        Err(response) => response,
    }
}

enum MaterializedSummaryLookup {
    Found(LearnerAssignmentSummarySnapshot),
    Failed(Response),
}

fn required_materialized_summary(
    result: Result<Option<LearnerAssignmentSummarySnapshot>, StoreError>,
) -> MaterializedSummaryLookup {
    match result {
        Ok(Some(summary)) => MaterializedSummaryLookup::Found(summary),
        Ok(None) => MaterializedSummaryLookup::Failed(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "assignment summary unavailable",
        )),
        Err(error) => MaterializedSummaryLookup::Failed(store_error_response(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialized_enrollment_without_summary_is_a_service_failure() {
        let response = match required_materialized_summary(Ok(None)) {
            MaterializedSummaryLookup::Found(_) => {
                panic!("missing materialized summary must fail")
            }
            MaterializedSummaryLookup::Failed(response) => response,
        };

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
