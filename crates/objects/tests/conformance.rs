//! Reusable ObjectStore conformance suite, first run against memory in WP-C4.

use objects::memory::MemoryObjectStore;
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest,
};
use question_model::{
    ActivityTimestamp, AssetId, CourseBannerCandidateId, CourseBannerId, CourseId, ObjectId,
    QuestionId, QuestionVersionNumber, QuestionVersionReference, WorkspaceId, WorkspaceImportId,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn question_version(version_number: u32) -> QuestionVersionReference {
    QuestionVersionReference {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        version_number: QuestionVersionNumber::new(version_number).expect("positive version"),
    }
}

async fn exercise_object_store(store: &dyn ObjectStore) {
    let key = ObjectKey::QuestionSource {
        question_version: question_version(2),
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
        question_version: question_version(42),
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
    let asset_key = ObjectKey::QuestionAsset {
        question_version: question_version(2),
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
        course: CourseId::from_uuid(id(4)),
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
        course: CourseId::from_uuid(id(51)),
        candidate: CourseBannerCandidateId::from_uuid(id(52)),
    };
    let banner_candidate_record = store
        .put(PutObject {
            key: banner_candidate_key.clone(),
            bytes: b"normalized candidate".to_vec(),
            media_type: "image/webp".to_string(),
            license: "course branding".to_string(),
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
        course: CourseId::from_uuid(id(51)),
        banner: CourseBannerId::from_uuid(id(53)),
    };
    let course_banner_record = store
        .put(PutObject {
            key: course_banner_key.clone(),
            bytes: b"current course banner".to_vec(),
            media_type: "image/webp".to_string(),
            license: "course branding".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("course banner put should succeed");
    assert_eq!(course_banner_record.bucket, Bucket::PrivateContent);
    assert_eq!(course_banner_record.category, ObjectCategory::CourseContent);
    store
        .signed_url(
            &course_banner_key,
            ActivityTimestamp::from_unix_millis(2_000),
        )
        .await
        .expect("current course banners are signable after separate pointer authorization");

    let workspace_source = ObjectKey::WorkspaceSource {
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        object: ObjectId::from_uuid(id(10)),
    };
    let workspace_question_source = ObjectKey::WorkspaceQuestionSource {
        workspace: WorkspaceId::from_uuid(id(8)),
        object: ObjectId::from_uuid(id(15)),
    };
    let workspace_asset = ObjectKey::WorkspaceAsset {
        workspace: WorkspaceId::from_uuid(id(8)),
        import: WorkspaceImportId::from_uuid(id(9)),
        asset: AssetId::from_uuid(id(11)),
        object: ObjectId::from_uuid(id(12)),
    };
    let workspace_question_asset = ObjectKey::WorkspaceQuestionAsset {
        workspace: WorkspaceId::from_uuid(id(8)),
        asset: AssetId::from_uuid(id(16)),
        object: ObjectId::from_uuid(id(17)),
    };
    for key in [
        workspace_source.clone(),
        workspace_question_source.clone(),
        workspace_asset.clone(),
        workspace_question_asset.clone(),
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
        assert_eq!(record.bucket, Bucket::PrivateContent);
        assert_eq!(record.question_version, None);
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
            record.question_version,
            record.size_bytes,
            stored.bytes,
            signed.expires_at,
            student_signed.expires_at,
            store.put(request).await,
        ),
        (
            Sha256Digest::compute(b"published source"),
            Bucket::PrivateContent,
            ObjectCategory::Source,
            Some(question_version(2)),
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
    store
        .delete(&workspace_question_asset)
        .await
        .expect("workspace question asset cleanup");
}

#[test]
fn workspace_object_paths_bind_workspace_and_import_identity() {
    let source = ObjectKey::WorkspaceSource {
        workspace: WorkspaceId::from_uuid(id(21)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    let other_workspace = ObjectKey::WorkspaceSource {
        workspace: WorkspaceId::from_uuid(id(24)),
        import: WorkspaceImportId::from_uuid(id(22)),
        object: ObjectId::from_uuid(id(23)),
    };
    assert_ne!(source, other_workspace);
    assert_ne!(source.path(), other_workspace.path());
    assert_eq!(source.bucket(), Bucket::PrivateContent);
    assert_eq!(source.category(), ObjectCategory::Source);
    assert_eq!(source.question_version(), None);
    assert!(source.path().starts_with("workspaces/"));
    assert!(!source.path().starts_with("problems/"));
}

#[test]
fn workspace_question_source_key_has_stable_workspace_path_and_is_private_source() {
    let source = ObjectKey::WorkspaceQuestionSource {
        workspace: WorkspaceId::from_uuid(id(31)),
        object: ObjectId::from_uuid(id(32)),
    };
    assert_eq!(
        source.path(),
        "workspaces/00000000-0000-0000-0000-00000000001f/questions/source/00000000-0000-0000-0000-000000000020",
        "workspace question source path should encode the workspace id"
    );
    assert_eq!(source.bucket(), Bucket::PrivateContent);
    assert_eq!(source.category(), ObjectCategory::Source);
    assert_eq!(source.question_version(), None);
    assert!(!source.path().contains("imports"));
}

#[test]
fn workspace_question_asset_key_is_private_content_without_import_or_version() {
    let asset = ObjectKey::WorkspaceQuestionAsset {
        workspace: WorkspaceId::from_uuid(id(34)),
        asset: AssetId::from_uuid(id(35)),
        object: ObjectId::from_uuid(id(36)),
    };
    assert_eq!(
        asset.path(),
        "workspaces/00000000-0000-0000-0000-000000000022/questions/assets/00000000-0000-0000-0000-000000000023/00000000-0000-0000-0000-000000000024"
    );
    assert_eq!(asset.bucket(), Bucket::PrivateContent);
    assert_eq!(asset.category(), ObjectCategory::Asset);
    assert_eq!(asset.object_id(), ObjectId::from_uuid(id(36)));
    assert_eq!(asset.question_version(), None);
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
