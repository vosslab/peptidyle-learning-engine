//! Published Pool detail, Bloom correction, and owned fork detail routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    PublishedQuestionPool, QuestionLibraryStore, QuestionPoolLibraryStore, SessionTokenHash,
    StoreError,
    postgres::{
        PostgresQuestionLibraryStore, PostgresQuestionPoolLibraryStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssessmentEntryId, AssessmentId, AssessmentQuestionPoolForkView,
    BloomClassificationCorrectionRequest, CourseInstanceId, QuestionPoolBloomCorrectionReceipt,
    QuestionPoolId, QuestionPoolMemberView, QuestionPoolView, UserRole,
};

use crate::{
    auth::{AuthError, resolve_session},
    question_library::answer_free_reusable_question_view,
};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
}

pub fn question_pool_library_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route("/api/question-pools/{question_pool_id}", get(current_pool))
        .route(
            "/api/question-pools/{question_pool_id}/bloom",
            post(correct_pool_bloom),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/question-pool-forks/{assessment_entry_id}",
            get(assessment_fork),
        )
        .with_state(RouteState {
            sessions,
            pools,
            questions,
            objects,
        })
}

async fn current_pool(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(question_pool_id): Path<String>,
) -> Response {
    let pool_id = match verified_id(&question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let (token, is_instructor) = match library_reader(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision = match state
        .pools
        .load_current_published_question_pool(token, &pool_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    match pool_view(&state, token, is_instructor, revision).await {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(response) => response,
    }
}

async fn correct_pool_bloom(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(question_pool_id): Path<String>,
    payload: Result<Json<BloomClassificationCorrectionRequest>, JsonRejection>,
) -> Response {
    let pool_id = match verified_id(&question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    // ASVS 8.2.1/8.3.1: correction is available to every active
    // Instructor and never to the read-only Sysadmin Pool surface.
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Json(request) = match payload {
        Ok(value) => value,
        Err(_) => {
            return response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bloom correction is invalid",
            );
        }
    };
    let bloom = match state
        .pools
        .correct_question_pool_bloom(
            token,
            &pool_id,
            request.expected_classification_edit_number,
            request.cognitive_process,
            request.knowledge_dimension,
        )
        .await
    {
        Ok(value) => value,
        Err(StoreError::RetryableTransaction | StoreError::Conflict) => {
            return response(
                StatusCode::PRECONDITION_FAILED,
                "Question Pool classification changed",
            );
        }
        Err(StoreError::InvalidRecord(_)) => {
            return response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bloom correction is invalid",
            );
        }
        Err(error) => return store_error(error),
    };
    let pin = match state
        .pools
        .load_current_published_question_pool(token, &pool_id)
        .await
    {
        Ok(value) => value.question_pool_id,
        Err(error) => return store_error(error),
    };
    crate::auth::no_store(
        Json(QuestionPoolBloomCorrectionReceipt {
            question_pool_id: pin,
            bloom,
        })
        .into_response(),
    )
}

async fn assessment_fork(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((course, assessment, entry)): Path<(String, String, String)>,
) -> Response {
    let course = match course.parse::<CourseInstanceId>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let assessment = match assessment.parse::<AssessmentId>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let entry = match uuid::Uuid::parse_str(&entry) {
        Ok(value) => AssessmentEntryId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let record = match state
        .pools
        .load_assessment_question_pool_fork(token, course, assessment, entry)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let members = match pool_members(&state, token, true, record.members).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    crate::auth::no_store(
        Json(AssessmentQuestionPoolForkView {
            metadata: record.metadata,
            assessment_entry_id: record.assessment_entry_id,
            question_pool_id: record.question_pool_id,
            owner_account_id: record.owner_account_id,
            question_type: record.question_type,
            backend: record.backend,
            license: record.license,
            question_pool_edit_number: record.question_pool_edit_number,
            selection_count: record.selection_count,
            bloom: record.bloom,
            members,
        })
        .into_response(),
    )
}

// Route handlers return these errors immediately; boxing them would add an
// allocation and require every handler to unwrap solely to preserve Axum's
// `Response` return type.
#[allow(clippy::result_large_err)]
async fn pool_view(
    state: &RouteState,
    token: SessionTokenHash,
    is_instructor: bool,
    pool: PublishedQuestionPool,
) -> Result<QuestionPoolView, Response> {
    let evidence = pool_evidence(state, token, is_instructor, &pool.question_pool_id).await?;
    let members = pool_members(state, token, is_instructor, pool.members).await?;
    Ok(QuestionPoolView {
        metadata: pool.metadata,
        question_pool_id: pool.question_pool_id,
        owner_account_id: pool.owner_account_id,
        question_type: pool.question_type,
        backend: pool.backend,
        license: pool.license,
        question_pool_edit_number: pool.question_pool_edit_number,
        bloom: pool.bloom,
        members,
        evidence,
    })
}

#[allow(clippy::result_large_err)]
async fn pool_evidence(
    state: &RouteState,
    token: SessionTokenHash,
    is_instructor: bool,
    question_pool_id: &QuestionPoolId,
) -> Result<question_model::QuestionStatistics, Response> {
    if !is_instructor {
        return Ok(question_model::QuestionStatistics::Unavailable);
    }
    let (pool_issued_count, pool_issued_contributor_floor, totals) = state
        .pools
        .load_question_pool_usage_statistics(token, question_pool_id)
        .await
        .map_err(store_error)?;
    Ok(totals.into_shared_statistics(
        None,
        Some((pool_issued_count, pool_issued_contributor_floor)),
    ))
}

#[allow(clippy::result_large_err)]
async fn pool_members(
    state: &RouteState,
    token: SessionTokenHash,
    is_instructor: bool,
    members: Vec<question_model::PublishedQuestionRevisionTuple>,
) -> Result<Vec<QuestionPoolMemberView>, Response> {
    let question_ids = members
        .iter()
        .map(|member| member.published_question_id.clone())
        .collect::<Vec<_>>();
    let evidence = crate::question_library::bulk_question_statistics(
        &state.questions,
        token,
        is_instructor,
        &question_ids,
    )
    .await?;
    let mut views = Vec::with_capacity(members.len());
    for (position, member) in members.into_iter().enumerate() {
        let entry = state
            .questions
            .load_published_question_revision_library_entry(token, &member)
            .await
            .map_err(store_error)?;
        let member_evidence =
            crate::question_library::evidence_for(&member.published_question_id, &evidence);
        let question = answer_free_reusable_question_view(&state.objects, entry, member_evidence)
            .await
            .map_err(|_| unavailable())?;
        views.push(QuestionPoolMemberView {
            member_position: u32::try_from(position).map_err(|_| unavailable())?,
            published_question_revision_tuple: member,
            question,
        });
    }
    Ok(views)
}

fn verified_id(value: &str) -> Option<QuestionPoolId> {
    value.parse().ok()
}

async fn instructor(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|values| !values.is_empty())
        .map(|values| values.join("; "));
    match resolve_session(state.sessions.as_ref(), cookies.as_deref()).await {
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

async fn library_reader(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<(SessionTokenHash, bool), Box<Response>> {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|values| !values.is_empty())
        .map(|values| values.join("; "));
    match resolve_session(state.sessions.as_ref(), cookies.as_deref()).await {
        Ok(session)
            if matches!(
                session.record.user_role,
                UserRole::Instructor | UserRole::Sysadmin
            ) =>
        {
            Ok((
                session.session_hash,
                session.record.user_role == UserRole::Instructor,
            ))
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden => concealed(),
        _ => unavailable(),
    }
}

fn concealed() -> Response {
    response(StatusCode::NOT_FOUND, "Question Pool unavailable")
}

fn unavailable() -> Response {
    response(StatusCode::SERVICE_UNAVAILABLE, "Question Pool unavailable")
}

fn response(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unreachable_pool() -> learning_data_access::postgres::Pool {
        learning_data_access::postgres::lazy_pool("postgres://ple:ple@127.0.0.1:1/ple")
            .expect("lazy pool")
    }

    fn unreachable_objects() -> S3ObjectStore {
        use objects::minio::{EndpointConfig, client};
        use objects::s3::BucketNames;
        S3ObjectStore::new(
            client(&EndpointConfig {
                endpoint_url: "http://127.0.0.1:1".to_string(),
                region: "us-east-1".to_string(),
                access_key_id: "unused".to_string(),
                secret_access_key: "unused".to_string(),
            }),
            BucketNames {
                public_assets: "public".to_string(),
                private_content: "private".to_string(),
                student_records: "student".to_string(),
                temp_processing: "temp".to_string(),
            },
        )
    }

    #[tokio::test]
    async fn current_pool_rejects_a_bad_checksum_before_database_lookup() {
        let pool = unreachable_pool();
        let state = RouteState {
            sessions: Arc::new(PostgresSessionStore::new(pool.clone())),
            pools: PostgresQuestionPoolLibraryStore::new(pool.clone()),
            questions: PostgresQuestionLibraryStore::new(pool.clone()),
            objects: unreachable_objects(),
        };
        let response = current_pool(
            State(state),
            HeaderMap::new(),
            Path("0000-N00N".to_string()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
