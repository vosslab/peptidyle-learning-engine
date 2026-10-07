//! Connected PostgreSQL oracle for a saved WebWork Type on a same-ID successor.

use super::*;
use learning_data_access::postgres::{
    PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore,
};
use learning_data_access::{
    AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionUuid,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    NewQuestionLineagePublicationInput, NewQuestionLineagePublicationStore,
    SaveAuthoringDraftInput,
};
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionAuthor,
    QuestionAuthorDisplayName, QuestionAuthorship, QuestionFormat, QuestionLicense,
    QuestionMetadata, QuestionRevisionNumber, QuestionRevisionReason, QuestionType, Timestamp,
    WorkspaceId,
};
use sqlx::Row;
use uuid::Uuid;

const WEBWORK_PG_PATH: &str = "Library/Genetics/m16_manual_type.pg";

fn source_record(workspace: WorkspaceId, object_uuid: Uuid, bytes: &[u8]) -> ObjectRecord {
    let object_id = ObjectId::from_uuid(object_uuid);
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
        media_type: "text/x-wework-pg".to_owned(),
        published_question_revision_tuple: None,
        created_at: Timestamp::from_unix_millis(1_780_000_000_000),
    }
}

fn published_source_record(
    tuple: PublishedQuestionRevisionTuple,
    object_uuid: Uuid,
    bytes: &[u8],
) -> ObjectRecord {
    let object_id = ObjectId::from_uuid(object_uuid);
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
        media_type: "text/x-wework-pg".to_owned(),
        published_question_revision_tuple: Some(tuple),
        created_at: Timestamp::from_unix_millis(1_780_000_001_000),
    }
}

async fn create_draft(
    authoring: &PostgresAuthoringDraftStore,
    workspace: WorkspaceId,
    draft_uuid: Uuid,
    source_object_uuid: Uuid,
    bytes: &[u8],
    question_type: QuestionType,
) -> learning_data_access::AuthoringDraft {
    authoring
        .create_authoring_draft(
            token(),
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(draft_uuid),
                source_record: source_record(workspace, source_object_uuid, bytes),
                question_format: QuestionFormat::WebworkPg,
                webwork_pg_path: Some(WEBWORK_PG_PATH.to_owned()),
                question_type: Some(question_type),
                metadata: QuestionMetadata {
                    question_title: "Manual WebWork Type".to_owned(),
                    question_description: "Saved Type must reach the successor.".to_owned(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::CcBy4_0),
                    question_citation: None,
                    language: Some("en".to_owned()),
                },
            },
        )
        .await
        .expect("create WebWork Draft")
}

