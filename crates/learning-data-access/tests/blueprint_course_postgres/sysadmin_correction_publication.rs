//! Connected PostgreSQL oracle for Sysadmin correction-Draft publication.

use super::*;
use learning_data_access::postgres::{
    PostgresAuthoringDraftStore, PostgresDraftQuestionImageStore,
    PostgresDraftQuestionSourceBindingStore, PostgresQuestionImageDeliveryStore,
};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionImageStore,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    PreparedQuestionImagePublication, QuestionImageDeliveryResolution, QuestionImageDeliveryStore,
    RegisterDraftQuestionImageInput, SaveAuthoringDraftInput, SessionTokenHash,
};
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionBackend, QuestionFormat,
    QuestionImageAssetId, QuestionLicense, QuestionMetadata, QuestionRevisionNumber,
    QuestionRevisionReason, QuestionType, Timestamp, WorkspaceId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const HOTSPOT_IMAGE_BYTES: &[u8] = b"verified source raster for native HOTSPOT correction";
static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

fn fresh_id() -> Uuid {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock follows Unix epoch")
        .as_nanos();
    let sequence = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    Uuid::new_v5(
        &Uuid::from_u128(0x1616_0000_0000_0000_0000_0000_0000_0001),
        format!("{}:{timestamp}:{sequence}", std::process::id()).as_bytes(),
    )
}

fn authoring_source_record(workspace: WorkspaceId, object_id: Uuid, bytes: &[u8]) -> ObjectRecord {
    let object_id = ObjectId::from_uuid(object_id);
    ObjectRecord {
        id: object_id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::WorkspaceQuestionSource {
            workspace_id: workspace,
            object_id,
        },
        sha256: Sha256Checksum::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: "application/vnd.peptidyle.question+json".to_owned(),
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(1_780_000_000_000),
    }
}

fn published_source_record(
    tuple: PublishedQuestionRevisionTuple,
    object_id: Uuid,
    bytes: &[u8],
) -> ObjectRecord {
    let object_id = ObjectId::from_uuid(object_id);
    ObjectRecord {
        id: object_id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::QuestionSource,
        address: ObjectAddress::QuestionSource {
            published_question_revision_tuple: tuple.clone(),
            object_id,
        },
        sha256: Sha256Checksum::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: "application/vnd.peptidyle.question+json".to_owned(),
        published_question_revision_tuple: Some(tuple),
        created_at: Timestamp::from_unix_millis(1_780_000_001_000),
    }
}

async fn create_bound_draft(
    authoring: &PostgresAuthoringDraftStore,
    source_bindings: &PostgresDraftQuestionSourceBindingStore,
    session: SessionTokenHash,
    workspace: WorkspaceId,
    bytes: &[u8],
    title: &str,
) -> AuthoringDraft {
    let draft_question_uuid = DraftQuestionUuid::from_uuid(fresh_id());
    let source = authoring_source_record(workspace, fresh_id(), bytes);
    let draft = authoring
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid,
                source_record: source.clone(),
                question_format: QuestionFormat::PleQuestionJson,
                webwork_pg_path: None,
                question_type: Some(QuestionType::MultipleChoice),
                metadata: QuestionMetadata {
                    question_title: title.to_owned(),
                    question_description: "A connected correction-Draft fixture.".to_owned(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::CcBy4_0),
                    question_citation: None,
                    language: Some("en".to_owned()),
                },
            },
        )
        .await
        .expect("ordinary Draft Question creation");
    let bound_edit_number = source_bindings
        .bind_draft_question_source(
            session,
            DraftQuestionSourceBindingInput {
                draft_question_uuid,
                expected_draft_question_edit_number: draft.edit_number,
                workspace,
                question_backend: QuestionBackend::Ple,
                question_format: QuestionFormat::PleQuestionJson,
                question_type: QuestionType::MultipleChoice,
                webwork_pg_path: None,
                draft_imathas_question_backend_binding: None,
                source_object_id: source.id,
                source_object_checksum: question_model::SourceObjectChecksum::parse(
                    source.sha256.to_string(),
                )
                .expect("canonical Draft source checksum"),
            },
        )
        .await
        .expect("registered Draft source binding");
    assert_eq!(bound_edit_number, draft.edit_number);
    draft
}

