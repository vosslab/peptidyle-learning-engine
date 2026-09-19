#![cfg(feature = "postgres")]

//! Connected oracle for the two database-owned Draft Question source shapes.

use learning_data_access::postgres::{
    PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore, lazy_pool,
};
use learning_data_access::{
    AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionSourceBindingInput,
    DraftQuestionSourceBindingStore, DraftQuestionUuid, SaveAuthoringDraftGeneralFeedbackInput,
    SessionTokenHash, StoreError,
};
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    ObjectId, QuestionBackend, QuestionFormat, QuestionType, SourceObjectChecksum, Timestamp,
    WorkspaceId,
};
use uuid::Uuid;

const SOURCE_BYTES: &[u8] = b"connected WeBWorK Draft Question source";
const DATABASE_URL_ENV: &str = "DATABASE_URL";

fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xa7; 32])
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_millis(1_800_000_000_000)
}

fn source_record(workspace: WorkspaceId, media_type: &str) -> ObjectRecord {
    source_record_with_id(workspace, media_type, 0xa711)
}

fn source_record_with_id(
    workspace: WorkspaceId,
    media_type: &str,
    object_id: u128,
) -> ObjectRecord {
    let id = ObjectId::from_uuid(Uuid::from_u128(object_id));
    ObjectRecord {
        id,
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::WorkspaceQuestionSource {
            workspace,
            object: id,
        },
        sha256: Sha256Checksum::compute(SOURCE_BYTES),
        size_bytes: SOURCE_BYTES.len() as u64,
        media_type: media_type.to_owned(),
        question_revision_tuple: None,
        created_at: timestamp(),
    }
}