async fn revision_state(tuple: &PublishedQuestionRevisionTuple) -> (String, Option<i32>, String) {
    let mut inspection = adoption_inspection_connection().await;
    let row = sqlx::query(
        "SELECT metadata.question_type::text, acceptance.parent_revision_number, \
                owner.owner_account_id::text \
           FROM ple_data.question_revision AS revision \
           JOIN ple_data.question_revision_metadata AS metadata USING \
                (published_question_id, revision_number) \
           JOIN ple_data.question_revision_acceptance AS acceptance USING \
                (published_question_id, revision_number) \
           JOIN ple_data.question_current_owner AS owner USING (published_question_id) \
          WHERE revision.published_question_id = $1 AND revision.revision_number = $2",
    )
    .bind(tuple.published_question_id.as_str())
    .bind(tuple.revision_number.get() as i32)
    .fetch_one(&mut inspection)
    .await
    .expect("exact WebWork Revision Type, parent, and owner");
    let state = (
        row.try_get("question_type").expect("Question Type"),
        row.try_get("parent_revision_number")
            .expect("parent Revision"),
        row.try_get("owner_account_id").expect("Question owner"),
    );
    inspection
        .close()
        .await
        .expect("inspection connection close");
    state
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn saved_manual_webwork_type_reaches_same_id_successor() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    seed_if_needed(&admin).await;
    let application_url = std::env::var("DATABASE_URL").expect("API application database URL");
    let application = lazy_pool(&application_url).expect("API application pool");
    let authoring = PostgresAuthoringDraftStore::new(application.clone());
    let publication = PostgresDraftQuestionSourceBindingStore::new(application);
    let workspace = authoring
        .ensure_own_authoring_workspace(token(), id(0xb10c))
        .await
        .expect("Instructor Authoring Workspace");
    let question_id =
        PublishedQuestionId::from_random_identifier("QM16MAN").expect("isolated Question ID");

    let parent_bytes = b"initial manual WebWork Type";
    let parent_draft = create_draft(
        &authoring,
        workspace,
        id(0xb10c_0001),
        id(0xb10c_0002),
        parent_bytes,
        QuestionType::MultipleChoice,
    )
    .await;
    let revision_one = PublishedQuestionRevisionTuple {
        published_question_id: question_id.clone(),
        revision_number: QuestionRevisionNumber::new(1).expect("Revision 1"),
    };
    let published = publication
        .publish_new_question_lineage(
            token(),
            NewQuestionLineagePublicationInput {
                hotspot_question_image: None,
                draft_question_uuid: parent_draft.draft_question_uuid,
                expected_draft_question_edit_number: parent_draft.edit_number,
                workspace,
                question_id: question_id.clone(),
                question_source_object_record: published_source_record(
                    revision_one.clone(),
                    id(0xb10c_0003),
                    parent_bytes,
                ),
                question_authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                    display_name: QuestionAuthorDisplayName::new("Fixture Instructor".to_owned())
                        .expect("author display name"),
                    account_id: None,
                }])
                .expect("authorship"),
                initial_shared_tags: Vec::new(),
                discipline_uuid: id(0xcc01),
                subject_uuid: id(0xcc02),
                topic_uuid: None,
                subtopic_uuid: None,
                question_license: QuestionLicense::CcBy4_0,
                question_revision_reason: QuestionRevisionReason::new(
                    "Initial WebWork Revision".to_owned(),
                )
                .expect("publication reason"),
                question_ownership_event_id: id(0xb10c_0008),
                question_publication_event_id: id(0xb10c_0009),
                question_availability_event_id: id(0xb10c_000a),
            },
        )
        .await
        .expect("publish initial WebWork Revision");
    assert_eq!(published, revision_one);
    let owner_before = revision_state(&revision_one).await.2;

    let correction_bytes = b"manual WebWork Type changed to Hotspot";
    let correction = create_draft(
        &authoring,
        workspace,
        id(0xb10c_0004),
        id(0xb10c_0005),
        correction_bytes,
        QuestionType::MultipleChoice,
    )
    .await;
    let correction = authoring
        .save_authoring_draft(
            token(),
            SaveAuthoringDraftInput {
                draft_question_uuid: correction.draft_question_uuid,
                expected_edit_number: correction.edit_number,
                source_record: source_record(workspace, id(0xb10c_0006), correction_bytes),
                question_type: Some(QuestionType::Hotspot),
            },
        )
        .await
        .expect("save manual WebWork Type correction");
    let revision_two = PublishedQuestionRevisionTuple {
        published_question_id: question_id.clone(),
        revision_number: QuestionRevisionNumber::new(2).expect("Revision 2"),
    };
    let published = publication
        .publish_question_revision(
            token(),
            ExistingQuestionRevisionPublicationInput {
                hotspot_question_image: None,
                draft_question_uuid: correction.draft_question_uuid,
                expected_draft_question_edit_number: correction.edit_number,
                workspace,
                parent_published_question_revision_tuple: revision_one.clone(),
                question_source_object_record: published_source_record(
                    revision_two.clone(),
                    id(0xb10c_0007),
                    correction_bytes,
                ),
                question_revision_reason: QuestionRevisionReason::new(
                    "Correct manual WebWork Type".to_owned(),
                )
                .expect("correction reason"),
                question_publication_event_id: id(0xb10c_000b),
            },
        )
        .await
        .expect("publish same-ID successor");

    assert_eq!(published, revision_two);
    let (question_type, parent_revision, owner_after) = revision_state(&revision_two).await;
    assert_eq!(question_type, "hotspot");
    assert_eq!(parent_revision, Some(1));
    assert_eq!(owner_after, owner_before);
    assert_eq!(revision_state(&revision_one).await.0, "multipleChoice");
}
