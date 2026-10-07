//! Active-Instructor creation and explicit forking of reusable Question Pools.
//!
//! Creation records deliberate Pool Title/Description, selected Question
//! Revision Tuples, and optional Library Object tags. Forking starts from the
//! source Pool's members. Discipline and Subject remain database-derived.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    CreateQuestionPoolError, CreateQuestionPoolInput, QuestionPoolCreationStore,
    QuestionPoolForkStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresQuestionPoolCreationStore, PostgresQuestionPoolForkStore, PostgresSessionStore,
    },
};
use question_model::{
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, PublishedQuestionRevisionTuple, QuestionPoolId,
    UserRole,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{QuestionPoolIdIssuer, RandomQuestionIdIssuer},
};

const MAX_CREATE_QUESTION_POOL_BYTES: usize = 128 * 1024;
const QUESTION_POOL_IDENTITY_ATTEMPTS: usize = 8;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: Arc<dyn QuestionPoolCreationStore>,
    question_id_issuer: Arc<dyn QuestionPoolIdIssuer>,
}

#[derive(Clone)]
struct ForkRouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: Arc<dyn QuestionPoolForkStore>,
    question_id_issuer: Arc<dyn QuestionPoolIdIssuer>,
}

/// Registers the narrow reusable Question Pool creation command.
pub fn question_pool_creation_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolCreationStore,
    question_id_issuer: RandomQuestionIdIssuer,
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

/// Closed browser request for one new reusable Question Pool.
/// ASVS 1.5.2: explicit fields exclude browser-issued identities/classification.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateQuestionPoolRequest {
    title: String,
    description: String,
    members: Vec<PublishedQuestionRevisionTuple>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Answer-free confirmation of a created Question Pool.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedQuestionPoolResponse {
    question_pool_id: String,
    question_pool_edit_number: u64,
}

/// Registers the narrow explicit fork command for ordinary reusable Pools.
pub fn question_pool_fork_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolForkStore,
    question_id_issuer: RandomQuestionIdIssuer,
) -> Router {
    question_pool_fork_router_with_trusted_dependencies(
        sessions,
        Arc::new(pools),
        Arc::new(question_id_issuer),
    )
}

fn question_pool_fork_router_with_trusted_dependencies(
    sessions: Arc<PostgresSessionStore>,
    pools: Arc<dyn QuestionPoolForkStore>,
    question_id_issuer: Arc<dyn QuestionPoolIdIssuer>,
) -> Router {
    Router::new()
        .route(
            "/api/question-pools/{source_question_pool_id}/fork",
            post(fork_question_pool),
        )
        .with_state(ForkRouteState {
            sessions,
            pools,
            question_id_issuer,
        })
}

