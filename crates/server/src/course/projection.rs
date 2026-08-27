use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use learning_data_access::StoreError;

use crate::auth::no_store;

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists => error_response(StatusCode::CONFLICT, "record already exists"),
        StoreError::Conflict => error_response(StatusCode::CONFLICT, "record changed; reload it"),
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "operation is not authorized")
        }
        // ASVS 16.5.1: Store errors retain diagnostic detail inside the
        // trusted boundary, while course HTTP responses expose one generic
        // correction at the existing 422 validation status.
        StoreError::InvalidRecord(_) | StoreError::RunModel(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "course request could not be completed",
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use domain::run::RunModelError;

    #[tokio::test]
    async fn invalid_store_details_share_one_browser_safe_course_response() {
        for error in [
            StoreError::InvalidRecord("postgres constraint ple_private_detail".to_string()),
            StoreError::RunModel(RunModelError::InvalidScore { score: 101.25 }),
        ] {
            let response = store_error_response(error);
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = to_bytes(response.into_body(), 8 * 1024)
                .await
                .expect("generic course error body");
            let body: serde_json::Value =
                serde_json::from_slice(&body).expect("generic course error JSON");
            assert_eq!(body["error"], "course request could not be completed");
            assert!(!body.to_string().contains("private_detail"));
        }
    }
}
