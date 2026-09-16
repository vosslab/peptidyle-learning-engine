//! Thin selective-update transport over one Store-owned transaction.

use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::{
    BlueprintForkApplyRequest, BlueprintForkApplyResponse,
};
use learning_data_access::{ApplyBlueprintForkInput, BlueprintCourseStore};

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_reference, route_error,
    store_error_response, unavailable,
};

pub(super) async fn apply_fork_update(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    payload: Result<Json<BlueprintForkApplyRequest>, JsonRejection>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.1/8.3.1: authenticate before interpreting the command; the Store
    // enforces current source visibility and fork ownership within its transaction.
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.5.2/2.2.1: deny unknown fields and accept only domain-typed choices.
    let Json(request) = match payload {
        Ok(value) => value,
        Err(error) => return route_error(error.status(), "Blueprint fork update is invalid"),
    };
    if request.expected_fork.reference != reference {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint fork reference does not match",
        );
    }
    let result = match state
        .blueprints
        .apply_blueprint_fork(
            session,
            ApplyBlueprintForkInput {
                expected_source: request.expected_source,
                expected_fork: request.expected_fork,
                expected_source_metadata_etag: request.expected_source_metadata_etag,
                expected_fork_metadata_etag: request.expected_fork_metadata_etag,
                source_short_name: request.source_short_name,
                source_long_name: request.source_long_name,
                selection: request.selection,
            },
        )
        .await
    {
        Ok(value) => value,
        // ASVS 16.5.1: preserve concealed errors and never expose Store details.
        Err(error) => return store_error_response(error),
    };
    // ASVS 2.3.3/8.2.3: project only the result committed together, not a later
    // reload; actor identity and private ordinary-Save receipt fields stay internal.
    let revision = result.save.blueprint_revision.revision;
    let mut response = crate::auth::no_store(
        Json(BlueprintForkApplyResponse {
            blueprint_revision: result.save.blueprint_revision,
            changed: result.save.changed,
            metadata: result.metadata,
        })
        .into_response(),
    );
    match HeaderValue::from_str(&format!("\"{revision}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}
