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
use objects::{ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, PutObject, Sha256Digest};
use question_model::{
    AssetId, DraftQuestionDefinition, ObjectId, ProblemVersionRef, PublicationScope,
    ResponseDefinition, WorkspaceId,
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

/// Prepares the verified HOTSPOT asset for a fresh, version-scoped identity.
///
/// Institutional content is copied before the catalog transaction because its
/// private key is not browser-deliverable. Public content instead creates only
/// a pending registry target that pins the private source; the post-commit
/// publisher worker materializes the public key.
pub(super) async fn copy_publication_candidate<O>(
    objects: &O,
    draft: &DraftRecord,
    publication: ProblemVersionRef,
    scope: PublicationScope,
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
    let key = ObjectKey::published_problem_asset(
        scope,
        publication.problem,
        publication.version,
        published_asset,
        object,
    );
    let candidate = PutObject {
        key: key.clone(),
        bytes: stored.bytes,
        media_type: asset.object.media_type.clone(),
        license: publication_license(draft),
        provenance: "PLE flat-question hotspot image".to_string(),
        created_at: asset.object.created_at,
    };
    let (record, asset_publication, pending_source) = if scope == PublicationScope::Public {
        // Never create a CDN-readable key until the catalog publication has
        // committed and the durable outbox has made recovery possible.
        (
            ObjectRecord {
                id: object,
                bucket: key.bucket(),
                key,
                sha256: asset.checksum(),
                size_bytes: asset.object.size_bytes,
                media_type: asset.object.media_type.clone(),
                category: ObjectCategory::Asset,
                version: Some(publication.version),
                license: candidate.license,
                provenance: candidate.provenance,
                created_at: candidate.created_at,
            },
            learning_data_access::AssetPublication::Pending,
            Some(asset.object.clone()),
        )
    } else {
        (
            objects
                .put(candidate)
                .await
                .map_err(object_error_response)?,
            learning_data_access::AssetPublication::Ready,
            None,
        )
    };
    if record.id != object
        || record.key
            != ObjectKey::published_problem_asset(
                scope,
                publication.problem,
                publication.version,
                published_asset,
                object,
            )
        || record.bucket != record.key.bucket()
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
            publication: asset_publication,
            pending_source,
        }],
        Some(published_asset),
    ))
}
