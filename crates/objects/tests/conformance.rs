//! Reusable ObjectStore conformance suite, first run against memory in WP-C4.

use objects::memory::MemoryObjectStore;
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest,
};
use question_model::{ActivityTimestamp, ObjectId, ProblemId, TenantId, VersionId};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn exercise_object_store(store: &dyn ObjectStore) {
    let key = ObjectKey::ProblemSource {
        problem: ProblemId::from_uuid(id(1)),
        version: VersionId::from_uuid(id(2)),
        object: ObjectId::from_uuid(id(3)),
    };
    let request = PutObject {
        key: key.clone(),
        bytes: b"published source".to_vec(),
        media_type: "application/zip".to_string(),
        license: "CC-BY-SA-4.0".to_string(),
        provenance: "fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1_000),
    };

    let record = store
        .put(request.clone())
        .await
        .expect("conforming put should succeed");
    let stored = store
        .get(&key)
        .await
        .expect("conforming get should succeed");
    assert_eq!(stored.record, record);
    let signed = store
        .signed_url(&key, ActivityTimestamp::from_unix_millis(2_000))
        .await
        .expect("content should be signable");
    let student_key = ObjectKey::StudentRecord {
        tenant: TenantId::from_uuid(id(4)),
        object: ObjectId::from_uuid(id(5)),
    };
    store
        .put(PutObject {
            key: student_key.clone(),
            bytes: b"student export".to_vec(),
            media_type: "application/pdf".to_string(),
            license: "educational-record".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("student record put should succeed");
    let student_signed = store
        .signed_url(&student_key, ActivityTimestamp::from_unix_millis(2_000))
        .await
        .expect("student record should be signable");
    let temporary_key = ObjectKey::Temporary {
        object: ObjectId::from_uuid(id(6)),
    };
    store
        .put(PutObject {
            key: temporary_key.clone(),
            bytes: b"temporary workspace".to_vec(),
            media_type: "application/octet-stream".to_string(),
            license: "private".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("temporary put should succeed");

    assert_eq!(
        (
            record.sha256,
            record.bucket,
            record.category,
            record.version,
            record.size_bytes,
            stored.bytes,
            signed.expires_at,
            student_signed.expires_at,
            store.put(request).await,
        ),
        (
            Sha256Digest::compute(b"published source"),
            Bucket::Content,
            ObjectCategory::Source,
            Some(VersionId::from_uuid(id(2))),
            16,
            b"published source".to_vec(),
            ActivityTimestamp::from_unix_millis(3_602_000),
            ActivityTimestamp::from_unix_millis(302_000),
            Err(ObjectStoreError::AlreadyExists),
        )
    );
    assert_eq!(
        store
            .signed_url(&temporary_key, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable)
    );

    store
        .delete(&key)
        .await
        .expect("conforming delete should succeed");
    assert_eq!(store.get(&key).await, Err(ObjectStoreError::NotFound));
    store
        .delete(&student_key)
        .await
        .expect("student record cleanup should succeed");
    store
        .delete(&temporary_key)
        .await
        .expect("temporary cleanup should succeed");
}

#[tokio::test]
async fn memory_object_store_conforms() {
    exercise_object_store(&MemoryObjectStore::default()).await;
}

#[cfg(feature = "s3")]
#[tokio::test]
#[ignore = "requires a running MinIO stack and explicit credentials"]
async fn minio_object_store_conforms() {
    use objects::minio::{EndpointConfig, client};
    use objects::s3::{BucketNames, S3ObjectStore};

    let settings = EndpointConfig {
        endpoint_url: std::env::var("PLE_S3_ENDPOINT").expect("PLE_S3_ENDPOINT must be set"),
        region: std::env::var("PLE_S3_REGION").expect("PLE_S3_REGION must be set"),
        access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set"),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
            .expect("AWS_SECRET_ACCESS_KEY must be set"),
    };
    let store = S3ObjectStore::new(client(&settings), BucketNames::default());

    exercise_object_store(&store).await;
}
