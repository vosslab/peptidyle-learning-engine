//! Active-Instructor published Pool discovery and owned fork detail routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::Query;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    ContentClassificationStore, ContentDisciplineDiscoveryStore, Cursor, PageRequest, PageSize,
    PublishedQuestionPool, QuestionLibraryStore, QuestionPoolDiscoveryFilter,
    QuestionPoolLibraryStore, QuestionPoolTextField, QuestionPoolTextFilter, QuestionPoolTextTerm,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresContentClassificationStore, PostgresQuestionLibraryStore,
        PostgresQuestionPoolLibraryStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssessmentEntryId, AssessmentId, AssessmentQuestionPoolForkView,
    BloomClassificationCorrectionRequest, BloomCognitiveProcess, BloomKnowledgeDimension,
    CourseInstanceId, ProductRole, QuestionId, QuestionPoolBloomCorrectionReceipt,
    QuestionPoolBloomFacets, QuestionPoolLibraryPage, QuestionPoolMemberView, QuestionPoolView,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    auth::{AuthError, resolve_session},
    question_library::answer_free_reusable_question_view,
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
}

pub fn question_pool_library_router(
    sessions: Arc<PostgresSessionStore>,
    pools: PostgresQuestionPoolLibraryStore,
    questions: PostgresQuestionLibraryStore,
    classifications: PostgresContentClassificationStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route("/api/question-pools", get(list_pools))
        .route("/api/question-pools/{question_pool_id}", get(current_pool))
        .route(
            "/api/question-pools/{question_pool_id}/bloom",
            post(correct_pool_bloom),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/question-pool-forks/{entry}",
            get(assessment_fork),
        )
        .with_state(RouteState {
            sessions,
            pools,
            questions,
            classifications,
            objects,
        })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
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
    #[serde(default)]
    bloom_cognitive_process: Option<BloomCognitiveProcess>,
    #[serde(default)]
    bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PoolCursor {
    version: u8,
    after: String,
    page_size: u16,
    filter_hash: [u8; 32],
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
    let (token, _) = match library_reader(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let filter = QuestionPoolDiscoveryFilter {
        discipline_uuid: query.discipline_uuid,
        subject_uuid: query.subject_uuid,
        topic_uuid: query.topic_uuid,
        subtopic_uuid: query.subtopic_uuid,
        cross_discipline: query.cross_discipline,
        bloom_cognitive_process: query.bloom_cognitive_process,
        bloom_knowledge_dimension: query.bloom_knowledge_dimension,
    };
    // ASVS 2.2.2, 2.2.3: validate the hierarchy at the trusted service boundary.
    if !filter.has_valid_structure() {
        return bad_request("Question Pool classification hierarchy is invalid");
    }
    match valid_classification(&state.classifications, token, filter).await {
        Ok(true) => {}
        Ok(false) => return bad_request("Question Pool classification is invalid"),
        Err(error) => return store_error(error),
    }
    let text_filter = match normalize_text_filter(query.text, query.tags) {
        Ok(value) => value,
        Err(message) => return bad_request(message),
    };
    let after = match query.cursor {
        Some(value) => match decode_cursor(&value, page_size.get(), filter, &text_filter) {
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
            text_filter.clone(),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let next_cursor = match page.next_cursor {
        Some(value) => match encode_cursor(value.as_str(), page_size.get(), filter, &text_filter) {
            Some(value) => Some(value),
            None => return unavailable(),
        },
        None => None,
    };
    crate::auth::no_store(
        Json(QuestionPoolLibraryPage {
            items: page.items,
            next_cursor,
            bloom_facets: QuestionPoolBloomFacets {
                cognitive_processes: page.cognitive_processes,
                knowledge_dimensions: page.knowledge_dimensions,
            },
        })
        .into_response(),
    )
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
    // ASVS 8.2.1/8.3.1: correction is available to every active vetted
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
    question_pool_id: &QuestionId,
) -> Result<question_model::QuestionStatistics, Response> {
    if !is_instructor {
        return Ok(question_model::QuestionStatistics::Unavailable);
    }
    let (pool_issued_count, totals) = state
        .pools
        .load_question_pool_usage_statistics(token, question_pool_id)
        .await
        .map_err(store_error)?;
    Ok(totals.into_available(None, Some(pool_issued_count)))
}

#[allow(clippy::result_large_err)]
async fn pool_members(
    state: &RouteState,
    token: SessionTokenHash,
    is_instructor: bool,
    members: Vec<question_model::QuestionRevisionReference>,
) -> Result<Vec<QuestionPoolMemberView>, Response> {
    let question_ids = members
        .iter()
        .map(|member| member.question_id.clone())
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
        let member_evidence = crate::question_library::evidence_for(&member.question_id, &evidence);
        let question = answer_free_reusable_question_view(&state.objects, entry, member_evidence)
            .await
            .map_err(|_| unavailable())?;
        views.push(QuestionPoolMemberView {
            member_position: u32::try_from(position).map_err(|_| unavailable())?,
            question_revision: member,
            question,
        });
    }
    Ok(views)
}

async fn valid_classification(
    store: &(impl ContentClassificationStore + ContentDisciplineDiscoveryStore),
    token: SessionTokenHash,
    filter: QuestionPoolDiscoveryFilter,
) -> Result<bool, StoreError> {
    let Some(discipline) = filter.discipline_uuid else {
        return Ok(true);
    };
    if !store
        .list_disciplines_including_retired(token)
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
        .list_subjects(token, discipline)
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
        .list_topics(token, subject)
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

fn verified_id(value: &str) -> Option<QuestionId> {
    value.parse().ok()
}

fn encode_cursor(
    after: &str,
    page_size: u16,
    filter: QuestionPoolDiscoveryFilter,
    text: &QuestionPoolTextFilter,
) -> Option<String> {
    serde_json::to_vec(&PoolCursor {
        version: 4,
        after: after.to_owned(),
        page_size,
        filter_hash: filter_hash(filter, text)?,
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(
    value: &str,
    page_size: u16,
    filter: QuestionPoolDiscoveryFilter,
    text: &QuestionPoolTextFilter,
) -> Option<Cursor> {
    // ASVS 1.5.2: bounded JSON into an explicit deny-unknown-fields cursor type.
    if value.len() > MAX_CURSOR_BYTES {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    let cursor: PoolCursor = serde_json::from_slice(&decoded).ok()?;
    if cursor.version != 4
        || cursor.page_size != page_size
        || cursor.filter_hash != filter_hash(filter, text)?
    {
        return None;
    }
    let id = cursor.after.parse::<QuestionId>().ok()?;
    Cursor::parse(id.to_string()).ok()
}

fn filter_hash(
    filter: QuestionPoolDiscoveryFilter,
    text: &QuestionPoolTextFilter,
) -> Option<[u8; 32]> {
    Some(Sha256::digest(serde_json::to_vec(&(filter, text)).ok()?).into())
}

fn normalize_text_filter(
    text: Option<String>,
    tags: Vec<String>,
) -> Result<QuestionPoolTextFilter, &'static str> {
    use crate::library_search_terms::{SearchField, parse_terms};
    use question_model::normalized_question_search_group_value as normalize;
    // ASVS 2.2.1, 2.2.2: mirror the Question search positive input bounds.
    if tags.len() > question_model::MAX_QUESTION_SEARCH_TAG_FILTERS {
        return Err("Question Pool Tags filter is invalid");
    }
    let normalized = text.as_deref().map(normalize).unwrap_or_default();
    if normalized.chars().count() > 256 {
        return Err("Question Pool text query is too long");
    }
    let mut normalized_tags = Vec::with_capacity(tags.len());
    for tag in tags {
        let tag = normalize(&tag);
        if tag.is_empty() || tag.chars().count() > 256 {
            return Err("Question Pool Tags filter is invalid");
        }
        normalized_tags.push(tag);
    }
    normalized_tags.sort();
    normalized_tags.dedup();
    let terms = parse_terms(&normalized)
        .into_iter()
        .map(|term| {
            let field = match term.field {
                None => QuestionPoolTextField::Any,
                Some(SearchField::Discipline) => QuestionPoolTextField::Discipline,
                Some(SearchField::Subject) => QuestionPoolTextField::Subject,
                Some(SearchField::Topic) => QuestionPoolTextField::Topic,
                Some(SearchField::Subtopic) => QuestionPoolTextField::Subtopic,
                Some(SearchField::Tags) => QuestionPoolTextField::Tags,
                Some(SearchField::QuestionType | SearchField::Author) => {
                    return Err("Question Type and author text fields are unavailable for Pools");
                }
            };
            Ok(QuestionPoolTextTerm {
                field,
                value: term.value,
                excluded: term.excluded,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(QuestionPoolTextFilter {
        text: (!normalized.is_empty()).then_some(normalized),
        terms,
        tags: normalized_tags,
    })
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
                session.record.product_role,
                ProductRole::Instructor | ProductRole::Sysadmin
            ) =>
        {
            Ok((
                session.session_hash,
                session.record.product_role == ProductRole::Instructor,
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

    #[test]
    fn pool_text_and_tags_keep_shared_grammar_and_normalized_meaning() {
        let filter = normalize_text_filter(
            Some("  Mendelian  topic:\"Chromosomal Inheritance\" -tags:Practice ".into()),
            vec![" Review  Set ".into(), "review set".into(), "EXAM".into()],
        )
        .expect("bounded shared grammar");
        assert_eq!(filter.tags, ["exam", "review set"]);
        assert_eq!(
            filter.terms,
            vec![
                QuestionPoolTextTerm {
                    field: QuestionPoolTextField::Any,
                    value: "mendelian".into(),
                    excluded: false
                },
                QuestionPoolTextTerm {
                    field: QuestionPoolTextField::Topic,
                    value: "chromosomal inheritance".into(),
                    excluded: false
                },
                QuestionPoolTextTerm {
                    field: QuestionPoolTextField::Tags,
                    value: "practice".into(),
                    excluded: true
                },
            ]
        );
        assert_eq!(
            normalize_text_filter(Some(" \t ".into()), vec![]).expect("blank text"),
            QuestionPoolTextFilter::default()
        );
        for text in ["type:numeric", "-author:someone", "author:\"Named Author\""] {
            assert!(normalize_text_filter(Some(text.into()), vec![]).is_err());
        }
        assert!(normalize_text_filter(Some("x".repeat(257)), vec![]).is_err());
        assert!(normalize_text_filter(None, vec![" ".into()]).is_err());
        assert!(normalize_text_filter(None, vec!["x".into(); 65]).is_err());
    }

    #[test]
    fn continuation_binds_normalized_text_and_tags() {
        let id = QuestionId::from_random_identifier("ABCDEFG").expect("Pool ID");
        let identities = QuestionPoolDiscoveryFilter::default();
        let filter =
            normalize_text_filter(Some(" REVIEW ".into()), vec![" Exam ".into()]).expect("filter");
        let encoded = encode_cursor(&id.to_string(), 25, identities, &filter).expect("cursor");
        let same = normalize_text_filter(Some("review".into()), vec!["exam".into(), "EXAM".into()])
            .expect("same meaning");
        assert!(decode_cursor(&encoded, 25, identities, &same).is_some());
        for changed in [
            normalize_text_filter(Some("practice".into()), vec!["exam".into()])
                .expect("text change"),
            normalize_text_filter(Some("review".into()), vec!["practice".into()])
                .expect("tag change"),
        ] {
            assert!(decode_cursor(&encoded, 25, identities, &changed).is_none());
        }
    }

    #[test]
    fn continuation_is_opaque_query_bound_and_checksum_validated() {
        let id = QuestionId::from_random_identifier("ABCDEFG").expect("Pool ID");
        let filter = QuestionPoolDiscoveryFilter::default();
        let encoded = encode_cursor(
            &id.to_string(),
            25,
            filter,
            &QuestionPoolTextFilter::default(),
        )
        .expect("cursor encodes");
        assert_eq!(
            decode_cursor(&encoded, 25, filter, &QuestionPoolTextFilter::default())
                .expect("matching query cursor")
                .as_str(),
            id.to_string()
        );
        assert!(decode_cursor(&encoded, 50, filter, &QuestionPoolTextFilter::default()).is_none());

        let forged = encode_cursor("0000-5000", 25, filter, &QuestionPoolTextFilter::default())
            .expect("forged cursor encodes");
        assert!(decode_cursor(&forged, 25, filter, &QuestionPoolTextFilter::default()).is_none());
    }

    #[test]
    fn continuation_rejects_changed_classification_and_old_format() {
        let id = QuestionId::from_random_identifier("ABCDEFG").expect("Pool ID");
        let filter = QuestionPoolDiscoveryFilter {
            discipline_uuid: Some(uuid::Uuid::from_u128(1)),
            subject_uuid: Some(uuid::Uuid::from_u128(2)),
            topic_uuid: Some(uuid::Uuid::from_u128(3)),
            subtopic_uuid: Some(uuid::Uuid::from_u128(4)),
            cross_discipline: true,
            bloom_cognitive_process: Some(BloomCognitiveProcess::Analyze),
            bloom_knowledge_dimension: Some(BloomKnowledgeDimension::ConceptualKnowledge),
        };
        let encoded = encode_cursor(
            &id.to_string(),
            25,
            filter,
            &QuestionPoolTextFilter::default(),
        )
        .expect("cursor encodes");
        assert!(decode_cursor(&encoded, 25, filter, &QuestionPoolTextFilter::default()).is_some());
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
            QuestionPoolDiscoveryFilter {
                bloom_cognitive_process: Some(BloomCognitiveProcess::Apply),
                ..filter
            },
            QuestionPoolDiscoveryFilter {
                bloom_knowledge_dimension: Some(BloomKnowledgeDimension::ProceduralKnowledge),
                ..filter
            },
        ] {
            assert!(
                decode_cursor(&encoded, 25, changed, &QuestionPoolTextFilter::default()).is_none()
            );
        }
        let old = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "after": id.to_string(), "pageSize": 25,
            }))
            .expect("old cursor JSON"),
        );
        assert!(
            decode_cursor(
                &old,
                25,
                QuestionPoolDiscoveryFilter::default(),
                &QuestionPoolTextFilter::default()
            )
            .is_none()
        );
    }
}
