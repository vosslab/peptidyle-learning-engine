//! Live Question Library browse and detail Server Routes.
//!
//! Private Question Source bytes are resolved only after the PostgreSQL Store
//! has installed the current session and confirmed the active Instructor
//! boundary.  The route serializes only browser-safe Question Library values.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::Query;
use learning_data_access::{
    PublishedQuestionLibraryEntry, QuestionLibraryBackendRestriction, QuestionLibrarySearchRequest,
    QuestionLibrarySearchSort, QuestionLibraryStore, SessionTokenHash, StoreError,
    postgres::{PostgresContentClassificationStore, PostgresQuestionLibraryStore},
};
use objects::{ObjectStore, s3::S3ObjectStore};
use question_model::{
    BloomClassificationCorrectionRequest, PublishedQuestionId, PublishedQuestionRevisionTuple,
    QuestionBackend, QuestionBloomCorrectionReceipt, QuestionLineageView, QuestionSearchPage,
    QuestionSearchRequest, QuestionSearchResult, QuestionStatistics,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::auth::{AuthError, resolve_session};
use question_model::ProductRole;

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_PAGE_SIZE: u16 = question_model::MAX_DISCOVERY_PAGE_SIZE as u16;
const WEBWORK_SOURCE_MEDIA_TYPE: &str = "text/x-wework-pg";

mod classification;
mod facets;
mod paging;
mod query;
mod search_query;
mod shared_metadata;
mod summaries;
mod usage_statistics;

use summaries::{
    ResolvedQuestionLibraryEntry, answer_free_question_library_entry, availability_response,
    concealed, entries_to_summaries, question_response, route_error, store_error_response,
    summary_from_entry, unavailable,
};
pub(crate) use usage_statistics::{
    answer_free_question_search_results, answer_free_reusable_question_view,
    bulk_question_statistics, evidence_for,
};

#[derive(Clone)]
struct QuestionLibraryRouteState {
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    store: PostgresQuestionLibraryStore,
    classifications: PostgresContentClassificationStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
}

/// Registers Question Library browse and detail reads for Instructors and Sysadmins.
pub fn question_library_router(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    store: PostgresQuestionLibraryStore,
    classifications: PostgresContentClassificationStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Router {
    Router::new()
        .route("/api/questions/search", get(search_questions))
        .route("/api/questions/by-id/{question_id}", get(resolve_question))
        .route(
            "/api/questions/by-id/{question_id}/detail",
            get(question_details),
        )
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}",
            get(question_revision_details),
        )
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/bloom",
            post(correct_question_revision_bloom),
        )
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/preview-document",
            get(question_revision_preview_document),
        )
        .route(
            "/api/questions/by-id/{question_id}/archive",
            post(archive_question),
        )
        .route(
            "/api/questions/by-id/{question_id}/restore",
            post(restore_question),
        )
        .route(
            "/api/questions/bulk-metadata/current",
            post(shared_metadata::load_current_shared_metadata),
        )
        .with_state(QuestionLibraryRouteState {
            sessions,
            store,
            classifications,
            objects,
            webwork,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveQuestionRequest {
    confirmation_title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuestionAvailabilityResponse {
    availability: question_model::QuestionAvailability,
    question_availability_edit_number: question_model::QuestionAvailabilityEditNumber,
}

use query::QuestionSearchQuery;
async fn search_questions(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Query(query): Query<QuestionSearchQuery>,
) -> Response {
    let query = match QuestionSearchRequest::try_from(query) {
        Ok(query) => query,
        Err((status, message)) => return route_error(status, message),
    };
    let (session_hash, is_instructor) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if let Err(response) =
        classification::validate(&state.classifications, session_hash, &query).await
    {
        return response;
    }
    search_question_library(
        &state.store,
        &state.objects,
        session_hash,
        is_instructor,
        query,
    )
    .await
}

/// One Question Library search after the session is known.
///
/// The opaque cursor is checked before any source read. Native Question source
/// is resolved only for the store's returned page.
async fn search_question_library<S, O>(
    store: &S,
    objects: &O,
    session_hash: SessionTokenHash,
    is_instructor: bool,
    query: QuestionSearchRequest,
) -> Response
where
    S: QuestionLibraryStore + QuestionLibraryPageStatistics,
    O: ObjectStore,
{
    let after = match paging::decode_position(&query) {
        Ok(after) => after,
        Err(()) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library continuation is invalid",
            );
        }
    };
    let text_query = search_query::QuestionTextQuery::parse(query.text.as_deref());
    let (exact_question_id, text_terms) = text_query.into_store_terms();
    let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    let request = QuestionLibrarySearchRequest {
        exact_question_id,
        text_terms,
        author_names: query.author_names.clone(),
        backends: eligible_backends(&query),
        tags: query.tags.clone(),
        subjects: query.subjects.clone(),
        topics: query.topics.clone(),
        discipline_uuid: query.discipline_uuid,
        subject_uuid: query.subject_uuid,
        topic_uuid: query.topic_uuid,
        subtopic_uuid: query.subtopic_uuid,
        cross_discipline: query.cross_discipline,
        bloom_cognitive_process: query.bloom_cognitive_process,
        bloom_knowledge_dimension: query.bloom_knowledge_dimension,
        question_types: query.question_types.clone(),
        question_licenses: query.question_licenses.clone(),
        used_in_current_account_courses: query.used_in_my_courses
            == question_model::QuestionSearchCourseUse::Used,
        authored_by_current_account: query.authorship
            == question_model::QuestionSearchAuthorship::AuthoredByCurrentAccount,
        sort: match query.sort {
            question_model::QuestionSearchSort::TitleAscending => {
                QuestionLibrarySearchSort::TitleAscending
            }
            question_model::QuestionSearchSort::PublishedNewest => {
                QuestionLibrarySearchSort::PublishedNewest
            }
        },
        page_size,
        after,
    };
    let search = match store
        .search_published_question_library_entries(session_hash, request)
        .await
    {
        Ok(search) => search,
        Err(error) => return store_error_response(error),
    };
    let next_cursor = search
        .next_position
        .map(|position| paging::encode_position(position, &query));
    let facets = facets::from_store(search.facets);
    let summaries = match entries_to_summaries(objects, search.items).await {
        Ok(summaries) => summaries,
        Err(()) => {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Library unavailable",
            );
        }
    };
    let question_ids = summaries
        .iter()
        .map(|entry| entry.summary.question_id.clone())
        .collect::<Vec<_>>();
    let evidence = match store
        .page_statistics(session_hash, is_instructor, &question_ids)
        .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    crate::auth::no_store(
        Json(QuestionSearchPage {
            items: summaries
                .iter()
                .map(|entry| QuestionSearchResult {
                    summary: entry.summary.clone(),
                    discipline_name: entry.discipline.clone().unwrap_or_default(),
                    discipline_is_retired: entry.discipline_is_retired,
                    evidence: usage_statistics::evidence_for(&entry.summary.question_id, &evidence),
                })
                .collect(),
            next_cursor,
            facets,
        })
        .into_response(),
    )
}

