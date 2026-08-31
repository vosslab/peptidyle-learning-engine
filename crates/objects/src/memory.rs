//! In-memory object backend (WP-C4, MOD-OBJ).

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::{
    ObjectAddress, ObjectRecord, ObjectStorageArea, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest, SignedUrl, StoredObject,
};
use question_model::ActivityTimestamp;

/// Object backend used by contract tests and lanes waiting for MinIO.
#[derive(Debug, Clone, Default)]
pub struct MemoryObjectStore {
    entries: Arc<RwLock<BTreeMap<ObjectAddress, StoredObject>>>,
}

#[async_trait]
impl ObjectStore for MemoryObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        let size_bytes =
            u64::try_from(request.bytes.len()).map_err(|_| ObjectStoreError::NumericOverflow)?;
        let record = ObjectRecord {
            id: request.address.object_id(),
            storage_area: request.address.storage_area(),
            data_class: request.address.data_class(),
            question_version: request.address.question_version().cloned(),
            sha256: Sha256Digest::compute(&request.bytes),
            size_bytes,
            media_type: request.media_type,
            license: request.license,
            provenance: request.provenance,
            created_at: request.created_at,
            address: request.address.clone(),
        };
        let stored = StoredObject {
            record: record.clone(),
            bytes: request.bytes,
        };
        let mut entries = self
            .entries
            .write()
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        if entries.contains_key(&request.address) {
            return Err(ObjectStoreError::AlreadyExists);
        }
        entries.insert(request.address, stored);
        Ok(record)
    }

    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        let entries = self
            .entries
            .read()
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        let stored = entries
            .get(address)
            .cloned()
            .ok_or(ObjectStoreError::NotFound)?;
        if Sha256Digest::compute(&stored.bytes) != stored.record.sha256 {
            return Err(ObjectStoreError::ChecksumMismatch);
        }
        Ok(stored)
    }

    async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        entries.remove(address).ok_or(ObjectStoreError::NotFound)?;
        Ok(())
    }

    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        if !address.may_issue_signed_url() {
            return Err(ObjectStoreError::NotSignable);
        }
        {
            let entries = self
                .entries
                .read()
                .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
            if !entries.contains_key(address) {
                return Err(ObjectStoreError::NotFound);
            }
        }
        let lifetime_millis = match address.storage_area() {
            ObjectStorageArea::PublicAssets | ObjectStorageArea::PrivateContent => {
                60_i64 * 60 * 1_000
            }
            ObjectStorageArea::StudentRecords => 5_i64 * 60 * 1_000,
            ObjectStorageArea::TempProcessing => return Err(ObjectStoreError::NotSignable),
        };
        let expires_millis = now
            .as_unix_millis()
            .checked_add(lifetime_millis)
            .ok_or(ObjectStoreError::NumericOverflow)?;
        let expires_at = ActivityTimestamp::from_unix_millis(expires_millis);
        Ok(SignedUrl {
            url: format!(
                "memory://{}/{}?expires={expires_millis}",
                address.storage_area().as_str(),
                address.path()
            ),
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::ObjectId;
    use uuid::Uuid;

    #[tokio::test]
    async fn read_refuses_bytes_that_no_longer_match_the_record() {
        let store = MemoryObjectStore::default();
        let key = ObjectAddress::Temporary {
            object: ObjectId::from_uuid(Uuid::from_u128(1)),
        };
        store
            .put(PutObject {
                address: key.clone(),
                bytes: b"original".to_vec(),
                media_type: "application/octet-stream".to_string(),
                license: None,
                provenance: "test".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("put should succeed");

        store
            .entries
            .write()
            .expect("test lock should be available")
            .get_mut(&key)
            .expect("test object should exist")
            .bytes = b"tampered".to_vec();

        assert_eq!(
            store.get(&key).await,
            Err(ObjectStoreError::ChecksumMismatch)
        );
    }
}
