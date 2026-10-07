//! Connected PostgreSQL oracle for Question fork identity and parent snapshots.

use super::*;
#[path = "question_fork_parents_support.rs"]
mod support;
use support::*;

use learning_data_access::postgres::{
    PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore,
    PostgresQuestionForkStore, PostgresQuestionLibraryStore, PostgresQuestionMetadataStore,
};
use learning_data_access::{
    AuthoringDraftStore, ForkPublishedQuestionInput, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore, QuestionForkStore, QuestionMetadataStore,
};
use question_model::{PublishedQuestionRevisionTuple, QuestionRevisionReason};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_fork_gets_new_identity_and_keeps_its_exact_parent_snapshot() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("fixture pool");
    blueprint_course_postgres_support::seed_if_needed(&admin).await;
    let application = lazy_pool(&std::env::var("DATABASE_URL").expect("application URL"))
        .expect("application pool");

    let source_id = question_model::PublishedQuestionId::from_random_identifier("M07SRC1")
        .expect("source Question ID");
    let source_tuple = PublishedQuestionRevisionTuple {
        published_question_id: source_id.clone(),
        revision_number: question_model::QuestionRevisionNumber::new(1).expect("Revision 1"),
    };
    seed_source_question(
        &admin,
        &source_tuple,
        id(0x07070000000000000000000000000010),
        SOURCE_BYTES,
    )
    .await;

    let authoring = PostgresAuthoringDraftStore::new(application.clone());
    let workspace = authoring
        .ensure_own_authoring_workspace(token(), id(0x0707000000000000000000000000000c))
        .await
        .expect("Instructor Authoring Workspace");
    let forks = PostgresQuestionForkStore::new(application.clone());
    let fork_request = ForkPublishedQuestionInput {
        source_published_question_revision_tuple: source_tuple.clone(),
        workspace,
        proposed_draft_question_id: id(DRAFT_ID),
        target_source_record: workspace_source_record(
            workspace,
            id(0x07070000000000000000000000000011),
            SOURCE_BYTES,
        ),
        hotspot_question_image: None,
        idempotency_key: id(FORK_REQUEST_KEY),
    };
    let fork = forks
        .fork_published_question_to_draft(token(), fork_request.clone())
        .await
        .expect("fork exact source Revision");
    assert!(fork.created_new);
    let retry = forks
        .fork_published_question_to_draft(token(), fork_request)
        .await
        .expect("return the existing Draft for the same actor and request key");
    assert!(!retry.created_new);
    assert_eq!(retry.draft_question_uuid, fork.draft_question_uuid);

    let mut reservation_read = admin
        .begin()
        .await
        .expect("reserved Question ID read transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *reservation_read)
        .await
        .expect("reserved Question ID fixture role");
    let child_id: question_model::PublishedQuestionId = sqlx::query_scalar::<_, String>(
        "SELECT public_id_reservation_id FROM ple_private.draft_question \
          WHERE draft_question_id = $1",
    )
    .bind(fork.draft_question_uuid.as_uuid())
    .fetch_one(&mut *reservation_read)
    .await
    .expect("fork reserves the child Question ID")
    .parse()
    .expect("reserved Question ID");
    reservation_read
        .commit()
        .await
        .expect("finish reserved Question ID read transaction");
    assert_ne!(child_id, source_id);

    let drafts = PostgresAuthoringDraftStore::new(application.clone());
    let draft = drafts
        .load_authoring_draft(token(), fork.draft_question_uuid)
        .await
        .expect("read fork Draft");
    assert_eq!(
        draft.parent_published_question_revision_tuple,
        Some(source_tuple.clone())
    );
    assert_eq!(draft.metadata.question_title, "M07 source title");

    PostgresQuestionMetadataStore::new(application.clone())
        .save_question_metadata(
            token(),
            metadata_replacement(
                source_tuple.clone(),
                1,
                "Parent changed after fork",
                "The already-created fork keeps its copied metadata.",
                &["m07-parent-edit"],
                None,
                None,
            ),
        )
        .await
        .expect("edit parent after fork creation");
    let unchanged_draft = drafts
        .load_authoring_draft(token(), fork.draft_question_uuid)
        .await
        .expect("read fork snapshot after parent edit");
    assert_eq!(unchanged_draft.metadata.question_title, "M07 source title");

    let child_tuple = PublishedQuestionRevisionTuple {
        published_question_id: child_id.clone(),
        revision_number: question_model::QuestionRevisionNumber::new(1).expect("child Revision 1"),
    };
    let published = PostgresDraftQuestionSourceBindingStore::new(application.clone())
        .publish_new_question_lineage(
            token(),
            NewQuestionLineagePublicationInput {
                hotspot_question_image: None,
                draft_question_uuid: fork.draft_question_uuid,
                expected_draft_question_edit_number: unchanged_draft.edit_number,
                workspace,
                question_id: child_id.clone(),
                question_source_object_record: published_source_record(
                    child_tuple.clone(),
                    id(0x07070000000000000000000000000012),
                    SOURCE_BYTES,
                ),
                question_authorship: source_authorship(),
                initial_shared_tags: unchanged_draft.metadata.tags.clone(),
                discipline_uuid: id(SOURCE_DISCIPLINE_ID),
                subject_uuid: id(SOURCE_SUBJECT_ID),
                topic_uuid: None,
                subtopic_uuid: None,
                question_license: question_model::QuestionLicense::CcBy4_0,
                question_revision_reason: QuestionRevisionReason::new(
                    "Publish the copied Question fork".to_owned(),
                )
                .expect("publication reason"),
                question_ownership_event_id: id(0x0707000000000000000000000000000d),
                question_publication_event_id: id(0x0707000000000000000000000000000e),
                question_availability_event_id: id(0x0707000000000000000000000000000f),
            },
        )
        .await
        .expect("publish fork as a new Question");
    assert_eq!(published, child_tuple);

    let library = PostgresQuestionLibraryStore::new(application.clone());
    let child = library
        .load_published_question_revision_library_entry(token(), &child_tuple)
        .await
        .expect("read exact child Revision");
    let parent = library
        .load_published_question_revision_library_entry(token(), &source_tuple)
        .await
        .expect("read exact parent Revision");
    assert_eq!(
        child
            .published_question_revision_tuple
            .published_question_id,
        child_id
    );
    assert_eq!(
        child.parent_published_question_revision_tuple,
        Some(source_tuple)
    );
    assert_eq!(child.question_title, "M07 source title");
    assert_eq!(parent.question_title, "Parent changed after fork");

    application.close().await;
    admin.close().await;
}
