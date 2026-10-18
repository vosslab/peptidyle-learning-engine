//! Authorized direct delivery of one immutable private Question Thumbnail.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use learning_data_access::{QuestionLibraryStore, StoreError};
use objects::{
    ObjectAddress, ObjectDataClass, ObjectStorageArea, ObjectStore, PreparedQuestionThumbnail,
};

use super::{
    QuestionLibraryRouteState, library_reader_session_hash,
    verified_published_question_revision_tuple,
};

/// Delivers the only private thumbnail representation after the exact Library
/// Revision resolver authorizes the current Instructor or Sysadmin.
pub(super) async fn get_question_thumbnail(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
) -> Response {
    let Some(published_question_revision_tuple) =
        verified_published_question_revision_tuple(&question_id, &revision_number)
    else {
        return concealed();
    };
    let (session_hash, _) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.1/8.2.2: this store call proves the caller may read this exact
    // Revision before the typed private object identity is ever used.
    let thumbnail = match state
        .store
        .resolve_question_thumbnail(session_hash, &published_question_revision_tuple)
        .await
    {
        Ok(value) => value,
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(_) => return unavailable(),
    };
    let expected_address = ObjectAddress::QuestionThumbnail {
        published_question_revision_tuple,
        object_id: thumbnail.object_record.id,
    };
    if thumbnail.object_record.address != expected_address
        || thumbnail.object_record.storage_area != ObjectStorageArea::PrivateContent
        || thumbnail.object_record.data_class != ObjectDataClass::QuestionThumbnail
        || thumbnail.object_record.media_type != "image/webp"
    {
        return unavailable();
    }
    let stored = match state.objects.get(&thumbnail.object_record.address).await {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let prepared = match PreparedQuestionThumbnail::new(stored.bytes) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if stored.record != thumbnail.object_record
        || prepared.width() != thumbnail.intrinsic_width
        || prepared.height() != thumbnail.intrinsic_height
    {
        return unavailable();
    }
    thumbnail_response(prepared.into_bytes())
}

fn thumbnail_response(bytes: Vec<u8>) -> Response {
    let mut response = crate::auth::no_store(Bytes::from(bytes).into_response());
    let headers = response.headers_mut();
    // ASVS 3.2.1, 3.4.4, and 5.3.2: exact measured media, no sniffing, and
    // same-origin private embedding prevent a thumbnail from becoming a file
    // or script delivery surface.
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

fn concealed() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

fn unavailable() -> Response {
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}
