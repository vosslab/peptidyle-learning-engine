//! Server-derived Published Question fork persistence.
//!
//! The browser names only an already-public Question Revision in the request
//! path. This contract keeps the source pin, private workspace, Draft identity,
//! and idempotency receipt inside trusted code.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::{QuestionAssetId, QuestionRevisionTuple, WorkspaceId};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Inputs the trusted server has resolved for one fork attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkPublishedQuestionInput {
    /// Existing available immutable source Revision, resolved from the path.
    pub source_question_revision_tuple: QuestionRevisionTuple,
    /// Existing owner workspace resolved through the ordinary authoring Store.
    pub workspace: WorkspaceId,
    /// Opaque server persistence identity; it never crosses the browser boundary.
    pub proposed_draft_question_id: Uuid,
    /// Fresh immutable copy of the exact source bytes in the target workspace.
    pub target_source_record: ObjectRecord,
    /// Fresh Draft-owned copy of the exact native HOTSPOT raster, when required.
    pub hotspot_asset: Option<ForkPublishedQuestionAssetInput>,
    /// Opaque caller retry key, scoped by PostgreSQL to the authenticated actor.
    pub idempotency_key: Uuid,
}

/// Fresh target facts for the one native HOTSPOT raster owned by the new Draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkPublishedQuestionAssetInput {
    pub asset_id: QuestionAssetId,
    pub target_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Exact immutable published HOTSPOT raster selected by its source Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionForkAsset {
    pub asset_id: QuestionAssetId,
    pub source_record: ObjectRecord,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

impl ForkPublishedQuestionInput {
    /// Refuses target objects outside this server-selected Draft/workspace.
    pub fn validate(&self) -> Result<(), StoreError> {
        let source = &self.target_source_record;
        if source.address
            != (ObjectAddress::WorkspaceQuestionSource {
                workspace_id: self.workspace,
                object_id: source.id,
            })
            || source.storage_area != ObjectStorageArea::PrivateContent
            || source.data_class != ObjectDataClass::AuthoringContent
            || source.question_revision_tuple.is_some()
        {
            return Err(StoreError::InvalidRecord(
                "Question Fork target source is not owned by its workspace".to_owned(),
            ));
        }
        if let Some(asset) = &self.hotspot_asset {
            let record = &asset.target_record;
            if record.address
                != (ObjectAddress::DraftQuestionAsset {
                    workspace_id: self.workspace,
                    draft_question_id: self.proposed_draft_question_id,
                    question_asset_id: asset.asset_id,
                    object_id: record.id,
                })
                || record.storage_area != ObjectStorageArea::PrivateContent
                || record.data_class != ObjectDataClass::AuthoringContent
                || record.question_revision_tuple.is_some()
            {
                return Err(StoreError::InvalidRecord(
                    "Question Fork target asset is not owned by its Draft".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

/// Browser-safe result for an authorized private fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkedPublishedQuestionDraft {
    /// Private Draft UUID for authenticated route transport only.
    pub draft_question_uuid: crate::DraftQuestionUuid,
    /// Server-only workspace that owns the Draft.
    pub workspace: WorkspaceId,
    /// Whether this invocation committed the aggregate rather than replaying its receipt.
    pub created_new: bool,
}

/// Session-authorized source resolution and atomic Draft fork capability.
#[async_trait]
pub trait QuestionForkStore: Send + Sync {
    /// Loads the exact immutable raster for a native HOTSPOT Revision.
    ///
    /// This historical evidence read intentionally has no availability
    /// predicate; the atomic mutation owns that new-operation precondition.
    async fn load_published_question_fork_asset(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision_tuple: &QuestionRevisionTuple,
    ) -> Result<Option<PublishedQuestionForkAsset>, StoreError>;

    /// Creates or returns the actor/key's one private Draft fork atomically.
    async fn fork_published_question_to_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: ForkPublishedQuestionInput,
    ) -> Result<ForkedPublishedQuestionDraft, StoreError>;
}
