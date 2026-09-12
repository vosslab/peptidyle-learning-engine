#![cfg(feature = "postgres")]

//! Real PostgreSQL plus MinIO acceptance for the Course Banner saga.

use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, GenericImageView, ImageEncoder, RgbaImage};
use learning_data_access::postgres::{PostgresCourseBannerStore, lazy_pool};
use learning_data_access::{
    CourseBannerObjectMetadata, CourseBannerStore, PrepareCourseBannerPromotion, SessionTokenHash,
    StageCourseBannerUpload,
};
use objects::minio::{EndpointConfig, client};
use objects::s3::{BucketNames, S3ObjectStore};
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum};
use question_model::{
    CourseBannerAlternativeText, CourseBannerInformativeText, CourseBannerReference,
    CourseBannerUpdate, CourseBannerUploadReference, CourseId, ObjectId, Timestamp,
};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Row};
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xca01;
const STUDENT: u128 = 0xca02;
const FOREIGN: u128 = 0xca03;
const COURSE: u128 = 0xcb01;
const FOREIGN_COURSE: u128 = 0xcb02;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
fn token(value: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[value; 32])
}
fn timestamp() -> Timestamp {
    Timestamp::from_unix_millis(1_800_000_000_000)
}
fn metadata(object_id: ObjectId, bytes: &[u8], media_type: &str) -> CourseBannerObjectMetadata {
    CourseBannerObjectMetadata {
        object_id,
        sha256: Sha256Checksum::compute(bytes),
        byte_length: bytes.len() as u64,
        media_type: media_type.to_owned(),
    }
}

fn webp(width: u32, height: u32, value: u8) -> Vec<u8> {
    let image = RgbaImage::from_pixel(width, height, image::Rgba([value, value, value, 255]));
    let mut bytes = Vec::new();
    WebPEncoder::new_lossless(&mut bytes)
        .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
        .expect("WebP acceptance fixture");
    bytes
}

async fn put(store: &S3ObjectStore, address: ObjectAddress, bytes: Vec<u8>, media_type: &str) {
    store
        .put(PutObject {
            address,
            bytes,
            media_type: media_type.to_owned(),
            created_at: timestamp(),
        })
        .await
        .expect("real MinIO immutable put");
}

