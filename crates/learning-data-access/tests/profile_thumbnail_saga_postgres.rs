#![cfg(feature = "postgres")]

//! Real PostgreSQL plus MinIO acceptance for the self-only Profile Thumbnail saga.

use learning_data_access::postgres::{PostgresProfileThumbnailStore, lazy_pool};
use learning_data_access::{ProfileThumbnailStore, SessionTokenHash};
use objects::minio::{EndpointConfig, client};
use objects::s3::{BucketNames, S3ObjectStore};
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum, profile_thumbnail_object_id};
use question_model::{ProfileThumbnailReference, Timestamp};
use sqlx::Connection;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xfa01;
const STUDENT: u128 = 0xfa02;
const FOREIGN: u128 = 0xfa03;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token(value: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[value; 32])
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_millis(1_800_000_000_000)
}

fn reference(value: u128) -> ProfileThumbnailReference {
    ProfileThumbnailReference::from_uuid(id(value))
}

fn address(reference: ProfileThumbnailReference) -> ObjectAddress {
    ObjectAddress::ProfileThumbnail {
        thumbnail: reference,
    }
}

async fn put(store: &S3ObjectStore, reference: ProfileThumbnailReference, bytes: &[u8]) {
    store
        .put(PutObject {
            address: address(reference),
            bytes: bytes.to_vec(),
            media_type: "image/webp".to_owned(),
            created_at: timestamp(),
        })
        .await
        .expect("real MinIO immutable put");
}

async fn set_inspection_role(connection: &mut PgConnection, role: &'static str) {
    let statement = match role {
        "ple_private_owner" => "SET ROLE ple_private_owner",
        "ple_audit_owner" => "SET ROLE ple_audit_owner",
        _ => panic!("unsupported acceptance inspection role"),
    };
    sqlx::query(statement)
        .execute(connection)
        .await
        .expect("inspection role");
}

async fn seed(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    for (account, role, session, token_hash) in [
        (INSTRUCTOR, "instructor", 0xfb01, token(41)),
        (STUDENT, "student", 0xfb02, token(42)),
        (FOREIGN, "instructor", 0xfb03, token(43)),
    ] {
        sqlx::query(
            "INSERT INTO ple_private.account (account_id, product_role, created_at) \
             VALUES ($1, $2, clock_timestamp())",
        )
        .bind(id(account))
        .bind(role)
        .execute(&mut *transaction)
        .await
        .expect("account");
        sqlx::query(
            "INSERT INTO ple_private.authenticated_session \
             (session_id, account_id, product_role, token_hash, created_at, expires_at) \
             VALUES ($1, $2, $3, decode($4, 'hex'), clock_timestamp(), \
             clock_timestamp() + interval '1 hour')",
        )
        .bind(id(session))
        .bind(id(account))
        .bind(role)
        .bind(token_hash.to_string())
        .execute(&mut *transaction)
        .await
        .expect("session");
    }
    transaction.commit().await.expect("fixture commit");
}

