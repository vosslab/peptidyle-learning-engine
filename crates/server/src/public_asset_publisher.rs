//! One-shot dedicated publisher for immutable public Question Asset renditions.
//!
//! It owns no listener, session, Account, generic Job, or caller-selected
//! object key. The registry claim fixes the private source and final public
//! destination before this process sees either object identity.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use learning_data_access::{ClaimedQuestionAssetPublication, PublicAssetPublicationStore};
use objects::{
    ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, ObjectStore, ObjectStoreError,
    PutObject, Sha256Checksum, image_validation::verify_still_image,
};
use question_model::Timestamp;
use uuid::Uuid;

/// Claims and publishes at most one registry-backed Question Asset Job.
///
/// A supervisor may invoke a fresh one-shot process for subsequent work. This
/// deliberately avoids a generic in-process queue or retry policy.
pub async fn publish_one<S, O>(store: &S, objects: &O) -> Result<bool>
where
    S: PublicAssetPublicationStore,
    O: ObjectStore,
{
    let lease_token = Uuid::now_v7();
    let Some(publication) = store
        .claim_question_asset_publication(lease_token)
        .await
        .map_err(|_| anyhow!("could not claim a Question Asset Publication Job"))?
    else {
        return Ok(false);
    };
    publish_claim(objects, &publication).await?;
    store
        .activate_question_asset_publication(publication.job_id, lease_token)
        .await
        .map_err(|_| anyhow!("could not activate a Question Asset Publication"))?;
    tracing::info!(event = "public_asset_publication_completed");
    Ok(true)
}

async fn publish_claim<O: ObjectStore>(
    objects: &O,
    publication: &ClaimedQuestionAssetPublication,
) -> Result<()> {
    let source_address = ObjectAddress::RestrictedQuestionAsset {
        question_revision: publication.question_revision.clone(),
        asset: publication.asset_id,
        object: publication.source_object_id,
    };
    let source = objects
        .get(&source_address)
        .await
        .map_err(|_| anyhow!("could not read the claimed restricted Question Asset"))?;
    require_exact_source(&source.record, &source_address, publication)?;
    let verified = verify_still_image(&source.bytes)
        .map_err(|_| anyhow!("claimed restricted Question Asset is not a valid still image"))?;
    if verified.media_type.canonical_media_type() != publication.verified_media_type
        || verified.width != publication.intrinsic_width
        || verified.height != publication.intrinsic_height
        || Sha256Checksum::compute(&source.bytes) != publication.public_checksum
        || u64::try_from(source.bytes.len()).ok() != Some(publication.public_byte_length)
    {
        return Err(anyhow!(
            "claimed Question Asset does not match its fixed public rendition"
        ));
    }

    let public_address = ObjectAddress::QuestionAsset {
        question_revision: publication.question_revision.clone(),
        asset: publication.asset_id,
        object: publication.public_object_id,
    };
    let expected = ExpectedPublicRecord {
        address: &public_address,
        checksum: publication.public_checksum,
        byte_length: publication.public_byte_length,
        media_type: &publication.verified_media_type,
    };
    match objects
        .put(PutObject {
            address: public_address.clone(),
            bytes: source.bytes,
            media_type: publication.verified_media_type.clone(),
            created_at: now(),
        })
        .await
    {
        Ok(record) => require_exact_public_record(&record, expected),
        Err(ObjectStoreError::AlreadyExists) => {
            let existing = objects
                .get(&public_address)
                .await
                .map_err(|_| anyhow!("could not verify the existing public Question Asset"))?;
            require_exact_public_record(&existing.record, expected)
        }
        Err(_) => Err(anyhow!(
            "could not write the immutable public Question Asset"
        )),
    }
}

