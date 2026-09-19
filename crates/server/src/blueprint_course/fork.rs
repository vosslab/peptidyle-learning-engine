//! Authenticated fork command for one exact reusable Blueprint Revision.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use learning_data_access::{BlueprintForkSource, BlueprintLineageStore};
use question_model::{BlueprintRevision, BlueprintRevisionTuple};

use super::{
    BlueprintCourseRouteState, RouteLoadError, blueprint_response, concealed,
    instructor_session_hash, load_view, parse_blueprint_course_id, request_checksum,
    store_error_response, unavailable,
};

/// Forks one immutable Public or Archived source Revision. The Store derives
/// the actor from the attested session and creates the separately editable
/// Private child; no client can choose its owner, availability, Star, or Watch.
pub(super) async fn fork_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path((blueprint_course_id, revision)): Path<(String, String)>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision = match revision.parse::<BlueprintRevision>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let checksum = match request_checksum(
        "fork-blueprint-course",
        &headers,
        &(blueprint_course_id.clone(), revision),
    ) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let receipt = match state
        .lineage
        .fork_blueprint_course(
            session,
            BlueprintForkSource {
                blueprint_revision_tuple: BlueprintRevisionTuple {
                    blueprint_course_id: blueprint_course_id,
                    revision,
                },
            },
            checksum,
            Default::default(),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(
        &state,
        session,
        receipt.blueprint_revision_tuple.blueprint_course_id,
    )
    .await
    {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}
