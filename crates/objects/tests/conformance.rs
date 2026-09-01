//! Reusable ObjectStore conformance suite, first run against memory in WP-C4.

use objects::memory::MemoryObjectStore;
use objects::{
    ObjectAddress, ObjectDataClass, ObjectStorageArea, ObjectStore, ObjectStoreError, PutObject,
    Sha256Checksum,
};
use question_model::{
    CourseBannerId, CourseBannerUploadReference, CourseId, ObjectId, QuestionAssetId, QuestionId,
    QuestionRevisionNumber, QuestionRevisionReference, Timestamp, WorkspaceId, WorkspaceImportId,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn question_revision(revision_number: u32) -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(revision_number).expect("positive version"),
    }
}

async fn exercise_object_store(store: &dyn ObjectStore) {
    let key = ObjectAddress::QuestionSource {
        question_revision: question_revision(2),
        object: ObjectId::from_uuid(id(3)),
    };
    let request = PutObject {
        address: key.clone(),
        bytes: b"published source".to_vec(),
        media_type: "application/zip".to_string(),
        created_at: Timestamp::from_unix_millis(1_000),
    };

    let record = store
        .put(request.clone())
        .await
        .expect("conforming put should succeed");
    assert_eq!(record.data_class, ObjectDataClass::QuestionSource);
    let stored = store
        .get(&key)
        .await
        .expect("conforming get should succeed");
    assert_eq!(stored.record, record);
    assert_eq!(
        store
            .signed_url(&key, Timestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "answer-bearing source must remain server-only"
    );
    let archive_key = ObjectAddress::PublishedImportArchive {
        question_revision: question_revision(42),
        import: WorkspaceImportId::from_uuid(id(43)),
        object: ObjectId::from_uuid(id(44)),
    };
    store
        .put(PutObject {
            address: archive_key.clone(),
            bytes: b"published import archive".to_vec(),
            media_type: "application/zip".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("published archive put should succeed");
    assert_eq!(
        store
            .signed_url(&archive_key, Timestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "published import evidence must remain server-only"
    );
    let asset_key = ObjectAddress::QuestionAsset {
        question_revision: question_revision(2),
        asset: QuestionAssetId::from_uuid(id(13)),
        object: ObjectId::from_uuid(id(14)),
    };
    store
        .put(PutObject {
            address: asset_key.clone(),
            bytes: b"published asset".to_vec(),
            media_type: "image/png".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("published asset put should succeed");
    let signed = store
        .signed_url(&asset_key, Timestamp::from_unix_millis(2_000))
        .await
        .expect("published assets should be signable");
    assert_eq!(
        store
            .get(&asset_key)
            .await
            .expect("published asset should be stored")
            .record
            .data_class,
        ObjectDataClass::QuestionAsset
    );
    let student_key = ObjectAddress::StudentRecord {
        course: CourseId::from_uuid(id(4)),
        object: ObjectId::from_uuid(id(5)),
    };
    store
        .put(PutObject {
            address: student_key.clone(),
            bytes: b"student export".to_vec(),
            media_type: "application/pdf".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("student record put should succeed");
    let student_signed = store
        .signed_url(&student_key, Timestamp::from_unix_millis(2_000))
        .await
        .expect("student record should be signable");
    assert_eq!(
        store
            .get(&student_key)
            .await
            .expect("student record should be stored")
            .record
            .data_class,
        ObjectDataClass::StudentRecord
    );
    let temporary_key = ObjectAddress::Temporary {
        object: ObjectId::from_uuid(id(6)),
    };
    store
        .put(PutObject {
            address: temporary_key.clone(),
            bytes: b"temporary workspace".to_vec(),
            media_type: "application/octet-stream".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("temporary put should succeed");

    let banner_upload_key = ObjectAddress::CourseBannerUpload {
        course: CourseId::from_uuid(id(51)),
        upload: CourseBannerUploadReference::from_uuid(id(52)),
    };
    let banner_upload_record = store
        .put(PutObject {
            address: banner_upload_key.clone(),
            bytes: b"normalized upload".to_vec(),
            media_type: "image/webp".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("course banner upload put should succeed");
    assert_eq!(
        banner_upload_record.storage_area,
        ObjectStorageArea::TempProcessing
    );
    assert_eq!(
        banner_upload_record.data_class,
        ObjectDataClass::CourseAppearance
    );
    assert_eq!(
        store
            .signed_url(&banner_upload_key, Timestamp::from_unix_millis(2_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "Course Banner Uploads must never be delivery targets"
    );

    let course_banner_key = ObjectAddress::CourseBanner {
        course: CourseId::from_uuid(id(51)),
        banner: CourseBannerId::from_uuid(id(53)),
    };
    let course_banner_record = store
        .put(PutObject {
            address: course_banner_key.clone(),
            bytes: b"current course banner".to_vec(),
            media_type: "image/webp".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("course banner put should succeed");
    assert_eq!(
        course_banner_record.storage_area,
        ObjectStorageArea::PrivateContent
    );
    assert_eq!(
        course_banner_record.data_class,
        ObjectDataClass::CourseAppearance
    );
    store
        .signed_url(&course_banner_key, Timestamp::from_unix_millis(2_000))
        .await
        .expect("current course banners are signable after separate pointer authorization");

    let workspace_import_source = ObjectAddress::WorkspaceImportSource {
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        object: ObjectId::from_uuid(id(10)),
    };
    let workspace_question_source = ObjectAddress::WorkspaceQuestionSource {
        workspace: WorkspaceId::from_uuid(id(8)),
        object: ObjectId::from_uuid(id(15)),
    };
    let workspace_import_asset = ObjectAddress::WorkspaceImportAsset {
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        asset: QuestionAssetId::from_uuid(id(11)),
        object: ObjectId::from_uuid(id(12)),
    };
    let workspace_question_asset = ObjectAddress::WorkspaceQuestionAsset {
        workspace: WorkspaceId::from_uuid(id(8)),
        asset: QuestionAssetId::from_uuid(id(16)),
        object: ObjectId::from_uuid(id(17)),
    };
    for key in [
        workspace_import_source.clone(),
        workspace_question_source.clone(),
        workspace_import_asset.clone(),
        workspace_question_asset.clone(),
    ] {
        let record = store
            .put(PutObject {
                address: key.clone(),
                bytes: b"private workspace import".to_vec(),
                media_type: "application/zip".to_string(),
                created_at: Timestamp::from_unix_millis(1_000),
            })
            .await
            .expect("workspace import put should succeed");
        assert_eq!(record.storage_area, ObjectStorageArea::PrivateContent);
        assert_eq!(record.data_class, ObjectDataClass::AuthoringContent);
        assert_eq!(record.question_revision, None);
        assert_eq!(
            store
                .signed_url(&key, Timestamp::from_unix_millis(2_000))
                .await,
            Err(ObjectStoreError::NotSignable)
        );
    }

    assert_eq!(
        (
            record.sha256,
            record.storage_area,
            record.question_revision,
            record.size_bytes,
            stored.bytes,
            signed.expires_at,
            student_signed.expires_at,
            store.put(request).await,
        ),
        (
            Sha256Checksum::compute(b"published source"),
            ObjectStorageArea::PrivateContent,
            Some(question_revision(2)),
            16,
            b"published source".to_vec(),
            Timestamp::from_unix_millis(3_602_000),
            Timestamp::from_unix_millis(302_000),
            Err(ObjectStoreError::AlreadyExists),
        )
    );
    assert_eq!(
        store
            .signed_url(&temporary_key, Timestamp::from_unix_millis(2_000))
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
        .delete(&banner_upload_key)
        .await
        .expect("course banner upload cleanup should succeed");
    store
        .delete(&course_banner_key)
        .await
        .expect("course banner cleanup should succeed");
    store
        .delete(&workspace_import_source)
        .await
        .expect("workspace source cleanup");
    store
        .delete(&workspace_question_source)
        .await
        .expect("workspace question source cleanup");
    store
        .delete(&workspace_import_asset)
        .await
        .expect("workspace asset cleanup");
    store
        .delete(&workspace_question_asset)
        .await
        .expect("workspace question asset cleanup");
}

#[test]
fn workspace_object_paths_bind_workspace_and_import_identity() {
    let source = ObjectAddress::WorkspaceImportSource {
        workspace: WorkspaceId::from_uuid(id(21)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    let other_workspace = ObjectAddress::WorkspaceImportSource {
        workspace: WorkspaceId::from_uuid(id(24)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    assert_ne!(source, other_workspace);
    assert_ne!(source.path(), other_workspace.path());
    assert_eq!(source.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(source.question_revision(), None);
    assert!(source.path().starts_with("workspaces/"));
    assert!(!source.path().starts_with("problems/"));
}

#[test]
fn workspace_question_source_key_has_stable_workspace_path_and_is_private_source() {
    let source = ObjectAddress::WorkspaceQuestionSource {
        workspace: WorkspaceId::from_uuid(id(31)),
        object: ObjectId::from_uuid(id(32)),
    };
    assert_eq!(
        source.path(),
        "workspaces/00000000-0000-0000-0000-00000000001f/questions/source/00000000-0000-0000-0000-000000000020",
        "workspace question source path should encode the workspace id"
    );
    assert_eq!(source.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(source.question_revision(), None);
    assert!(!source.path().contains("imports"));
}

#[test]
fn workspace_question_asset_key_is_private_content_without_import_or_version() {
    let asset = ObjectAddress::WorkspaceQuestionAsset {
        workspace: WorkspaceId::from_uuid(id(34)),
        asset: QuestionAssetId::from_uuid(id(35)),
        object: ObjectId::from_uuid(id(36)),
    };
    assert_eq!(
        asset.path(),
        "workspaces/00000000-0000-0000-0000-000000000022/questions/assets/00000000-0000-0000-0000-000000000023/00000000-0000-0000-0000-000000000024"
    );
    assert_eq!(asset.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(asset.object_id(), ObjectId::from_uuid(id(36)));
    assert_eq!(asset.question_revision(), None);
    assert!(!asset.may_issue_signed_url());
    assert!(!asset.path().contains("imports"));
    assert!(!asset.path().contains("versions"));
}

#[tokio::test]
async fn memory_object_store_conforms() {
    exercise_object_store(&MemoryObjectStore::default()).await;
}

#[cfg(feature = "s3")]
#[tokio::test]
#[ignore = "requires a running MinIO stack and explicit credentials"]
async fn minio_object_store_conforms() {
    let runtime = acceptance_runtime::CourseAppearanceRuntime::load()
        .expect("validated course-appearance acceptance runtime");
    use objects::minio::{EndpointConfig, client};
    use objects::s3::{BucketNames, S3ObjectStore};

    let minio = runtime.minio();
    let settings = EndpointConfig {
        endpoint_url: minio.endpoint_url().to_owned(),
        region: minio.region().to_owned(),
        access_key_id: minio.access_key_id().to_owned(),
        secret_access_key: minio.secret_access_key().to_owned(),
    };
    let store = S3ObjectStore::new(client(&settings), BucketNames::default());

    exercise_object_store(&store).await;
}
