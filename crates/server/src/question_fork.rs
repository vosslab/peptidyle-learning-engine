//! Active-Instructor command for forked private Draft Questions.
//!
//! The canonical path is the sole browser source selector. Its empty request
//! body deliberately cannot carry a Question ID, source bytes, attribution,
//! authorship, workspace, or Draft payload.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, HeaderName},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    ForkPublishedQuestionError, ForkPublishedQuestionInput, QuestionForkStore, SessionTokenHash,
    StoreError,
    postgres::{PostgresQuestionForkStore, PostgresSessionStore},
};
use question_model::{
    DraftQuestionReference, ProductRole, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{HmacQuestionIdIssuer, QuestionIdIssuer},
};

const QUESTION_FORK_IDENTITY_ATTEMPTS: usize = 8;
const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    question_id_issuer: Arc<dyn QuestionForkIdIssuer>,
}

/// Trusted issuance and validation capability for a fork's future Question ID.
trait QuestionForkIdIssuer: Send + Sync {
    fn issue_question_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError>;
    fn validates_question_id(&self, value: &QuestionId) -> bool;
}

impl QuestionForkIdIssuer for HmacQuestionIdIssuer {
    fn issue_question_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError> {
        QuestionIdIssuer::issue_question_id(self)
    }

    fn validates_question_id(&self, value: &QuestionId) -> bool {
        self.validates_question_id(value)
    }
}

/// Registers the canonical active-Instructor fork command.
pub fn question_fork_router(
    sessions: Arc<PostgresSessionStore>,
    forks: PostgresQuestionForkStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    question_fork_router_with_trusted_dependencies(
        sessions,
        Arc::new(forks),
        Arc::new(question_id_issuer),
    )
}

fn question_fork_router_with_trusted_dependencies(
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    question_id_issuer: Arc<dyn QuestionForkIdIssuer>,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/fork",
            post(fork_published_question),
        )
        .with_state(RouteState {
            sessions,
            forks,
            question_id_issuer,
        })
}

/// Answer-free navigation result for one distinct private Draft.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ForkedQuestionResponse {
    draft_question: DraftQuestionReference,
}

async fn fork_published_question(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    if !body.is_empty() {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid");
    }
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let idempotency_key = match idempotency_key(&headers) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Fork retry key is invalid",
            );
        }
    };
    let source_question_revision = match canonical_source(&state, question_id, revision_number) {
        Some(value) => value,
        None => return concealed(),
    };
    let source_question_revision = match state
        .forks
        .load_available_question_fork_source(session, &source_question_revision)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };

    for _ in 0..QUESTION_FORK_IDENTITY_ATTEMPTS {
        let forked_question_id = match state.question_id_issuer.issue_question_id() {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Question Fork is unavailable",
                );
            }
        };
        let input = ForkPublishedQuestionInput {
            source_question_revision: source_question_revision.clone(),
            forked_question_id,
            proposed_workspace_id: Uuid::now_v7(),
            proposed_draft_question_id: Uuid::now_v7(),
            idempotency_key,
        };
        match state
            .forks
            .fork_published_question_to_draft(session, input)
            .await
        {
            Ok(fork) => {
                return crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(ForkedQuestionResponse {
                            draft_question: fork.draft_question,
                        }),
                    )
                        .into_response(),
                );
            }
            Err(ForkPublishedQuestionError::IdentityCollision) => continue,
            Err(ForkPublishedQuestionError::Store(error)) => return store_error_response(error),
        }
    }
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Fork is unavailable",
    )
}

fn canonical_source(
    state: &RouteState,
    question_id: String,
    revision_number: String,
) -> Option<QuestionRevisionReference> {
    let question_id = question_id.parse::<QuestionId>().ok()?;
    if !state.question_id_issuer.validates_question_id(&question_id) {
        return None;
    }
    let revision_number = revision_number
        .parse::<u32>()
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())?;
    Some(QuestionRevisionReference {
        question_id,
        revision_number,
    })
}

fn idempotency_key(headers: &HeaderMap) -> Option<Uuid> {
    headers.get(&IDEMPOTENCY_KEY)?.to_str().ok()?.parse().ok()
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
            "Question Fork authentication is unavailable",
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

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Fork changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Fork conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Fork is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Fork not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
