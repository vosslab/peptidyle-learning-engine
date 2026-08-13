//! Immutable private image assets for native flat-question authoring.
//!
//! The object store owns bytes. This contract owns the verified metadata that a
//! workspace question may reference. Workspace assets are never browser routes
//! and must be promoted through a later, explicitly authorized publication flow.

use async_trait::async_trait;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{AssetId, TenantId, WorkspaceId};
use serde::{Deserialize, Serialize};

use crate::{StoreError, TenantContext};

/// Maximum user-visible image label length after trimming.
pub const MAX_WORKSPACE_FLAT_QUESTION_ASSET_LABEL_CHARS: usize = 160;
/// Maximum retained provenance length after trimming.
pub const MAX_WORKSPACE_FLAT_QUESTION_ASSET_PROVENANCE_CHARS: usize = 1_024;

/// Image formats accepted for native flat-question workspace assets.
///
/// SVG is intentionally excluded: a hotspot surface needs intrinsic raster
/// dimensions and must not introduce active-document semantics into the
/// private authoring asset boundary.
pub const WORKSPACE_FLAT_QUESTION_IMAGE_MEDIA_TYPES: &[&str] =
    &["image/jpeg", "image/png", "image/webp"];

/// Verified immutable metadata for one private workspace-question image.
///
/// The exact object record is retained so a later promotion can bind the
/// published object to the author-verified checksum rather than accepting a
/// browser path, URL, or claimed digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFlatQuestionAsset {
    /// Tenant which owns the private workspace.
    pub tenant: TenantId,
    /// Private workspace that owns the asset.
    pub workspace: WorkspaceId,
    /// Logical image identity referenced by native flat-question source.
    pub asset: AssetId,
    /// Exact immutable object metadata selected by the authoring workflow.
    pub object: ObjectRecord,
    /// Verified intrinsic image width in pixels.
    pub intrinsic_width: u32,
    /// Verified intrinsic image height in pixels.
    pub intrinsic_height: u32,
    /// Short visible name for the instructor's asset picker.
    pub display_label: String,
}

