//! HOTSPOT workspace-asset resolution and immutable publication promotion.
//!
//! The parent route module owns request authorization and compensation. This
//! module owns the narrower asset boundary: verify the private workspace
//! candidate, mint its version-scoped public identity, copy it, then verify
//! the object store returned precisely that immutable candidate.

use axum::response::Response;
use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, DraftRecord, FlatQuestionAssetStore,
    WorkspaceFlatQuestionAsset,
};
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectStore, PutObject, Sha256Digest};
use question_model::{
    AssetId, DraftQuestionDefinition, ObjectId, ProblemVersionRef, ResponseDefinition, WorkspaceId,
};

use super::{
    flat_source_changed_response, object_error_response, private_store_error, publication_license,
};

/// Resolves the exact private HOTSPOT image named by a compiled workspace draft.
pub(super) async fn resolve_workspace_hotspot_asset<S>(
    store: &S,
    context: learning_data_access::TenantContext,
    workspace: WorkspaceId,
    question: &DraftQuestionDefinition,
) -> Result<Option<WorkspaceFlatQuestionAsset>, Response>
where
    S: FlatQuestionAssetStore,
{
    let ResponseDefinition::Hotspot { surface, .. } = &question.response else {
        return Ok(None);
    };
    let checksum = serde_json::from_str::<Sha256Digest>(&format!("\"{}\"", surface.checksum))
        .map_err(|_| flat_source_changed_response())?;
    match store
        .resolve_workspace_flat_question_asset(context, workspace, surface.asset, checksum)
        .await
    {
        Ok(Some(asset)) => Ok(Some(asset)),
        Ok(None) => Err(flat_source_changed_response()),
        Err(error) => Err(private_store_error(error)),
    }
}

/// Copies the verified private HOTSPOT asset to a fresh, version-scoped public identity.
pub(super) async fn copy_publication_candidate<O>(
    objects: &O,
    draft: &DraftRecord,
    publication: ProblemVersionRef,
    asset: Option<&WorkspaceFlatQuestionAsset>,
) -> Result<(Vec<AssetDeliveryRecord>, Option<AssetId>), Response>
where
    O: ObjectStore,
{
    let Some(asset) = asset else {
        return Ok((Vec::new(), None));
    };
    let stored = objects
        .get(&asset.object.key)
        .await
        .map_err(object_error_response)?;
    if stored.record != asset.object || Sha256Digest::compute(&stored.bytes) != asset.checksum() {
        return Err(flat_source_changed_response());
    }
    let published_asset = AssetId::generate();
    let object = ObjectId::generate();
    let candidate = PutObject {
        key: ObjectKey::ProblemAsset {
            problem: publication.problem,
            version: publication.version,
            asset: published_asset,
            object,
        },
        bytes: stored.bytes,
        media_type: asset.object.media_type.clone(),
        license: publication_license(draft),
        provenance: "PLE flat-question hotspot image".to_string(),
        created_at: asset.object.created_at,
    };
    let record = objects
        .put(candidate)
        .await
        .map_err(object_error_response)?;
    if record.id != object
        || record.key
            != (ObjectKey::ProblemAsset {
                problem: publication.problem,
                version: publication.version,
                asset: published_asset,
                object,
            })
        || record.bucket != Bucket::Content
        || record.category != ObjectCategory::Asset
        || record.version != Some(publication.version)
        || record.sha256 != asset.checksum()
        || record.size_bytes != asset.object.size_bytes
        || record.media_type != asset.object.media_type
    {
        return Err(flat_source_changed_response());
    }
    Ok((
        vec![AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(published_asset),
            object: record,
            intrinsic_width: Some(asset.intrinsic_width),
            intrinsic_height: Some(asset.intrinsic_height),
            scope: AssetDeliveryScope::Catalog {
                asset: published_asset,
                reference: publication,
            },
        }],
        Some(published_asset),
    ))
}
