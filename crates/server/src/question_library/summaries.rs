//! Answer-free Question Library summaries, search matching, and HTTP mapping.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
};
use learning_data_access::{PublishedQuestionLibraryEntry, StoreError};
use objects::{ResolvedQuestionSource, s3::S3ObjectStore};
use question_model::{
    Capability, QuestionBackend, QuestionBackendCapabilities, QuestionSearchAuthorship,
    QuestionSearchCourseUse, QuestionSearchRequest, QuestionSummary,
    normalized_question_search_group_value,
};

use super::{QuestionAvailabilityResponse, classification, search_query};

pub(super) struct ResolvedQuestionLibraryEntry {
    pub(super) summary: QuestionSummary,
    pub(super) prompt: Vec<question_model::QuestionContentBlock>,
    pub(super) response_preview: Option<question_model::QuestionResponsePreview>,
    pub(super) authored_by_current_account: bool,
    pub(super) used_in_current_account_courses: bool,
    pub(super) subject: Option<String>,
    pub(super) topic: Option<String>,
    pub(super) discipline: Option<String>,
    pub(super) discipline_is_retired: bool,
    pub(super) subtopic: Option<String>,
    pub(super) classification: question_model::PublishedQuestionSharedMetadata,
}

pub(super) async fn entries_to_summaries(
    objects: &S3ObjectStore,
    entries: Vec<PublishedQuestionLibraryEntry>,
) -> Result<Vec<ResolvedQuestionLibraryEntry>, ()> {
    let mut summaries = Vec::with_capacity(entries.len());
    for entry in entries {
        summaries.push(answer_free_question_library_entry(objects, entry).await?);
    }
    Ok(summaries)
}

pub(super) async fn summary_from_entry(
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
pub(super) async fn answer_free_question_library_entry(
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
    let subject = Some(entry.subject_name.clone());
    let topic = entry.topic_name.clone();
    Ok(ResolvedQuestionLibraryEntry {
        summary: QuestionSummary {
            question_id: entry.question_revision_tuple.question_id.clone(),
            question_revision_tuple: entry.question_revision_tuple,
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
            bloom: entry.bloom,
        },
        prompt: Vec::new(),
        response_preview: None,
        authored_by_current_account: entry.authored_by_current_account,
        used_in_current_account_courses,
        subject,
        topic,
        discipline: Some(entry.discipline_name),
        discipline_is_retired: entry.discipline_is_retired,
        subtopic: entry.subtopic_name,
        classification: entry.shared_metadata,
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
        entry.question_revision_tuple.clone(),
        entry.source_object_id.clone(),
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
    let subject = Some(entry.subject_name.clone());
    let topic = entry.topic_name.clone();
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
            question_id: entry.question_revision_tuple.question_id.clone(),
            question_revision_tuple: entry.question_revision_tuple,
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
            bloom: entry.bloom,
        },
        prompt: presentation.prompt().to_vec(),
        response_preview: Some(
            question_model::QuestionResponsePreview::from_native_response(presentation.response())
                .ok_or(())?,
        ),
        authored_by_current_account: entry.authored_by_current_account,
        used_in_current_account_courses,
        subject,
        topic,
        discipline: Some(entry.discipline_name),
        discipline_is_retired: entry.discipline_is_retired,
        subtopic: entry.subtopic_name,
        classification: entry.shared_metadata,
    })
}

pub(super) fn matches_query(
    entry: &&ResolvedQuestionLibraryEntry,
    query: &QuestionSearchRequest,
    text_query: &search_query::QuestionTextQuery,
) -> bool {
    let summary = &entry.summary;
    if !classification::matches(&entry.classification, query) {
        return false;
    }
    if !text_query.matches(entry) {
        return false;
    }
    if query.bloom_cognitive_process.is_some_and(|value| {
        summary
            .bloom
            .as_ref()
            .is_none_or(|bloom| value != bloom.cognitive_process)
    }) {
        return false;
    }
    if query.bloom_knowledge_dimension.is_some_and(|value| {
        summary
            .bloom
            .as_ref()
            .is_none_or(|bloom| value != bloom.knowledge_dimension)
    }) {
        return false;
    }
    if !query.author_names.is_empty()
        && !summary.authorship.authors.iter().any(|author| {
            query.author_names.iter().any(|name| {
                name == &normalized_question_search_group_value(author.display_name.as_str())
            })
        })
    {
        return false;
    }
    if !query.backends.is_empty() && !query.backends.contains(&summary.backend) {
        return false;
    }
    if !query.tags.is_empty()
        && !summary.metadata.tags.iter().any(|tag| {
            query
                .tags
                .contains(&normalized_question_search_group_value(tag.as_str()))
        })
    {
        return false;
    }
    if !query.subjects.is_empty()
        && !entry.subject.as_ref().is_some_and(|subject| {
            query
                .subjects
                .contains(&normalized_question_search_group_value(subject))
        })
    {
        return false;
    }
    if !query.topics.is_empty()
        && !entry.topic.as_ref().is_some_and(|topic| {
            query
                .topics
                .contains(&normalized_question_search_group_value(topic))
        })
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

pub(super) fn store_error_response(error: StoreError) -> Response {
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

pub(super) fn question_response(
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

pub(super) fn availability_response(
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

pub(super) fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Library unavailable")
}

pub(super) fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Library unavailable",
    )
}

pub(super) fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}