async fn seed_instructor(admin: &sqlx::postgres::PgPool, token: SessionTokenHash) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    let account_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ('U00000009', 'instructor', clock_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("Instructor Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
         clock_timestamp() + interval '1 hour')",
    )
    .bind(Uuid::from_u128(0xa702))
    .bind(&account_id)
    .bind(token.to_string())
    .execute(&mut *transaction)
    .await
    .expect("Instructor session");
    transaction.commit().await.expect("fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn webwork_draft_creation_keeps_the_initial_source_binding_on_confirmation() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let application_database_url =
        std::env::var(DATABASE_URL_ENV).expect("API database URL supplied by the baseline owner");
    let application = lazy_pool(&application_database_url).expect("API application pool");
    let session = token();
    seed_instructor(&admin, session).await;
    let drafts = PostgresAuthoringDraftStore::new(application.clone());
    let bindings = PostgresDraftQuestionSourceBindingStore::new(application);
    let workspace = drafts
        .ensure_own_authoring_workspace(session, Uuid::from_u128(0xa703))
        .await
        .expect("Instructor workspace");

    for (media_type, webwork_pg_path) in [
        (
            "application/vnd.peptidyle.question+json",
            Some("Library/Algebra.pg"),
        ),
        ("text/x-wework-pg", None),
    ] {
        let rejected = drafts
            .create_authoring_draft(
                session,
                workspace,
                CreateAuthoringDraftInput {
                    draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(0xa704)),
                    source_record: source_record(workspace, media_type),
                    question_format: QuestionFormat::WebworkPg,
                    webwork_pg_path: webwork_pg_path.map(str::to_owned),
                    question_type: QuestionType::MultipleChoice,
                    title: "Rejected source tuple".to_owned(),
                    description: "This tuple must not create a Draft Question.".to_owned(),
                    language: "en".to_owned(),
                },
            )
            .await;
        assert!(
            matches!(rejected, Err(StoreError::InvalidRecord(_))),
            "crossed or incomplete media tuple is rejected before persistence"
        );
    }

    let source_record = source_record(workspace, "text/x-wework-pg");
    let draft = drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(0xa705)),
                source_record: source_record.clone(),
                question_format: QuestionFormat::WebworkPg,
                webwork_pg_path: Some("Library/Genetics/linked_traits.pg".to_owned()),
                question_type: QuestionType::MultipleChoice,
                title: "Connected WeBWorK Draft".to_owned(),
                description: "A store-level source-binding oracle.".to_owned(),
                language: "en".to_owned(),
            },
        )
        .await
        .expect("WeBWorK Draft Question creation");
    let mut catalog_transaction = admin.begin().await.expect("catalog assertion transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *catalog_transaction)
        .await
        .expect("private catalog assertion role");
    let tuple: (String, String, String, Option<String>) = sqlx::query_as(
        "SELECT backend, question_format, question_type, webwork_pg_path \
         FROM ple_private.draft_question_source_binding WHERE draft_question_uuid = $1",
    )
    .bind(draft.draft_question_uuid.as_uuid())
    .fetch_one(&mut *catalog_transaction)
    .await
    .expect("initial source binding");
    catalog_transaction
        .commit()
        .await
        .expect("catalog assertion commit");
    assert_eq!(
        tuple,
        (
            "webwork".to_owned(),
            "webworkPg".to_owned(),
            "multipleChoice".to_owned(),
            Some("Library/Genetics/linked_traits.pg".to_owned()),
        )
    );

    // PGML is trusted explicit import/authoring provenance, never a filename
    // inference. The same WebWork backend stores its exact immutable format.
    let pgml_source_record = source_record_with_id(workspace, "text/x-wework-pg", 0xa712);
    let pgml_draft = drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(0xa706)),
                source_record: pgml_source_record,
                question_format: QuestionFormat::WebworkPgml,
                webwork_pg_path: Some("Library/Genetics/reviewed.pgml".to_owned()),
                question_type: QuestionType::MultipleChoice,
                title: "Connected reviewed PGML Draft".to_owned(),
                description: "A store-level explicit PGML provenance oracle.".to_owned(),
                language: "en".to_owned(),
            },
        )
        .await
        .expect("reviewed PGML Draft Question creation");
    let mut pgml_catalog_transaction = admin
        .begin()
        .await
        .expect("PGML catalog assertion transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *pgml_catalog_transaction)
        .await
        .expect("PGML private catalog assertion role");
    let pgml_format: String = sqlx::query_scalar(
        "SELECT question_format FROM ple_private.draft_question_source_binding \
         WHERE draft_question_uuid = $1",
    )
    .bind(pgml_draft.draft_question_uuid.as_uuid())
    .fetch_one(&mut *pgml_catalog_transaction)
    .await
    .expect("reviewed PGML source format");
    pgml_catalog_transaction
        .commit()
        .await
        .expect("PGML catalog assertion commit");
    assert_eq!(pgml_format, "webworkPgml");

    let confirmation = DraftQuestionSourceBindingInput {
        draft_question_uuid: draft.draft_question_uuid,
        expected_draft_question_edit_number: draft.edit_number,
        workspace,
        question_backend: QuestionBackend::Webwork,
        question_format: QuestionFormat::WebworkPg,
        question_type: QuestionType::MultipleChoice,
        webwork_pg_path: Some("Library/Genetics/linked_traits.pg".to_owned()),
        draft_imathas_question_backend_binding: None,
        source_object_id: source_record.id,
        source_object_checksum: SourceObjectChecksum::parse(source_record.sha256.to_string())
            .expect("source checksum"),
    };
    let first_edit = bindings
        .bind_draft_question_source(session, confirmation.clone())
        .await
        .expect("initial Pilot binding confirms creation tuple");
    assert_eq!(first_edit, draft.edit_number);
    let replay_edit = bindings
        .bind_draft_question_source(session, confirmation.clone())
        .await
        .expect("replayed Pilot binding remains a no-op");
    assert_eq!(replay_edit, first_edit);
    let confirmed = drafts
        .load_authoring_draft(session, draft.draft_question_uuid)
        .await
        .expect("confirmed Draft Question");
    assert_eq!(confirmed.edit_number, draft.edit_number);

    let edited = drafts
        .save_authoring_draft_general_feedback(
            session,
            SaveAuthoringDraftGeneralFeedbackInput {
                draft_question_uuid: draft.draft_question_uuid,
                expected_edit_number: first_edit,
                general_feedback: Some("Reviewed general feedback.".to_owned()),
            },
        )
        .await
        .expect("intervening metadata-only edit");
    assert!(matches!(
        bindings
            .bind_draft_question_source(session, confirmation.clone())
            .await,
        Err(StoreError::RetryableTransaction)
    ));
    let mut changed_binding = confirmation;
    changed_binding.expected_draft_question_edit_number = edited.edit_number;
    changed_binding.webwork_pg_path = Some("Library/Genetics/linked_traits_reviewed.pg".to_owned());
    let changed_edit = bindings
        .bind_draft_question_source(session, changed_binding.clone())
        .await
        .expect("current binding change returns committed edit");
    assert_eq!(
        changed_edit.as_postgres_bigint(),
        edited.edit_number.as_postgres_bigint() + 1
    );
    assert!(matches!(
        bindings
            .bind_draft_question_source(session, changed_binding.clone())
            .await,
        Err(StoreError::RetryableTransaction)
    ));
    changed_binding.expected_draft_question_edit_number = changed_edit;
    assert_eq!(
        bindings
            .bind_draft_question_source(session, changed_binding)
            .await
            .expect("exact current no-op"),
        changed_edit
    );
}
