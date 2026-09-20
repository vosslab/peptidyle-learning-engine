//! Blueprint response projection, strong validators, and concealed error protocol.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::BlueprintCourseSaveResponse;
use learning_data_access::StoreError;
use question_model::{BlueprintCourseView, BlueprintMetadataState};

pub(super) fn blueprint_response(status: StatusCode, view: BlueprintCourseView) -> Response {
    let revision_number = view.current_revision_tuple.revision_number;
    let mut response = crate::auth::no_store((status, Json(view)).into_response());
    match HeaderValue::from_str(&format!("\"{revision_number}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}
pub(super) fn blueprint_save_response(view: BlueprintCourseView, changed: bool) -> Response {
    let revision_number = view.current_revision_tuple.revision_number;
    let mut response = crate::auth::no_store(
        Json(BlueprintCourseSaveResponse {
            blueprint_course: view,
            changed,
        })
        .into_response(),
    );
    match HeaderValue::from_str(&format!("\"{revision_number}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}
pub(super) fn metadata_response(state: BlueprintMetadataState) -> Response {
    let etag = state.blueprint_edit_number;
    let mut response = crate::auth::no_store(Json(state).into_response());
    match HeaderValue::from_str(&format!("\"{etag}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Blueprint Course changed")
        }
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Blueprint Course lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Blueprint Course is invalid",
        ),
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Blueprint Course conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => unavailable(),
    }
}
pub(super) fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Blueprint Course not found")
}
pub(super) fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Blueprint Course unavailable",
    )
}
pub(super) fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
