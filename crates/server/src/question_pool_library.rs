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
    ContentClassificationStore, Cursor, PageRequest, PageSize, QuestionLibraryStore,
    QuestionPoolDiscoveryFilter, QuestionPoolLibraryStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresContentClassificationStore, PostgresQuestionLibraryStore,
        PostgresQuestionPoolLibraryStore, PostgresSessionStore,
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
const MAX_CURSOR_BYTES: usize = 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    classifications: PostgresContentClassificationStore,
    objects: S3ObjectStore,
    issuer: HmacQuestionIdIssuer,
}

pub fn question_pool_library_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    classifications: PostgresContentClassificationStore,
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
            classifications,
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
    #[serde(default)]
    discipline_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    subject_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    topic_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    subtopic_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    cross_discipline: bool,
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
    discipline_uuid: Option<uuid::Uuid>,
    subject_uuid: Option<uuid::Uuid>,
    topic_uuid: Option<uuid::Uuid>,
    subtopic_uuid: Option<uuid::Uuid>,
    cross_discipline: bool,
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
    let filter = QuestionPoolDiscoveryFilter {
        discipline_uuid: query.discipline_uuid,
        subject_uuid: query.subject_uuid,
        topic_uuid: query.topic_uuid,
        subtopic_uuid: query.subtopic_uuid,
        cross_discipline: query.cross_discipline,
    };
    // ASVS 2.2.2, 2.2.3: validate the hierarchy at the trusted service boundary.
    if !filter.has_valid_structure() {
        return bad_request("Question Pool classification hierarchy is invalid");
    }
    match valid_classification(&state.classifications, token.clone(), filter).await {
        Ok(true) => {}
        Ok(false) => return bad_request("Question Pool classification is invalid"),
        Err(error) => return store_error(error),
    }
    let after = match query.cursor {
        Some(value) => match decode_cursor(&state.issuer, &value, page_size.get(), filter) {
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
            filter,
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
        Some(value) => match encode_cursor(value.as_str(), page_size.get(), filter) {
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
        revision.metadata,
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
        record.metadata,
        record.members,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    crate::auth::no_store(
        Json(AssessmentQuestionPoolForkView {
            metadata: revision.metadata,
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
    metadata: question_model::QuestionPoolMetadata,
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
        metadata,
        question_pool_revision,
        members: views,
    })
}

async fn valid_classification(
    store: &impl ContentClassificationStore,
    token: SessionTokenHash,
    filter: QuestionPoolDiscoveryFilter,
) -> Result<bool, StoreError> {
    let Some(discipline) = filter.discipline_uuid else {
        return Ok(true);
    };
    if !store
        .list_disciplines(token.clone())
        .await?
        .iter()
        .any(|item| item.uuid == discipline)
    {
        return Ok(false);
    }
    let Some(subject) = filter.subject_uuid else {
        return Ok(true);
    };
    // Cross-Discipline mode still validates the selected Discipline/Subject edge.
    if !store
        .list_subjects(token.clone(), discipline)
        .await?
        .iter()
        .any(|item| item.uuid == subject)
    {
        return Ok(false);
    }
    let Some(topic) = filter.topic_uuid else {
        return Ok(true);
    };
    if !store
        .list_topics(token.clone(), subject)
        .await?
        .iter()
        .any(|item| item.uuid == topic)
    {
        return Ok(false);
    }
    let Some(subtopic) = filter.subtopic_uuid else {
        return Ok(true);
    };
    Ok(store
        .list_subtopics(token, topic)
        .await?
        .iter()
        .any(|item| item.uuid == subtopic))
}

fn verified_id(issuer: &HmacQuestionIdIssuer, value: &str) -> Option<QuestionId> {
    let id = value.parse().ok()?;
    issuer.validates_question_id(&id).then_some(id)
}

fn encode_cursor(
    after: &str,
    page_size: u16,
    filter: QuestionPoolDiscoveryFilter,
) -> Option<String> {
    serde_json::to_vec(&PoolCursor {
        version: 2,
        after: after.to_owned(),
        page_size,
        discipline_uuid: filter.discipline_uuid,
        subject_uuid: filter.subject_uuid,
        topic_uuid: filter.topic_uuid,
        subtopic_uuid: filter.subtopic_uuid,
        cross_discipline: filter.cross_discipline,
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(
    issuer: &HmacQuestionIdIssuer,
    value: &str,
    page_size: u16,
    filter: QuestionPoolDiscoveryFilter,
) -> Option<Cursor> {
    if value.len() > MAX_CURSOR_BYTES {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    let cursor: PoolCursor = serde_json::from_slice(&decoded).ok()?;
    if cursor.version != 2
        || cursor.page_size != page_size
        || cursor.discipline_uuid != filter.discipline_uuid
        || cursor.subject_uuid != filter.subject_uuid
        || cursor.topic_uuid != filter.topic_uuid
        || cursor.subtopic_uuid != filter.subtopic_uuid
        || cursor.cross_discipline != filter.cross_discipline
    {
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
        let filter = QuestionPoolDiscoveryFilter::default();
        let encoded = encode_cursor(&id.to_string(), 25, filter).expect("cursor encodes");
        assert_eq!(
            decode_cursor(&issuer, &encoded, 25, filter)
                .expect("matching query cursor")
                .as_str(),
            id.to_string()
        );
        assert!(decode_cursor(&issuer, &encoded, 50, filter).is_none());

        let wrong = QuestionId::from_canonical_parts(
            id.identifier_compact(),
            if id.validation_character() == '0' {
                '1'
            } else {
                '0'
            },
        )
        .expect("syntax-valid alternate HMAC character");
        let forged = encode_cursor(&wrong.to_string(), 25, filter).expect("forged cursor encodes");
        assert!(decode_cursor(&issuer, &forged, 25, filter).is_none());
    }

    #[test]
    fn continuation_rejects_changed_classification_and_old_format() {
        let issuer = HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([7; 32]));
        let id = issuer.issue_question_id().expect("Pool ID");
        let filter = QuestionPoolDiscoveryFilter {
            discipline_uuid: Some(uuid::Uuid::from_u128(1)),
            subject_uuid: Some(uuid::Uuid::from_u128(2)),
            topic_uuid: Some(uuid::Uuid::from_u128(3)),
            subtopic_uuid: Some(uuid::Uuid::from_u128(4)),
            cross_discipline: true,
        };
        let encoded = encode_cursor(&id.to_string(), 25, filter).expect("cursor encodes");
        assert!(decode_cursor(&issuer, &encoded, 25, filter).is_some());
        for changed in [
            QuestionPoolDiscoveryFilter {
                discipline_uuid: Some(uuid::Uuid::from_u128(5)),
                ..filter
            },
            QuestionPoolDiscoveryFilter {
                subject_uuid: Some(uuid::Uuid::from_u128(5)),
                ..filter
            },
            QuestionPoolDiscoveryFilter {
                topic_uuid: None,
                ..filter
            },
            QuestionPoolDiscoveryFilter {
                subtopic_uuid: None,
                ..filter
            },
            QuestionPoolDiscoveryFilter {
                cross_discipline: false,
                ..filter
            },
        ] {
            assert!(decode_cursor(&issuer, &encoded, 25, changed).is_none());
        }
        let old = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "after": id.to_string(), "pageSize": 25,
            }))
            .expect("old cursor JSON"),
        );
        assert!(decode_cursor(&issuer, &old, 25, QuestionPoolDiscoveryFilter::default()).is_none());
    }
}
