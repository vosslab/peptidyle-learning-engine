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
    CreateLiveAssessmentInput, LiveAssessmentStore, SaveBaseAssessmentPolicyInput,
    SaveLiveAssessmentInlineInput, SaveLiveAssessmentInput, SessionTokenHash, StoreError,
    postgres::{PostgresLiveAssessmentStore, PostgresSessionStore},
};
use question_model::{
    AssessmentEditNumber, AssessmentEntry, AssessmentReference, AssessmentTitle,
    CourseInstanceReference, ProductRole, QuestionRevisionReference,
};
use serde::Deserialize;

use crate::auth::{AuthError, resolve_session};
use crate::question_publication::HmacQuestionIdIssuer;

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    assessments: PostgresLiveAssessmentStore,
    question_id_issuer: HmacQuestionIdIssuer,
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
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assessment-question-picker",
            get(list_picker),
        )
        .route(
            "/api/course-instances/{course}/assessment-source-choices",
            get(list_assessment_source_choices),
        )
        .route(
            "/api/course-instances/{course}/assessments",
            get(list_assessments).post(create_assessment),
        )
        .route("/api/assessments/due-soon", get(list_assessments_due_soon))
        .route(
            "/api/course-instances/{course}/assessments/{assessment}",
            get(load_assessment).put(save_assessment),
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
            question_id_issuer,
        })
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

async fn list_assessment_source_choices(
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
        .assessments
        .list_course_assessment_source_choices(token, course)
        .await
    {
        Ok(choices) => crate::auth::no_store(Json(choices).into_response()),
        Err(error) => store_error(error),
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

/// Accepts only Question IDs minted for this deployment in every Assessment
/// Entry variant.  PostgreSQL still authorizes the exact revision and current
/// availability; this server boundary keeps its HMAC capability private.
fn has_verified_question_references(
    question_id_issuer: &HmacQuestionIdIssuer,
    entries: &[AssessmentEntry],
) -> bool {
    entries.iter().all(|entry| match entry {
        AssessmentEntry::FixedQuestion(entry) => {
            has_verified_question_reference(question_id_issuer, &entry.reference)
        }
        // The Assessment-owned fork is already an immutable stored Pool
        // Revision. Its members are never re-supplied by this current-state
        // save, so validate only the fork's server-issued public identity.
        AssessmentEntry::QuestionPool(entry) => {
            question_id_issuer.validates_question_id(&entry.question_pool_revision.question_pool_id)
        }
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
    use question_model::{
        AssessmentEntryAvailability, AssessmentEntryId, AssessmentEntryScoringRule,
        AssessmentPointValue, FixedQuestionAssessmentEntry, QuestionAttemptLimit,
        QuestionAttemptTimeLimit, QuestionPoolAssessmentEntry, QuestionPoolRevisionNumber,
        QuestionPoolRevisionReference, QuestionPoolSelectedQuestionOrder,
        QuestionPoolSelectionRule, QuestionRevisionNumber,
    };
    use std::num::NonZeroU32;
    use uuid::Uuid;

    use super::{
        AssessmentEntry, HmacQuestionIdIssuer, QuestionRevisionReference, UnreleaseAssessmentInput,
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
        let entries = vec![AssessmentEntry::FixedQuestion(
            FixedQuestionAssessmentEntry {
                id: AssessmentEntryId::from_uuid(Uuid::from_u128(1)),
                reference,
                points_possible: AssessmentPointValue::from_whole(1),
                availability: AssessmentEntryAvailability::Available,
                scoring_rule: AssessmentEntryScoringRule::Normal,
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            },
        )];

        assert!(!has_verified_question_references(&issuer, &entries));
    }

    #[test]
    fn workspace_rejects_an_unverified_question_pool_fork_before_store_access() {
        let issuer = issuer();
        let valid = issuer.issue_question_id().expect("Pool ID mints");
        let invalid_pool_id = invalid_reference(&QuestionRevisionReference {
            question_id: valid,
            revision_number: QuestionRevisionNumber::new(1).expect("revision"),
        })
        .question_id;
        let entries = vec![AssessmentEntry::QuestionPool(QuestionPoolAssessmentEntry {
            id: AssessmentEntryId::from_uuid(Uuid::from_u128(2)),
            question_pool_revision: QuestionPoolRevisionReference {
                question_pool_id: invalid_pool_id,
                revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
            },
            availability: AssessmentEntryAvailability::Available,
            scoring_rule: AssessmentEntryScoringRule::Normal,
            selection_count: NonZeroU32::new(1).expect("positive selection count"),
            points_per_item: AssessmentPointValue::from_whole(1),
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
            },
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        })];

        assert!(!has_verified_question_references(&issuer, &entries));
    }

    #[test]
    fn workspace_accepts_verified_fixed_and_pool_question_references() {
        let issuer = issuer();
        let fixed = reference(&issuer);
        let pooled = issuer.issue_question_id().expect("Pool ID mints");
        let entries = vec![
            AssessmentEntry::FixedQuestion(FixedQuestionAssessmentEntry {
                id: AssessmentEntryId::from_uuid(Uuid::from_u128(4)),
                reference: fixed,
                points_possible: AssessmentPointValue::from_whole(1),
                availability: AssessmentEntryAvailability::Available,
                scoring_rule: AssessmentEntryScoringRule::Normal,
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            }),
            AssessmentEntry::QuestionPool(QuestionPoolAssessmentEntry {
                id: AssessmentEntryId::from_uuid(Uuid::from_u128(5)),
                question_pool_revision: QuestionPoolRevisionReference {
                    question_pool_id: pooled,
                    revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
                },
                availability: AssessmentEntryAvailability::Available,
                scoring_rule: AssessmentEntryScoringRule::Normal,
                selection_count: NonZeroU32::new(1).expect("positive selection count"),
                points_per_item: AssessmentPointValue::from_whole(1),
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
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
