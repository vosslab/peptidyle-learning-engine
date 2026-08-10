//! Browser-safe retention projections, cache controls, and Store error mapping.

use axum::Json;
use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CourseRetentionView, RETENTION_ARCHIVE_NOTIFICATION_COPY, RetentionNotificationIntent,
    RetentionNotificationView, RetentionRequestOutcome, RetentionRequestResult, StoreError,
};
use question_model::ActivityTimestamp;
use serde::Serialize;

use crate::auth::no_store;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionNotificationProjection {
    intent: RetentionNotificationIntent,
    created_at: ActivityTimestamp,
    copy: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionViewResponse {
    #[serde(flatten)]
    retention: CourseRetentionView,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification: Option<RetentionNotificationProjection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionActionResponse {
    #[serde(flatten)]
    retention: CourseRetentionView,
    outcome: RetentionRequestOutcome,
}

pub(super) fn retention_action_response(result: RetentionRequestResult) -> Response {
    let status = match result.outcome {
        RetentionRequestOutcome::Scheduled | RetentionRequestOutcome::InProgress => {
            StatusCode::ACCEPTED
        }
        RetentionRequestOutcome::Completed => StatusCode::OK,
    };
    let revision = result.retention.revision.value();
    let mut response = no_store(
        (
            status,
            Json(RetentionActionResponse {
                retention: result.retention,
                outcome: result.outcome,
            }),
        )
            .into_response(),
    );
    add_retention_etag(&mut response, revision);
    response
}

pub(super) fn retention_response(
    status: StatusCode,
    retention: CourseRetentionView,
    notification: Option<RetentionNotificationView>,
) -> Response {
    let notification = notification.map(|notification| RetentionNotificationProjection {
        intent: notification.intent,
        created_at: notification.created_at,
        copy: RETENTION_ARCHIVE_NOTIFICATION_COPY,
    });
    let revision = retention.revision.value();
    let mut response = no_store(
        (
            status,
            Json(RetentionViewResponse {
                retention,
                notification,
            }),
        )
            .into_response(),
    );
    add_retention_etag(&mut response, revision);
    response
}

fn add_retention_etag(response: &mut Response, revision: u64) {
    let etag = format!("\"{}\"", revision)
        .parse()
        .expect("retention revision must be a valid ETag");
    response.headers_mut().insert(ETAG, etag);
}

pub(super) async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

pub(super) fn route_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch => {
            error_response(StatusCode::NOT_FOUND, "course retention not found")
        }
        StoreError::Conflict | StoreError::TimedOut | StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "record changed; reload it")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message.to_string())
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "retention service unavailable",
        ),
    }
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}