#[async_trait::async_trait]
trait QuestionLibraryPageStatistics: Send + Sync {
    async fn page_statistics(
        &self,
        session_hash: SessionTokenHash,
        is_instructor: bool,
        question_ids: &[PublishedQuestionId],
    ) -> Result<BTreeMap<PublishedQuestionId, QuestionStatistics>, Response>;
}

#[async_trait::async_trait]
impl QuestionLibraryPageStatistics for PostgresQuestionLibraryStore {
    async fn page_statistics(
        &self,
        session_hash: SessionTokenHash,
        is_instructor: bool,
        question_ids: &[PublishedQuestionId],
    ) -> Result<BTreeMap<PublishedQuestionId, QuestionStatistics>, Response> {
        usage_statistics::bulk_question_statistics(self, session_hash, is_instructor, question_ids)
            .await
    }
}

fn eligible_backends(query: &QuestionSearchRequest) -> QuestionLibraryBackendRestriction {
    if query.backends.is_empty() && query.capabilities.is_empty() {
        return QuestionLibraryBackendRestriction::Any;
    }
    let candidates = if query.backends.is_empty() {
        QuestionBackend::ALL.to_vec()
    } else {
        query.backends.clone()
    };
    QuestionLibraryBackendRestriction::Only(
        candidates
            .into_iter()
            .filter(|backend| {
                query
                    .capabilities
                    .iter()
                    .all(|capability| facets::backend_capabilities(*backend).supports(*capability))
            })
            .collect(),
    )
}

async fn resolve_question(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(question_id): Path<String>,
) -> Response {
    let (session_hash, _) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let entry = match load_verified_question_library_entry(&state.store, session_hash, &question_id)
        .await
    {
        Ok(Some(entry)) => entry,
        Ok(None) => return concealed(),
        Err(error) => return store_error_response(error),
    };
    let edit_number = entry.availability_edit_number;
    let viewer_may_archive = entry.viewer_may_archive;
    match summary_from_entry(&state.objects, entry).await {
        Ok(summary) => question_response(
            Json(QuestionLineageView {
                summary,
                viewer_may_archive,
            })
            .into_response(),
            edit_number,
        ),
        Err(()) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Library unavailable",
        ),
    }
}

