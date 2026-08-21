//! Browser-safe retention projections, cache controls, and Store error mapping.

use axum::Json;
use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentDefinitionDisposition, CourseRetentionState, CourseRetentionView,
    RETENTION_ARCHIVE_NOTIFICATION_COPY, RetentionNotificationIntent, RetentionRequestOutcome,
    RetentionRequestResult, StoreError,
};
use question_model::{
    RetentionActionOutcomeView, RetentionActionResponse, RetentionDispositionView,
    RetentionNotificationIntentView, RetentionNotificationView, RetentionReadView,
    RetentionStateView, TeachingOperationRevision,
};

use crate::auth::no_store;

pub(super) fn retention_action_response(result: RetentionRequestResult) -> Response {
    let status = match result.outcome {
        RetentionRequestOutcome::Scheduled | RetentionRequestOutcome::InProgress => {
            StatusCode::ACCEPTED
        }
        RetentionRequestOutcome::Completed => StatusCode::OK,
    };
    let revision = result.retention.revision.value();
    let response_body = RetentionActionResponse {
        state: retention_state(result.retention.state),
        assignment_definitions: retention_disposition(result.retention.assignment_definitions),
        revision: teaching_revision(revision),
        outcome: retention_action_outcome(result.outcome),
    };
    let mut response = no_store((status, Json(response_body)).into_response());
    add_retention_etag(&mut response, revision);
    response
}

pub(super) fn retention_response(
    status: StatusCode,
    retention: CourseRetentionView,
    notification: Option<learning_data_access::RetentionNotificationView>,
) -> Response {
    let notification = notification.map(|notification| RetentionNotificationView {
        intent: retention_notification_intent(notification.intent),
        created_at: notification.created_at,
        copy: RETENTION_ARCHIVE_NOTIFICATION_COPY.to_owned(),
    });
    let revision = retention.revision.value();
    let response_body = RetentionReadView {
        state: retention_state(retention.state),
        assignment_definitions: retention_disposition(retention.assignment_definitions),
        revision: teaching_revision(revision),
        notification,
    };
    let mut response = no_store((status, Json(response_body)).into_response());
    add_retention_etag(&mut response, revision);
    response
}

fn retention_state(state: CourseRetentionState) -> RetentionStateView {
    match state {
        CourseRetentionState::Active => RetentionStateView::Active,
        CourseRetentionState::NotificationDue => RetentionStateView::NotificationDue,
        CourseRetentionState::StudentRecordsArchived => RetentionStateView::StudentRecordsArchived,
        CourseRetentionState::StudentRecordsDeleted => RetentionStateView::StudentRecordsDeleted,
    }
}

fn retention_disposition(disposition: AssignmentDefinitionDisposition) -> RetentionDispositionView {
    match disposition {
        AssignmentDefinitionDisposition::Retain => RetentionDispositionView::Retain,
        AssignmentDefinitionDisposition::Delete => RetentionDispositionView::Delete,
    }
}

fn retention_notification_intent(
    intent: RetentionNotificationIntent,
) -> RetentionNotificationIntentView {
    match intent {
        RetentionNotificationIntent::Archive => RetentionNotificationIntentView::Archive,
        RetentionNotificationIntent::Delete => RetentionNotificationIntentView::Delete,
        RetentionNotificationIntent::Extend => RetentionNotificationIntentView::Extend,
    }
}

fn retention_action_outcome(outcome: RetentionRequestOutcome) -> RetentionActionOutcomeView {
    match outcome {
        RetentionRequestOutcome::Scheduled => RetentionActionOutcomeView::Scheduled,
        RetentionRequestOutcome::InProgress => RetentionActionOutcomeView::InProgress,
        RetentionRequestOutcome::Completed => RetentionActionOutcomeView::Completed,
    }
}

fn teaching_revision(revision: u64) -> TeachingOperationRevision {
    TeachingOperationRevision::new(revision)
        .expect("retention revision must be a positive PostgreSQL-safe teaching revision")
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
