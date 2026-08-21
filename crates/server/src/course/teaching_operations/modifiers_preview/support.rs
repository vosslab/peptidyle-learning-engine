//! Small closed-projection helpers shared by policy-preview handlers.

use axum::http::StatusCode;
use axum::response::Response;
use question_model::teaching_operations::TeachingDisplayLabel;

use crate::course::projection::error_response;

pub(super) fn hypothetical_source_response() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "preview provenance is invalid",
    )
}

#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
pub(super) fn label(value: &str) -> Result<TeachingDisplayLabel, Response> {
    TeachingDisplayLabel::try_from(value.to_owned())
        .map_err(|_| error_response(StatusCode::SERVICE_UNAVAILABLE, "preview label is invalid"))
}