async fn set_inspection_role(connection: &mut PgConnection, role: &'static str) {
    let statement = match role {
        "ple_data_owner" => "SET ROLE ple_data_owner",
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
    // The oracle is deliberately deterministic: all capability decisions below
    // come through a normal session-bound PostgresCourseBannerStore.
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    for (account, role) in [
        (INSTRUCTOR, "instructor"),
        (STUDENT, "student"),
        (FOREIGN, "instructor"),
    ] {
        sqlx::query("INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES ($1,$2,clock_timestamp())")
            .bind(id(account)).bind(role).execute(&mut *transaction).await.expect("account");
    }
    sqlx::query("INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at) VALUES ($1,$2,'instructor',decode($3,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour'),($4,$5,'student',decode($6,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour'),($7,$8,'instructor',decode($9,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour')")
        .bind(id(0xcc01)).bind(id(INSTRUCTOR)).bind(token(1).to_string())
        .bind(id(0xcc02)).bind(id(STUDENT)).bind(token(2).to_string())
        .bind(id(0xcc03)).bind(id(FOREIGN)).bind(token(3).to_string())
        .execute(&mut *transaction).await.expect("sessions");
    sqlx::query("INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at) VALUES ($1,$2,'instructor',decode($3,'hex'),clock_timestamp()-interval '2 hours',clock_timestamp()-interval '1 hour')")
        .bind(id(0xcc04)).bind(id(INSTRUCTOR)).bind(token(4).to_string())
        .execute(&mut *transaction).await.expect("expired session");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *transaction)
        .await
        .expect("data fixture role");
    sqlx::query("INSERT INTO ple_data.blueprint_course (blueprint_id, reference_number, owner_account_id, created_at) OVERRIDING SYSTEM VALUE VALUES ($1,1,$2,clock_timestamp())")
        .bind(id(0xcd01)).bind(id(INSTRUCTOR)).execute(&mut *transaction).await.expect("blueprint");
    sqlx::query("INSERT INTO ple_data.blueprint_course_revision (blueprint_course_reference_number, blueprint_revision_number, title, content, content_checksum, published_at) VALUES (1,1,'Banner oracle','{}',decode(repeat('00',32),'hex'),clock_timestamp())")
        .execute(&mut *transaction).await.expect("revision");
    for (course, assigned) in [(COURSE, INSTRUCTOR), (FOREIGN_COURSE, FOREIGN)] {
        sqlx::query("INSERT INTO ple_data.course_instance (course_id, blueprint_course_reference_number, blueprint_revision_number, assigned_instructor_account_id, course_short_name, course_long_name, term_starts_on, term_ends_on, created_at) VALUES ($1,1,1,$2,'Banner','Banner course',current_date,current_date + 1,clock_timestamp())")
            .bind(id(course)).bind(id(assigned)).execute(&mut *transaction).await.expect("course");
    }
    sqlx::query("INSERT INTO ple_data.student_record (student_record_id, course_id, student_account_id, created_at) VALUES ($1,$2,$3,clock_timestamp())")
        .bind(id(0xce10)).bind(id(COURSE)).bind(id(STUDENT)).execute(&mut *transaction).await.expect("Student Record");
    for (membership, course, account, role) in [
        (0xce01, COURSE, INSTRUCTOR, "instructor"),
        (0xce02, COURSE, STUDENT, "student"),
        (0xce03, FOREIGN_COURSE, FOREIGN, "instructor"),
    ] {
        sqlx::query("INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, student_record_id, joined_at) VALUES ($1,$2,$3,$4,CASE WHEN $4='student' THEN $5 ELSE NULL END,clock_timestamp())")
            .bind(id(membership)).bind(id(course)).bind(id(account)).bind(role).bind(id(0xce10)).execute(&mut *transaction).await.expect("membership");
    }
    transaction.commit().await.expect("fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 and MinIO course-appearance oracle"]
async fn course_banner_saga_is_durable_authorized_and_cross_store() {
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
    let store = PostgresCourseBannerStore::new(application);
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
    let course = CourseId::from_uuid(id(COURSE));
    let foreign_course = CourseId::from_uuid(id(FOREIGN_COURSE));
    let upload = CourseBannerUploadReference::from_uuid(id(0xcf01));
    let upload_bytes = b"source-banner";
    let upload_object = ObjectId::from_uuid(id(0xd001));
    let staged = store
        .stage_course_banner_upload(
            token(1),
            StageCourseBannerUpload {
                course,
                upload,
                metadata: metadata(upload_object, upload_bytes, "image/png"),
                width: 1200,
                height: 200,
                expires_at_unix_millis: 1_900_000_000_000,
            },
        )
        .await
        .expect("Instructor stages upload");
    assert!(
        store
            .stage_course_banner_upload(
                token(4),
                StageCourseBannerUpload {
                    course,
                    upload: CourseBannerUploadReference::from_uuid(id(0xcf04)),
                    metadata: metadata(ObjectId::from_uuid(id(0xd004)), upload_bytes, "image/png"),
                    width: 1,
                    height: 1,
                    expires_at_unix_millis: 1_900_000_000_000
                }
            )
            .await
            .is_err(),
        "expired session is refused"
    );
    let pending: String = sqlx::query(
        "SELECT state FROM ple_private.course_banner_work WHERE course_banner_work_id=$1",
    )
    .bind(staged.put_work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("pre-put work")
    .try_get(0)
    .expect("state");
    assert_eq!(pending, "pending", "work is durable before real object put");
    assert!(
        store
            .stage_course_banner_upload(
                token(2),
                StageCourseBannerUpload {
                    course,
                    upload: CourseBannerUploadReference::from_uuid(id(0xcf02)),
                    metadata: metadata(ObjectId::from_uuid(id(0xd002)), upload_bytes, "image/png"),
                    width: 1,
                    height: 1,
                    expires_at_unix_millis: 1_900_000_000_000
                }
            )
            .await
            .is_err(),
        "Student cannot stage"
    );
    assert!(
        store
            .stage_course_banner_upload(
                token(3),
                StageCourseBannerUpload {
                    course,
                    upload: CourseBannerUploadReference::from_uuid(id(0xcf03)),
                    metadata: metadata(ObjectId::from_uuid(id(0xd003)), upload_bytes, "image/png"),
                    width: 1,
                    height: 1,
                    expires_at_unix_millis: 1_900_000_000_000
                }
            )
            .await
            .is_err(),
        "foreign Instructor concealed"
    );
    put(
        &object_store,
        staged.address.clone(),
        upload_bytes.to_vec(),
        "image/png",
    )
    .await;
    store
        .finalize_course_banner_upload_stage(token(1), course, upload)
        .await
        .expect("upload completion");
    let claimed = store
        .read_staged_course_banner_upload(token(1), course, upload)
        .await
        .expect("exact account/course claim");
    assert!(
        store
            .read_staged_course_banner_upload(token(2), course, upload)
            .await
            .is_err(),
        "Student cannot claim"
    );
    assert!(
        store
            .read_staged_course_banner_upload(token(1), foreign_course, upload)
            .await
            .is_err(),
        "foreign course cannot claim"
    );

    let banner = CourseBannerReference::from_uuid(id(0xcf10));
    let source = b"source-banner".to_vec();
    let hero = webp(1200, 200, 7);
    let card = webp(1000, 400, 8);
    let source_id = ObjectId::from_uuid(id(0xd010));
    let hero_id = ObjectId::from_uuid(id(0xd011));
    let card_id = ObjectId::from_uuid(id(0xd012));
    let prepared = store
        .prepare_course_banner_promotion(
            token(1),
            PrepareCourseBannerPromotion {
                course,
                upload,
                banner,
                update: CourseBannerUpdate {
                    upload,
                    alternative_text: CourseBannerAlternativeText::Informative {
                        text: CourseBannerInformativeText::try_from(
                            "Molecular landscape".to_owned(),
                        )
                        .expect("text"),
                    },
                },
                source: metadata(source_id, &source, "image/png"),
                hero: metadata(hero_id, &hero, "image/webp"),
                card: metadata(card_id, &card, "image/webp"),
            },
        )
        .await
        .expect("prepare promotion");
    assert!(
        store
            .read_current_course_banner(token(1), course)
            .await
            .expect("read")
            .is_none(),
        "pointer stays absent before all puts"
    );
    put(&object_store, prepared.source.clone(), source, "image/png").await;
    store
        .complete_prepared_course_banner_object(token(1), course, banner, source_id)
        .await
        .expect("source completion");
    assert!(
        store
            .finalize_course_banner_promotion(token(1), course, upload, banner)
            .await
            .is_err(),
        "incomplete promotion never advances pointer"
    );
    put(&object_store, prepared.hero.clone(), hero, "image/webp").await;
    put(&object_store, prepared.card.clone(), card, "image/webp").await;
    store
        .complete_prepared_course_banner_object(token(1), course, banner, hero_id)
        .await
        .expect("hero completion");
    store
        .complete_prepared_course_banner_object(token(1), course, banner, card_id)
        .await
        .expect("card completion");
    let finalized = store
        .finalize_course_banner_promotion(token(1), course, upload, banner)
        .await
        .expect("complete promotion");
    assert_eq!(finalized.banner.reference, banner);
    assert_eq!(
        store
            .read_current_course_banner(token(2), course)
            .await
            .expect("Student aggregate")
            .expect("banner")
            .reference,
        banner
    );
    assert!(
        store
            .read_current_course_banner(token(3), course)
            .await
            .expect("foreign concealed read")
            .is_none(),
        "foreign Account cannot distinguish a current banner from no banner"
    );
    for (address, dimensions) in [(prepared.hero, (1200, 200)), (prepared.card, (1000, 400))] {
        let stored = object_store
            .get(&address)
            .await
            .expect("stored WebP rendition");
        let decoded = image::load_from_memory(&stored.bytes).expect("stored WebP decodes");
        assert_eq!(
            decoded.dimensions(),
            dimensions,
            "fixed rendition dimensions"
        );
        assert_eq!(stored.record.media_type, "image/webp");
    }
    assert!(
        store
            .read_staged_course_banner_upload(token(1), course, upload)
            .await
            .is_err(),
        "a promoted upload is single-use"
    );

    // A replacement has its own upload and work.  Its incomplete preparation
    // must not disturb the first, already-visible current banner.
    let replacement_upload = CourseBannerUploadReference::from_uuid(id(0xcf20));
    let replacement_upload_id = ObjectId::from_uuid(id(0xd020));
    let replacement_staged = store
        .stage_course_banner_upload(
            token(1),
            StageCourseBannerUpload {
                course,
                upload: replacement_upload,
                metadata: metadata(replacement_upload_id, upload_bytes, "image/png"),
                width: 1200,
                height: 200,
                expires_at_unix_millis: 1_900_000_000_000,
            },
        )
        .await
        .expect("stage replacement upload");
    put(
        &object_store,
        replacement_staged.address,
        upload_bytes.to_vec(),
        "image/png",
    )
    .await;
    store
        .finalize_course_banner_upload_stage(token(1), course, replacement_upload)
        .await
        .expect("complete replacement upload");
    let replacement_banner = CourseBannerReference::from_uuid(id(0xcf21));
    let replacement_source_id = ObjectId::from_uuid(id(0xd021));
    let replacement_hero_id = ObjectId::from_uuid(id(0xd022));
    let replacement_card_id = ObjectId::from_uuid(id(0xd023));
    let replacement_source = upload_bytes.to_vec();
    let replacement_hero = webp(1200, 200, 21);
    let replacement_card = webp(1000, 400, 22);
    let replacement_prepared = store
        .prepare_course_banner_promotion(
            token(1),
            PrepareCourseBannerPromotion {
                course,
                upload: replacement_upload,
                banner: replacement_banner,
                update: CourseBannerUpdate {
                    upload: replacement_upload,
                    alternative_text: CourseBannerAlternativeText::Decorative,
                },
                source: metadata(replacement_source_id, &replacement_source, "image/png"),
                hero: metadata(replacement_hero_id, &replacement_hero, "image/webp"),
                card: metadata(replacement_card_id, &replacement_card, "image/webp"),
            },
        )
        .await
        .expect("prepare replacement");
    put(
        &object_store,
        replacement_prepared.source.clone(),
        replacement_source,
        "image/png",
    )
    .await;
    store
        .complete_prepared_course_banner_object(
            token(1),
            course,
            replacement_banner,
            replacement_source_id,
        )
        .await
        .expect("complete replacement source");
    assert!(
        store
            .finalize_course_banner_promotion(
                token(1),
                course,
                replacement_upload,
                replacement_banner
            )
            .await
            .is_err(),
        "incomplete replacement preserves old pointer"
    );
    assert_eq!(
        store
            .read_current_course_banner(token(2), course)
            .await
            .expect("Student old pointer")
            .expect("old banner")
            .reference,
        banner
    );
    put(
        &object_store,
        replacement_prepared.hero.clone(),
        replacement_hero,
        "image/webp",
    )
    .await;
    put(
        &object_store,
        replacement_prepared.card.clone(),
        replacement_card,
        "image/webp",
    )
    .await;
    store
        .complete_prepared_course_banner_object(
            token(1),
            course,
            replacement_banner,
            replacement_hero_id,
        )
        .await
        .expect("complete replacement hero");
    store
        .complete_prepared_course_banner_object(
            token(1),
            course,
            replacement_banner,
            replacement_card_id,
        )
        .await
        .expect("complete replacement card");
    let replacement = store
        .finalize_course_banner_promotion(token(1), course, replacement_upload, replacement_banner)
        .await
        .expect("complete replacement");
    assert_eq!(
        store
            .read_current_course_banner(token(2), course)
            .await
            .expect("Student replacement")
            .expect("replacement banner")
            .reference,
        replacement_banner
    );
    let retired = replacement
        .retired
        .expect("replacement returns retired first banner");
    let verified_delete = store
        .prepare_course_banner_object_deletion(token(1), retired.hero_put_work_id)
        .await
        .expect("durable pre-delete work for retired hero");
    store
        .require_course_banner_deletion_repair(token(1), verified_delete)
        .await
        .expect("uncertain retired hero delete");
    store
        .record_course_banner_cleanup_check(
            token(1),
            verified_delete,
            true,
            Some(Sha256Checksum::compute(&webp(1200, 200, 7))),
        )
        .await
        .expect("verified cleanup observation");
    set_inspection_role(&mut inspection, "ple_audit_owner").await;
    let verified_disposition: String =
        sqlx::query_scalar("SELECT disposition FROM ple_audit.object_cleanup_receipt")
            .fetch_one(&mut inspection)
            .await
            .expect("verified-present cleanup receipt");
    assert_eq!(verified_disposition, "retained");
    set_inspection_role(&mut inspection, "ple_private_owner").await;
    let verified_work_state: String = sqlx::query_scalar(
        "SELECT state FROM ple_private.course_banner_work WHERE course_banner_work_id=$1",
    )
    .bind(verified_delete.work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("verified work state");
    assert_eq!(verified_work_state, "completed");

    let removal = store
        .prepare_course_banner_removal(token(1), course)
        .await
        .expect("remove preparation");
    set_inspection_role(&mut inspection, "ple_data_owner").await;
    let theme_before: String =
        sqlx::query_scalar("SELECT course_theme FROM ple_data.course_instance WHERE course_id=$1")
            .bind(course.as_uuid())
            .fetch_one(&mut inspection)
            .await
            .expect("theme before remove");
    assert!(
        store
            .read_current_course_banner(token(1), course)
            .await
            .expect("read")
            .is_none(),
        "remove clears pointer without theme mutation"
    );
    let theme_after: String =
        sqlx::query_scalar("SELECT course_theme FROM ple_data.course_instance WHERE course_id=$1")
            .bind(course.as_uuid())
            .fetch_one(&mut inspection)
            .await
            .expect("theme after remove");
    assert_eq!(
        theme_after, theme_before,
        "removing a banner never changes the independent Theme"
    );
    let delete = store
        .prepare_course_banner_object_deletion(token(1), removal.hero_put_work_id)
        .await
        .expect("pre-delete work");
    set_inspection_role(&mut inspection, "ple_private_owner").await;
    let delete_state: String = sqlx::query_scalar(
        "SELECT state FROM ple_private.course_banner_work WHERE course_banner_work_id=$1",
    )
    .bind(delete.work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("durable delete work");
    assert_eq!(
        delete_state, "pending",
        "actual object delete follows durable pre-delete work"
    );
    object_store
        .delete(&removal.hero)
        .await
        .expect("confirmed real hero delete");
    store
        .complete_course_banner_object_deletion(token(1), delete)
        .await
        .expect("record confirmed delete");
    let missing_delete = store
        .prepare_course_banner_object_deletion(token(1), removal.card_put_work_id)
        .await
        .expect("durable pre-delete card work");
    object_store
        .delete(&removal.card)
        .await
        .expect("real card delete before missing check");
    store
        .require_course_banner_deletion_repair(token(1), missing_delete)
        .await
        .expect("uncertain delete repair");
    store
        .record_course_banner_cleanup_check(token(1), missing_delete, false, None)
        .await
        .expect("verified repair observation");
    set_inspection_role(&mut inspection, "ple_audit_owner").await;
    let absent_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_audit.object_cleanup_receipt WHERE disposition='already_absent'",
    )
    .fetch_one(&mut inspection)
    .await
    .expect("missing receipt");
    assert_eq!(
        absent_receipts, 1,
        "confirmed-missing cleanup records exactly one already-absent receipt"
    );
    set_inspection_role(&mut inspection, "ple_private_owner").await;
    let missing_work_state: String = sqlx::query_scalar(
        "SELECT state FROM ple_private.course_banner_work WHERE course_banner_work_id=$1",
    )
    .bind(missing_delete.work_id)
    .fetch_one(&mut inspection)
    .await
    .expect("missing work state");
    assert_eq!(missing_work_state, "completed");
    assert!(
        claimed.put_work_id == staged.put_work_id,
        "staged work identity is stable and single-use promotion consumed its upload"
    );
}
