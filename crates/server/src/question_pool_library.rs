//! Published Pool detail, ordinary metadata, and owned fork routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
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
    AccountId, MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, QuestionPoolId,
    QuestionPoolMemberView, QuestionPoolView, SaveQuestionPoolMembersRequest,
    SaveQuestionPoolMetadataRequest, UserRole,
};

use crate::{
    auth::{AuthError, resolve_session},
    question_library::answer_free_reusable_question_view,
};

const MAX_QUESTION_POOL_METADATA_REQUEST_BYTES: usize = 64 * 1024;
const MAX_QUESTION_POOL_MEMBERS_REQUEST_BYTES: usize = 160 * 1024;

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
            "/api/question-pools/{question_pool_id}/metadata",
            get(current_pool_metadata).put(save_question_pool_metadata),
        )
        .route(
            "/api/question-pools/{question_pool_id}/members",
            axum::routing::put(save_question_pool_members),
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
    let viewer = match library_reader(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision = match state
        .pools
        .load_current_published_question_pool(viewer.session_hash, &pool_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    match pool_view(&state, &viewer, revision).await {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(response) => response,
    }
}

async fn current_pool_metadata(
    State(state): State<RouteState>,
    Path(question_pool_id): Path<String>,
    request: Request,
) -> Response {
    let pool_id = match verified_id(&question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    if request.uri().query().is_some() {
        return response(StatusCode::BAD_REQUEST, "Invalid Pool metadata query");
    }
    let headers = request.headers().clone();
    if to_bytes(request.into_body(), 0).await.is_err() {
        return response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Invalid Pool metadata request",
        );
    }
    let viewer = match library_reader(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .pools
        .load_current_question_pool_metadata(viewer.session_hash, &pool_id)
        .await
    {
        Ok(metadata) => crate::auth::no_store(Json(metadata).into_response()),
        Err(error) => store_error(error),
    }
}

async fn save_question_pool_metadata(
    State(state): State<RouteState>,
    Path(question_pool_id): Path<String>,
    request: Request,
) -> Response {
    let pool_id = match verified_id(&question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    if !has_json_content_type(request.headers()) {
        return response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Invalid Pool metadata");
    }
    if request.uri().query().is_some() {
        return response(StatusCode::BAD_REQUEST, "Invalid Pool metadata query");
    }
    let viewer = match library_reader(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let body = match to_bytes(
        request.into_body(),
        MAX_QUESTION_POOL_METADATA_REQUEST_BYTES,
    )
    .await
    {
        Ok(body) => body,
        Err(_) => return response(StatusCode::PAYLOAD_TOO_LARGE, "Invalid Pool metadata"),
    };
    let save = match serde_json::from_slice::<SaveQuestionPoolMetadataRequest>(&body) {
        Ok(value) => value,
        Err(_) => return response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool metadata"),
    };
    if save.question_pool_id != pool_id {
        return response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool metadata");
    }
    match state
        .pools
        .save_question_pool_metadata(viewer.session_hash, save)
        .await
    {
        Ok(receipt) => crate::auth::no_store(Json(receipt).into_response()),
        Err(error) => metadata_store_error(error),
    }
}

async fn save_question_pool_members(
    State(state): State<RouteState>,
    Path(question_pool_id): Path<String>,
    request: Request,
) -> Response {
    let pool_id = match verified_id(&question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    if !has_json_content_type(request.headers()) {
        return response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Invalid Pool members");
    }
    if request.uri().query().is_some() {
        return response(StatusCode::BAD_REQUEST, "Invalid Pool members query");
    }
    let headers = request.headers().clone();
    let viewer = match library_reader(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.5/2.2: cap the body before closed-model decoding; malformed bodies receive generic errors.
    let body = match to_bytes(request.into_body(), MAX_QUESTION_POOL_MEMBERS_REQUEST_BYTES).await {
        Ok(body) => body,
        Err(_) => return response(StatusCode::PAYLOAD_TOO_LARGE, "Invalid Pool members"),
    };
    let save = match serde_json::from_slice::<SaveQuestionPoolMembersRequest>(&body) {
        Ok(value) => value,
        Err(_) => return response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool members"),
    };
    if save.question_pool_id != pool_id
        || save.members.is_empty()
        || save.members.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
    {
        return response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool members");
    }
    let current = match state
        .pools
        .load_current_published_question_pool(viewer.session_hash, &pool_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    // ASVS 1.2/4.1: bind writes to the Pool owner or explicit Sysadmin authority at the server boundary.
    if !may_edit_pool_members(&viewer, &current.owner_account_id) {
        return concealed();
    }
    match state
        .pools
        .save_question_pool_members(viewer.session_hash, save)
        .await
    {
        Ok(receipt) => crate::auth::no_store(Json(receipt).into_response()),
        Err(error) => members_store_error(error),
    }
}

fn may_edit_pool_members(viewer: &LibraryViewer, owner: &AccountId) -> bool {
    viewer.role == UserRole::Sysadmin
        || (viewer.role == UserRole::Instructor && viewer.account_id == *owner)
}

// Route handlers return these errors immediately; boxing them would add an
// allocation and require every handler to unwrap solely to preserve Axum's
// `Response` return type.
#[allow(clippy::result_large_err)]
async fn pool_view(
    state: &RouteState,
    viewer: &LibraryViewer,
    pool: PublishedQuestionPool,
) -> Result<QuestionPoolView, Response> {
    let is_instructor = viewer.role == UserRole::Instructor;
    let can_edit_metadata =
        viewer.role == UserRole::Sysadmin || viewer.account_id == pool.owner_account_id;
    let evidence = pool_evidence(
        state,
        viewer.session_hash,
        is_instructor,
        &pool.question_pool_id,
    )
    .await?;
    let members = pool_members(state, viewer.session_hash, is_instructor, pool.members).await?;
    Ok(QuestionPoolView {
        metadata: pool.metadata,
        question_pool_id: pool.question_pool_id,
        owner_account_id: pool.owner_account_id,
        can_edit_metadata,
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
    for member in members {
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
            published_question_revision_tuple: member,
            question,
        });
    }
    Ok(views)
}

fn verified_id(value: &str) -> Option<QuestionPoolId> {
    value.parse().ok()
}

async fn library_reader(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<LibraryViewer, Box<Response>> {
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
            Ok(LibraryViewer {
                session_hash: session.session_hash,
                account_id: session.record.account_id(),
                role: session.record.user_role,
            })
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

#[derive(Clone)]
struct LibraryViewer {
    session_hash: SessionTokenHash,
    account_id: AccountId,
    role: UserRole,
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

fn store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden => concealed(),
        _ => unavailable(),
    }
}

fn metadata_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict => response(StatusCode::PRECONDITION_FAILED, "Pool metadata changed"),
        StoreError::InvalidRecord(_) => {
            response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool metadata")
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => unavailable(),
        _ => unavailable(),
    }
}

fn members_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            response(StatusCode::PRECONDITION_FAILED, "Pool membership changed")
        }
        StoreError::InvalidRecord(_) => {
            response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Pool members")
        }
        StoreError::Unavailable(_) => unavailable(),
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

    #[test]
    fn pool_member_write_requires_owner_or_sysadmin() {
        let owner = AccountId::from_debug_serial(1);
        let other = AccountId::from_debug_serial(2);
        let make_viewer = |account_id, role| LibraryViewer {
            session_hash: SessionTokenHash::compute(b"pool-members-test"),
            account_id,
            role,
        };

        assert!(may_edit_pool_members(
            &make_viewer(owner.clone(), UserRole::Instructor),
            &owner,
        ));
        assert!(!may_edit_pool_members(
            &make_viewer(other.clone(), UserRole::Instructor),
            &owner,
        ));
        assert!(may_edit_pool_members(
            &make_viewer(other, UserRole::Sysadmin),
            &owner,
        ));
        assert!(!may_edit_pool_members(
            &make_viewer(owner.clone(), UserRole::Student),
            &owner,
        ));
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
