use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use learning_data_access::{AssignmentRecord, Page, StoreError};
use question_model::AssignmentSummary;

use crate::auth::no_store;

pub(super) fn assignment_page(page: Page<AssignmentRecord>) -> Page<AssignmentSummary> {
    Page {
        items: page
            .items
            .into_iter()
            .map(|assignment| assignment.summary())
            .collect(),
        next_cursor: page.next_cursor,
    }
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists => error_response(StatusCode::CONFLICT, "record already exists"),
        StoreError::Conflict => error_response(StatusCode::CONFLICT, "record changed; reload it"),
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "operation is not authorized")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => error_response(StatusCode::CONFLICT, "question attempt timed out"),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "course storage unavailable",
        ),
    }
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}
