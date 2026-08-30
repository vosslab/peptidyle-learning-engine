//! Acceptance contract for immutable published QTI import archives.
//!
//! WP-QTI-9 owns the server copy. Its required sequence is deliberately
//! specified here without adding that behavior to this crate:
//!
//! 1. `get` the typed workspace archive and verify its record and bytes;
//! 2. derive the typed published candidate and `put` it;
//! 3. on `AlreadyExists`, `get` the candidate and accept only an exact
//!    key/category/media-type/size/digest match.

use objects::memory::MemoryObjectStore;
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest, published_import_archive_object_id,
};
use question_model::{
    ActivityTimestamp, ObjectId, ProblemId, VersionId, WorkspaceId, WorkspaceImportId,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

/// The complete `AlreadyExists` acceptance comparison from the locked WP-QTI protocol.
///
/// `bucket` is included as well because it is derived from the typed key and must not drift from
/// the content-bucket provenance contract. License, provenance, and creation time are deliberately
/// not replay-match fields; the frozen protocol names exactly these archive properties.
fn is_exact_published_archive_replay(record: &ObjectRecord, candidate: &PutObject) -> bool {
    record.key == candidate.key
        && record.bucket == candidate.key.bucket()
        && record.category == candidate.key.category()
        && record.media_type == candidate.media_type
        && record.size_bytes == candidate.bytes.len() as u64
        && record.sha256 == Sha256Digest::compute(&candidate.bytes)
}

#[tokio::test]
async fn published_import_archive_candidate_is_deterministic_non_signable_and_exact_on_replay() {
    let store = MemoryObjectStore::default();
    let workspace = WorkspaceId::from_uuid(id(2));
    let import = WorkspaceImportId::from_uuid(id(3));
    let problem = ProblemId::from_uuid(id(4));
    let version = VersionId::from_uuid(id(5));
    let archive_bytes = b"verified QTI archive bytes".to_vec();
    let workspace_key = ObjectKey::WorkspaceSource {
        workspace,
        import,
        object: ObjectId::from_uuid(id(6)),
    };

    let workspace_record = store
        .put(PutObject {
            key: workspace_key.clone(),
            bytes: archive_bytes.clone(),
            media_type: "application/zip".to_string(),
            license: "private provenance".to_string(),
            provenance: "QTI workspace import".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        })
        .await
        .expect("workspace archive fixture should be stored");

    // Step 1: the later server path must work from a verified typed workspace archive.
    let verified_workspace_archive = store
        .get(&workspace_key)
        .await
        .expect("workspace archive fixture should be readable");
    assert_eq!(verified_workspace_archive.record, workspace_record);
    assert_eq!(verified_workspace_archive.bytes, archive_bytes);

    let archive_object = published_import_archive_object_id(
        problem,
        version,
        import,
        verified_workspace_archive.record.sha256,
    );
    let candidate = PutObject {
        key: ObjectKey::PublishedImportArchive {
            problem,
            version,
            import,
            object: archive_object,
        },
        bytes: verified_workspace_archive.bytes,
        media_type: verified_workspace_archive.record.media_type,
        license: "private published provenance".to_string(),
        provenance: "published from verified QTI workspace archive".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(2_000),
    };

    // Step 2: an exact typed candidate is immutable and carries only source metadata.
    let first_record = store
        .put(candidate.clone())
        .await
        .expect("first immutable archive candidate should be stored");
    assert_eq!(
        first_record.id,
        published_import_archive_object_id(problem, version, import, first_record.sha256),
        "the candidate object identity must be derived from its complete typed identity"
    );
    assert_eq!(first_record.bucket, Bucket::PrivateContent);
    assert_eq!(first_record.category, ObjectCategory::Source);
    assert!(!first_record.key.may_issue_signed_url());
    assert_eq!(
        store
            .signed_url(&candidate.key, ActivityTimestamp::from_unix_millis(3_000))
            .await,
        Err(ObjectStoreError::NotSignable),
        "published import provenance must remain server-only"
    );

    // Step 3: exact replays receive `AlreadyExists`, then must re-read and compare the record.
    assert_eq!(
        store.put(candidate.clone()).await,
        Err(ObjectStoreError::AlreadyExists)
    );
    let replay = store
        .get(&candidate.key)
        .await
        .expect("already-existing candidate should be re-readable for exact replay verification");
    assert!(is_exact_published_archive_replay(
        &replay.record,
        &candidate
    ));

    let mismatched_key = ObjectRecord {
        key: ObjectKey::Temporary {
            object: ObjectId::from_uuid(id(7)),
        },
        ..replay.record.clone()
    };
    let mismatched_category = ObjectRecord {
        category: ObjectCategory::Asset,
        ..replay.record.clone()
    };
    let mismatched_media_type = ObjectRecord {
        media_type: "application/octet-stream".to_string(),
        ..replay.record.clone()
    };
    let mismatched_size = ObjectRecord {
        size_bytes: replay.record.size_bytes + 1,
        ..replay.record.clone()
    };
    let mismatched_digest = ObjectRecord {
        sha256: Sha256Digest::compute(b"different archive bytes"),
        ..replay.record
    };
    for mismatch in [
        mismatched_key,
        mismatched_category,
        mismatched_media_type,
        mismatched_size,
        mismatched_digest,
    ] {
        assert!(
            !is_exact_published_archive_replay(&mismatch, &candidate),
            "a replay mismatch must refuse rather than treating a divergent immutable candidate as success"
        );
    }
}