impl WorkspaceFlatQuestionAsset {
    /// Builds a verified descriptor after image inspection and object storage.
    pub fn new(
        tenant: TenantId,
        workspace: WorkspaceId,
        asset: AssetId,
        object: ObjectRecord,
        intrinsic_width: u32,
        intrinsic_height: u32,
        display_label: String,
    ) -> Result<Self, StoreError> {
        validate_workspace_flat_question_asset_record(&tenant, &workspace, asset, &object)?;
        if intrinsic_width == 0 || intrinsic_height == 0 {
            return Err(StoreError::InvalidRecord(
                "workspace flat-question image dimensions must be nonzero".to_string(),
            ));
        }
        let display_label = validate_safe_text(
            "workspace flat-question image label",
            &display_label,
            MAX_WORKSPACE_FLAT_QUESTION_ASSET_LABEL_CHARS,
        )?;
        validate_safe_text(
            "workspace flat-question image provenance",
            &object.provenance,
            MAX_WORKSPACE_FLAT_QUESTION_ASSET_PROVENANCE_CHARS,
        )?;
        let descriptor = Self {
            tenant,
            workspace,
            asset,
            object,
            intrinsic_width,
            intrinsic_height,
            display_label,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Checksum of the exact object bytes selected for this logical asset.
    pub fn checksum(&self) -> Sha256Digest {
        self.object.sha256
    }

    /// Revalidates a descriptor decoded from durable storage before use.
    pub fn validate(&self) -> Result<(), StoreError> {
        validate_workspace_flat_question_asset_record(
            &self.tenant,
            &self.workspace,
            self.asset,
            &self.object,
        )?;
        if self.intrinsic_width == 0 || self.intrinsic_height == 0 {
            return Err(StoreError::InvalidRecord(
                "workspace flat-question image dimensions must be nonzero".to_string(),
            ));
        }
        if validate_safe_text(
            "workspace flat-question image label",
            &self.display_label,
            MAX_WORKSPACE_FLAT_QUESTION_ASSET_LABEL_CHARS,
        )? != self.display_label
        {
            return Err(StoreError::InvalidRecord(
                "workspace flat-question image label must be trimmed".to_string(),
            ));
        }
        if validate_safe_text(
            "workspace flat-question image provenance",
            &self.object.provenance,
            MAX_WORKSPACE_FLAT_QUESTION_ASSET_PROVENANCE_CHARS,
        )? != self.object.provenance
        {
            return Err(StoreError::InvalidRecord(
                "workspace flat-question image provenance must be trimmed".to_string(),
            ));
        }
        Ok(())
    }
}

/// Validates an exact private workspace-question image record.
pub fn validate_workspace_flat_question_asset_record(
    tenant: &TenantId,
    workspace: &WorkspaceId,
    asset: AssetId,
    record: &ObjectRecord,
) -> Result<(), StoreError> {
    let ObjectKey::WorkspaceQuestionAsset {
        tenant: key_tenant,
        workspace: key_workspace,
        asset: key_asset,
        object,
    } = &record.key
    else {
        return Err(StoreError::InvalidRecord(
            "flat-question image must use the workspace question asset key".to_string(),
        ));
    };
    if key_tenant != tenant
        || key_workspace != workspace
        || *key_asset != asset
        || record.id != *object
    {
        return Err(StoreError::InvalidRecord(
            "flat-question image key must match its workspace asset identity".to_string(),
        ));
    }
    if record.bucket != Bucket::PrivateContent || record.key.bucket() != Bucket::PrivateContent {
        return Err(StoreError::InvalidRecord(
            "flat-question image must be stored in the private-content bucket".to_string(),
        ));
    }
    if record.category != ObjectCategory::Asset || record.key.category() != ObjectCategory::Asset {
        return Err(StoreError::InvalidRecord(
            "flat-question image must be an asset object".to_string(),
        ));
    }
    if record.version.is_some() {
        return Err(StoreError::InvalidRecord(
            "flat-question image must not have a published version".to_string(),
        ));
    }
    if record.size_bytes == 0 {
        return Err(StoreError::InvalidRecord(
            "flat-question image must contain bytes".to_string(),
        ));
    }
    if !WORKSPACE_FLAT_QUESTION_IMAGE_MEDIA_TYPES.contains(&record.media_type.as_str()) {
        return Err(StoreError::InvalidRecord(
            "flat-question image media type is not allowed".to_string(),
        ));
    }
    if record.license.trim().is_empty() {
        return Err(StoreError::InvalidRecord(
            "flat-question image license is required".to_string(),
        ));
    }
    Ok(())
}

fn validate_safe_text(name: &str, value: &str, max_chars: usize) -> Result<String, StoreError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidRecord(format!(
            "{name} must be nonempty, at most {max_chars} characters, and contain no control characters"
        )));
    }
    Ok(value.to_string())
}

/// Private immutable image registry for native flat-question workspaces.
#[async_trait]
pub trait FlatQuestionAssetStore: Send + Sync {
    /// Registers a descriptor once. An exact retry returns its original value;
    /// reuse of the logical asset identity with any divergent metadata fails.
    async fn register_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        descriptor: WorkspaceFlatQuestionAsset,
    ) -> Result<WorkspaceFlatQuestionAsset, StoreError>;

    /// Lists one workspace's descriptors in stable logical-asset order.
    async fn list_workspace_flat_question_assets(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Vec<WorkspaceFlatQuestionAsset>, StoreError>;

    /// Resolves only an exact private asset/checksum pair.
    ///
    /// Absence, foreign ownership, and a checksum mismatch all return `None`.
    /// Callers therefore cannot use this capability to discover another
    /// workspace's private object metadata.
    async fn resolve_workspace_flat_question_asset(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        asset: AssetId,
        checksum: Sha256Digest,
    ) -> Result<Option<WorkspaceFlatQuestionAsset>, StoreError>;
}
