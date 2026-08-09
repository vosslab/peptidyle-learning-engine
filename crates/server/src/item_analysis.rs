//! Instructor-only current course item-analysis reports.
//!
//! The Store is the authorization boundary: it returns no report for an
//! absent assignment or an actor who does not directly instruct its course.
//! This HTTP projection deliberately omits tenant, learner, attempt, raw
//! response, answer, and object identity fields.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use domain::item_analysis::{
    AssignmentItemAnalysis, CourseItemAnalysisReport, ItemAnalysisResponseDistribution,
};
use learning_data_access::{CourseItemAnalysisStore, SessionStore, Store, StoreError};
use question_model::{ActivityTimestamp, AssignmentId, AssignmentItemId, CourseId};
use serde::Serialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

/// Builds the protected current item-analysis route.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + CourseItemAnalysisStore + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/assignments/{assignment}/item-analysis",
            get(get_course_item_analysis::<S>),
        )
        .with_state(ItemAnalysisRouteState { store })
}

struct ItemAnalysisRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for ItemAnalysisRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

/// Strict browser DTO with aggregate fields only.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CourseItemAnalysisResponse {
    source_scoring_generation: u64,
    analyzed_at: ActivityTimestamp,
    completed_run_count: u32,
    in_progress_run_count: u32,
    incomplete_manual_grading: bool,
    recent_rescoring: bool,
    assignment_average_score: Option<f64>,
    average_completion_time_millis: Option<u64>,
    items: Vec<AssignmentItemAnalysisResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentItemAnalysisResponse {
    assignment_item: AssignmentItemId,
    source_scoring_generation: u64,
    analyzed_at: ActivityTimestamp,
    graded_attempt_count: u32,
    unanswered_attempt_count: u32,
    pending_manual_attempt_count: u32,
    difficulty: Option<f64>,
    average_credit: Option<f64>,
    credit_standard_deviation: Option<f64>,
    discrimination: Option<f64>,
    response_distribution: ItemAnalysisResponseDistribution,
    average_completion_time_millis: Option<u64>,
}

impl From<CourseItemAnalysisReport> for CourseItemAnalysisResponse {
    fn from(report: CourseItemAnalysisReport) -> Self {
        Self {
            source_scoring_generation: report.source_scoring_generation.value(),
            analyzed_at: report.analyzed_at,
            completed_run_count: report.completed_run_count,
            in_progress_run_count: report.in_progress_run_count,
            incomplete_manual_grading: report.incomplete_manual_grading,
            recent_rescoring: report.recent_rescoring,
            assignment_average_score: report.assignment_average_score,
            average_completion_time_millis: report.average_completion_time_millis,
            items: report
                .items
                .into_iter()
                .map(AssignmentItemAnalysisResponse::from)
                .collect(),
        }
    }
}

impl From<AssignmentItemAnalysis> for AssignmentItemAnalysisResponse {
    fn from(item: AssignmentItemAnalysis) -> Self {
        Self {
            assignment_item: item.assignment_item,
            source_scoring_generation: item.source_scoring_generation.value(),
            analyzed_at: item.analyzed_at,
            graded_attempt_count: item.graded_attempt_count,
            unanswered_attempt_count: item.unanswered_attempt_count,
            pending_manual_attempt_count: item.pending_manual_attempt_count,
            difficulty: item.difficulty,
            average_credit: item.average_credit,
            credit_standard_deviation: item.credit_standard_deviation,
            discrimination: item.discrimination,
            response_distribution: item.response_distribution,
            average_completion_time_millis: item.average_completion_time_millis,
        }
    }
}

async fn get_course_item_analysis<S>(
    State(state): State<ItemAnalysisRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
) -> Response
where
    S: Store + CourseItemAnalysisStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    match state
        .store
        .course_item_analysis(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            assignment,
        )
        .await
    {
        Ok(Some(report)) => {
            no_store(Json(CourseItemAnalysisResponse::from(report)).into_response())
        }
        // This Store call proves both assignment existence and direct instructor
        // authorization; never distinguish the two to the browser.
        Ok(None)
        | Err(StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch) => {
            error_response(StatusCode::NOT_FOUND, "item analysis not found")
        }
        Err(StoreError::Unavailable(_) | StoreError::RetryableTransaction) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "item analysis unavailable")
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "item analysis not found"),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod http_tests;

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use domain::item_analysis::{
        AssignmentItemAnalysis, CourseItemAnalysisReport, ItemAnalysisResponseDistribution,
    };
    use learning_data_access::in_memory::MemoryStore;
    use question_model::{
        AssignmentItemId, ProblemId, ProblemVersionRef, ScoringGeneration, TenantId, VersionId,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn response_projection_contains_aggregate_fields_without_storage_or_answer_identity() {
        let tenant = TenantId::from_uuid(id(1));
        let course = CourseId::from_uuid(id(2));
        let assignment = AssignmentId::from_uuid(id(3));
        let report = CourseItemAnalysisReport {
            tenant,
            course,
            assignment,
            source_scoring_generation: ScoringGeneration::new(4).expect("generation"),
            analyzed_at: ActivityTimestamp::from_unix_millis(5),
            completed_run_count: 6,
            in_progress_run_count: 7,
            incomplete_manual_grading: true,
            recent_rescoring: false,
            assignment_average_score: Some(0.5),
            average_completion_time_millis: Some(8),
            items: vec![AssignmentItemAnalysis {
                tenant,
                course,
                assignment,
                assignment_item: AssignmentItemId::from_uuid(id(9)),
                reference: ProblemVersionRef {
                    problem: ProblemId::from_uuid(id(10)),
                    version: VersionId::from_uuid(id(11)),
                },
                source_scoring_generation: ScoringGeneration::new(4).expect("generation"),
                analyzed_at: ActivityTimestamp::from_unix_millis(5),
                graded_attempt_count: 12,
                unanswered_attempt_count: 13,
                pending_manual_attempt_count: 14,
                difficulty: Some(0.5),
                average_credit: Some(0.4),
                credit_standard_deviation: Some(0.3),
                discrimination: Some(0.2),
                response_distribution: ItemAnalysisResponseDistribution {
                    correct: 1,
                    partial: 2,
                    incorrect: 3,
                    unanswered: 4,
                    pending_manual: 5,
                },
                average_completion_time_millis: Some(15),
            }],
        };

        let value = serde_json::to_value(CourseItemAnalysisResponse::from(report))
            .expect("serialize projection");
        let object = value.as_object().expect("object response");
        assert!(!object.contains_key("tenant"));
        assert!(!object.contains_key("course"));
        assert!(!object.contains_key("assignment"));
        let item = object["items"][0].as_object().expect("item response");
        assert!(!item.contains_key("tenant"));
        assert!(!item.contains_key("reference"));
        assert!(!item.contains_key("response"));
        assert!(!item.contains_key("answer"));
        assert_eq!(item["responseDistribution"]["pendingManual"], 5);
    }

    #[tokio::test]
    async fn unauthenticated_route_is_no_store() {
        let app = router(Arc::new(MemoryStore::default()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/courses/00000000-0000-0000-0000-000000000002/assignments/00000000-0000-0000-0000-000000000003/item-analysis")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}
