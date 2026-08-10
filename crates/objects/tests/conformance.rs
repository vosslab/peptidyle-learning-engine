//! Reusable ObjectStore conformance suite, first run against memory in WP-C4.

use objects::memory::MemoryObjectStore;
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest,
};
use question_model::{
    ActivityTimestamp, AssetId, CourseBannerCandidateId, CourseBannerId, CourseId, ObjectId,
    ProblemId, TenantId, VersionId, WorkspaceId, WorkspaceImportId,
};
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
    assert_eq!(
        store
            .signed_url(&key, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "answer-bearing source must remain server-only"
    );
    let archive_key = ObjectKey::PublishedImportArchive {
        tenant: TenantId::from_uuid(id(40)),
        problem: ProblemId::from_uuid(id(41)),
        version: VersionId::from_uuid(id(42)),
        import: WorkspaceImportId::from_uuid(id(43)),
        object: ObjectId::from_uuid(id(44)),
    };
    store
        .put(PutObject {
            key: archive_key.clone(),
            bytes: b"published import archive".to_vec(),
            media_type: "application/zip".to_string(),
            license: "private provenance".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("published archive put should succeed");
    assert_eq!(
        store
            .signed_url(&archive_key, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "published import provenance must remain server-only"
    );
    let asset_key = ObjectKey::ProblemAsset {
        problem: ProblemId::from_uuid(id(1)),
        version: VersionId::from_uuid(id(2)),
        asset: AssetId::from_uuid(id(13)),
        object: ObjectId::from_uuid(id(14)),
    };
    store
        .put(PutObject {
            key: asset_key.clone(),
            bytes: b"published asset".to_vec(),
            media_type: "image/png".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("published asset put should succeed");
    let signed = store
        .signed_url(&asset_key, ActivityTimestamp::from_unix_millis(2_000))
        .await
        .expect("published assets should be signable");
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

    let banner_candidate_key = ObjectKey::CourseBannerCandidate {
        tenant: TenantId::from_uuid(id(50)),
        course: CourseId::from_uuid(id(51)),
        candidate: CourseBannerCandidateId::from_uuid(id(52)),
    };
    let banner_candidate_record = store
        .put(PutObject {
            key: banner_candidate_key.clone(),
            bytes: b"normalized candidate".to_vec(),
            media_type: "image/webp".to_string(),
            license: "tenant course branding".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("course banner candidate put should succeed");
    assert_eq!(banner_candidate_record.bucket, Bucket::TempProcessing);
    assert_eq!(banner_candidate_record.category, ObjectCategory::Temporary);
    assert_eq!(
        store
            .signed_url(
                &banner_candidate_key,
                ActivityTimestamp::from_unix_millis(2_000)
            )
            .await,
        Err(ObjectStoreError::NotSignable),
        "course banner candidates must never be delivery targets"
    );

    let course_banner_key = ObjectKey::CourseBanner {
        tenant: TenantId::from_uuid(id(50)),
        course: CourseId::from_uuid(id(51)),
        banner: CourseBannerId::from_uuid(id(53)),
    };
    let course_banner_record = store
        .put(PutObject {
            key: course_banner_key.clone(),
            bytes: b"current course banner".to_vec(),
            media_type: "image/webp".to_string(),
            license: "tenant course branding".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("course banner put should succeed");
    assert_eq!(course_banner_record.bucket, Bucket::Content);
    assert_eq!(course_banner_record.category, ObjectCategory::CourseContent);
    store
        .signed_url(
            &course_banner_key,
            ActivityTimestamp::from_unix_millis(2_000),
        )
        .await
        .expect("current course banners are signable after separate pointer authorization");

    let workspace_source = ObjectKey::WorkspaceSource {
        tenant: TenantId::from_uuid(id(7)),
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        object: ObjectId::from_uuid(id(10)),
    };
    let workspace_question_source = ObjectKey::WorkspaceQuestionSource {
        tenant: TenantId::from_uuid(id(7)),
        workspace: WorkspaceId::from_uuid(id(8)),
        object: ObjectId::from_uuid(id(15)),
    };
    let workspace_asset = ObjectKey::WorkspaceAsset {
        tenant: TenantId::from_uuid(id(7)),
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        asset: AssetId::from_uuid(id(11)),
        object: ObjectId::from_uuid(id(12)),
    };
    for key in [
        workspace_source.clone(),
        workspace_question_source.clone(),
        workspace_asset.clone(),
    ] {
        let record = store
            .put(PutObject {
                key: key.clone(),
                bytes: b"private workspace import".to_vec(),
                media_type: "application/zip".to_string(),
                license: "private".to_string(),
                provenance: "fixture".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1_000),
            })
            .await
            .expect("workspace import put should succeed");
        assert_eq!(record.bucket, Bucket::Content);
        assert_eq!(record.version, None);
        assert_eq!(
            store
                .signed_url(&key, ActivityTimestamp::from_unix_millis(2_000))
                .await,
            Err(ObjectStoreError::NotSignable)
        );
    }

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
    store
        .delete(&archive_key)
        .await
        .expect("published archive cleanup should succeed");
    assert_eq!(store.get(&key).await, Err(ObjectStoreError::NotFound));
    store
        .delete(&student_key)
        .await
        .expect("student record cleanup should succeed");
    store
        .delete(&asset_key)
        .await
        .expect("published asset cleanup should succeed");
    store
        .delete(&temporary_key)
        .await
        .expect("temporary cleanup should succeed");
    store
        .delete(&banner_candidate_key)
        .await
        .expect("course banner candidate cleanup should succeed");
    store
        .delete(&course_banner_key)
        .await
        .expect("course banner cleanup should succeed");
    store
        .delete(&workspace_source)
        .await
        .expect("workspace source cleanup");
    store
        .delete(&workspace_question_source)
        .await
        .expect("workspace question source cleanup");
    store
        .delete(&workspace_asset)
        .await
        .expect("workspace asset cleanup");
}

#[test]
fn workspace_object_paths_bind_tenant_workspace_and_import_identity() {
    let source = ObjectKey::WorkspaceSource {
        tenant: TenantId::from_uuid(id(20)),
        workspace: WorkspaceId::from_uuid(id(21)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    let other_tenant = ObjectKey::WorkspaceSource {
        tenant: TenantId::from_uuid(id(24)),
        workspace: WorkspaceId::from_uuid(id(21)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    assert_ne!(source, other_tenant);
    assert_ne!(source.path(), other_tenant.path());
    assert_eq!(source.bucket(), Bucket::Content);
    assert_eq!(source.category(), ObjectCategory::Source);
    assert_eq!(source.version_id(), None);
    assert!(source.path().starts_with("workspaces/"));
    assert!(!source.path().starts_with("problems/"));
}

#[test]
fn workspace_question_source_key_has_stable_workspace_path_and_is_private_source() {
    let source = ObjectKey::WorkspaceQuestionSource {
        tenant: TenantId::from_uuid(id(30)),
        workspace: WorkspaceId::from_uuid(id(31)),
        object: ObjectId::from_uuid(id(32)),
    };
    assert_eq!(
        source.path(),
        "workspaces/00000000-0000-0000-0000-00000000001e/00000000-0000-0000-0000-00000000001f/questions/source/00000000-0000-0000-0000-000000000020",
        "workspace question source path should encode tenant and workspace ids"
    );
    assert_eq!(source.bucket(), Bucket::Content);
    assert_eq!(source.category(), ObjectCategory::Source);
    assert_eq!(source.version_id(), None);
    assert!(!source.path().contains("imports"));
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
