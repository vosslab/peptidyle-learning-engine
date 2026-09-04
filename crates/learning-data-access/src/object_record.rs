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
        workspace: address_workspace,
        object,
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
        || record.question_revision.is_some()
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

#[cfg(test)]
mod tests {
    use objects::{ObjectDataClass, ObjectStorageArea, Sha256Checksum};
    use question_model::{ObjectId, Timestamp};
    use uuid::Uuid;

    use super::*;

    fn object_record(workspace: WorkspaceId) -> ObjectRecord {
        let id = ObjectId::from_uuid(Uuid::from_u128(2));
        let address = ObjectAddress::WorkspaceQuestionSource {
            workspace,
            object: id,
        };
        ObjectRecord {
            id,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::AuthoringContent,
            address,
            sha256: Sha256Checksum::compute(b"Question Source"),
            size_bytes: 15,
            media_type: "application/json".to_string(),
            question_revision: None,
            created_at: Timestamp::from_unix_millis(1_000),
        }
    }

    #[test]
    fn workspace_question_source_object_record_requires_its_exact_owner_address() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
        let record = object_record(workspace);

        assert_eq!(
            validate_workspace_question_source_object_record(workspace, &record),
            Ok(())
        );
        assert_eq!(
            validate_workspace_question_source_object_record(
                WorkspaceId::from_uuid(Uuid::from_u128(3)),
                &record,
            ),
            Err(StoreError::OwnershipMismatch)
        );
    }
}