async fn question_details(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(question_id): Path<String>,
) -> Response {
    let (session_hash, is_instructor) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let entry = match load_verified_question_library_entry(&state.store, session_hash, &question_id)
        .await
    {
        Ok(Some(entry)) => entry,
        Ok(None) => return concealed(),
        Err(error) => return store_error_response(error),
    };
    let published_id = entry
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let edit_number = entry.availability_edit_number;
    let resolved = match answer_free_question_library_entry(&state.objects, entry).await {
        Ok(resolved) => resolved,
        Err(()) => {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Library unavailable",
            );
        }
    };
    let evidence = match usage_statistics::question_detail_statistics(
        &state.store,
        session_hash,
        is_instructor,
        &published_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let detail = usage_statistics::details_from_resolved(resolved, evidence);
    question_response(Json(detail).into_response(), edit_number)
}

/// Resolves one exact immutable Question Revision. This route deliberately
/// bypasses ordinary discovery availability so retained Assessment evidence
/// remains interpretable after the stable lineage is archived.
async fn question_revision_details(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
) -> Response {
    let (session_hash, is_instructor) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let published_question_revision_tuple =
        match verified_published_question_revision_tuple(&question_id, &revision_number) {
            Some(published_question_revision_tuple) => published_question_revision_tuple,
            None => return concealed(),
        };
    let entry = match state
        .store
        .load_published_question_revision_library_entry(
            session_hash,
            &published_question_revision_tuple,
        )
        .await
    {
        Ok(entry) => entry,
        Err(error) => return store_error_response(error),
    };
    let published_id = entry
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let edit_number = entry.availability_edit_number;
    let resolved = match answer_free_question_library_entry(&state.objects, entry).await {
        Ok(resolved) => resolved,
        Err(()) => return unavailable(),
    };
    let evidence = match usage_statistics::question_detail_statistics(
        &state.store,
        session_hash,
        is_instructor,
        &published_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let detail = usage_statistics::details_from_resolved(resolved, evidence);
    question_response(Json(detail).into_response(), edit_number)
}

async fn correct_question_revision_bloom(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
    payload: Result<Json<BloomClassificationCorrectionRequest>, JsonRejection>,
) -> Response {
    let published_question_revision_tuple =
        match verified_published_question_revision_tuple(&question_id, &revision_number) {
            Some(published_question_revision_tuple) => published_question_revision_tuple,
            None => return concealed(),
        };
    // ASVS 8.2.1/8.3.1: only an active vetted Instructor reaches correction;
    // a Sysadmin retains the read-only Library surface and receives concealment.
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.5.2/2.2.1: serde rejects missing, unknown, or open-string fields.
    let Json(request) = match payload {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bloom correction is invalid",
            );
        }
    };
    let bloom = match state
        .store
        .correct_question_revision_bloom(
            session_hash,
            &published_question_revision_tuple,
            request.expected_classification_edit_number,
            request.cognitive_process,
            request.knowledge_dimension,
        )
        .await
    {
        Ok(value) => value,
        Err(StoreError::InvalidRecord(_)) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Bloom correction is invalid",
            );
        }
        Err(error) => return store_error_response(error),
    };
    crate::auth::no_store(
        Json(QuestionBloomCorrectionReceipt {
            published_question_revision_tuple,
            bloom,
        })
        .into_response(),
    )
}

/// Renders one ephemeral, answer-free WeBWorK example for the exact Revision.
///
/// The authorized library read supplies every private binding fact. This route
/// creates no Attempt, Student Work, response, submission, or grade.
async fn question_revision_preview_document(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
) -> Response {
    let (session_hash, _) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let published_question_revision_tuple =
        match verified_published_question_revision_tuple(&question_id, &revision_number) {
            Some(published_question_revision_tuple) => published_question_revision_tuple,
            None => return concealed(),
        };
    let entry = match state
        .store
        .load_published_question_revision_library_entry(
            session_hash,
            &published_question_revision_tuple,
        )
        .await
    {
        Ok(entry) => entry,
        Err(error) => return store_error_response(error),
    };
    if entry.backend != QuestionBackend::Webwork {
        return concealed();
    }
    if entry.source_media_type != WEBWORK_SOURCE_MEDIA_TYPE {
        return unavailable();
    }
    let Some(webwork_pg_path) = entry.webwork_pg_path else {
        return unavailable();
    };
    let binding = match WebworkQuestionSourceBinding::new(
        entry.published_question_revision_tuple.clone(),
        webwork_pg_path,
    ) {
        Ok(binding) => binding,
        Err(_) => return unavailable(),
    };
    let source = match ResolvedWebworkQuestionSource::resolve(
        &state.objects,
        binding,
        entry.source_object_id,
        entry.source_object_checksum,
    )
    .await
    {
        Ok(source) => source,
        Err(_) => return unavailable(),
    };
    let document = match state
        .webwork
        .preview_document(
            match preview_question_seed() {
                Ok(seed) => seed,
                Err(()) => return unavailable(),
            },
            &source,
        )
        .await
    {
        Ok(document) => document,
        Err(_) => return unavailable(),
    };
    match String::from_utf8(document) {
        Ok(document) => crate::webwork_document_route::preview_backend_document_response(document),
        Err(_) => unavailable(),
    }
}

