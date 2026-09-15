//! Active-Instructor creation of one reusable Published Question Pool.
//!
//! This route owns no Pool selection policy, backend behavior, Pool UI, or
//! client-chosen identifier. It accepts only ordered exact Published Question
//! Revision references and the Instructor's interchangeability attestation.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    CreateQuestionPoolError, CreateQuestionPoolInput, QuestionPoolCreationStore, SessionTokenHash,
    StoreError,
    postgres::{PostgresQuestionPoolCreationStore, PostgresSessionStore},
};
use question_model::{
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, ProductRole, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{HmacQuestionIdIssuer, QuestionIdIssuer},
};

const MAX_CREATE_QUESTION_POOL_BYTES: usize = 128 * 1024;
const QUESTION_POOL_IDENTITY_ATTEMPTS: usize = 8;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: Arc<dyn QuestionPoolCreationStore>,
    question_id_issuer: Arc<dyn QuestionPoolIdIssuer>,
}

/// Trusted issuance and verification capability for Pool references.
///
/// Production supplies the deployment HMAC issuer. Keeping this capability at
/// the server boundary lets an isolated proof force the otherwise improbable
/// collision branch without making any browser-controlled value an issuer.
trait QuestionPoolIdIssuer: Send + Sync {
    fn issue_question_pool_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError>;
    fn validates_question_pool_id(&self, value: &QuestionId) -> bool;
}

impl QuestionPoolIdIssuer for HmacQuestionIdIssuer {
    fn issue_question_pool_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError> {
        self.issue_question_id()
    }

    fn validates_question_pool_id(&self, value: &QuestionId) -> bool {
        self.validates_question_id(value)
    }
}

/// Registers the narrow reusable Question Pool creation command.
pub fn question_pool_creation_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolCreationStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    question_pool_creation_router_with_trusted_dependencies(
        sessions,
        Arc::new(pools),
        Arc::new(question_id_issuer),
    )
}

fn question_pool_creation_router_with_trusted_dependencies(
    sessions: Arc<PostgresSessionStore>,
    pools: Arc<dyn QuestionPoolCreationStore>,
    question_id_issuer: Arc<dyn QuestionPoolIdIssuer>,
) -> Router {
    Router::new()
        .route("/api/question-pools", post(create_question_pool))
        .with_state(RouteState {
            sessions,
            pools,
            question_id_issuer,
        })
}

/// One browser-selected exact Published Question Revision, in Pool order.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuestionPoolMemberRequest {
    question_id: String,
    revision_number: u32,
}

/// Closed browser request for one new reusable Question Pool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateQuestionPoolRequest {
    members: Vec<QuestionPoolMemberRequest>,
    interchangeability_attested: bool,
}

/// Answer-free confirmation of a created immutable first Pool Revision.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedQuestionPoolResponse {
    question_pool_id: String,
    revision_number: u64,
}

async fn create_question_pool(State(state): State<RouteState>, request: Request) -> Response {
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool requires JSON",
        );
    }
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let bytes = match to_bytes(request.into_body(), MAX_CREATE_QUESTION_POOL_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Question Pool is too large"),
    };
    let request = match serde_json::from_slice::<CreateQuestionPoolRequest>(&bytes) {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Pool is invalid"),
    };
    let members = match verified_members(state.question_id_issuer.as_ref(), request.members) {
        Some(value) => value,
        None => return concealed(),
    };
    if members.is_empty()
        || members.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
        || !request.interchangeability_attested
    {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Pool is invalid");
    }

    for _ in 0..QUESTION_POOL_IDENTITY_ATTEMPTS {
        let public_question_pool_id = match state.question_id_issuer.issue_question_pool_id() {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Question Pool is unavailable",
                );
            }
        };
        let input = CreateQuestionPoolInput {
            question_pool_id: Uuid::now_v7(),
            public_question_pool_id,
            members: members.clone(),
            interchangeability_attested: true,
        };
        match state
            .pools
            .create_question_pool(session.clone(), input)
            .await
        {
            Ok(created) => {
                return crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(CreatedQuestionPoolResponse {
                            question_pool_id: created.public_question_pool_id.to_string(),
                            revision_number: created.revision_number,
                        }),
                    )
                        .into_response(),
                );
            }
            Err(CreateQuestionPoolError::IdentityCollision) => continue,
            Err(CreateQuestionPoolError::Store(error)) => return store_error_response(error),
        }
    }
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Pool is unavailable",
    )
}

fn verified_members(
    issuer: &dyn QuestionPoolIdIssuer,
    values: Vec<QuestionPoolMemberRequest>,
) -> Option<Vec<QuestionRevisionReference>> {
    let mut members = Vec::with_capacity(values.len());
    for value in values {
        let question_id = value.question_id.parse::<QuestionId>().ok()?;
        if !issuer.validates_question_pool_id(&question_id) {
            return None;
        }
        let revision_number = QuestionRevisionNumber::new(value.revision_number).ok()?;
        members.push(QuestionRevisionReference {
            question_id,
            revision_number,
        });
    }
    Some(members)
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool authentication is unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(';').next().is_some_and(|media_type| {
                media_type.trim().eq_ignore_ascii_case("application/json")
            })
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Pool is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Pool changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Pool conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Pool is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Pool not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
