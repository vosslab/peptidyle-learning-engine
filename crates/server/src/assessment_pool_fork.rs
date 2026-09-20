//! Closed Instructor commands for Assessment-owned immutable Question Pool forks.

use std::{num::NonZeroU32, sync::Arc};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{
        HeaderMap, StatusCode,
        header::{ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{post, put},
};
use learning_data_access::{
    AppendAssessmentPoolForkMembersInput, AssessmentPoolForkStore, ImportAssessmentPoolForkInput,
    SessionTokenHash, StoreError, postgres::PostgresAssessmentPoolForkStore,
};
use question_model::{
    AssessmentEditNumber, AssessmentEntryId, AssessmentEntryScoringRule, AssessmentId,
    AssessmentPointValue, CourseInstanceId, ProductRole, QuestionId, QuestionPoolEditNumber,
    QuestionPoolSelectedQuestionOrder, QuestionRevisionNumber, QuestionRevisionTuple,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{QuestionIdIssuer, RandomQuestionIdIssuer},
};

const MAX_POOL_FORK_REQUEST_BYTES: usize = 128 * 1024;
const POOL_IDENTITY_ATTEMPTS: usize = 8;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    forks: Arc<dyn AssessmentPoolForkStore>,
    issuer: RandomQuestionIdIssuer,
}

/// Registers only trusted import and append commands; ordinary Assessment saves
/// cannot create or rebind a Pool lineage.
pub fn assessment_pool_fork_router(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    forks: PostgresAssessmentPoolForkStore,
    issuer: RandomQuestionIdIssuer,
) -> Router {
    assessment_pool_fork_router_with_store(sessions, Arc::new(forks), issuer)
}