async fn archive_question(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(question_id): Path<String>,
    Json(input): Json<ArchiveQuestionRequest>,
) -> Response {
    transition_question_availability(
        &state,
        &headers,
        question_id,
        Some(input.confirmation_title),
    )
    .await
}

async fn restore_question(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(question_id): Path<String>,
) -> Response {
    transition_question_availability(&state, &headers, question_id, None).await
}

/// ASVS 2.2.1 and 2.3.3: the HTTP boundary requires a qualified edit number;
/// PostgreSQL then atomically locks, authorizes, validates, and records the
/// availability transition.
async fn transition_question_availability(
    state: &QuestionLibraryRouteState,
    headers: &HeaderMap,
    value: String,
    confirmation_title: Option<String>,
) -> Response {
    let question_id = match verified_question_id(&value) {
        Some(question_id) => question_id,
        None => return concealed(),
    };
    let expected = match expected_availability_edit_number(headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let result = match confirmation_title {
        Some(title) => {
            state
                .store
                .archive_published_question(session_hash, &question_id, expected, &title)
                .await
        }
        None => {
            state
                .store
                .restore_published_question(session_hash, &question_id, expected)
                .await
        }
    };
    match result {
        Ok(value) => availability_response(value.availability, value.edit_number),
        Err(StoreError::InvalidRecord(_)) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question availability transition is invalid",
        ),
        Err(error) => store_error_response(error),
    }
}

fn expected_availability_edit_number(
    headers: &HeaderMap,
) -> Result<question_model::QuestionAvailabilityEditNumber, Box<Response>> {
    let Some(value) = headers.get(IF_MATCH).and_then(|value| value.to_str().ok()) else {
        return Err(Box::new(route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Question Availability Edit Number is required",
        )));
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
    else {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Question Availability Edit Number is invalid",
        )));
    };
    question_model::QuestionAvailabilityEditNumber::from_str(number).map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Question Availability Edit Number is invalid",
        ))
    })
}

/// Parses only a browser-supplied exact canonical ID. The shared model verifies
/// syntax and checksum, and this route intentionally conceals invalid values.
fn verified_question_id(value: &str) -> Option<PublishedQuestionId> {
    value.parse().ok()
}

fn verified_published_question_revision_tuple(
    question_id: &str,
    revision_number: &str,
) -> Option<PublishedQuestionRevisionTuple> {
    Some(PublishedQuestionRevisionTuple {
        published_question_id: verified_question_id(question_id)?,
        revision_number: revision_number
            .parse::<u32>()
            .ok()
            .and_then(|value| question_model::QuestionRevisionNumber::new(value).ok())?,
    })
}

fn preview_question_seed() -> Result<question_model::generation::QuestionSeed, ()> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(question_model::generation::QuestionSeed::new(
        u64::from_be_bytes(bytes),
    ))
}

/// Resolves only a checksum-validated Question ID through the authorized
/// Question Library Store. An invalid canonical ID is concealed before any
/// Question lookup can occur.
async fn load_verified_question_library_entry(
    store: &impl QuestionLibraryStore,
    session_hash: SessionTokenHash,
    value: &str,
) -> Result<Option<PublishedQuestionLibraryEntry>, StoreError> {
    let Some(question_id) = verified_question_id(value) else {
        return Ok(None);
    };
    store
        .load_published_question_library_entry(session_hash, &question_id)
        .await
        .map(Some)
}

async fn instructor_session_hash(
    state: &QuestionLibraryRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookie_header = joined_cookie_header(headers);
    match resolve_session(state.sessions.as_ref(), cookie_header.as_deref()).await {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Library authentication unavailable",
        ))),
    }
}

async fn library_reader_session_hash(
    state: &QuestionLibraryRouteState,
    headers: &HeaderMap,
) -> Result<(SessionTokenHash, bool), Box<Response>> {
    let cookie_header = joined_cookie_header(headers);
    match resolve_session(state.sessions.as_ref(), cookie_header.as_deref()).await {
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
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Library authentication unavailable",
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

#[cfg(test)]
mod tests;
