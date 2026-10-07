//! Fixtures and exact-row readers for the ordinary Question fork oracle.

use super::*;

use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    AccountId, BloomCognitiveProcess, BloomKnowledgeDimension, ObjectId,
    PublishedQuestionRevisionTuple, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship,
    QuestionMetadataReplacement, SaveQuestionMetadataRequest, Tag, Timestamp, WorkspaceId,
};

pub(super) const SOURCE_BYTES: &[u8] = br#"{
  "format": "pleQuestionJson",
  "prompt": "M07 exact source bytes",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "choice-a", "text": "Choice A"},
      {"id": "choice-b", "text": "Choice B"}
    ],
    "correctChoice": "choice-a"
  }
}"#;
pub(super) const QUESTION_SOURCE_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";
pub(super) const SOURCE_DISCIPLINE_ID: u128 = 0xcc01;
pub(super) const SOURCE_SUBJECT_ID: u128 = 0xcc02;
pub(super) const DRAFT_ID: u128 = 0x07070000000000000000000000000001;
pub(super) const FORK_REQUEST_KEY: u128 = 0x07070000000000000000000000000006;

pub(super) fn source_author_account() -> AccountId {
    instructor_account_id()
        .parse()
        .expect("seeded Instructor Account ID")
}

pub(super) fn source_authorship() -> QuestionAuthorship {
    QuestionAuthorship::new(vec![
        QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new("M07 External Author".to_owned())
                .expect("external author display name"),
            account_id: None,
        },
        QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new("M07 Instructor Author".to_owned())
                .expect("Instructor author display name"),
            account_id: Some(source_author_account()),
        },
    ])
    .expect("ordered source authorship")
}

pub(super) fn workspace_source_record(
    workspace: WorkspaceId,
    object_id: Uuid,
    bytes: &[u8],
) -> ObjectRecord {
    ObjectRecord {
        id: ObjectId::from_uuid(object_id),
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::WorkspaceQuestionSource {
            workspace_id: workspace,
            object_id: ObjectId::from_uuid(object_id),
        },
        sha256: Sha256Checksum::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: QUESTION_SOURCE_MEDIA_TYPE.to_owned(),
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(1_791_200_000_000),
    }
}

pub(super) fn published_source_record(
    tuple: PublishedQuestionRevisionTuple,
    object_id: Uuid,
    bytes: &[u8],
) -> ObjectRecord {
    ObjectRecord {
        id: ObjectId::from_uuid(object_id),
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::QuestionSource,
        address: ObjectAddress::QuestionSource {
            published_question_revision_tuple: tuple.clone(),
            object_id: ObjectId::from_uuid(object_id),
        },
        sha256: Sha256Checksum::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: QUESTION_SOURCE_MEDIA_TYPE.to_owned(),
        published_question_revision_tuple: Some(tuple),
        created_at: Timestamp::from_unix_millis(1_791_200_001_000),
    }
}

pub(super) fn metadata_replacement(
    tuple: PublishedQuestionRevisionTuple,
    expected_metadata_edit_number: u64,
    title: &str,
    description: &str,
    tags: &[&str],
    bloom_cognitive_process: Option<BloomCognitiveProcess>,
    bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
) -> SaveQuestionMetadataRequest {
    SaveQuestionMetadataRequest {
        published_question_revision_tuple: tuple,
        expected_metadata_edit_number,
        metadata: QuestionMetadataReplacement {
            question_title: title.to_owned(),
            question_description: description.to_owned(),
            question_type: question_model::QuestionType::MultipleChoice,
            tags: tags.iter().map(|tag| Tag::new(*tag)).collect(),
            discipline_uuid: id(SOURCE_DISCIPLINE_ID),
            subject_uuid: id(SOURCE_SUBJECT_ID),
            topic_uuid: None,
            subtopic_uuid: None,
            bloom_cognitive_process,
            bloom_knowledge_dimension,
        },
    }
}