async fn authenticated_sysadmin(admin: &sqlx::postgres::PgPool) -> (SessionTokenHash, String) {
    let session_id = fresh_id();
    let token_material = fresh_id();
    let session_token = SessionTokenHash::compute(token_material.as_bytes());
    let mut transaction = admin.begin().await.expect("Sysadmin session transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("Sysadmin fixture role");
    let account_id: String = sqlx::query_scalar(
        "SELECT account_id FROM ple_private.account WHERE user_role = 'sysadmin' \
         ORDER BY created_at LIMIT 1",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("fixture Sysadmin Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, user_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'sysadmin', decode($3, 'hex'), transaction_timestamp(), \
                 transaction_timestamp() + interval '1 hour')",
    )
    .bind(session_id)
    .bind(&account_id)
    .bind(session_token.to_string())
    .execute(&mut *transaction)
    .await
    .expect("authenticated Sysadmin session");
    transaction.commit().await.expect("Sysadmin session commit");
    (session_token, account_id)
}

async fn current_question_state(
    admin: &sqlx::postgres::PgPool,
    question_id: &PublishedQuestionId,
) -> (String, String, i32) {
    let mut inspection = admin
        .begin()
        .await
        .expect("Published Question inspection transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *inspection)
        .await
        .expect("Published Question inspection role");
    let state = sqlx::query_as(
        "SELECT owner.owner_account_id::text, question.availability::text, \
                max(revision.revision_number) \
           FROM ple_data.published_question AS question \
           JOIN ple_data.question_current_owner AS owner USING (published_question_id) \
           JOIN ple_data.question_revision AS revision USING (published_question_id) \
          WHERE question.published_question_id = $1 \
          GROUP BY owner.owner_account_id, question.availability",
    )
    .bind(question_id.as_str())
    .fetch_one(&mut *inspection)
    .await
    .expect("current Published Question owner and Revision");
    inspection
        .commit()
        .await
        .expect("Published Question inspection commit");
    state
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn sysadmin_correction_draft_publishes_same_id_without_changing_owner() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let application_database_url = std::env::var("DATABASE_URL").expect("API database URL");
    let application = lazy_pool(&application_database_url).expect("API application pool");
    seed_if_needed(&admin).await;

    let authoring = PostgresAuthoringDraftStore::new(application.clone());
    let images = PostgresDraftQuestionImageStore::new(application.clone());
    let source_bindings = PostgresDraftQuestionSourceBindingStore::new(application.clone());
    let image_delivery = PostgresQuestionImageDeliveryStore::new(application);

    let (sysadmin_session, sysadmin_account_id) = authenticated_sysadmin(&admin).await;
    let sysadmin_workspace = authoring
        .ensure_own_authoring_workspace(sysadmin_session, fresh_id())
        .await
        .expect("Sysadmin authoring workspace");
    let initial_sysadmin_draft = create_bound_draft(
        &authoring,
        &source_bindings,
        sysadmin_session,
        sysadmin_workspace,
        b"Initial Sysadmin correction Draft source",
        "Sysadmin correction Draft",
    )
    .await;
    let image_asset_id = QuestionImageAssetId::from_uuid(fresh_id());
    let image_checksum = Sha256Checksum::compute(HOTSPOT_IMAGE_BYTES).to_string();
    let correction_source = format!(
        r#"{{"format":"pleQuestionJson","prompt":"Select the active site.","response":{{"kind":"hotspot","surface":{{"questionImageAssetId":"{image_asset_id}","checksum":"{image_checksum}","description":"Protein structure"}},"regions":[{{"id":"active-site","label":"Active site","x":10,"y":10,"width":20,"height":20}}],"correctRegions":["active-site"]}}}}"#
    );
    let sysadmin_draft = authoring
        .save_authoring_draft(
            sysadmin_session,
            SaveAuthoringDraftInput {
                draft_question_uuid: initial_sysadmin_draft.draft_question_uuid,
                expected_edit_number: initial_sysadmin_draft.edit_number,
                source_record: authoring_source_record(
                    sysadmin_workspace,
                    fresh_id(),
                    correction_source.as_bytes(),
                ),
                question_type: Some(QuestionType::Hotspot),
            },
        )
        .await
        .expect("Sysadmin saves the ordinary correction Draft");
    assert_eq!(
        sysadmin_draft.parent_published_question_revision_tuple, None,
        "the correction remains an ordinary unparented Draft"
    );

    let image_object_id = ObjectId::from_uuid(fresh_id());
    let draft_image_record = ObjectRecord {
        id: image_object_id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::DraftQuestionImage {
            workspace_id: sysadmin_workspace,
            draft_question_id: sysadmin_draft.draft_question_uuid.as_uuid(),
            question_image_asset_id: image_asset_id,
            object_id: image_object_id,
        },
        sha256: Sha256Checksum::compute(HOTSPOT_IMAGE_BYTES),
        size_bytes: HOTSPOT_IMAGE_BYTES.len() as u64,
        media_type: "image/png".to_owned(),
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(1_780_000_000_000),
    };
    images
        .register_draft_question_image(
            sysadmin_session,
            RegisterDraftQuestionImageInput {
                draft_question_uuid: sysadmin_draft.draft_question_uuid,
                expected_edit_number: sysadmin_draft.edit_number,
                question_image_asset_id: image_asset_id,
                source_record: draft_image_record,
                intrinsic_width: 10,
                intrinsic_height: 10,
            },
        )
        .await
        .expect("Sysadmin registers the Native HOTSPOT image in the owned Draft");
    let loaded_image = images
        .load_draft_question_image(
            sysadmin_session,
            sysadmin_draft.draft_question_uuid,
            image_asset_id,
        )
        .await
        .expect("Sysadmin loads the registered Native HOTSPOT Draft image");

    let question_id: PublishedQuestionId = QUESTION.parse().expect("fixture Question ID");
    let (owner_before, availability, parent_revision_number) =
        current_question_state(&admin, &question_id).await;
    assert_eq!(availability, "available", "correction target is eligible");
    assert_ne!(
        owner_before, sysadmin_account_id,
        "this is a non-owner Sysadmin correction"
    );
    let parent_tuple = PublishedQuestionRevisionTuple {
        published_question_id: question_id.clone(),
        revision_number: QuestionRevisionNumber::new(parent_revision_number as u32)
            .expect("current Question Revision number"),
    };
    let expected_tuple = PublishedQuestionRevisionTuple {
        published_question_id: question_id,
        revision_number: QuestionRevisionNumber::new(parent_revision_number as u32 + 1)
            .expect("successor Question Revision number"),
    };
    let restricted_image_object_id = ObjectId::from_uuid(fresh_id());
    let published_tuple = source_bindings
        .publish_question_revision(
            sysadmin_session,
            ExistingQuestionRevisionPublicationInput {
                hotspot_question_image: Some(PreparedQuestionImagePublication {
                    question_image_asset_id: image_asset_id,
                    restricted_source_record: ObjectRecord {
                        id: restricted_image_object_id,
                        storage_area: ObjectStorageArea::PrivateContent,
                        data_class: ObjectDataClass::QuestionImage,
                        address: ObjectAddress::RestrictedQuestionImage {
                            published_question_revision_tuple: expected_tuple.clone(),
                            question_image_asset_id: image_asset_id,
                            object_id: restricted_image_object_id,
                        },
                        sha256: loaded_image.source_record.sha256,
                        size_bytes: loaded_image.source_record.size_bytes,
                        media_type: loaded_image.source_record.media_type.clone(),
                        published_question_revision_tuple: Some(expected_tuple.clone()),
                        created_at: Timestamp::from_unix_millis(1_780_000_002_000),
                    },
                    public_object_id: ObjectId::from_uuid(fresh_id()),
                    intrinsic_width: loaded_image.intrinsic_width,
                    intrinsic_height: loaded_image.intrinsic_height,
                    delivery_id: fresh_id(),
                    job_id: fresh_id(),
                }),
                draft_question_uuid: sysadmin_draft.draft_question_uuid,
                expected_draft_question_edit_number: sysadmin_draft.edit_number,
                workspace: sysadmin_workspace,
                parent_published_question_revision_tuple: parent_tuple,
                question_source_object_record: published_source_record(
                    expected_tuple.clone(),
                    fresh_id(),
                    correction_source.as_bytes(),
                ),
                question_revision_reason: QuestionRevisionReason::new(
                    "Apply the reviewed Sysadmin correction.".to_owned(),
                )
                .expect("correction reason"),
                question_publication_event_id: fresh_id(),
            },
        )
        .await
        .expect("Sysadmin publishes the correction through the existing same-ID operation");
    assert_eq!(published_tuple, expected_tuple);

    assert_eq!(
        image_delivery
            .resolve_question_image_delivery(
                sysadmin_session,
                published_tuple.clone(),
                image_asset_id,
            )
            .await
            .expect("Sysadmin resolves the exact pending Question Image"),
        QuestionImageDeliveryResolution::Pending,
        "the resolver exposes only the authorized pending state before activation"
    );

    let mut publication_inspection = admin
        .begin()
        .await
        .expect("published image inspection transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *publication_inspection)
        .await
        .expect("published image inspection role");
    let published_image_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_private.question_image_publication \
          WHERE published_question_id = $1 AND revision_number = $2 \
            AND question_image_asset_id = $3",
    )
    .bind(published_tuple.published_question_id.as_str())
    .bind(published_tuple.revision_number.get() as i32)
    .bind(image_asset_id.as_uuid())
    .fetch_one(&mut *publication_inspection)
    .await
    .expect("published HOTSPOT image binding");
    publication_inspection
        .commit()
        .await
        .expect("published image inspection transaction commit");
    assert_eq!(
        published_image_count, 1,
        "correction publishes its registered image"
    );

    let (owner_after, availability_after, current_revision_number) =
        current_question_state(&admin, &published_tuple.published_question_id).await;
    assert_eq!(
        owner_after, owner_before,
        "correction preserves the Instructor owner"
    );
    assert_eq!(availability_after, "available");
    assert_eq!(
        current_revision_number,
        published_tuple.revision_number.get() as i32
    );
}