pub(crate) fn assessment_pool_fork_router_with_store(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    forks: Arc<dyn AssessmentPoolForkStore>,
    issuer: RandomQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/question-pool-forks",
            post(import_fork),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/question-pool-forks/{assessment_entry_id}",
            put(append_fork_revision),
        )
        .with_state(RouteState {
            sessions,
            forks,
            issuer,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportForkRequest {
    source_question_pool_id: String,
    authored_position: u32,
    selection_count: NonZeroU32,
    points_per_item: AssessmentPointValue,
    selected_question_order: QuestionPoolSelectedQuestionOrder,
    scoring_rule: AssessmentEntryScoringRule,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ForkMemberRequest {
    question_id: String,
    revision_number: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppendForkRequest {
    expected_question_pool_edit_number: QuestionPoolEditNumber,
    members: Vec<ForkMemberRequest>,
    interchangeability_attested: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedForkResponse {
    assessment_entry_id: AssessmentEntryId,
    question_pool_id: String,
    question_pool_edit_number: u64,
    assessment_edit_number: AssessmentEditNumber,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppendedForkResponse {
    assessment_entry_id: AssessmentEntryId,
    question_pool_edit_number: u64,
    assessment_edit_number: AssessmentEditNumber,
}

async fn import_fork(
    State(state): State<RouteState>,
    Path((course, assessment)): Path<(String, String)>,
    request: Request,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Some(value) => value,
        None => return concealed(),
    };
    let expected_assessment_edit_number = match edit_header(request.headers()) {
        Some(value) => value,
        None => return concealed(),
    };
    let token = match instructor(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match decode_json::<ImportForkRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let source_question_pool_id = match verified_question_id(&request.source_question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    for _ in 0..POOL_IDENTITY_ATTEMPTS {
        let fork_question_pool_id = match state.issuer.issue_question_id() {
            Ok(value) => value,
            Err(_) => return unavailable(),
        };
        let input = ImportAssessmentPoolForkInput {
            course: course.clone(),
            assessment: assessment.clone(),
            assessment_entry: AssessmentEntryId::from_uuid(Uuid::now_v7()),
            expected_assessment_edit_number,
            fork_question_pool_id,
            source_question_pool_id: source_question_pool_id.clone(),
            authored_position: request.authored_position,
            selection_count: request.selection_count,
            points_per_item: request.points_per_item,
            selected_question_order: request.selected_question_order,
            scoring_rule: request.scoring_rule,
        };
        match state
            .forks
            .import_assessment_question_pool_fork(token, input)
            .await
        {
            Ok(result) => {
                let mut response = crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(ImportedForkResponse {
                            assessment_entry_id: result.assessment_entry,
                            question_pool_id: result.question_pool_id.to_string(),
                            question_pool_edit_number: result.question_pool_edit_number.get(),
                            assessment_edit_number: result.assessment_edit_number,
                        }),
                    )
                        .into_response(),
                );
                if let Ok(header) = format!("\"{}\"", result.assessment_edit_number.value()).parse()
                {
                    response.headers_mut().insert(ETAG, header);
                }
                return response;
            }
            // The candidate entry and fork UUIDs are fresh, so the only expected
            // uniqueness race is the canonical public Pool identity.
            Err(StoreError::AlreadyExists) => continue,
            Err(error) => return store_error(error),
        }
    }
    unavailable()
}

async fn append_fork_revision(
    State(state): State<RouteState>,
    Path((course, assessment, entry)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Some(value) => value,
        None => return concealed(),
    };
    let entry = match Uuid::parse_str(&entry) {
        Ok(value) => AssessmentEntryId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let expected_assessment_edit_number = match edit_header(request.headers()) {
        Some(value) => value,
        None => return concealed(),
    };
    let token = match instructor(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match decode_json::<AppendForkRequest>(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let members = match verified_members(request.members) {
        Some(value) if !value.is_empty() && request.interchangeability_attested => value,
        _ => return invalid(),
    };
    match state
        .forks
        .append_assessment_question_pool_fork_members(
            token,
            AppendAssessmentPoolForkMembersInput {
                course,
                assessment,
                assessment_entry: entry,
                expected_assessment_edit_number,
                expected_question_pool_edit_number: request.expected_question_pool_edit_number,
                members,
                interchangeability_attested: true,
            },
        )
        .await
    {
        Ok(result) => {
            let mut response = crate::auth::no_store(
                (
                    StatusCode::OK,
                    Json(AppendedForkResponse {
                        assessment_entry_id: result.assessment_entry,
                        question_pool_edit_number: result.question_pool_edit_number.get(),
                        assessment_edit_number: result.assessment_edit_number,
                    }),
                )
                    .into_response(),
            );
            if let Ok(header) = format!("\"{}\"", result.assessment_edit_number.value()).parse() {
                response.headers_mut().insert(ETAG, header);
            }
            response
        }
        Err(error) => store_error(error),
    }
}

// Route handlers return decoding failures immediately; boxing would add an
// allocation and require every handler to unwrap solely to preserve Axum's
// `Response` return type.
#[allow(clippy::result_large_err)]
async fn decode_json<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Response> {
    let is_json = request
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        });
    if !is_json {
        return Err(crate::auth::no_store(
            (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Question Pool fork requires JSON",
            )
                .into_response(),
        ));
    }
    let body = to_bytes(request.into_body(), MAX_POOL_FORK_REQUEST_BYTES)
        .await
        .map_err(|_| {
            crate::auth::no_store(
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Question Pool fork is too large",
                )
                    .into_response(),
            )
        })?;
    serde_json::from_slice(&body).map_err(|_| invalid())
}

fn refs(course: &str, assessment: &str) -> Option<(CourseInstanceId, AssessmentId)> {
    Some((course.parse().ok()?, assessment.parse().ok()?))
}

fn verified_question_id(raw: &str) -> Option<QuestionId> {
    raw.parse().ok()
}

fn verified_members(values: Vec<ForkMemberRequest>) -> Option<Vec<QuestionRevisionTuple>> {
    values
        .into_iter()
        .map(|member| {
            Some(QuestionRevisionTuple {
                question_id: verified_question_id(&member.question_id)?,
                revision_number: QuestionRevisionNumber::new(member.revision_number).ok()?,
            })
        })
        .collect()
}

fn edit_header(headers: &HeaderMap) -> Option<AssessmentEditNumber> {
    headers
        .get(IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .and_then(|value| value.parse().ok())
}

async fn instructor(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(value) if value.record.product_role == ProductRole::Instructor => Ok(value.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all("cookie")
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => invalid(),
        StoreError::Conflict | StoreError::RetryableTransaction => crate::auth::no_store(
            (
                StatusCode::PRECONDITION_FAILED,
                "Question Pool fork changed",
            )
                .into_response(),
        ),
        StoreError::LifecycleConflict | StoreError::AlreadyExists => crate::auth::no_store(
            (StatusCode::CONFLICT, "Question Pool fork conflicts").into_response(),
        ),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => unavailable(),
    }
}

fn concealed() -> Response {
    crate::auth::no_store((StatusCode::NOT_FOUND, "Question Pool fork not found").into_response())
}
fn invalid() -> Response {
    crate::auth::no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question Pool fork is invalid",
        )
            .into_response(),
    )
}
fn unavailable() -> Response {
    crate::auth::no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool fork unavailable",
        )
            .into_response(),
    )
}
