//! Live Question Library browse and detail Server Routes.
//!
//! Private Question Source bytes are resolved only after the PostgreSQL Store
//! has installed the current session and confirmed the active Instructor
//! boundary.  The route serializes only browser-safe Question Library values.

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
    PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash, StoreError,
    postgres::{PostgresContentClassificationStore, PostgresQuestionLibraryStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    BloomClassificationCorrectionRequest, QuestionBackend, QuestionBloomCorrectionReceipt,
    QuestionId, QuestionLineageView, QuestionRevisionTuple, QuestionSearchPage,
    QuestionSearchRequest, QuestionSearchResult,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::auth::{AuthError, resolve_session};
use question_model::ProductRole;

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_PAGE_SIZE: u16 = 100;
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
    concealed, entries_to_summaries, matches_query, question_response, route_error,
    store_error_response, summary_from_entry, unavailable,
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
    edit_number: question_model::QuestionAvailabilityEditNumber,
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
    let entries = match state
        .store
        .list_published_question_library_entries(session_hash)
        .await
    {
        Ok(entries) => entries,
        Err(error) => return store_error_response(error),
    };
    let summaries = match entries_to_summaries(&state.objects, entries).await {
        Ok(summaries) => summaries,
        Err(()) => {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Library unavailable",
            );
        }
    };
    let text_query = search_query::QuestionTextQuery::parse(query.text.as_deref());
    let mut matching = summaries
        .iter()
        .filter(|entry| matches_query(entry, &query, &text_query))
        .collect::<Vec<_>>();
    // Facets describe the complete authorized predicate intersection. Cursor
    // position and page size select returned rows only and never narrow counts.
    let facets = facets::facets(&matching);
    let (items, next_cursor) = match paging::page(&mut matching, &query) {
        Ok(page) => page,
        Err(()) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library continuation is invalid",
            );
        }
    };
    let question_ids = items
        .iter()
        .map(|entry| entry.summary.question_id.clone())
        .collect::<Vec<_>>();
    let evidence = match usage_statistics::bulk_question_statistics(
        &state.store,
        session_hash,
        is_instructor,
        &question_ids,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let page = QuestionSearchPage {
        items: items
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
    };
    crate::auth::no_store(Json(page).into_response())
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
    let published_id = entry.question_revision_tuple.question_id.clone();
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
    let question_revision_tuple = match verified_question_revision(&question_id, &revision_number) {
        Some(question_revision_tuple) => question_revision_tuple,
        None => return concealed(),
    };
    let entry = match state
        .store
        .load_published_question_revision_library_entry(session_hash, &question_revision_tuple)
        .await
    {
        Ok(entry) => entry,
        Err(error) => return store_error_response(error),
    };
    let published_id = entry.question_revision_tuple.question_id.clone();
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
    let question_revision_tuple = match verified_question_revision(&question_id, &revision_number) {
        Some(question_revision_tuple) => question_revision_tuple,
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
            &question_revision_tuple,
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
            question_revision_tuple,
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
    let question_revision_tuple = match verified_question_revision(&question_id, &revision_number) {
        Some(question_revision_tuple) => question_revision_tuple,
        None => return concealed(),
    };
    let entry = match state
        .store
        .load_published_question_revision_library_entry(session_hash, &question_revision_tuple)
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
        entry.question_revision_tuple.clone(),
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
fn verified_question_id(value: &str) -> Option<QuestionId> {
    value.parse().ok()
}

fn verified_question_revision(
    question_id: &str,
    revision_number: &str,
) -> Option<QuestionRevisionTuple> {
    Some(QuestionRevisionTuple {
        question_id: verified_question_id(question_id)?,
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
