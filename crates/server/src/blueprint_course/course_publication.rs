//! Create a new Private Blueprint from authorized Course reusable structure.

use axum::{
    Json, extract::Path, extract::State, http::HeaderMap, http::StatusCode, response::Response,
};
use learning_data_access::CourseBlueprintPublicationStore;
use question_model::{CourseInstanceId, CreateBlueprintFromCourseInstanceInput};

use super::{
    BlueprintCourseRouteState, RouteLoadError, blueprint_response, concealed,
    instructor_session_hash, load_view, request_checksum, store_error_response, unavailable,
};

pub(super) async fn create(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
    Json(input): Json<CreateBlueprintFromCourseInstanceInput>,
) -> Response {
    let course = match CourseInstanceId::new(course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let checksum = match request_checksum(
        "create-blueprint-from-course-instance",
        &headers,
        &(course.clone(), &input),
    ) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let receipt = match state
        .course_publication
        .create_blueprint_from_course_instance(session, course, checksum, input, Default::default())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(
        &state,
        session,
        receipt.blueprint_revision.blueprint_course_id,
    )
    .await
    {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}
