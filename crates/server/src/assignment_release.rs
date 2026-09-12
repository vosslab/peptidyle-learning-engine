//! Direct-Instructor Assignment Workspace routes.

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
    CreateLiveAssignmentInput, LiveAssignmentStore, SaveLiveAssignmentInlineInput,
    SaveLiveAssignmentInput, SessionTokenHash, StoreError,
    postgres::{PostgresLiveAssignmentStore, PostgresSessionStore},
};
use question_model::{
    AssignmentEditNumber, AssignmentEntry, AssignmentReference, AssignmentTitle,
    CourseInstanceReference, ProductRole, QuestionRevisionReference,
};
use serde::Deserialize;

use crate::auth::{AuthError, resolve_session};
use crate::question_publication::HmacQuestionIdIssuer;

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    assignments: PostgresLiveAssignmentStore,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// The closed confirmation payload for the irreversible Unrelease transition.
///
/// The title is intentionally parsed at the HTTP boundary; PostgreSQL repeats
/// the exact comparison while it holds the Assignment lock.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UnreleaseAssignmentInput {
    confirmation_title: AssignmentTitle,
}

/// Registers answer-free current Assignment authoring and immutable release routes.
pub fn assignment_release_router(
    sessions: Arc<PostgresSessionStore>,
    assignments: PostgresLiveAssignmentStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assignment-question-picker",
            get(list_picker),
        )
        .route(
            "/api/course-instances/{course}/assignment-source-choices",
            get(list_assignment_source_choices),
        )
        .route(
            "/api/course-instances/{course}/assignments",
            get(list_assignments).post(create_assignment),
        )
        .route("/api/assignments/due-soon", get(list_assignments_due_soon))
        .route(
            "/api/course-instances/{course}/assignments/{assignment}",
            get(load_assignment).put(save_assignment),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/inline",
            put(save_assignment_inline),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/release-validation",
            get(validate_release),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/preview",
            get(assignment_preview),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/release",
            post(release_assignment),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/unrelease-impact",
            get(unrelease_impact),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/unrelease",
            post(unrelease_assignment),
        )
        .with_state(StateData {
            sessions,
            assignments,
            question_id_issuer,
        })
}