async fn fork_question_pool(
    State(state): State<ForkRouteState>,
    Path(source_id): Path<String>,
    request: Request,
) -> Response {
    let source_question_pool_id: QuestionPoolId = match source_id.parse() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let headers = request.headers().clone();
    let bytes = match to_bytes(request.into_body(), 0).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Pool fork is invalid",
            );
        }
    };
    if !bytes.is_empty() {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question Pool fork is invalid",
        );
    }
    let session = match instructor_session_hash(state.sessions.as_ref(), &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };

    for _ in 0..QUESTION_POOL_IDENTITY_ATTEMPTS {
        let question_pool_id = match state.question_id_issuer.issue_question_pool_id() {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Question Pool is unavailable",
                );
            }
        };
        match state
            .pools
            .fork_question_pool(session, question_pool_id, source_question_pool_id.clone())
            .await
        {
            Ok(forked) => {
                return crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(CreatedQuestionPoolResponse {
                            question_pool_id: forked.question_pool_id.to_string(),
                            question_pool_edit_number: forked.edit_number,
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

async fn create_question_pool(State(state): State<RouteState>, request: Request) -> Response {
    if !has_json_content_type(request.headers()) {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Pool requires JSON",
        );
    }
    let session = match instructor_session_hash(state.sessions.as_ref(), request.headers()).await {
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
    let members = request.members;
    if members.is_empty() || members.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Pool is invalid");
    }

    for _ in 0..QUESTION_POOL_IDENTITY_ATTEMPTS {
        let question_pool_id = match state.question_id_issuer.issue_question_pool_id() {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Question Pool is unavailable",
                );
            }
        };
        let input = CreateQuestionPoolInput {
            title: request.title.clone(),
            description: request.description.clone(),
            question_pool_id,
            members: members.clone(),
            tags: request.tags.clone(),
        };
        if input.validate().is_err() {
            return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Pool is invalid");
        }
        match state.pools.create_question_pool(session, input).await {
            Ok(created) => {
                return crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(CreatedQuestionPoolResponse {
                            question_pool_id: created.question_pool_id.to_string(),
                            question_pool_edit_number: created.edit_number,
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

async fn instructor_session_hash(
    sessions: &PostgresSessionStore,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(sessions, joined_cookie_header(headers).as_deref()).await {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
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

#[cfg(test)]
mod tests {
    use super::CreateQuestionPoolRequest;

    #[test]
    fn creation_requires_deliberate_pool_text_and_refuses_client_classification() {
        let request = serde_json::json!({
            "title": "Interchangeable enzyme Questions",
            "description": "Questions assessing enzyme inhibition.",
            "members": [],
        });
        let parsed: CreateQuestionPoolRequest =
            serde_json::from_value(request.clone()).expect("closed creation request");
        assert_eq!(parsed.title, "Interchangeable enzyme Questions");
        assert_eq!(parsed.description, "Questions assessing enzyme inhibition.");
        for field in ["title", "description"] {
            let mut missing = request.clone();
            missing
                .as_object_mut()
                .expect("request object")
                .remove(field);
            assert!(serde_json::from_value::<CreateQuestionPoolRequest>(missing).is_err());
        }
        let mut classified = request;
        classified["disciplineUuid"] = serde_json::json!(uuid::Uuid::nil());
        assert!(serde_json::from_value::<CreateQuestionPoolRequest>(classified).is_err());
    }

    #[test]
    fn creation_accepts_exact_published_question_revision_tuple_members() {
        let request = serde_json::json!({
            "title": "Enzyme Questions",
            "description": "Questions assessing enzyme inhibition.",
            "members": [{
                "publishedQuestionId": "7K3M-79QP",
                "revisionNumber": 3
            }]
        });

        let parsed: CreateQuestionPoolRequest =
            serde_json::from_value(request).expect("canonical Published Question tuple");

        assert_eq!(
            serde_json::to_value(parsed.members).expect("members serialize"),
            serde_json::json!([{
                "publishedQuestionId": "7K3M-79QP",
                "revisionNumber": 3
            }])
        );
    }

    #[test]
    fn creation_rejects_invalid_or_unknown_published_question_tuple_members() {
        let invalid_members = [
            serde_json::json!({
                "questionId": "7K3M-79QP",
                "revisionNumber": 3
            }),
            serde_json::json!({
                "publishedQuestionId": "7K3M-79QP",
                "revisionNumber": 0
            }),
            serde_json::json!({
                "publishedQuestionId": "not-a-published-question-id",
                "revisionNumber": 3
            }),
            serde_json::json!({
                "publishedQuestionId": "7K3M-79QP",
                "revisionNumber": 3,
                "answer": "not part of a Question Revision Tuple"
            }),
        ];

        for member in invalid_members {
            let request = serde_json::json!({
                "title": "Enzyme Questions",
                "description": "Questions assessing enzyme inhibition.",
                "members": [member.clone()]
            });
            assert!(
                serde_json::from_value::<CreateQuestionPoolRequest>(request).is_err(),
                "accepted invalid member tuple: {member}"
            );
        }
    }
}
