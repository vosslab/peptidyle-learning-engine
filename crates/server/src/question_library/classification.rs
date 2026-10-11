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
    let Some(discipline) = query.discipline_uuid else {
        return Ok(());
    };
    let invalid = || {
        route_error(
            StatusCode::BAD_REQUEST,
            "Question Library classification is invalid",
        )
    };
    if !store
        .list_disciplines_including_retired(token)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == discipline)
    {
        return Err(invalid());
    }
    let Some(subject) = query.subject_uuid else {
        return Ok(());
    };
    if !store
        .list_subjects(token, discipline)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == subject)
    {
        return Err(invalid());
    }
    let Some(topic) = query.topic_uuid else {
        return Ok(());
    };
    if !store
        .list_topics(token, subject)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == topic)
    {
        return Err(invalid());
    }
    let Some(subtopic) = query.subtopic_uuid else {
        return Ok(());
    };
    if !store
        .list_subtopics(token, topic)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == subtopic)
    {
        return Err(invalid());
    }
    Ok(())
}
