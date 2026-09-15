//! Live Question Library browse and detail Server Routes.
//!
//! Private Question Source bytes are resolved only after the PostgreSQL Store
//! has installed the current session and confirmed the active Instructor
//! boundary.  The route serializes only browser-safe Question Library values.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::Query;
use learning_data_access::{
    PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash, StoreError,
    postgres::PostgresQuestionLibraryStore,
};
use objects::{ResolvedQuestionSource, s3::S3ObjectStore};
use question_model::{
    Capability, QuestionBackend, QuestionBackendCapabilities, QuestionDetails,
    QuestionDetailsPromptView, QuestionId, QuestionLineageView, QuestionRevisionReference,
    QuestionSearchAuthorFacet, QuestionSearchAuthorship, QuestionSearchBackendFacet,
    QuestionSearchCapabilityFacet, QuestionSearchCourseUse, QuestionSearchCourseUseFacet,
    QuestionSearchFacets, QuestionSearchPage, QuestionSearchQuestionLicenseFacet,
    QuestionSearchRequest, QuestionSearchResult, QuestionSearchTagFacet, QuestionStatistics,
    QuestionSummary, QuestionTypeFacet, QuestionUseDetails, QuestionUseSummary,
    ReusableQuestionView, ReusableSelectionAvailability,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::auth::{AuthError, resolve_session};
use crate::question_publication::HmacQuestionIdIssuer;
use question_model::ProductRole;

const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_PAGE_SIZE: u16 = 100;

mod paging;
mod search_query;
mod shared_metadata;

#[derive(Clone)]
struct QuestionLibraryRouteState {
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    store: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers the Instructor-only Question Library browse and detail routes.
pub fn question_library_router(
    sessions: Arc<learning_data_access::postgres::PostgresSessionStore>,
    store: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
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
            objects,
            question_id_issuer,
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

/// URL form of the current Question Search request.
///
/// The model's transport form intentionally has no defaults because saved
/// searches must record every field. HTTP uses defaults for omitted filters,
/// then immediately constructs the same strict model value.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestionSearchQuery {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    author_names: Vec<String>,
    #[serde(default)]
    backends: Vec<QuestionBackend>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    question_types: Vec<question_model::QuestionType>,
    #[serde(default)]
    capabilities: Vec<Capability>,
    #[serde(default)]
    question_licenses: Vec<question_model::QuestionLicense>,
    #[serde(default)]
    used_in_my_courses: QuestionSearchCourseUse,
    #[serde(default)]
    authorship: QuestionSearchAuthorship,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    page_size: Option<u16>,
}

impl TryFrom<QuestionSearchQuery> for QuestionSearchRequest {
    type Error = (StatusCode, &'static str);

    fn try_from(query: QuestionSearchQuery) -> Result<Self, Self::Error> {
        if query
            .page_size
            .is_some_and(|size| size == 0 || size > MAX_PAGE_SIZE)
        {
            return Err((
                StatusCode::BAD_REQUEST,
                "Question Library page size is invalid",
            ));
        }
        QuestionSearchRequest {
            text: query.text,
            author_names: query.author_names,
            backends: query.backends,
            tags: query.tags,
            question_types: query.question_types,
            capabilities: query.capabilities,
            question_licenses: query.question_licenses,
            used_in_my_courses: query.used_in_my_courses,
            authorship: query.authorship,
            cursor: query.cursor,
            page_size: query.page_size.or(Some(DEFAULT_PAGE_SIZE)),
        }
        .normalized()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Question Library query is invalid"))
    }
}

async fn search_questions(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Query(query): Query<QuestionSearchQuery>,
) -> Response {
    let query = match QuestionSearchRequest::try_from(query) {
        Ok(query) => query,
        Err((status, message)) => return route_error(status, message),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
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
    let (items, next_cursor) = match paging::page(&mut matching, &query) {
        Ok(page) => page,
        Err(()) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library continuation is invalid",
            );
        }
    };
    let page = QuestionSearchPage {
        items: items
            .iter()
            .map(|entry| QuestionSearchResult {
                summary: entry.summary.clone(),
                evidence: QuestionStatistics::Unavailable,
            })
            .collect(),
        next_cursor,
        facets: facets(&matching),
    };
    crate::auth::no_store(Json(page).into_response())
}

async fn resolve_question(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(question_id): Path<String>,
) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let entry = match load_verified_question_library_entry(
        &state.store,
        &state.question_id_issuer,
        session_hash,
        &question_id,
    )
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
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let entry = match load_verified_question_library_entry(
        &state.store,
        &state.question_id_issuer,
        session_hash,
        &question_id,
    )
    .await
    {
        Ok(Some(entry)) => entry,
        Ok(None) => return concealed(),
        Err(error) => return store_error_response(error),
    };
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
    let detail = details_from_resolved(resolved);
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
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let question_id = match verified_question_id(&state.question_id_issuer, &question_id) {
        Some(question_id) => question_id,
        None => return concealed(),
    };
    let revision_number = match revision_number
        .parse::<u32>()
        .ok()
        .and_then(|value| question_model::QuestionRevisionNumber::new(value).ok())
    {
        Some(value) => value,
        None => return concealed(),
    };
    let reference = QuestionRevisionReference {
        question_id,
        revision_number,
    };
    let entry = match state
        .store
        .load_published_question_revision_library_entry(session_hash, &reference)
        .await
    {
        Ok(entry) => entry,
        Err(error) => return store_error_response(error),
    };
    let edit_number = entry.availability_edit_number;
    let resolved = match answer_free_question_library_entry(&state.objects, entry).await {
        Ok(resolved) => resolved,
        Err(()) => return unavailable(),
    };
    let detail = details_from_resolved(resolved);
    question_response(Json(detail).into_response(), edit_number)
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
    let question_id = match verified_question_id(&state.question_id_issuer, &value) {
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

fn details_from_resolved(resolved: ResolvedQuestionLibraryEntry) -> QuestionDetails {
    QuestionDetails {
        summary: resolved.summary,
        prompt: QuestionDetailsPromptView::Static {
            blocks: resolved.prompt,
        },
        evidence: QuestionStatistics::Unavailable,
        usage: QuestionUseDetails {
            summary: QuestionUseSummary {
                global_course_count: 0,
                global_assessment_count: 0,
                own_course_count: 0,
                own_assessment_count: 0,
            },
            own_courses: Vec::new(),
            own_courses_truncated: false,
        },
    }
}

/// Parses a browser-supplied exact ID and accepts only the deployment-issued
/// canonical identity. Syntax remains a shared model concern; the HMAC check
/// is server-only and intentionally uses the concealed resolution outcome.
fn verified_question_id(
    question_id_issuer: &HmacQuestionIdIssuer,
    value: &str,
) -> Option<QuestionId> {
    let question_id = value.parse::<QuestionId>().ok()?;
    question_id_issuer
        .validates_question_id(&question_id)
        .then_some(question_id)
}

/// Resolves only a server-HMAC-validated Question ID through the authorized
/// Question Library Store. A syntax-valid ID with another validation character
/// is concealed before any Question lookup can occur.
async fn load_verified_question_library_entry(
    store: &impl QuestionLibraryStore,
    question_id_issuer: &HmacQuestionIdIssuer,
    session_hash: SessionTokenHash,
    value: &str,
) -> Result<Option<PublishedQuestionLibraryEntry>, StoreError> {
    let Some(question_id) = verified_question_id(question_id_issuer, value) else {
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

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

struct ResolvedQuestionLibraryEntry {
    summary: QuestionSummary,
    prompt: Vec<question_model::QuestionContentBlock>,
    authored_by_current_account: bool,
    used_in_current_account_courses: bool,
    subject: Option<String>,
    topic: Option<String>,
}

async fn entries_to_summaries(
    objects: &S3ObjectStore,
    entries: Vec<PublishedQuestionLibraryEntry>,
) -> Result<Vec<ResolvedQuestionLibraryEntry>, ()> {
    let mut summaries = Vec::with_capacity(entries.len());
    for entry in entries {
        summaries.push(answer_free_question_library_entry(objects, entry).await?);
    }
    Ok(summaries)
}

/// Resolves current answer-free Question Library rows for another Instructor
/// content route. The input Store entries remain server-only because they
/// contain the private source binding needed for verified PLE compilation.
pub(crate) async fn answer_free_question_search_results(
    objects: &S3ObjectStore,
    entries: Vec<PublishedQuestionLibraryEntry>,
) -> Result<BTreeMap<QuestionRevisionReference, QuestionSearchResult>, ()> {
    let mut results = BTreeMap::new();
    for entry in entries {
        let question_revision = entry.question_revision.clone();
        let resolved = answer_free_question_library_entry(objects, entry).await?;
        results.insert(
            question_revision,
            QuestionSearchResult {
                summary: resolved.summary,
                evidence: QuestionStatistics::Unavailable,
            },
        );
    }
    Ok(results)
}

/// Reuses the exact answer-free Question Library projection for another
/// Instructor content surface without exposing its private source binding.
pub(crate) async fn answer_free_reusable_question_view(
    objects: &S3ObjectStore,
    entry: PublishedQuestionLibraryEntry,
) -> Result<ReusableQuestionView, ()> {
    let selection_availability = match entry.availability {
        question_model::QuestionAvailability::Available => ReusableSelectionAvailability::Available,
        question_model::QuestionAvailability::Archived => ReusableSelectionAvailability::Retained,
    };
    let resolved = answer_free_question_library_entry(objects, entry).await?;
    Ok(ReusableQuestionView {
        question_library: QuestionSearchResult {
            summary: resolved.summary,
            evidence: QuestionStatistics::Unavailable,
        },
        selection_availability,
    })
}

async fn summary_from_entry(
    objects: &S3ObjectStore,
    entry: PublishedQuestionLibraryEntry,
) -> Result<QuestionSummary, ()> {
    Ok(answer_free_question_library_entry(objects, entry)
        .await?
        .summary)
}

/// Produces the one browser-safe Question Library entry shape from the
/// backend-owned source boundary. Private source bindings never cross this
/// boundary into a summary or search result.
async fn answer_free_question_library_entry(
    objects: &S3ObjectStore,
    entry: PublishedQuestionLibraryEntry,
) -> Result<ResolvedQuestionLibraryEntry, ()> {
    match entry.backend {
        QuestionBackend::Ple => resolved_ple_question(objects, entry).await,
        QuestionBackend::Webwork => webwork_question_library_entry(entry),
        QuestionBackend::Imathas => Err(()),
    }
}

/// WeBWorK publication metadata is already database-authoritative and
/// browser-safe. Its private PG source and renderer-only details are not
/// needed to construct the current library summary.
fn webwork_question_library_entry(
    entry: PublishedQuestionLibraryEntry,
) -> Result<ResolvedQuestionLibraryEntry, ()> {
    let used_in_current_account_courses = entry.used_in_current_account_courses;
    let tags = entry.shared_metadata.tags.clone();
    let subject = entry.shared_metadata.subject.clone();
    let topic = entry.shared_metadata.topic.clone();
    Ok(ResolvedQuestionLibraryEntry {
        summary: QuestionSummary {
            question_id: entry.question_revision.question_id.clone(),
            latest_question_revision: entry.question_revision,
            backend: entry.backend,
            question_format: entry.question_format,
            question_type: entry.question_type,
            capabilities: adapter_webwork::webwork_source_capabilities(QuestionBackend::Webwork)
                .map_err(|_| ())?,
            metadata: question_model::QuestionMetadata {
                question_title: entry.question_title,
                question_description: entry.question_description,
                tags,
                question_license: Some(entry.question_license),
                question_citation: None,
                language: "en".to_string(),
            },
            authorship: entry.authorship,
            availability: entry.availability,
            published_at: entry.published_at,
        },
        prompt: Vec::new(),
        authored_by_current_account: entry.authored_by_current_account,
        used_in_current_account_courses,
        subject,
        topic,
    })
}

async fn resolved_ple_question(
    objects: &S3ObjectStore,
    entry: PublishedQuestionLibraryEntry,
) -> Result<ResolvedQuestionLibraryEntry, ()> {
    if entry.backend != QuestionBackend::Ple {
        return Err(());
    }
    let source = ResolvedQuestionSource::resolve(
        objects,
        entry.question_revision.clone(),
        entry.source_object_reference.clone(),
        entry.source_object_checksum.clone(),
    )
    .await
    .map_err(|_| ())?;
    if source.media_type() != adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE
        || source.media_type() != entry.source_media_type
    {
        return Err(());
    }
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source.bytes())
        .map_err(|_| ())?;
    let compiled = document.compile().map_err(|_| ())?;
    let presentation = compiled.presentation();
    let mut metadata = presentation.metadata().clone();
    let used_in_current_account_courses = entry.used_in_current_account_courses;
    let subject = entry.shared_metadata.subject.clone();
    let topic = entry.shared_metadata.topic.clone();
    if metadata.question_title != entry.question_title
        || metadata.question_description != entry.question_description
        || metadata.question_license.as_ref() != Some(&entry.question_license)
        || presentation.question_type() != entry.question_type
    {
        return Err(());
    }
    metadata.tags = entry.shared_metadata.tags.clone();
    metadata.question_license = Some(entry.question_license.clone());
    Ok(ResolvedQuestionLibraryEntry {
        summary: QuestionSummary {
            question_id: entry.question_revision.question_id.clone(),
            latest_question_revision: entry.question_revision,
            backend: entry.backend,
            question_format: entry.question_format,
            question_type: entry.question_type,
            capabilities: QuestionBackendCapabilities::from_iter([
                Capability::ClientRendering,
                Capability::ServerGrading,
            ]),
            metadata,
            authorship: entry.authorship,
            availability: entry.availability,
            published_at: entry.published_at,
        },
        prompt: presentation.prompt().to_vec(),
        authored_by_current_account: entry.authored_by_current_account,
        used_in_current_account_courses,
        subject,
        topic,
    })
}

fn matches_query(
    entry: &&ResolvedQuestionLibraryEntry,
    query: &QuestionSearchRequest,
    text_query: &search_query::QuestionTextQuery,
) -> bool {
    let summary = &entry.summary;
    if !text_query.matches(entry) {
        return false;
    }
    if !query.author_names.is_empty()
        && !summary.authorship.authors.iter().any(|author| {
            query
                .author_names
                .iter()
                .any(|name| name == &author.display_name.as_str().to_lowercase())
        })
    {
        return false;
    }
    if !query.backends.is_empty() && !query.backends.contains(&summary.backend) {
        return false;
    }
    if !query.tags.is_empty()
        && !summary
            .metadata
            .tags
            .iter()
            .any(|tag| query.tags.contains(&tag.as_str().to_lowercase()))
    {
        return false;
    }
    if !query.question_types.is_empty() && !query.question_types.contains(&summary.question_type) {
        return false;
    }
    if !query
        .capabilities
        .iter()
        .all(|capability| summary.capabilities.supports(*capability))
    {
        return false;
    }
    if !query.question_licenses.is_empty()
        && !summary
            .metadata
            .question_license
            .as_ref()
            .is_some_and(|license| query.question_licenses.contains(license))
    {
        return false;
    }
    if query.used_in_my_courses == QuestionSearchCourseUse::Used
        && !entry.used_in_current_account_courses
    {
        return false;
    }
    query.authorship != QuestionSearchAuthorship::AuthoredByCurrentAccount
        || entry.authored_by_current_account
}

fn facets(entries: &[&ResolvedQuestionLibraryEntry]) -> QuestionSearchFacets {
    let mut authors = BTreeMap::<String, u64>::new();
    let mut backends = BTreeMap::<QuestionBackend, u64>::new();
    let mut tags = BTreeMap::<String, u64>::new();
    let mut question_types = BTreeMap::<question_model::QuestionType, u64>::new();
    let mut capabilities = BTreeMap::<Capability, u64>::new();
    let mut licenses = BTreeMap::<question_model::QuestionLicense, u64>::new();
    for entry in entries {
        let summary = &entry.summary;
        for author in &summary.authorship.authors {
            *authors
                .entry(author.display_name.as_str().to_string())
                .or_default() += 1;
        }
        *backends.entry(summary.backend).or_default() += 1;
        for tag in &summary.metadata.tags {
            *tags.entry(tag.as_str().to_string()).or_default() += 1;
        }
        *question_types.entry(summary.question_type).or_default() += 1;
        for capability in summary.capabilities.declared() {
            *capabilities.entry(capability).or_default() += 1;
        }
        if let Some(license) = &summary.metadata.question_license {
            *licenses.entry(license.clone()).or_default() += 1;
        }
    }
    QuestionSearchFacets {
        author_names: authors
            .into_iter()
            .map(|(author_name, count)| QuestionSearchAuthorFacet { author_name, count })
            .collect(),
        backends: backends
            .into_iter()
            .map(|(backend, count)| QuestionSearchBackendFacet { backend, count })
            .collect(),
        tags: tags
            .into_iter()
            .map(|(tag, count)| QuestionSearchTagFacet { tag, count })
            .collect(),
        question_types: question_types
            .into_iter()
            .map(|(question_type, count)| QuestionTypeFacet {
                question_type,
                count,
            })
            .collect(),
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| QuestionSearchCapabilityFacet { capability, count })
            .collect(),
        question_licenses: licenses
            .into_iter()
            .map(
                |(question_license, count)| QuestionSearchQuestionLicenseFacet {
                    question_license,
                    count,
                },
            )
            .collect(),
        used_in_my_courses: QuestionSearchCourseUseFacet {
            used: entries
                .iter()
                .filter(|entry| entry.used_in_current_account_courses)
                .count() as u64,
        },
    }
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden => concealed(),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Library unavailable",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Library changed")
        }
        StoreError::AlreadyExists
        | StoreError::OwnershipMismatch
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Library unavailable",
        ),
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Question Library lifecycle conflict")
        }
    }
}

fn question_response(
    response: Response,
    edit_number: question_model::QuestionAvailabilityEditNumber,
) -> Response {
    let mut response = crate::auth::no_store(response);
    match HeaderValue::from_str(&format!("\"{edit_number}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}

fn availability_response(
    availability: question_model::QuestionAvailability,
    edit_number: question_model::QuestionAvailabilityEditNumber,
) -> Response {
    question_response(
        Json(QuestionAvailabilityResponse {
            availability,
            edit_number,
        })
        .into_response(),
        edit_number,
    )
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Library unavailable")
}

fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Library unavailable",
    )
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests;