async fn prepare_put_finalize(
    store: &PostgresProfileThumbnailStore,
    object_store: &S3ObjectStore,
    thumbnail: ProfileThumbnailReference,
    bytes: &[u8],
) -> learning_data_access::FinalizedProfileThumbnail {
    let prepared = store
        .prepare_profile_thumbnail(
            token(41),
            thumbnail,
            profile_thumbnail_object_id(thumbnail),
            Sha256Checksum::compute(bytes),
            bytes.len() as u64,
        )
        .await
        .expect("Instructor prepares exact thumbnail object");
    assert_eq!(prepared.reference, thumbnail);
    assert_eq!(prepared.object_id, profile_thumbnail_object_id(thumbnail));
    put(object_store, thumbnail, bytes).await;
    store
        .complete_profile_thumbnail_put(token(41), prepared.work_id)
        .await
        .expect("put completion is durable after real object write");
    store
        .finalize_profile_thumbnail(token(41), prepared.work_id)
        .await
        .expect("complete put finalizes current thumbnail")
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 and MinIO profile-thumbnail oracle"]
async fn profile_thumbnail_saga_is_self_only_durable_and_cross_store() {
    let runtime = acceptance_runtime::CourseAppearanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    seed(&admin).await;
    let mut inspection = PgConnection::connect(migration_url)
        .await
        .expect("inspection connection");
    set_inspection_role(&mut inspection, "ple_private_owner").await;
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let store = PostgresProfileThumbnailStore::new(application);
    let minio = runtime.minio();
    let object_store = S3ObjectStore::new(
        client(&EndpointConfig {
            endpoint_url: minio.endpoint_url().to_owned(),
            region: minio.region().to_owned(),
            access_key_id: minio.access_key_id().to_owned(),
            secret_access_key: minio.secret_access_key().to_owned(),
        }),
        BucketNames::default(),
    );
    let first = reference(0xfc01);
    let first_bytes = b"first normalized webp";

    assert!(
        store
            .prepare_profile_thumbnail(
                token(42),
                first,
                profile_thumbnail_object_id(first),
                Sha256Checksum::compute(first_bytes),
                first_bytes.len() as u64,
            )
            .await
            .is_err(),
        "Student cannot prepare a profile thumbnail"
    );
    let foreign = reference(0xfc02);
    let foreign_prepared = store
        .prepare_profile_thumbnail(
            token(43),
            foreign,
            profile_thumbnail_object_id(foreign),
            Sha256Checksum::compute(first_bytes),
            first_bytes.len() as u64,
        )
        .await
        .expect("foreign Instructor prepares their own thumbnail");
    assert!(
        store
            .complete_profile_thumbnail_put(token(41), foreign_prepared.work_id)
            .await
            .is_err(),
        "another Instructor cannot complete foreign work"
    );

    let first_finalized = prepare_put_finalize(&store, &object_store, first, first_bytes).await;
    assert_eq!(first_finalized.reference, first);
    assert!(
        first_finalized.retired.is_none(),
        "first thumbnail retires nothing"
    );
    assert_eq!(
        store
            .read_current_profile_thumbnail(token(41))
            .await
            .expect("self current read"),
        Some(first)
    );
    assert_eq!(
        store
            .resolve_current_profile_thumbnail(token(41), first)
            .await
            .expect("self resolves current reference"),
        profile_thumbnail_object_id(first)
    );
    assert!(
        store
            .resolve_current_profile_thumbnail(token(42), first)
            .await
            .is_err(),
        "Student cannot resolve an Instructor thumbnail"
    );
    assert!(
        store
            .resolve_current_profile_thumbnail(token(43), first)
            .await
            .is_err(),
        "foreign Instructor cannot resolve another current thumbnail"
    );
    assert_eq!(
        object_store
            .get(&address(first))
            .await
            .expect("real MinIO current object")
            .bytes,
        first_bytes,
        "the finalized reference owns the object written to MinIO"
    );

    let first_work: Uuid = sqlx::query_scalar(
        "SELECT profile_thumbnail_work_id FROM ple_private.profile_thumbnail_work \
         WHERE profile_thumbnail_id=$1 AND operation_kind='put'",
    )
    .bind(first.as_uuid())
    .fetch_one(&mut inspection)
    .await
    .expect("first put work");
    assert!(
        store
            .finalize_profile_thumbnail(token(41), first_work)
            .await
            .is_err(),
        "finalization is one-time and replay keeps the current thumbnail available"
    );
    assert_eq!(
        store
            .resolve_current_profile_thumbnail(token(41), first)
            .await
            .expect("replay cannot retire the current thumbnail"),
        profile_thumbnail_object_id(first)
    );
    assert!(
        store
            .prepare_profile_thumbnail_deletion(token(41), first_work)
            .await
            .is_err(),
        "a current finalized thumbnail is never eligible for compensation deletion"
    );

    let compensated = reference(0xfc04);
    let compensated_bytes = b"compensation before finalization";
    let compensated_put = store
        .prepare_profile_thumbnail(
            token(41),
            compensated,
            profile_thumbnail_object_id(compensated),
            Sha256Checksum::compute(compensated_bytes),
            compensated_bytes.len() as u64,
        )
        .await
        .expect("prepare a replacement eligible for compensation");
    put(&object_store, compensated, compensated_bytes).await;
    store
        .complete_profile_thumbnail_put(token(41), compensated_put.work_id)
        .await
        .expect("completed put can be compensated before finalization");
    let compensation = store
        .prepare_profile_thumbnail_deletion(token(41), compensated_put.work_id)
        .await
        .expect("exact compensation is durable before object deletion");
    assert!(
        store
            .finalize_profile_thumbnail(token(41), compensated_put.work_id)
            .await
            .is_err(),
        "a compensation work item prevents publication of that deleted object"
    );
    assert_eq!(
        store
            .resolve_current_profile_thumbnail(token(41), first)
            .await
            .expect("failed candidate finalization preserves current thumbnail"),
        profile_thumbnail_object_id(first)
    );
    object_store
        .delete(&address(compensated))
        .await
        .expect("real MinIO compensation deletion");
    store
        .complete_profile_thumbnail_deletion(token(41), compensation)
        .await
        .expect("confirmed compensation deletion completes its exact work");

    let second = reference(0xfc03);
    let second_bytes = b"second normalized webp";
    let replacement = prepare_put_finalize(&store, &object_store, second, second_bytes).await;
    let retired = replacement
        .retired
        .expect("replacement returns exact retired delete work");
    assert_eq!(retired.reference, first);
    assert!(
        store
            .resolve_current_profile_thumbnail(token(41), first)
            .await
            .is_err(),
        "retired reference is concealed immediately"
    );
    assert_eq!(
        store
            .resolve_current_profile_thumbnail(token(41), second)
            .await
            .expect("replacement is current"),
        profile_thumbnail_object_id(second)
    );

    object_store
        .delete(&address(first))
        .await
        .expect("exact retired object deletion in real MinIO");
    store
        .require_profile_thumbnail_deletion_repair(token(41), retired)
        .await
        .expect("uncertain delete enters repair state");
    store
        .record_profile_thumbnail_cleanup_check(token(41), retired, false, None)
        .await
        .expect("actual missing MinIO object records the cleanup observation");
    let state: String = sqlx::query_scalar(
        "SELECT state FROM ple_private.profile_thumbnail_work WHERE profile_thumbnail_work_id=$1",
    )
    .bind(retired.work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("retired delete work state");
    assert_eq!(state, "completed", "confirmed absence completes repair");
    let manifest_id: Uuid = sqlx::query_scalar(
        "SELECT manifest.object_cleanup_manifest_id \
         FROM ple_private.object_cleanup_manifest AS manifest \
         JOIN ple_private.object_storage_check AS storage_check \
           ON storage_check.object_storage_check_id = manifest.object_storage_check_id \
         JOIN ple_private.profile_thumbnail_work AS work \
           ON work.delivery_id = storage_check.delivery_id \
         WHERE work.profile_thumbnail_work_id = $1",
    )
    .bind(retired.work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("cleanup manifest");
    set_inspection_role(&mut inspection, "ple_audit_owner").await;
    let receipt: String = sqlx::query_scalar(
        "SELECT disposition FROM ple_audit.object_cleanup_receipt \
         WHERE object_cleanup_manifest_id = $1",
    )
    .bind(manifest_id)
    .fetch_one(&mut inspection)
    .await
    .expect("cleanup receipt");
    assert_eq!(receipt, "already_absent");
}
