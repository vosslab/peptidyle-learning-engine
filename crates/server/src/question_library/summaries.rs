//! Answer-free Question Library summaries and HTTP mapping.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
};
use learning_data_access::{PublishedQuestionLibraryEntry, StoreError};
use objects::{ObjectStore, ResolvedQuestionSource};
use question_model::{QuestionBackend, QuestionSummary};

use super::QuestionAvailabilityResponse;

pub(super) struct ResolvedQuestionLibraryEntry {
    pub(super) summary: QuestionSummary,
    pub(super) prompt: Vec<question_model::QuestionContentBlock>,
    pub(super) response_preview: Option<question_model::QuestionResponsePreview>,
    pub(super) subject: Option<String>,
    pub(super) discipline: Option<String>,
    pub(super) discipline_is_retired: bool,
}

pub(super) async fn entries_to_summaries<O: ObjectStore>(
    objects: &O,
    entries: Vec<PublishedQuestionLibraryEntry>,
) -> Result<Vec<ResolvedQuestionLibraryEntry>, ()> {
    let mut summaries = Vec::with_capacity(entries.len());
    for entry in entries {
        summaries.push(answer_free_question_library_entry(objects, entry).await?);
    }
    Ok(summaries)
}

pub(super) async fn summary_from_entry<O: ObjectStore>(
    objects: &O,
    entry: PublishedQuestionLibraryEntry,
) -> Result<QuestionSummary, ()> {
    Ok(answer_free_question_library_entry(objects, entry)
        .await?
        .summary)
}

/// Produces the one browser-safe Question Library entry shape from the
/// backend-owned source boundary. Private source bindings never cross this
/// boundary into a summary or search result.
pub(super) async fn answer_free_question_library_entry<O: ObjectStore>(
    objects: &O,
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
    let tags = entry.shared_metadata.tags.clone();
    Ok(ResolvedQuestionLibraryEntry {
        summary: QuestionSummary {
            question_id: entry
                .published_question_revision_tuple
                .published_question_id
                .clone(),
            published_question_revision_tuple: entry.published_question_revision_tuple,
            backend: entry.backend,
            question_format: entry.question_format,
            question_type: entry.question_type,
            capabilities: super::facets::backend_capabilities(QuestionBackend::Webwork),
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
        subject: Some(entry.subject_name.clone()),
        discipline: Some(entry.discipline_name),
        discipline_is_retired: entry.discipline_is_retired,
    })
}

async fn resolved_ple_question<O: ObjectStore>(
    objects: &O,
    entry: PublishedQuestionLibraryEntry,
) -> Result<ResolvedQuestionLibraryEntry, ()> {
    if entry.backend != QuestionBackend::Ple {
        return Err(());
    }
    let source = ResolvedQuestionSource::resolve(
        objects,
        entry.published_question_revision_tuple.clone(),
        entry.source_object_id,
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
            question_id: entry
                .published_question_revision_tuple
                .published_question_id
                .clone(),
            published_question_revision_tuple: entry.published_question_revision_tuple,
            backend: entry.backend,
            question_format: entry.question_format,
            question_type: entry.question_type,
            capabilities: super::facets::backend_capabilities(QuestionBackend::Ple),
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
        subject: Some(entry.subject_name.clone()),
        discipline: Some(entry.discipline_name),
        discipline_is_retired: entry.discipline_is_retired,
    })
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
            question_availability_edit_number: edit_number,
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
