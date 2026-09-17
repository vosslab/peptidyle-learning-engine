//! Direct-Instructor Assessment Workspace routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use learning_data_access::{
    ApplyAssessmentBlueprintUpdateInput, CreateLiveAssessmentInput, LiveAssessmentStore,
    SaveBaseAssessmentPolicyInput, SaveLiveAssessmentInlineInput, SaveLiveAssessmentInput,
    SessionTokenHash, StoreError,
    postgres::{PostgresLiveAssessmentStore, PostgresSessionStore},
};
use question_model::{
    AssessmentEditNumber, AssessmentReference, AssessmentTitle, CourseInstanceReference,
    ProductRole,
};
use serde::Deserialize;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    assessments: PostgresLiveAssessmentStore,
}

/// The closed confirmation payload for the irreversible Unrelease transition.
///
/// The title is intentionally parsed at the HTTP boundary; PostgreSQL repeats
/// the exact comparison while it holds the Assessment lock.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UnreleaseAssessmentInput {
    confirmation_title: AssessmentTitle,
}

/// Registers answer-free current Assessment authoring and immutable release routes.
pub fn assessment_release_router(
    sessions: Arc<PostgresSessionStore>,
    assessments: PostgresLiveAssessmentStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assessment-question-picker",
            get(list_picker),
        )
        .route(
            "/api/course-instances/{course}/assessments",
            get(list_assessments).post(create_assessment),
        )
        .route("/api/assessments/due-soon", get(list_assessments_due_soon))
        .route(
            "/api/course-instances/{course}/blueprint-update-review",
            get(review_course_blueprint_update),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}",
            get(load_assessment).put(save_assessment),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/blueprint-update",
            get(review_blueprint_update).post(apply_blueprint_update),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/inline",
            put(save_assessment_inline),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/policies",
            put(save_base_assessment_policy),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/release-validation",
            get(validate_release),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/release",
            post(release_assessment),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/unrelease-impact",
            get(unrelease_impact),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/unrelease",
            post(unrelease_assessment),
        )
        .with_state(StateData {
            sessions,
            assessments,
        })
}

async fn review_course_blueprint_update(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2, 8.3.1, 14.3.2: direct Course authority and readable parent
    // are checked together in the Store; every success and denial is no-store.
    match state
        .assessments
        .review_course_blueprint_update(token, course)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(error) => store_error(error),
    }
}

async fn review_blueprint_update(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2, 8.3.1: scoped authorization remains in the trusted Store.
    match state
        .assessments
        .review_assessment_blueprint_update(token, course, assessment)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(error) => store_error(error),
    }
}

async fn apply_blueprint_update(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
    input: Result<
        Json<ApplyAssessmentBlueprintUpdateInput>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.5.2, 2.2.1: closed input carries only both CAS preconditions.
    let input = match input {
        Ok(Json(value)) => value,
        Err(error) => return crate::auth::no_store(error.into_response()),
    };
    match state
        .assessments
        .apply_assessment_blueprint_update(token, course, assessment, input, Default::default())
        .await
    {
        Ok(value) => workspace_response(StatusCode::OK, &value),
        Err(error) => store_error(error),
    }
}

