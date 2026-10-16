//! Global identity validation and classification predicates for Library search.
use axum::{http::StatusCode, response::Response};
use learning_data_access::{
    ContentClassificationStore, ContentDisciplineDiscoveryStore, SessionTokenHash,
};
use question_model::LibraryObjectSearchRequest;

use super::{route_error, store_error_response};

/// ASVS 2.2.1/2.2.2/2.2.3: validate real global identities and their
/// associations after authentication, including the selected cross-mode parent.
// The route returns this response immediately; boxing it would add an allocation
// and require the route boundary to unwrap solely to preserve Axum's `Response`.
#[allow(clippy::result_large_err)]
pub(super) async fn validate(
    store: &(impl ContentClassificationStore + ContentDisciplineDiscoveryStore),
    token: SessionTokenHash,
    query: &LibraryObjectSearchRequest,
) -> Result<(), Response> {
    if query.discipline_uuids.is_empty() {
        return Ok(());
    }
    let invalid = || {
        route_error(
            StatusCode::BAD_REQUEST,
            "Question Library classification is invalid",
        )
    };
    let disciplines = store
        .list_disciplines_including_retired(token)
        .await
        .map_err(store_error_response)?;
    if query
        .discipline_uuids
        .iter()
        .any(|discipline| !disciplines.iter().any(|item| item.uuid == *discipline))
    {
        return Err(invalid());
    }
    let mut subjects = Vec::new();
    for discipline in &query.discipline_uuids {
        subjects.extend(
            store
                .list_subjects(token, *discipline)
                .await
                .map_err(store_error_response)?,
        );
    }
    if query
        .subject_uuids
        .iter()
        .any(|subject| !subjects.iter().any(|item| item.uuid == *subject))
    {
        return Err(invalid());
    }
    let mut topics = Vec::new();
    for subject in &query.subject_uuids {
        topics.extend(
            store
                .list_topics(token, *subject)
                .await
                .map_err(store_error_response)?,
        );
    }
    if query
        .topic_uuids
        .iter()
        .any(|topic| !topics.iter().any(|item| item.uuid == *topic))
    {
        return Err(invalid());
    }
    let mut subtopics = Vec::new();
    for topic in &query.topic_uuids {
        subtopics.extend(
            store
                .list_subtopics(token, *topic)
                .await
                .map_err(store_error_response)?,
        );
    }
    if query
        .subtopic_uuids
        .iter()
        .any(|subtopic| !subtopics.iter().any(|item| item.uuid == *subtopic))
    {
        return Err(invalid());
    }
    Ok(())
}
