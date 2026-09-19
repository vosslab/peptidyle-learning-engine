#![cfg(feature = "postgres")]

//! Real PostgreSQL plus MinIO acceptance for the Course Banner saga.

use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use learning_data_access::postgres::{PostgresCourseBannerStore, lazy_pool};
use learning_data_access::{
    CourseBannerObjectMetadata, CourseBannerStore, PrepareCourseBannerPromotion, SessionTokenHash,
    StageCourseBannerUpload,
};
use objects::minio::{EndpointConfig, client};
use objects::s3::{BucketNames, S3ObjectStore};
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum};
use question_model::{
    CourseBannerAlternativeText, CourseBannerId, CourseBannerInformativeText,
    CourseBannerRendition, CourseBannerUpdate, CourseBannerUploadId, CourseInstanceId, ObjectId,
    Timestamp,
};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Row};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
fn token(value: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[value; 32])
}
fn timestamp() -> Timestamp {
    Timestamp::from_unix_millis(1_800_000_000_000)
}
fn metadata(
    object_id: ObjectId,
    bytes: &[u8],
    media_type: &str,
    width: u32,
    height: u32,
) -> CourseBannerObjectMetadata {
    CourseBannerObjectMetadata {
        object_id,
        sha256: Sha256Checksum::compute(bytes),
        byte_length: bytes.len() as u64,
        media_type: media_type.to_owned(),
        width,
        height,
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

struct BannerFixture {
    course_id: String,
    foreign_course_id: String,
}

async fn mint_account(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    role: &str,
) -> String {
    sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ('U00000009', $1, clock_timestamp()) RETURNING account_id",
    )
    .bind(role)
    .fetch_one(&mut **transaction)
    .await
    .expect("account")
}

async fn seed(admin: &sqlx::postgres::PgPool) -> BannerFixture {
    // The oracle is deliberately deterministic: all capability decisions below
    // come through a normal session-bound PostgresCourseBannerStore.
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("classification fixture owner");
    sqlx::query(
        "INSERT INTO ple_data.content_discipline (content_discipline_id, name) \
         VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') \
         ON CONFLICT (content_discipline_id) DO NOTHING",
    )
    .execute(&mut *transaction)
    .await
    .expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    let instructor_id = mint_account(&mut transaction, "instructor").await;
    let student_id = mint_account(&mut transaction, "student").await;
    let foreign_id = mint_account(&mut transaction, "instructor").await;
    sqlx::query("INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at) VALUES ($1,$2,'instructor',decode($3,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour'),($4,$5,'student',decode($6,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour'),($7,$8,'instructor',decode($9,'hex'),clock_timestamp(),clock_timestamp()+interval '1 hour')")
        .bind(id(0xcc01)).bind(&instructor_id).bind(token(1).to_string())
        .bind(id(0xcc02)).bind(&student_id).bind(token(2).to_string())
        .bind(id(0xcc03)).bind(&foreign_id).bind(token(3).to_string())
        .execute(&mut *transaction).await.expect("sessions");
    sqlx::query("INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at) VALUES ($1,$2,'instructor',decode($3,'hex'),clock_timestamp()-interval '2 hours',clock_timestamp()-interval '1 hour')")
        .bind(id(0xcc04)).bind(&instructor_id).bind(token(4).to_string())
        .execute(&mut *transaction).await.expect("expired session");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *transaction)
        .await
        .expect("data fixture role");
    let blueprint_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.blueprint_course \
         (blueprint_course_id, owner_account_id, short_name, long_name, blueprint_edit_number, \
          created_at, content_discipline_id, tags) \
         VALUES ('BP0000000' || ple_private.crockford_checksum_character('BP0000000'), \
                 $1, 'BANNER', 'Banner oracle', 1, clock_timestamp(), \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING blueprint_course_id",
    )
    .bind(&instructor_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("blueprint");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course_revision \
         (blueprint_course_id, blueprint_revision_number, content, content_checksum, saved_at) \
         VALUES ($1, 1, '{}', decode(repeat('00', 32), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .execute(&mut *transaction)
    .await
    .expect("revision");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_id, blueprint_revision_number, actor_account_id, request_checksum, \
          occurred_at) VALUES ($1, 1, $2, decode(repeat('cd', 32), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .bind(&instructor_id)
    .execute(&mut *transaction)
    .await
    .expect("revision event");
    let course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, \
          content_discipline_id, tags) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'adopted', $1, 1, 'Banner', 'Banner course', current_date, current_date + 1, \
                 clock_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING course_instance_id",
    )
    .bind(&blueprint_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("course");
    let foreign_course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, \
          content_discipline_id, tags) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'adopted', $1, 1, 'Banner', 'Banner course', current_date, current_date + 1, \
                 clock_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING course_instance_id",
    )
    .bind(&blueprint_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("foreign course");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, clock_timestamp())",
    )
    .bind(id(0xce10))
    .bind(&course_id)
    .bind(&student_id)
    .execute(&mut *transaction)
    .await
    .expect("Student Record");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp()), \
                ($4, $2, $5, 'student', $6, clock_timestamp()), \
                ($7, $8, $9, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(0xce01))
    .bind(&course_id)
    .bind(&instructor_id)
    .bind(id(0xce02))
    .bind(&student_id)
    .bind(id(0xce10))
    .bind(id(0xce03))
    .bind(&foreign_course_id)
    .bind(&foreign_id)
    .execute(&mut *transaction)
    .await
    .expect("membership");
    transaction.commit().await.expect("fixture commit");
    BannerFixture {
        course_id,
        foreign_course_id,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 and MinIO course-appearance oracle"]
async fn course_banner_saga_is_durable_authorized_and_cross_store() {
    let runtime = acceptance_runtime::CourseAppearanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let fixture = seed(&admin).await;
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
    let course = CourseInstanceId::new(&fixture.course_id).expect("Course Instance ID");
    let foreign_course =
        CourseInstanceId::new(&fixture.foreign_course_id).expect("foreign Course Instance ID");
    // Geometry belongs to the Course Banner production contract.  This saga
    // needs valid metadata to exercise persistence, not a second frozen copy
    // of a chosen pixel size or a resize policy.
    let (banner_width, banner_height) = CourseBannerRendition::Banner.dimensions();
    let upload = CourseBannerUploadId::from_uuid(id(0xcf01));
    let upload_bytes = b"source-banner";
    let upload_object = ObjectId::from_uuid(id(0xd001));
    let staged = store
        .stage_course_banner_upload(
            token(1),
            StageCourseBannerUpload {
                course: course.clone(),
                upload,
                metadata: metadata(
                    upload_object,
                    upload_bytes,
                    "image/png",
                    banner_width,
                    banner_height,
                ),
                width: banner_width,
                height: banner_height,
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
                    course: course.clone(),
                    upload: CourseBannerUploadId::from_uuid(id(0xcf04)),
                    metadata: metadata(
                        ObjectId::from_uuid(id(0xd004)),
                        upload_bytes,
                        "image/png",
                        banner_width,
                        banner_height
                    ),
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
                    course: course.clone(),
                    upload: CourseBannerUploadId::from_uuid(id(0xcf02)),
                    metadata: metadata(
                        ObjectId::from_uuid(id(0xd002)),
                        upload_bytes,
                        "image/png",
                        banner_width,
                        banner_height
                    ),
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
                    course: course.clone(),
                    upload: CourseBannerUploadId::from_uuid(id(0xcf03)),
                    metadata: metadata(
                        ObjectId::from_uuid(id(0xd003)),
                        upload_bytes,
                        "image/png",
                        banner_width,
                        banner_height
                    ),
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
        .finalize_course_banner_upload_stage(token(1), course.clone(), upload)
        .await
        .expect("upload completion");
    let claimed = store
        .read_staged_course_banner_upload(token(1), course.clone(), upload)
        .await
        .expect("exact account/course claim");
    assert!(
        store
            .read_staged_course_banner_upload(token(2), course.clone(), upload)
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

    let banner = CourseBannerId::from_uuid(id(0xcf10));
    let source = b"source-banner".to_vec();
    let rendition = webp(banner_width, banner_height, 7);
    let rendition_checksum = Sha256Checksum::compute(&rendition);
    let source_id = ObjectId::from_uuid(id(0xd010));
    let rendition_id = ObjectId::from_uuid(id(0xd011));
    let prepared = store
        .prepare_course_banner_promotion(
            token(1),
            PrepareCourseBannerPromotion {
                course: course.clone(),
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
                source: metadata(source_id, &source, "image/png", banner_width, banner_height),
                rendition: metadata(
                    rendition_id,
                    &rendition,
                    "image/webp",
                    banner_width,
                    banner_height,
                ),
            },
        )
        .await
        .expect("prepare promotion");
    assert!(
        store
            .read_current_course_banner(token(1), course.clone())
            .await
            .expect("read")
            .is_none(),
        "pointer stays absent before all puts"
    );
    put(&object_store, prepared.source.clone(), source, "image/png").await;
    store
        .complete_prepared_course_banner_object(token(1), course.clone(), banner, source_id)
        .await
        .expect("source completion");
    assert!(
        store
            .finalize_course_banner_promotion(token(1), course.clone(), upload, banner)
            .await
            .is_err(),
        "incomplete promotion never advances pointer"
    );
    put(
        &object_store,
        prepared.rendition.clone(),
        rendition,
        "image/webp",
    )
    .await;
    store
        .complete_prepared_course_banner_object(token(1), course.clone(), banner, rendition_id)
        .await
        .expect("banner rendition completion");
    let finalized = store
        .finalize_course_banner_promotion(token(1), course.clone(), upload, banner)
        .await
        .expect("complete promotion");
    assert_eq!(finalized.banner.id, banner);
    assert_eq!(
        store
            .read_current_course_banner(token(2), course.clone())
            .await
            .expect("Student aggregate")
            .expect("banner")
            .id,
        banner
    );
    assert!(
        store
            .read_current_course_banner(token(3), course.clone())
            .await
            .expect("foreign concealed read")
            .is_none(),
        "foreign Account cannot distinguish a current banner from no banner"
    );
    assert!(
        store
            .read_staged_course_banner_upload(token(1), course.clone(), upload)
            .await
            .is_err(),
        "a promoted upload is single-use"
    );

    // A replacement has its own upload and work.  Its incomplete preparation
    // must not disturb the first, already-visible current banner.
    let replacement_upload = CourseBannerUploadId::from_uuid(id(0xcf20));
    let replacement_upload_id = ObjectId::from_uuid(id(0xd020));
    let replacement_staged = store
        .stage_course_banner_upload(
            token(1),
            StageCourseBannerUpload {
                course: course.clone(),
                upload: replacement_upload,
                metadata: metadata(
                    replacement_upload_id,
                    upload_bytes,
                    "image/png",
                    banner_width,
                    banner_height,
                ),
                width: banner_width,
                height: banner_height,
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
        .finalize_course_banner_upload_stage(token(1), course.clone(), replacement_upload)
        .await
        .expect("complete replacement upload");
    let replacement_banner = CourseBannerId::from_uuid(id(0xcf21));
    let replacement_source_id = ObjectId::from_uuid(id(0xd021));
    let replacement_rendition_id = ObjectId::from_uuid(id(0xd022));
    let replacement_source = upload_bytes.to_vec();
    let replacement_rendition = webp(banner_width, banner_height, 21);
    let replacement_prepared = store
        .prepare_course_banner_promotion(
            token(1),
            PrepareCourseBannerPromotion {
                course: course.clone(),
                upload: replacement_upload,
                banner: replacement_banner,
                update: CourseBannerUpdate {
                    upload: replacement_upload,
                    alternative_text: CourseBannerAlternativeText::Decorative,
                },
                source: metadata(
                    replacement_source_id,
                    &replacement_source,
                    "image/png",
                    banner_width,
                    banner_height,
                ),
                rendition: metadata(
                    replacement_rendition_id,
                    &replacement_rendition,
                    "image/webp",
                    banner_width,
                    banner_height,
                ),
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
            course.clone(),
            replacement_banner,
            replacement_source_id,
        )
        .await
        .expect("complete replacement source");
    assert!(
        store
            .finalize_course_banner_promotion(
                token(1),
                course.clone(),
                replacement_upload,
                replacement_banner
            )
            .await
            .is_err(),
        "incomplete replacement preserves old pointer"
    );
    assert_eq!(
        store
            .read_current_course_banner(token(2), course.clone())
            .await
            .expect("Student old pointer")
            .expect("old banner")
            .id,
        banner
    );
    put(
        &object_store,
        replacement_prepared.rendition.clone(),
        replacement_rendition,
        "image/webp",
    )
    .await;
    store
        .complete_prepared_course_banner_object(
            token(1),
            course.clone(),
            replacement_banner,
            replacement_rendition_id,
        )
        .await
        .expect("complete replacement banner rendition");
    let replacement = store
        .finalize_course_banner_promotion(
            token(1),
            course.clone(),
            replacement_upload,
            replacement_banner,
        )
        .await
        .expect("complete replacement");
    assert_eq!(
        store
            .read_current_course_banner(token(2), course.clone())
            .await
            .expect("Student replacement")
            .expect("replacement banner")
            .id,
        replacement_banner
    );
    let retired = replacement
        .retired
        .expect("replacement returns retired first banner");
    let verified_delete = store
        .prepare_course_banner_object_deletion(token(1), retired.rendition_put_work_id)
        .await
        .expect("durable pre-delete work for retired banner rendition");
    store
        .require_course_banner_deletion_repair(token(1), verified_delete)
        .await
        .expect("uncertain retired banner rendition delete");
    store
        .record_course_banner_cleanup_check(
            token(1),
            verified_delete,
            true,
            Some(rendition_checksum),
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
        .prepare_course_banner_removal(token(1), course.clone())
        .await
        .expect("remove preparation");
    set_inspection_role(&mut inspection, "ple_data_owner").await;
    let theme_before: String = sqlx::query_scalar(
        "SELECT course_theme_id FROM ple_data.course_instance WHERE course_instance_id=$1",
    )
    .bind(course.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("theme before remove");
    assert!(
        store
            .read_current_course_banner(token(1), course.clone())
            .await
            .expect("read")
            .is_none(),
        "remove clears pointer without theme mutation"
    );
    let theme_after: String = sqlx::query_scalar(
        "SELECT course_theme_id FROM ple_data.course_instance WHERE course_instance_id=$1",
    )
    .bind(course.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("theme after remove");
    assert_eq!(
        theme_after, theme_before,
        "removing a banner never changes the independent Theme"
    );
    let delete = store
        .prepare_course_banner_object_deletion(token(1), removal.rendition_put_work_id)
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
        .delete(&removal.rendition)
        .await
        .expect("confirmed real banner rendition delete");
    store
        .complete_course_banner_object_deletion(token(1), delete)
        .await
        .expect("record confirmed delete");
    let missing_delete = store
        .prepare_course_banner_object_deletion(token(1), removal.source_put_work_id)
        .await
        .expect("durable pre-delete source work");
    object_store
        .delete(&removal.source)
        .await
        .expect("real source delete before missing check");
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
