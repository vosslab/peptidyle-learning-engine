//! Authorized registration of private Workspace Question Source Object Records.
//!
//! Object bytes are written first through `objects::ObjectStore`. This
//! persistence boundary then records the exact immutable metadata only after
//! current workspace authorization succeeds.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectRecord};
use question_model::WorkspaceId;

use crate::{SessionTokenHash, StoreError};

/// Validates the exact private Object Record shape accepted for a workspace
/// Workspace Question Source Object creation.
pub fn validate_workspace_question_source_object_record(
    workspace: WorkspaceId,
    record: &ObjectRecord,
) -> Result<(), StoreError> {
    let ObjectAddress::WorkspaceQuestionSource {
        workspace_id: address_workspace,
        object_id: object,
    } = &record.address
    else {
        return Err(StoreError::InvalidRecord(
            "Workspace Question Source Object creation requires its exact Object Address"
                .to_string(),
        ));
    };
    if *address_workspace != workspace {
        return Err(StoreError::OwnershipMismatch);
    }
    if *object != record.id
        || record.storage_area != record.address.storage_area()
        || record.data_class != record.address.data_class()
        || record.published_question_revision_tuple.is_some()
    {
        return Err(StoreError::InvalidRecord(
            "Object Record metadata must be derived from its Workspace Question Source Object Address"
                .to_string(),
        ));
    }
    Ok(())
}

/// Persists one immutable private Question Source Object Record after bytes-first
/// object storage and current workspace authorization.
#[async_trait]
pub trait WorkspaceQuestionSourceObjectRecordStore: Send + Sync {
    /// Registers an exact Object Record owned by the authenticated workspace.
    async fn register_workspace_question_source_object(
        &self,
        session_token_hash: SessionTokenHash,
        workspace: WorkspaceId,
        record: ObjectRecord,
    ) -> Result<(), StoreError>;
}