pub(super) async fn seed_source_question(
    admin: &sqlx::PgPool,
    source_tuple: &PublishedQuestionRevisionTuple,
    source_object_id: Uuid,
    bytes: &[u8],
) {
    let source_id = source_tuple.published_question_id.as_str();
    let checksum = Sha256Checksum::compute(bytes);
    let mut transaction = admin.begin().await.expect("M07 source fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question data fixture role");
    sqlx::query(
        "INSERT INTO ple_data.published_question (\
             published_question_id, parent_published_question_id, parent_revision_number, created_at\
         ) VALUES ($1, NULL, NULL, clock_timestamp())",
    )
    .bind(source_id)
    .execute(&mut *transaction)
    .await
    .expect("source Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision (\
             published_question_id, revision_number, backend, general_feedback, hint,\
             worked_solution, published_at\
         ) VALUES ($1, 1, 'ple', 'Source feedback', 'Source hint',\
                   'Source worked solution', clock_timestamp())",
    )
    .bind(source_id)
    .execute(&mut *transaction)
    .await
    .expect("source Revision 1");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_metadata (\
             published_question_id, revision_number, question_title, question_description,\
             language, question_type, tags, content_discipline_id, content_subject_id,\
             bloom_cognitive_process, bloom_knowledge_dimension, created_at, updated_at\
         ) VALUES ($1, 1, 'M07 source title', 'M07 source description', 'en',\
                   'multipleChoice', ARRAY['m07-source'], $2, $3, 'Apply', NULL,\
                   clock_timestamp(), clock_timestamp())",
    )
    .bind(source_id)
    .bind(id(SOURCE_DISCIPLINE_ID))
    .bind(id(SOURCE_SUBJECT_ID))
    .execute(&mut *transaction)
    .await
    .expect("source Revision metadata and partial Bloom classification");

    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question source object fixture role");
    sqlx::query(
        "INSERT INTO ple_private.object_record (\
             object_record_id, object_address, object_storage_area, object_data_class,\
             sha256, size_bytes, media_type, created_at, updated_at\
         ) VALUES (\
             $1, jsonb_build_object(\
                 'kind', 'questionSource',\
                 'publishedQuestionRevisionTuple',\
                     jsonb_build_object('publishedQuestionId', $2, 'revisionNumber', 1),\
                 'objectId', $1\
             ), 'private-content', 'question-source', $3, $4, $5,\
             clock_timestamp(), clock_timestamp()\
         )",
    )
    .bind(source_object_id)
    .bind(source_id)
    .bind(checksum.as_bytes().to_vec())
    .bind(bytes.len() as i64)
    .bind(QUESTION_SOURCE_MEDIA_TYPE)
    .execute(&mut *transaction)
    .await
    .expect("exact source Object Record");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding (\
             published_question_id, revision_number, backend, native_question_type,\
             question_format, source_object_record_id, source_object_checksum, created_at\
         ) VALUES ($1, 1, 'ple', 'multipleChoice', 'pleQuestionJson', $2, $3,\
                   clock_timestamp())",
    )
    .bind(source_id)
    .bind(source_object_id)
    .bind(checksum.to_string())
    .execute(&mut *transaction)
    .await
    .expect("source binding with matching Native Type guard");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question stewardship fixture role");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_authorship (\
             published_question_id, revision_number, author_position, author_display_name,\
             author_account_id\
         ) VALUES ($1, 1, 1, 'M07 External Author', NULL),\
                  ($1, 1, 2, 'M07 Instructor Author', $2)",
    )
    .bind(source_id)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("source Revision authorship");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_license (\
             published_question_id, revision_number, spdx_expression\
         ) VALUES ($1, 1, 'CC-BY-4.0')",
    )
    .bind(source_id)
    .execute(&mut *transaction)
    .await
    .expect("source Revision license");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_citation (\
             published_question_id, revision_number, citation_text,\
             created_at, updated_at\
         ) VALUES ($1, 1, 'M07 source citation',\
                   clock_timestamp(), clock_timestamp())",
    )
    .bind(source_id)
    .execute(&mut *transaction)
    .await
    .expect("source Revision citation");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_acceptance (\
             published_question_id, revision_number, parent_revision_number, editor_account_id,\
             accepted_by_account_id, accepted_at, reason_for_edit\
         ) VALUES ($1, 1, NULL, $2, $2, (\
             SELECT published_at FROM ple_data.question_revision \
             WHERE published_question_id = $1 AND revision_number = 1\
         ), 'Create the M07 parent fixture')",
    )
    .bind(source_id)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("source Revision acceptance");
    sqlx::query(
        "INSERT INTO ple_data.question_ownership_event (\
             question_ownership_event_id, published_question_id, owner_account_id,\
             recorded_by_account_id, event_kind, occurred_at\
         ) VALUES ($1, $2, $3, $3, 'initial', clock_timestamp())",
    )
    .bind(id(0x07070000000000000000000000000008))
    .bind(source_id)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("source Question ownership");
    sqlx::query(
        "INSERT INTO ple_data.question_publication_event (\
             event_id, published_question_id, revision_number, actor_account_id, occurred_at\
         ) VALUES ($1, $2, 1, $3, clock_timestamp())",
    )
    .bind(id(0x07070000000000000000000000000009))
    .bind(source_id)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("source Question publication event");
    sqlx::query(
        "INSERT INTO ple_data.question_availability_event (\
             event_id, published_question_id, actor_account_id, availability, edit_number,\
             reason, occurred_at\
         ) VALUES ($1, $2, $3, 'available', 1, NULL, clock_timestamp())",
    )
    .bind(id(0x0707000000000000000000000000000a))
    .bind(source_id)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("source Question is available for a new fork");
    transaction
        .commit()
        .await
        .expect("commit M07 parent fixture");
}