async fn list_assignments_due_soon(State(state): State<StateData>, headers: HeaderMap) -> Response {
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state.assignments.list_assignments_due_soon(token).await {
        // ASVS 4.1.1 and 8.3.1: Axum serializes this closed JSON projection;
        // it includes no Student, Attempt, response, answer, or grading record.
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_assignments(
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
        .assignments
        .list_course_assignments(token, course)
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
        .assignments
        .list_assignment_question_picker(token, course)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}

async fn list_assignment_source_choices(
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
    match state
        .assignments
        .list_course_assignment_source_choices(token, course)
        .await
    {
        Ok(choices) => crate::auth::no_store(Json(choices).into_response()),
        Err(error) => store_error(error),
    }
}
async fn create_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(course): Path<String>,
    Json(input): Json<CreateLiveAssignmentInput>,
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
        .assignments
        .create_live_assignment(token, course, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::CREATED, &v),
        Err(e) => store_error(e),
    }
}
async fn load_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .load_live_assignment(token, course, assignment)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
    Json(mut input): Json<SaveLiveAssignmentInput>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let expected = match edit_header(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    // ASVS 2.2.1, 2.2.2, and 11.4.1: exact Question Revision references are
    // browser input.  Verify the server-held HMAC character before the Store
    // can resolve an ID, while retaining the route's concealed outcome.
    if !has_verified_question_references(&state.question_id_issuer, &input.entries) {
        return concealed();
    }
    input.expected_edit_number = expected;
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .save_live_assignment(token, course, assignment, input)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}
async fn save_assignment_inline(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
    Json(input): Json<SaveLiveAssignmentInlineInput>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
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
        .assignments
        .save_live_assignment_inline(token, course, assignment, expected, input)
        .await
    {
        Ok(value) => summary_response(&value),
        Err(error_value) => store_error(error_value),
    }
}
async fn validate_release(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .validate_live_assignment_release(token, course, assignment)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn assignment_preview(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let token = match instructor(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match state
        .assignments
        .load_live_assignment_preview(token, course, assignment)
        .await
    {
        Ok(v) => crate::auth::no_store(Json(v).into_response()),
        Err(e) => store_error(e),
    }
}
async fn release_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
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
        .assignments
        .release_live_assignment(token, course, assignment, expected)
        .await
    {
        Ok(v) => workspace_response(StatusCode::OK, &v),
        Err(e) => store_error(e),
    }
}

/// Returns the released-only, aggregate-only confirmation projection for the
/// Assignment Properties Danger Zone.  It deliberately contains no Student
/// identifiers, responses, or grades.
async fn unrelease_impact(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .assignments
        .read_live_assignment_unrelease_impact(token, course, assignment)
        .await
    {
        // ASVS 8.2.2 and 8.3.1: the Store procedure applies the current
        // Course-membership authorization before this aggregate is disclosed.
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(error_value) => store_error(error_value),
    }
}

/// Atomically deletes rooted Student Work and restores a Released Assignment
/// to Unreleased after a strong ETag and exact-title confirmation.
async fn unrelease_assignment(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
    Json(input): Json<UnreleaseAssignmentInput>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
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
        .assignments
        .unrelease_live_assignment(
            token,
            course,
            assignment,
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
    value: &learning_data_access::LiveAssignmentWorkspace,
) -> Response {
    let mut response = crate::auth::no_store((status, Json(value)).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn summary_response(value: &learning_data_access::CourseAssignmentSummary) -> Response {
    let mut response = crate::auth::no_store(Json(value).into_response());
    if let Ok(header) = format!("\"{}\"", value.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}
fn unreleased_response(value: &learning_data_access::UnreleasedLiveAssignment) -> Response {
    let mut response = crate::auth::no_store(Json(value).into_response());
    if let Ok(header) = format!("\"{}\"", value.assignment.edit_number.value()).parse() {
        response.headers_mut().insert(ETAG, header);
    }
    response
}

/// Accepts only Question IDs minted for this deployment in every Assignment
/// Entry variant.  PostgreSQL still authorizes the exact revision and current
/// availability; this server boundary keeps its HMAC capability private.
fn has_verified_question_references(
    question_id_issuer: &HmacQuestionIdIssuer,
    entries: &[AssignmentEntry],
) -> bool {
    entries.iter().all(|entry| match entry {
        AssignmentEntry::FixedQuestion(entry) => {
            has_verified_question_reference(question_id_issuer, &entry.reference)
        }
        AssignmentEntry::QuestionPool(entry) => entry
            .items
            .iter()
            .all(|item| has_verified_question_reference(question_id_issuer, &item.reference)),
    })
}

fn has_verified_question_reference(
    question_id_issuer: &HmacQuestionIdIssuer,
    reference: &QuestionRevisionReference,
) -> bool {
    question_id_issuer.validates_question_id(&reference.question_id)
}
fn course_reference(value: &str) -> Result<CourseInstanceReference, Box<Response>> {
    CourseInstanceReference::from_str(value).map_err(|_| Box::new(concealed()))
}
fn refs(
    course_value: &str,
    assignment_value: &str,
) -> Result<(CourseInstanceReference, AssignmentReference), Box<Response>> {
    Ok((
        course_reference(course_value)?,
        AssignmentReference::from_str(assignment_value).map_err(|_| Box::new(concealed()))?,
    ))
}
fn edit_header(headers: &HeaderMap) -> Result<AssignmentEditNumber, Box<Response>> {
    let value = headers
        .get(IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix('"').and_then(|x| x.strip_suffix('"')))
        .ok_or_else(|| Box::new(concealed()))?;
    AssignmentEditNumber::from_str(value).map_err(|_| Box::new(concealed()))
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
            "Assignment Workspace authentication unavailable",
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
            "Assignment Workspace changed",
        ),
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Assignment Workspace lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assignment Workspace is invalid",
        ),
        StoreError::AlreadyExists => error(StatusCode::CONFLICT, "Assignment Workspace conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Assignment Workspace unavailable",
            )
        }
    }
}
fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Assignment Workspace not found")
}
fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use learning_data_access::StoreError;
    use question_model::{
        AssignmentEntryAvailability, AssignmentEntryId, AssignmentEntryScoringRule,
        AssignmentPointValue, FixedQuestionAssignmentEntry, QuestionAttemptLimit,
        QuestionAttemptTimeLimit, QuestionPoolAssignmentEntry, QuestionPoolItem,
        QuestionPoolItemAvailability, QuestionPoolItemId, QuestionPoolSelectedQuestionOrder,
        QuestionPoolSelectionRule, QuestionRevisionNumber,
    };
    use uuid::Uuid;

    use super::{
        AssignmentEntry, HmacQuestionIdIssuer, QuestionRevisionReference, UnreleaseAssignmentInput,
        has_verified_question_references,
    };
    use crate::question_publication::{QuestionIdIssuer, QuestionIdSecret};

    fn issuer() -> HmacQuestionIdIssuer {
        HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([41; 32]))
    }

    fn reference(issuer: &HmacQuestionIdIssuer) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: issuer.issue_question_id().expect("question ID mints"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
        }
    }

    fn invalid_reference(reference: &QuestionRevisionReference) -> QuestionRevisionReference {
        let mut value = reference.question_id.to_string();
        let replacement = if value.ends_with('0') { '1' } else { '0' };
        value.pop();
        value.push(replacement);
        QuestionRevisionReference {
            question_id: value.parse().expect("syntactically valid Question ID"),
            revision_number: reference.revision_number,
        }
    }

    #[test]
    fn workspace_rejects_an_unverified_fixed_question_reference_before_store_access() {
        let issuer = issuer();
        let reference = invalid_reference(&reference(&issuer));
        let entries = vec![AssignmentEntry::FixedQuestion(
            FixedQuestionAssignmentEntry {
                id: AssignmentEntryId::from_uuid(Uuid::from_u128(1)),
                reference,
                points_possible: AssignmentPointValue::from_whole(1),
                availability: AssignmentEntryAvailability::Available,
                scoring_rule: AssignmentEntryScoringRule::Normal,
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            },
        )];

        assert!(!has_verified_question_references(&issuer, &entries));
    }

    #[test]
    fn workspace_rejects_an_unverified_question_pool_item_before_store_access() {
        let issuer = issuer();
        let valid = reference(&issuer);
        let entries = vec![AssignmentEntry::QuestionPool(QuestionPoolAssignmentEntry {
            id: AssignmentEntryId::from_uuid(Uuid::from_u128(2)),
            availability: AssignmentEntryAvailability::Available,
            scoring_rule: AssignmentEntryScoringRule::Normal,
            selection_count: 1,
            points_per_item: AssignmentPointValue::from_whole(1),
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
            },
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            items: vec![QuestionPoolItem {
                id: QuestionPoolItemId::from_uuid(Uuid::from_u128(3)),
                reference: invalid_reference(&valid),
                availability: QuestionPoolItemAvailability::Available,
            }],
        })];

        assert!(!has_verified_question_references(&issuer, &entries));
    }

    #[test]
    fn workspace_accepts_verified_fixed_and_pool_question_references() {
        let issuer = issuer();
        let fixed = reference(&issuer);
        let pooled = reference(&issuer);
        let entries = vec![
            AssignmentEntry::FixedQuestion(FixedQuestionAssignmentEntry {
                id: AssignmentEntryId::from_uuid(Uuid::from_u128(4)),
                reference: fixed,
                points_possible: AssignmentPointValue::from_whole(1),
                availability: AssignmentEntryAvailability::Available,
                scoring_rule: AssignmentEntryScoringRule::Normal,
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            }),
            AssignmentEntry::QuestionPool(QuestionPoolAssignmentEntry {
                id: AssignmentEntryId::from_uuid(Uuid::from_u128(5)),
                availability: AssignmentEntryAvailability::Available,
                scoring_rule: AssignmentEntryScoringRule::Normal,
                selection_count: 1,
                points_per_item: AssignmentPointValue::from_whole(1),
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                items: vec![QuestionPoolItem {
                    id: QuestionPoolItemId::from_uuid(Uuid::from_u128(6)),
                    reference: pooled,
                    availability: QuestionPoolItemAvailability::Available,
                }],
            }),
        ];

        assert!(has_verified_question_references(&issuer, &entries));
    }

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
        let input: UnreleaseAssignmentInput =
            serde_json::from_str(r#"{"confirmationTitle":"M10 quiz"}"#)
                .expect("valid exact confirmation payload");
        assert_eq!(input.confirmation_title.as_str(), "M10 quiz");
        assert!(
            serde_json::from_str::<UnreleaseAssignmentInput>(
                r#"{"confirmationTitle":"M10 quiz","attemptCount":0}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<UnreleaseAssignmentInput>(r#"{"confirmationTitle":"   "}"#)
                .is_err()
        );
    }
}
