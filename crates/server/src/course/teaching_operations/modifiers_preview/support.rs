//! Small closed-projection helpers shared by policy-preview handlers.

use axum::http::StatusCode;
use question_model::teaching_operations::TeachingDisplayLabel;

use crate::course::projection::error_response;
use crate::http_refusal::{HttpRefusal, HttpResult};

pub(super) fn hypothetical_source_response() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "preview provenance is invalid",
    )
}

pub(super) fn label(value: &str) -> HttpResult<TeachingDisplayLabel> {
    TeachingDisplayLabel::try_from(value.to_owned()).map_err(|_| {
        HttpRefusal::from(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "preview label is invalid",
        ))
    })
}
use axum::response::Response;
