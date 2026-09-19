//! Scoped, answer-free reads of the current Blueprint Assessment's pinned Pool.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::BlueprintPoolMembersView;
use learning_data_access::BlueprintCourseStore;
use question_model::{BlueprintAssessmentId, QuestionId};

use super::{
    BlueprintCourseRouteState, concealed, instructor_session_hash, parse_blueprint_course_id,
    store_error_response,
};

pub(super) async fn load_pool_members(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path((blueprint_course_id, assessment, pool)): Path<(String, String, String)>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.1/2: parse the exact checksum-bearing typed identity.
    let assessment = match assessment.parse::<BlueprintAssessmentId>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let pool = match pool.parse::<QuestionId>() {
        Ok(value) => value,
        _ => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2/3, 8.3.1: Store enforces ordinary Blueprint visibility and
    // current Assessment membership; this is not an arbitrary Revision reader.
    match state
        .blueprints
        .load_blueprint_pool_members(session, blueprint_course_id, assessment, pool)
        .await
    {
        Ok(value) => crate::auth::no_store(
            Json(BlueprintPoolMembersView {
                question_pool_id: value.question_pool_id,
                question_pool_edit_number: value.question_pool_edit_number,
                members: value.members,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}