fn require_exact_source(
    record: &ObjectRecord,
    address: &ObjectAddress,
    publication: &ClaimedQuestionAssetPublication,
) -> Result<()> {
    if record.address != *address
        || record.id != publication.source_object_id
        || record.storage_area != ObjectStorageArea::PrivateContent
        || record.data_class != ObjectDataClass::QuestionAsset
        || record.sha256 != publication.source_checksum
        || record.media_type != publication.verified_media_type
    {
        return Err(anyhow!(
            "claimed restricted Question Asset record is not exact"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ExpectedPublicRecord<'a> {
    address: &'a ObjectAddress,
    checksum: Sha256Checksum,
    byte_length: u64,
    media_type: &'a str,
}

fn require_exact_public_record(
    record: &ObjectRecord,
    expected: ExpectedPublicRecord<'_>,
) -> Result<()> {
    if record.address != *expected.address
        || record.id != expected.address.object_id()
        || record.storage_area != ObjectStorageArea::PublicAssets
        || record.data_class != ObjectDataClass::QuestionAsset
        || record.sha256 != expected.checksum
        || record.size_bytes != expected.byte_length
        || record.media_type != expected.media_type
    {
        return Err(anyhow!("public Question Asset record is not exact"));
    }
    Ok(())
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
    use learning_data_access::{PublicAssetPublicationStore, StoreError};
    use objects::memory::MemoryObjectStore;
    use question_model::{
        ObjectId, QuestionAssetId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference,
    };

    use super::*;

    #[derive(Clone)]
    struct RecordingPublicationStore {
        publication: ClaimedQuestionAssetPublication,
        activated: Arc<Mutex<Option<(Uuid, Uuid)>>>,
    }

    #[async_trait]
    impl PublicAssetPublicationStore for RecordingPublicationStore {
        async fn claim_question_asset_publication(
            &self,
            _lease_token: Uuid,
        ) -> Result<Option<ClaimedQuestionAssetPublication>, StoreError> {
            Ok(Some(self.publication.clone()))
        }

        async fn activate_question_asset_publication(
            &self,
            job_id: Uuid,
            lease_token: Uuid,
        ) -> Result<(), StoreError> {
            *self.activated.lock().expect("activation lock") = Some((job_id, lease_token));
            Ok(())
        }
    }

    fn png() -> Vec<u8> {
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&[0, 32, 64, 128, 160, 192], 2, 1, ColorType::Rgb8.into())
            .expect("image fixture encodes");
        encoded
    }

    #[tokio::test]
    async fn publisher_copies_only_its_claimed_restricted_asset_to_the_fixed_public_address() {
        let objects = MemoryObjectStore::default();
        let question_revision = QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
        };
        let asset_id = QuestionAssetId::from_uuid(Uuid::from_u128(1));
        let source_object_id = ObjectId::from_uuid(Uuid::from_u128(2));
        let public_object_id = ObjectId::from_uuid(Uuid::from_u128(3));
        let bytes = png();
        let source_address = ObjectAddress::RestrictedQuestionAsset {
            question_revision: question_revision.clone(),
            asset: asset_id,
            object: source_object_id,
        };
        let source_record = objects
            .put(PutObject {
                address: source_address,
                bytes: bytes.clone(),
                media_type: "image/png".to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("restricted source");
        let publication = ClaimedQuestionAssetPublication {
            job_id: Uuid::from_u128(4),
            question_revision: question_revision.clone(),
            asset_id,
            source_object_id,
            source_checksum: source_record.sha256,
            public_object_id,
            public_checksum: source_record.sha256,
            public_byte_length: source_record.size_bytes,
            verified_media_type: "image/png".to_string(),
            intrinsic_width: 2,
            intrinsic_height: 1,
        };
        let activated = Arc::new(Mutex::new(None));
        let store = RecordingPublicationStore {
            publication,
            activated: Arc::clone(&activated),
        };

        assert!(publish_one(&store, &objects).await.expect("publisher run"));
        let public_address = ObjectAddress::QuestionAsset {
            question_revision,
            asset: asset_id,
            object: public_object_id,
        };
        let public = objects.get(&public_address).await.expect("public asset");
        assert_eq!(public.bytes, bytes);
        assert_eq!(public.record.address, public_address);
        assert_eq!(
            activated
                .lock()
                .expect("activation lock")
                .as_ref()
                .map(|(job, _)| *job),
            Some(Uuid::from_u128(4))
        );
    }
}
