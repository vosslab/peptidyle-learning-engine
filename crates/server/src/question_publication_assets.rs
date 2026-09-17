//! Verified Draft image loading and immutable revision preparation.

use axum::body::Bytes;
use learning_data_access::{
    DraftQuestionUuid, OwnedDraftQuestionAsset, PreparedQuestionAssetPublication, StoreError,
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    ObjectId, QuestionResponseFormat, QuestionRevisionReference, Timestamp, WorkspaceId,
};
use uuid::Uuid;

use crate::{
    authoring_assets::{require_surface, verified_asset_bytes},
    question_publication::{AuthoringAssetContext, QuestionPublicationError},
};

/// Read the one surface through the authoritative native compiler, not another parser.
pub(crate) async fn load_hotspot_asset<O: ObjectStore>(
    objects: &O,
    context: Option<&AuthoringAssetContext>,
    session_hash: learning_data_access::SessionTokenHash,
    workspace: WorkspaceId,
    draft: DraftQuestionUuid,
    source: &[u8],
    media_type: &str,
) -> Result<Option<(OwnedDraftQuestionAsset, Bytes)>, QuestionPublicationError> {
    if media_type != "application/vnd.peptidyle.question+json" {
        return Ok(None);
    }
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source)
        .map_err(|_| invalid_source())?;
    let compiled = document.compile().map_err(|_| invalid_source())?;
    let QuestionResponseFormat::Hotspot { surface, .. } = compiled.presentation().response() else {
        return Ok(None);
    };
    let context = context.ok_or_else(invalid_source)?;
    let asset = require_surface(
        context.store.as_ref(),
        session_hash,
        context.draft_question_uuid,
        surface,
    )
    .await
    .map_err(QuestionPublicationError::Store)?;
    // ASVS 8.2.2/8.3.1: never accept an image selected from another Draft context.
    if !matches!(&asset.source_record.address,
        ObjectAddress::DraftQuestionAsset { workspace: owner, draft_question_uuid, asset: id, object }
        if *owner == workspace && *draft_question_uuid == draft.as_uuid()
            && *id == surface.question_asset && *object == asset.source_record.id)
    {
        return Err(QuestionPublicationError::SourceObjectRecordMismatch);
    }
    // ASVS 5.2.2/5.2.6: repeat full raster verification at the publication boundary.
    let bytes = verified_asset_bytes(objects, &asset)
        .await
        .map_err(|()| QuestionPublicationError::SourceObjectRecordMismatch)?;
    Ok(Some((asset, bytes)))
}

fn invalid_source() -> QuestionPublicationError {
    QuestionPublicationError::Store(StoreError::InvalidRecord(
        "Native Question source or Draft-owned HOTSPOT image is invalid".into(),
    ))
}

pub(crate) async fn prepare_hotspot_asset<O: ObjectStore>(
    objects: &O,
    asset: Option<&(OwnedDraftQuestionAsset, Bytes)>,
    revision: &QuestionRevisionReference,
    stored_at: Timestamp,
) -> Result<Option<PreparedQuestionAssetPublication>, QuestionPublicationError> {
    let Some((asset, bytes)) = asset else {
        return Ok(None);
    };
    // ASVS 5.3.2: new physical identity; stable logical image UUID and exact bytes.
    let record = objects
        .put(PutObject {
            address: ObjectAddress::RestrictedQuestionAsset {
                question_revision: revision.clone(),
                asset: asset.asset_id,
                object: ObjectId::generate(),
            },
            bytes: bytes.to_vec(),
            media_type: asset.source_record.media_type.clone(),
            created_at: stored_at,
        })
        .await
        .map_err(QuestionPublicationError::ObjectStore)?;
    Ok(Some(PreparedQuestionAssetPublication {
        asset_id: asset.asset_id,
        restricted_source_record: record,
        public_object_id: ObjectId::generate(),
        intrinsic_width: asset.intrinsic_width,
        intrinsic_height: asset.intrinsic_height,
        delivery_id: Uuid::now_v7(),
        job_id: Uuid::now_v7(),
    }))
}

/// Only call for conclusive transaction rollback, never an ambiguous store outcome.
pub(crate) async fn cleanup_targets<O: ObjectStore>(
    objects: &O,
    source: &ObjectAddress,
    image: Option<&ObjectAddress>,
) -> Result<(), QuestionPublicationError> {
    // Attempt both exact request-owned targets even if either deletion fails.
    let source_result = objects.delete(source).await;
    let image_result = match image {
        Some(image) => objects.delete(image).await,
        None => Ok(()),
    };
    source_result
        .and(image_result)
        .map_err(QuestionPublicationError::ObjectStore)
}
