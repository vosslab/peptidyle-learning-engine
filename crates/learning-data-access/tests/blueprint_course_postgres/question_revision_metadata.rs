//! Connected PostgreSQL oracle for exact-current metadata saves and authority.

use super::*;
use learning_data_access::postgres::{PostgresQuestionLibraryStore, PostgresQuestionMetadataStore};
use learning_data_access::{QuestionMetadataStore, StoreError};
use question_model::{
    BloomCognitiveProcess, PublishedQuestionRevisionTuple, QuestionAvailability,
    QuestionMetadataReplacement, QuestionRevisionNumber, QuestionType, SaveQuestionMetadataRequest,
    Tag,
};

#[path = "question_revision_metadata/manual_type_correction.rs"]
mod manual_type_correction;

fn metadata_request(
    tuple: PublishedQuestionRevisionTuple,
    edit_number: u64,
    title: &str,
) -> SaveQuestionMetadataRequest {
    SaveQuestionMetadataRequest {
        published_question_revision_tuple: tuple,
        expected_metadata_edit_number: edit_number,
        metadata: QuestionMetadataReplacement {
            question_title: title.to_owned(),
            question_description: "The Revision source stays unchanged.".to_owned(),
            question_type: QuestionType::MultipleChoice,
            tags: vec![Tag::new("metadata")],
            discipline_uuid: id(0xcc01),
            subject_uuid: id(0xcc02),
            topic_uuid: None,
            subtopic_uuid: None,
            bloom_cognitive_process: Some(BloomCognitiveProcess::Analyze),
            bloom_knowledge_dimension: None,
        },
    }
}

async fn sysadmin_session(admin: &sqlx::postgres::PgPool) -> SessionTokenHash {
    let token_material = id(0xb10c);
    let token = SessionTokenHash::compute(token_material.as_bytes());
    let mut tx = admin.begin().await.expect("Sysadmin session transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("Sysadmin fixture role");
    let account_id: String = sqlx::query_scalar(
        "SELECT account_id FROM ple_private.account WHERE user_role = 'sysadmin' \
         ORDER BY created_at LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("fixture Sysadmin Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, user_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'sysadmin', decode($3, 'hex'), transaction_timestamp(), \
                 transaction_timestamp() + interval '1 hour')",
    )
    .bind(id(0xb10b))
    .bind(account_id)
    .bind(token.to_string())
    .execute(&mut *tx)
    .await
    .expect("authenticated Sysadmin session");
    tx.commit().await.expect("Sysadmin session commit");
    token
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn exact_current_metadata_uses_cas_preserves_source_and_obeys_authority() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let application_url = std::env::var("DATABASE_URL").expect("API application database URL");
    let application = lazy_pool(&application_url).expect("API application pool");
    seed_if_needed(&admin).await;

    let question_id: question_model::PublishedQuestionId =
        QUESTION.parse().expect("fixture Question ID");
    let tuple = PublishedQuestionRevisionTuple {
        published_question_id: question_id.clone(),
        revision_number: QuestionRevisionNumber::new(1).expect("Revision 1"),
    };
    let metadata = PostgresQuestionMetadataStore::new(application.clone());
    let library = PostgresQuestionLibraryStore::new(application);
    let before = library
        .load_published_question_revision_library_entry(token(), &tuple)
        .await
        .expect("read exact current Revision");
    let current = library
        .load_current_published_question_shared_metadata(
            token(),
            std::slice::from_ref(&question_id),
        )
        .await
        .expect("read current metadata")
        .remove(0);

    let request = metadata_request(
        tuple.clone(),
        current.metadata_edit_number,
        "Corrected exact Revision",
    );
    let mut invalid_type = request.clone();
    invalid_type.metadata.question_type = QuestionType::Numeric;
    assert!(
        metadata
            .save_question_metadata(token(), invalid_type)
            .await
            .is_err(),
        "Native Question Type remains bound to its immutable source"
    );
    assert!(
        matches!(
            metadata
                .save_question_metadata(reader_token(), request.clone())
                .await,
            Err(StoreError::Forbidden)
        ),
        "a different Instructor cannot edit this Question"
    );
    let saved = metadata
        .save_question_metadata(token(), request.clone())
        .await
        .expect("owner edits exact current metadata");
    assert_eq!(saved.published_question_revision_tuple, tuple);
    assert_eq!(saved.metadata_edit_number, current.metadata_edit_number + 1);

    let after = library
        .load_published_question_revision_library_entry(token(), &tuple)
        .await
        .expect("read exact Revision after metadata edit");
    assert_eq!(after.question_title, "Corrected exact Revision");
    assert_eq!(after.source_object_id, before.source_object_id);
    assert_eq!(after.source_object_checksum, before.source_object_checksum);
    assert!(
        metadata
            .save_question_metadata(token(), request)
            .await
            .is_err(),
        "a stale metadata Edit Number cannot overwrite the current record"
    );

    let sysadmin = sysadmin_session(&admin).await;
    let sysadmin_metadata = library
        .load_current_published_question_shared_metadata(
            sysadmin,
            std::slice::from_ref(&question_id),
        )
        .await
        .expect("Sysadmin can load current metadata before opening its editor")
        .remove(0);
    let sysadmin_save = metadata
        .save_question_metadata(
            sysadmin,
            metadata_request(
                tuple.clone(),
                sysadmin_metadata.metadata_edit_number,
                "Sysadmin correction",
            ),
        )
        .await
        .expect("Sysadmin edits available metadata");
    assert_eq!(sysadmin_save.published_question_revision_tuple, tuple);

    let available = library
        .load_published_question_revision_library_entry(sysadmin, &tuple)
        .await
        .expect("read current Question before archive");
    assert!(available.viewer_may_archive);
    let archived = library
        .archive_published_question(
            sysadmin,
            &question_id,
            available.availability_edit_number,
            "Sysadmin correction",
        )
        .await
        .expect("Sysadmin archives the Question");
    assert_eq!(archived.availability, QuestionAvailability::Archived);
    let archived_read = library
        .load_published_question_revision_library_entry(sysadmin, &tuple)
        .await
        .expect("read archived exact Revision");
    assert!(!archived_read.viewer_may_edit_metadata);
    assert!(archived_read.viewer_may_archive);
    assert!(
        metadata
            .save_question_metadata(
                sysadmin,
                metadata_request(
                    tuple.clone(),
                    sysadmin_save.metadata_edit_number,
                    "Archived edit must fail",
                ),
            )
            .await
            .is_err(),
        "archived Question metadata remains read-only for Sysadmin"
    );
    assert_eq!(
        library
            .restore_published_question(sysadmin, &question_id, archived.edit_number)
            .await
            .expect("Sysadmin restores the Question")
            .availability,
        QuestionAvailability::Available
    );
}
