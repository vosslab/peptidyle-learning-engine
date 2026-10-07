#![cfg(feature = "postgres")]

//! Connected oracle for raw ordinary Draft source saves and binding reads.

use learning_data_access::postgres::{PostgresAuthoringDraftStore, lazy_pool};
use learning_data_access::{
    AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionUuid, SaveAuthoringDraftInput,
    SessionTokenHash, StoreError,
};
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    AccountId, ObjectId, QuestionBackend, QuestionFormat, QuestionMetadata, QuestionType,
    Timestamp, WorkspaceId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DATABASE_URL_ENV: &str = "DATABASE_URL";
const TEST_NAMESPACE: Uuid = Uuid::from_u128(0x6d15_7261_775f_736f_7572_6365_7465_7374);
static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

fn unique_uuid() -> Uuid {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let identity = format!("{}-{timestamp}-{id}", std::process::id());
    Uuid::new_v5(&TEST_NAMESPACE, identity.as_bytes())
}

fn session_token() -> SessionTokenHash {
    let mut token = [0_u8; 32];
    token[..16].copy_from_slice(unique_uuid().as_bytes());
    token[16..].copy_from_slice(unique_uuid().as_bytes());
    SessionTokenHash::compute(&token)
}

fn source_record(workspace: WorkspaceId, media_type: &str, bytes: &[u8]) -> ObjectRecord {
    let id = ObjectId::from_uuid(unique_uuid());
    ObjectRecord {
        id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::WorkspaceQuestionSource {
            workspace_id: workspace,
            object_id: id,
        },
        sha256: Sha256Checksum::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: media_type.to_owned(),
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(1_700_000_000_000),
    }
}

async fn seed_instructor(admin: &sqlx::postgres::PgPool, token: SessionTokenHash) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    let account_id = AccountId::from_debug_serial(unique_uuid().as_u128()).to_string();
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ($1, 'instructor', pg_catalog.transaction_timestamp())",
    )
    .bind(&account_id)
    .execute(&mut *transaction)
    .await
    .expect("Instructor Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, user_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), pg_catalog.transaction_timestamp(), \
         pg_catalog.transaction_timestamp() + interval '1 hour')",
    )
    .bind(unique_uuid())
    .bind(&account_id)
    .bind(token.to_string())
    .execute(&mut *transaction)
    .await
    .expect("Instructor session");
    transaction.commit().await.expect("fixture commit");
}

fn empty_metadata() -> QuestionMetadata {
    QuestionMetadata {
        question_title: String::new(),
        question_description: String::new(),
        tags: Vec::new(),
        question_license: None,
        question_citation: None,
        language: None,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn raw_empty_incomplete_and_broken_sources_keep_their_binding_and_edit_cas() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let application_database_url =
        std::env::var(DATABASE_URL_ENV).expect("API database URL supplied by the baseline owner");
    let application = lazy_pool(&application_database_url).expect("API application pool");
    let token = session_token();
    seed_instructor(&admin, token).await;
    let drafts = PostgresAuthoringDraftStore::new(application);
    let workspace = drafts
        .ensure_own_authoring_workspace(token, unique_uuid())
        .await
        .expect("Instructor workspace");

    // The tuple is valid Native HOTSPOT evidence, but this asset is never registered.
    let hotspot_asset_id = unique_uuid();
    let hotspot_checksum = Sha256Checksum::compute(b"unregistered HOTSPOT image bytes").to_string();
    let hotspot_source = format!(
        r#"{{
            "format": "pleQuestionJson",
            "prompt": "Select the active site.",
            "response": {{
                "kind": "hotspot",
                "surface": {{
                    "questionImageAssetId": "{hotspot_asset_id}",
                    "checksum": "{hotspot_checksum}",
                    "description": "Protein structure"
                }},
                "regions": [{{
                    "id": "active-site",
                    "label": "Active site",
                    "x": 10,
                    "y": 10,
                    "width": 20,
                    "height": 20
                }}],
                "correctRegions": ["active-site"]
            }}
        }}"#
    );

    let cases = [
        (
            QuestionBackend::Ple,
            QuestionFormat::PleQuestionJson,
            None,
            "application/vnd.peptidyle.question+json",
            b"initial source".as_slice(),
            b"".as_slice(),
            None,
        ),
        (
            QuestionBackend::Ple,
            QuestionFormat::PleQuestionJson,
            None,
            "application/vnd.peptidyle.question+json",
            b"initial source".as_slice(),
            b"{ malformed Native JSON".as_slice(),
            None,
        ),
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg,
            Some("Library/Genetics/broken.pg"),
            "text/x-wework-pg",
            b"initial source".as_slice(),
            b"DOCUMENT();\n$answer = qr/[".as_slice(),
            None,
        ),
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPgml,
            Some("Library/Genetics/incomplete.pgml"),
            "text/x-wework-pg",
            b"initial source".as_slice(),
            b"[% incomplete".as_slice(),
            None,
        ),
        (
            QuestionBackend::Ple,
            QuestionFormat::PleQuestionJson,
            None,
            "application/vnd.peptidyle.question+json",
            b"".as_slice(),
            hotspot_source.as_bytes(),
            Some(QuestionType::Hotspot),
        ),
    ];

    for (backend, format, path, media_type, initial_bytes, bytes, question_type) in cases {
        let created = drafts
            .create_authoring_draft(
                token,
                workspace,
                CreateAuthoringDraftInput {
                    draft_question_uuid: DraftQuestionUuid::from_uuid(unique_uuid()),
                    source_record: source_record(workspace, media_type, initial_bytes),
                    question_format: format,
                    webwork_pg_path: path.map(str::to_owned),
                    metadata: empty_metadata(),
                    question_type: None,
                },
            )
            .await
            .expect("Draft accepts empty metadata and an initial registered source");

        let saved_record = source_record(workspace, media_type, bytes);
        let saved = drafts
            .save_authoring_draft(
                token,
                SaveAuthoringDraftInput {
                    draft_question_uuid: created.draft_question_uuid,
                    expected_edit_number: created.edit_number,
                    source_record: saved_record.clone(),
                    question_type,
                },
            )
            .await
            .expect("Draft saves raw source without publication validation");
        let reopened = drafts
            .load_authoring_draft(token, created.draft_question_uuid)
            .await
            .expect("saved Draft reopens");

        assert_eq!(reopened.source_record, saved_record);
        assert_eq!(
            reopened.source_record.sha256,
            Sha256Checksum::compute(bytes)
        );
        assert_eq!(reopened.source_record.size_bytes, bytes.len() as u64);
        assert_eq!(reopened.source_record.media_type, media_type);
        assert_eq!(reopened.question_backend, backend);
        assert_eq!(reopened.question_format, format);
        assert_eq!(reopened.webwork_pg_path.as_deref(), path);
        assert_eq!(reopened.question_type, question_type);
        assert_eq!(
            reopened.parent_published_question_revision_tuple,
            created.parent_published_question_revision_tuple
        );
        assert_eq!(reopened.edit_number, saved.edit_number);

        let stale = drafts
            .save_authoring_draft(
                token,
                SaveAuthoringDraftInput {
                    draft_question_uuid: created.draft_question_uuid,
                    expected_edit_number: created.edit_number,
                    source_record: source_record(workspace, media_type, b"stale edit"),
                    question_type,
                },
            )
            .await;
        assert!(
            matches!(
                stale,
                Err(StoreError::Conflict | StoreError::RetryableTransaction)
            ),
            "a stale Edit Number cannot replace the saved bytes"
        );
    }
}
