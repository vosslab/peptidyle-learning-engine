//! Active-Instructor published Pool discovery and owned fork detail routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::Query;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    Cursor, PageRequest, PageSize, QuestionLibraryStore, QuestionPoolLibraryStore,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresQuestionLibraryStore, PostgresQuestionPoolLibraryStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssessmentEntryId, AssessmentQuestionPoolForkView, AssessmentReference,
    CourseInstanceReference, ProductRole, QuestionId, QuestionPoolLibrarySummary,
    QuestionPoolRevisionMemberView, QuestionPoolRevisionView,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    question_library::answer_free_reusable_question_view,
    question_publication::HmacQuestionIdIssuer,
};

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_CURSOR_BYTES: usize = 256;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    issuer: HmacQuestionIdIssuer,
}

pub fn question_pool_library_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route("/api/question-pools", get(list_pools))
        .route("/api/question-pools/{question_pool_id}", get(current_pool))
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/question-pool-forks/{entry}",
            get(assessment_fork),
        )
        .with_state(RouteState {
            sessions,
            pools,
            questions,
            objects,
            issuer,
        })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    page_size: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    items: Vec<QuestionPoolLibrarySummary>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PoolCursor {
    version: u8,
    after: String,
    page_size: u16,
}

async fn list_pools(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let page_size = match PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE)) {
        Ok(value) => value,
        Err(_) => return bad_request("Question Pool page size is invalid"),
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let after = match query.cursor {
        Some(value) => match decode_cursor(&state.issuer, &value, page_size.get()) {
            Some(value) => Some(value),
            None => return bad_request("Question Pool continuation is invalid"),
        },
        None => None,
    };
    let page = match state
        .pools
        .list_published_question_pools(
            token,
            PageRequest {
                after,
                size: page_size,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    if page.items.iter().any(|item| {
        !state
            .issuer
            .validates_question_id(&item.question_pool_revision.question_pool_id)
    }) {
        return unavailable();
    }
    let next_cursor = match page.next_cursor {
        Some(value) => match encode_cursor(value.as_str(), page_size.get()) {
            Some(value) => Some(value),
            None => return unavailable(),
        },
        None => None,
    };
    crate::auth::no_store(
        Json(ListResponse {
            items: page.items,
            next_cursor,
        })
        .into_response(),
    )
}

async fn current_pool(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(question_pool_id): Path<String>,
) -> Response {
    let pool_id = match verified_id(&state.issuer, &question_pool_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision = match state
        .pools
        .load_current_published_question_pool(token.clone(), &pool_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    match revision_view(
        &state,
        token,
        revision.question_pool_revision,
        revision.members,
    )
    .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(response) => response,
    }
}

async fn assessment_fork(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((course, assessment, entry)): Path<(String, String, String)>,
) -> Response {
    let course = match course.parse::<CourseInstanceReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let assessment = match assessment.parse::<AssessmentReference>() {
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
        .load_assessment_question_pool_fork(token.clone(), course, assessment, entry)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    if !state
        .issuer
        .validates_question_id(&record.question_pool_revision.question_pool_id)
    {
        return unavailable();
    }
    let revision = match revision_view(
        &state,
        token,
        record.question_pool_revision.clone(),
        record.members,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    crate::auth::no_store(
        Json(AssessmentQuestionPoolForkView {
            assessment_entry_id: record.assessment_entry_id,
            question_pool_revision: record.question_pool_revision,
            pool_metadata_etag: record.pool_metadata_etag,
            selection_count: record.selection_count,
            members: revision.members,
        })
        .into_response(),
    )
}

async fn revision_view(
    state: &RouteState,
    token: SessionTokenHash,
    question_pool_revision: question_model::QuestionPoolRevisionReference,
    members: Vec<question_model::QuestionRevisionReference>,
) -> Result<QuestionPoolRevisionView, Response> {
    let mut views = Vec::with_capacity(members.len());
    for (position, member) in members.into_iter().enumerate() {
        if !state.issuer.validates_question_id(&member.question_id) {
            return Err(unavailable());
        }
        let entry = state
            .questions
            .load_published_question_revision_library_entry(token.clone(), &member)
            .await
            .map_err(store_error)?;
        let question = answer_free_reusable_question_view(&state.objects, entry)
            .await
            .map_err(|_| unavailable())?;
        views.push(QuestionPoolRevisionMemberView {
            member_position: u32::try_from(position).map_err(|_| unavailable())?,
            question_revision: member,
            question,
        });
    }
    Ok(QuestionPoolRevisionView {
        question_pool_revision,
        members: views,
    })
}

fn verified_id(issuer: &HmacQuestionIdIssuer, value: &str) -> Option<QuestionId> {
    let id = value.parse().ok()?;
    issuer.validates_question_id(&id).then_some(id)
}

fn encode_cursor(after: &str, page_size: u16) -> Option<String> {
    serde_json::to_vec(&PoolCursor {
        version: 1,
        after: after.to_owned(),
        page_size,
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(issuer: &HmacQuestionIdIssuer, value: &str, page_size: u16) -> Option<Cursor> {
    if value.len() > MAX_CURSOR_BYTES {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    let cursor: PoolCursor = serde_json::from_slice(&decoded).ok()?;
    if cursor.version != 1 || cursor.page_size != page_size {
        return None;
    }
    let id = cursor.after.parse::<QuestionId>().ok()?;
    if !issuer.validates_question_id(&id) {
        return None;
    }
    Cursor::parse(id.to_string()).ok()
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
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
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

fn bad_request(message: &'static str) -> Response {
    response(StatusCode::BAD_REQUEST, message)
}

fn unavailable() -> Response {
    response(StatusCode::SERVICE_UNAVAILABLE, "Question Pool unavailable")
}

fn response(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use question_model::QuestionId;

    use super::*;
    use crate::question_publication::{QuestionIdIssuer, QuestionIdSecret};

    #[test]
    fn continuation_is_opaque_query_bound_and_hmac_validated() {
        let issuer = HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([7; 32]));
        let id = issuer.issue_question_id().expect("Pool ID");
        let encoded = encode_cursor(&id.to_string(), 25).expect("cursor encodes");
        assert_eq!(
            decode_cursor(&issuer, &encoded, 25)
                .expect("matching query cursor")
                .as_str(),
            id.to_string()
        );
        assert!(decode_cursor(&issuer, &encoded, 50).is_none());

        let wrong = QuestionId::from_canonical_parts(
            id.identifier_compact(),
            if id.validation_character() == '0' {
                '1'
            } else {
                '0'
            },
        )
        .expect("syntax-valid alternate HMAC character");
        let forged = encode_cursor(&wrong.to_string(), 25).expect("forged cursor encodes");
        assert!(decode_cursor(&issuer, &forged, 25).is_none());
    }
}
