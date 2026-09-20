//! Private immutable Draft-owned raster upload and verified preview.

use axum::{
    Json,
    body::{Bytes, to_bytes},
    extract::{Path, Request, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use learning_data_access::{
    AuthoringDraftStore, DraftQuestionImageStore, OwnedDraftQuestionImage,
    RegisterDraftQuestionImageInput, SessionTokenHash, StoreError,
};
use objects::{
    ObjectAddress, ObjectStore, PutObject, Sha256Checksum,
    image_validation::{MAX_STILL_IMAGE_BYTES, verify_still_image},
};
use question_model::{ObjectId, QuestionImageAssetId, QuestionImageAssetTuple};
use serde::Serialize;

use crate::authoring::{
    AuthoringRouteState, concealed, expected_edit_number, instructor_session_hash, now,
    parse_draft_question_uuid, private_error, private_store_error,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadedQuestionImage {
    question_image_asset_id: QuestionImageAssetId,
    checksum: String,
    media_type: String,
    intrinsic_width: u32,
    intrinsic_height: u32,
}

pub(crate) async fn upload(
    State(state): State<AuthoringRouteState>,
    Path(draft_question_id): Path<String>,
    request: Request,
) -> Response {
    let headers = request.headers();
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.1/8.2.2/8.3.1: authorize the real existing Draft before any put.
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(value) => value,
        Err(error) => return private_store_error(error),
    };
    if draft.edit_number != expected_edit_number {
        return private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this upload",
        );
    }
    let declared_media = match raster_media_type(headers) {
        Some(value) => value.to_string(),
        None => {
            return private_error(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "PNG, JPEG, or WebP image media type is required",
            );
        }
    };
    // ASVS 5.2.1/5.2.2/5.2.6: limit bytes before buffering; decode bounded raster.
    let bytes = match to_bytes(request.into_body(), MAX_STILL_IMAGE_BYTES).await {
        Ok(value) => value,
        Err(_) => {
            return private_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Draft Question image is too large",
            );
        }
    };
    let verified = match verify_still_image(&bytes) {
        Ok(value) => value,
        Err(_) => return private_error(StatusCode::BAD_REQUEST, "Draft Question image is invalid"),
    };
    if verified.media_type.canonical_media_type() != declared_media {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question image media type does not match its bytes",
        );
    }
    let question_image_asset_id = QuestionImageAssetId::generate();
    // ASVS 5.3.2: random physical identity under a typed owner-derived address.
    let record = match state
        .objects
        .put(PutObject {
            address: ObjectAddress::DraftQuestionImage {
                workspace_id: draft.workspace,
                draft_question_id: draft.draft_question_uuid.as_uuid(),
                question_image_asset_id,
                object_id: ObjectId::generate(),
            },
            bytes: bytes.to_vec(),
            media_type: declared_media,
            created_at: now(),
        })
        .await
    {
        Ok(value) => value,
        Err(_) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    // ASVS 2.3.3/2.3.4: final row-locked registration repeats ownership and CAS.
    match state
        .draft_question_images
        .register_draft_question_image(
            session_hash,
            RegisterDraftQuestionImageInput {
                draft_question_uuid,
                expected_edit_number,
                question_image_asset_id,
                source_record: record,
                intrinsic_width: verified.width,
                intrinsic_height: verified.height,
            },
        )
        .await
    {
        Ok(asset) => crate::auth::no_store(
            (
                StatusCode::CREATED,
                Json(UploadedQuestionImage {
                    question_image_asset_id: asset.question_image_asset_id,
                    checksum: asset.source_record.sha256.to_string(),
                    media_type: asset.source_record.media_type,
                    intrinsic_width: asset.intrinsic_width,
                    intrinsic_height: asset.intrinsic_height,
                }),
            )
                .into_response(),
        ),
        // An ambiguous registration error must not delete potentially committed bytes.
        Err(error) => private_store_error(error),
    }
}

fn raster_media_type(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    matches!(value, "image/png" | "image/jpeg" | "image/webp").then_some(value)
}

pub(crate) async fn require_surface<S: DraftQuestionImageStore + ?Sized>(
    store: &S,
    session_hash: SessionTokenHash,
    draft_question_uuid: learning_data_access::DraftQuestionUuid,
    question_image_asset_tuple: &QuestionImageAssetTuple,
) -> Result<OwnedDraftQuestionImage, StoreError> {
    let asset = store
        .load_draft_question_image(
            session_hash,
            draft_question_uuid,
            question_image_asset_tuple.question_image_asset_id,
        )
        .await?;
    if asset.source_record.sha256.to_string() != question_image_asset_tuple.checksum {
        return Err(StoreError::InvalidRecord(
            "HOTSPOT image checksum does not match this Draft".into(),
        ));
    }
    Ok(asset)
}

pub(crate) async fn verified_question_image_bytes<O: ObjectStore>(
    objects: &O,
    asset: &OwnedDraftQuestionImage,
) -> Result<Bytes, ()> {
    let image = objects
        .get(&asset.source_record.address)
        .await
        .map_err(|_| ())?;
    let measured = verify_still_image(&image.bytes).map_err(|_| ())?;
    if image.record != asset.source_record
        || Sha256Checksum::compute(&image.bytes) != asset.source_record.sha256
        || u64::try_from(image.bytes.len()).ok() != Some(asset.source_record.size_bytes)
        || measured.media_type.canonical_media_type() != asset.source_record.media_type
        || measured.width != asset.intrinsic_width
        || measured.height != asset.intrinsic_height
    {
        return Err(());
    }
    Ok(Bytes::from(image.bytes))
}

pub(crate) async fn preview(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path((draft_question_id, question_image_asset_id)): Path<(String, String)>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let question_image_asset_id = match uuid::Uuid::parse_str(&question_image_asset_id) {
        Ok(value) if value.to_string() == question_image_asset_id => {
            QuestionImageAssetId::from_uuid(value)
        }
        _ => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let asset = match state
        .draft_question_images
        .load_draft_question_image(session_hash, draft_question_uuid, question_image_asset_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return private_store_error(error),
    };
    let bytes = match verified_question_image_bytes(&state.objects, &asset).await {
        Ok(value) => value,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let Ok(media_type) = HeaderValue::from_str(&asset.source_record.media_type) else {
        return private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Authoring storage is unavailable",
        );
    };
    let mut response = crate::auth::no_store(bytes.into_response());
    // ASVS 3.4.4/3.5.8/5.4.1: measured type, no sniffing, private embedding only.
    response.headers_mut().insert(CONTENT_TYPE, media_type);
    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    response
}