async fn list_assessments_due_soon(State(state): State<StateData>, headers: HeaderMap) -> Response {
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state.assessments.list_assessments_due_soon(token).await {
        // ASVS 4.1.1 and 8.3.1: Axum serializes this closed JSON projection;
        // it includes no Student, Attempt, response, answer, or grading record.
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_assessments(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .list_course_assessments(token, course)
        .await
    {
        // ASVS 8.3.1 and 8.3.4: this closed projection carries no Student,
        // response, answer, grading, or other protected educational record.
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_picker(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .list_assessment_question_picker(token, course)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn create_assessment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
    Json(input): Json<CreateLiveAssessmentInput>,
) -> Response {
    let course = match course_reference(&course) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .create_live_assessment(token, course, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::CREATED, &v),
        Err(e) => store_error(e),
    }
}
async fn load_assessment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .load_live_assessment(token, course, assessment)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assessment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
    Json(mut input): Json<SaveLiveAssessmentInput>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let expected = match edit_header(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    input.expected_edit_number = expected;
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .save_live_assessment(token, course, assessment, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assessment_inline(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
    Json(input): Json<SaveLiveAssessmentInlineInput>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match edit_header(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assessments
        .save_live_assessment_inline(token, course, assessment, expected, input)
        .await
    {
        Ok(value) => summary_response(&value),
        Err(error_value) => store_error(error_value),
    }
}
async fn save_base_assessment_policy(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
    Json(mut input): Json<SaveBaseAssessmentPolicyInput>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match edit_header(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    input.expected_edit_number = expected;
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assessments
        .save_base_assessment_policy(token, course, assessment, input)
        .await
    {
        Ok(value) => workspace_response(StatusCode::OK, &value),
        Err(error_value) => store_error(error_value),
    }
}
async fn validate_release(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .validate_live_assessment_release(token, course, assessment)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn release_assessment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let expected = match edit_header(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assessments
        .release_live_assessment(token, course, assessment, expected)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}

/// Returns the released-only, aggregate-only confirmation projection for the
/// Assessment Properties Danger Zone.  It deliberately contains no Student
/// identifiers, responses, or grades.
async fn unrelease_impact(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assessments
        .read_live_assessment_unrelease_impact(token, course, assessment)
        .await
    {
        // ASVS 8.2.2 and 8.3.1: the Store procedure applies the current
        // Course-membership authorization before this aggregate is disclosed.
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(error_value) => store_error(error_value),
    }
}

/// Atomically deletes rooted Student Work and restores a Released Assessment
/// to Unreleased after a strong ETag and exact-title confirmation.
async fn unrelease_assessment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
    Json(input): Json<UnreleaseAssessmentInput>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match edit_header(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assessments
        .unrelease_live_assessment(
            token,
            course,
            assessment,
            expected,
            input.confirmation_title,
        )
        .await
    {
        // ASVS 2.3.1, 2.3.3, and 8.3.1: the Store invokes the single guarded
        // database transaction, which owns authorization, locking, deletion,
        // statistics rebuild, and the redacted audit receipt.
        Ok(value) => unreleased_response(&value),
        Err(error_value) => store_error(error_value),
    }
}

fn workspace_response(
    status: StatusCode,
    value: &learning_data_access::LiveAssessmentWorkspace,
) -> Response {
    let mut response = crate::auth::no_store((status, Json(value)).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn summary_response(value: &learning_data_access::CourseAssessmentSummary) -> Response {
    let mut response = crate::auth::no_store(Json(value).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn unreleased_response(value: &learning_data_access::UnreleasedLiveAssessment) -> Response {
    let mut response = crate::auth::no_store(Json(value).into_response());
    if let Ok(header) = format!("\"{}\"", value.assessment.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}

fn course_reference(value: &str) -> Result<CourseInstanceReference, Box<Response>> {
    CourseInstanceReference::from_str(value).map_err(|_| Box::new(concealed()))
}
fn refs(
    course_value: &str,
    assessment_value: &str,
) -> Result<(CourseInstanceReference, AssessmentReference), Box<Response>> {
    Ok((
        course_reference(course_value)?,
        AssessmentReference::from_str(assessment_value).map_err(|_| Box::new(concealed()))?,
    ))
}
fn edit_header(headers: &HeaderMap) -> Result<AssessmentEditNumber, Box<Response>> {
    let value = headers
        .get(IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix('"').and_then(|x| x.strip_suffix('"')))
        .ok_or_else(|| Box::new(concealed()))?;
    AssessmentEditNumber::from_str(value).map_err(|_| Box::new(concealed()))
}
async fn instructor(
    state: &StateData,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(v) if v.record.product_role == ProductRole::Instructor => Ok(v.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Assessment Workspace authentication unavailable",
        ))),
    }
}
fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|v| v.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}
fn store_error(error_value: StoreError) -> Response {
    match error_value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => error(
            StatusCode::PRECONDITION_FAILED,
            "Assessment Workspace changed",
        ),
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Assessment Workspace lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assessment Workspace is invalid",
        ),
        StoreError::AlreadyExists => error(StatusCode::CONFLICT, "Assessment Workspace conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Assessment Workspace unavailable",
        ),
    }
}
fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Assessment Workspace not found")
}
fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use learning_data_access::StoreError;

    use super::UnreleaseAssessmentInput;

    #[test]
    fn lifecycle_and_edit_number_conflicts_have_distinct_statuses() {
        assert_eq!(
            super::store_error(StoreError::Conflict).status(),
            StatusCode::PRECONDITION_FAILED
        );
        assert_eq!(
            super::store_error(StoreError::LifecycleConflict).status(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn unrelease_confirmation_accepts_only_the_closed_title_payload() {
        let input: UnreleaseAssessmentInput =
            serde_json::from_str(r#"{"confirmationTitle":"M10 quiz"}"#)
                .expect("valid exact confirmation payload");
        assert_eq!(input.confirmation_title.as_str(), "M10 quiz");
        assert!(
            serde_json::from_str::<UnreleaseAssessmentInput>(
                r#"{"confirmationTitle":"M10 quiz","attemptCount":0}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<UnreleaseAssessmentInput>(r#"{"confirmationTitle":"   "}"#)
                .is_err()
        );
    }
}
