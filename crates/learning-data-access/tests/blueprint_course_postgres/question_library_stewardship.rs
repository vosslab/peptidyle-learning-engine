//! Connected Watch delivery from ordinary Question and Pool actions.

use learning_data_access::postgres::{
    PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore,
    PostgresLibraryWatchNotificationStore, PostgresQuestionForkStore,
    PostgresQuestionPoolCreationStore, PostgresQuestionPoolForkStore,
    PostgresQuestionPoolLibraryStore, PostgresQuestionPoolStewardshipStore,
    PostgresQuestionWatchStore,
};
use learning_data_access::{
    AuthoringDraftStore, CreateQuestionPoolInput, ForkPublishedQuestionInput, LibraryWatchActivity,
    LibraryWatchInboxStore, LibraryWatchTargetKind, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore, QuestionForkStore, QuestionPoolCreationStore,
    QuestionPoolForkStore, QuestionPoolLibraryStore, QuestionPoolStewardshipStore,
    QuestionWatchStore,
};
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionAuthor, QuestionAuthorDisplayName,
    QuestionAuthorship, QuestionLicense, QuestionPoolEditNumber, QuestionRevisionNumber,
    QuestionRevisionReason, SaveQuestionPoolMembersRequest,
};

use super::*;

fn draft_source_record(
    workspace: question_model::WorkspaceId,
    object_id: Uuid,
    sha256: Sha256Checksum,
    size_bytes: u64,
    media_type: String,
) -> ObjectRecord {
    ObjectRecord {
        id: ObjectId::from_uuid(object_id),
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::AuthoringContent,
        address: ObjectAddress::WorkspaceQuestionSource {
            workspace_id: workspace,
            object_id: ObjectId::from_uuid(object_id),
        },
        sha256,
        size_bytes,
        media_type,
        published_question_revision_tuple: None,
        created_at: question_model::Timestamp::from_unix_millis(1_780_000_000_000),
    }
}

fn published_source_record(
    tuple: question_model::PublishedQuestionRevisionTuple,
    object_id: Uuid,
    sha256: Sha256Checksum,
    size_bytes: u64,
    media_type: String,
) -> ObjectRecord {
    ObjectRecord {
        id: ObjectId::from_uuid(object_id),
        storage_area: ObjectStorageArea::PrivateContent,
        data_class: ObjectDataClass::QuestionSource,
        address: ObjectAddress::QuestionSource {
            published_question_revision_tuple: tuple.clone(),
            object_id: ObjectId::from_uuid(object_id),
        },
        sha256,
        size_bytes,
        media_type,
        published_question_revision_tuple: Some(tuple),
        created_at: question_model::Timestamp::from_unix_millis(1_780_000_001_000),
    }
}

async fn source_record_evidence(
    admin: &sqlx::PgPool,
    source_revision_tuple: &question_model::PublishedQuestionRevisionTuple,
) -> (Sha256Checksum, u64, String) {
    let mut transaction = admin.begin().await.expect("source record read transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("source record fixture role");
    let row = sqlx::query(
        "SELECT record.sha256, record.size_bytes, record.media_type \
           FROM ple_private.question_revision_source_binding AS binding \
           JOIN ple_private.object_record AS record \
             ON record.object_record_id = binding.source_object_record_id \
          WHERE binding.published_question_id = $1 AND binding.revision_number = $2",
    )
    .bind(source_revision_tuple.published_question_id.as_str())
    .bind(i32::try_from(source_revision_tuple.revision_number.get()).expect("Revision Number"))
    .fetch_one(&mut *transaction)
    .await
    .expect("source Question Object Record");
    transaction
        .commit()
        .await
        .expect("finish source record read transaction");

    let checksum: Vec<u8> = row.try_get("sha256").expect("source checksum");
    let checksum: [u8; 32] = checksum.try_into().expect("32-byte source checksum");
    let size_bytes = u64::try_from(row.try_get::<i64, _>("size_bytes").expect("source size"))
        .expect("nonnegative source size");
    let media_type = row.try_get("media_type").expect("source media type");
    (Sha256Checksum::from_bytes(checksum), size_bytes, media_type)
}

async fn publish_question_fork(
    admin: &sqlx::postgres::PgPool,
    application: &sqlx::PgPool,
    fixture_id_base: u128,
) -> question_model::PublishedQuestionId {
    let source_question_id: question_model::PublishedQuestionId =
        QUESTION.parse().expect("fixture Question ID");
    let source_revision_tuple = question_model::PublishedQuestionRevisionTuple {
        published_question_id: source_question_id.clone(),
        revision_number: QuestionRevisionNumber::new(1).expect("source Revision 1"),
    };
    let authoring = PostgresAuthoringDraftStore::new(application.clone());
    let workspace = authoring
        .ensure_own_authoring_workspace(token(), id(fixture_id_base + 1))
        .await
        .expect("Instructor authoring workspace");
    let (source_sha256, source_size_bytes, source_media_type) =
        source_record_evidence(admin, &source_revision_tuple).await;
    let forked = PostgresQuestionForkStore::new(application.clone())
        .fork_published_question_to_draft(
            token(),
            ForkPublishedQuestionInput {
                source_published_question_revision_tuple: source_revision_tuple,
                workspace,
                proposed_draft_question_id: id(fixture_id_base + 2),
                target_source_record: draft_source_record(
                    workspace,
                    id(fixture_id_base + 3),
                    source_sha256,
                    source_size_bytes,
                    source_media_type.clone(),
                ),
                hotspot_question_image: None,
                idempotency_key: id(fixture_id_base + 4),
            },
        )
        .await
        .expect("create ordinary Question fork Draft");
    let fork_draft = authoring
        .load_authoring_draft(token(), forked.draft_question_uuid)
        .await
        .expect("load Question fork Draft");
    let mut reservation_read = admin
        .begin()
        .await
        .expect("reserved Question ID read transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *reservation_read)
        .await
        .expect("reserved Question ID fixture role");
    let fork_question_id: question_model::PublishedQuestionId = sqlx::query_scalar::<_, String>(
        "SELECT public_id_reservation_id FROM ple_private.draft_question \
          WHERE draft_question_id = $1",
    )
    .bind(forked.draft_question_uuid.as_uuid())
    .fetch_one(&mut *reservation_read)
    .await
    .expect("fork Draft reserves its Question ID")
    .parse()
    .expect("reserved fork Question ID");
    reservation_read
        .commit()
        .await
        .expect("finish reserved Question ID read transaction");

    let classification = fork_draft
        .classification
        .expect("fork Draft retains its source Question classification");
    let discipline_uuid = classification
        .discipline_uuid
        .expect("source Question discipline classification");
    let subject_uuid = classification
        .subject_uuid
        .expect("source Question subject classification");
    let fork_revision_tuple = question_model::PublishedQuestionRevisionTuple {
        published_question_id: fork_question_id.clone(),
        revision_number: QuestionRevisionNumber::new(1).expect("fork Revision 1"),
    };
    PostgresDraftQuestionSourceBindingStore::new(application.clone())
        .publish_new_question_lineage(
            token(),
            NewQuestionLineagePublicationInput {
                hotspot_question_image: None,
                draft_question_uuid: fork_draft.draft_question_uuid,
                expected_draft_question_edit_number: fork_draft.edit_number,
                workspace,
                question_id: fork_question_id.clone(),
                question_source_object_record: published_source_record(
                    fork_revision_tuple,
                    id(fixture_id_base + 5),
                    source_sha256,
                    source_size_bytes,
                    source_media_type,
                ),
                question_authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                    display_name: QuestionAuthorDisplayName::new(
                        "Library fixture Instructor".to_owned(),
                    )
                    .expect("authored display name"),
                    account_id: None,
                }])
                .expect("fork authorship"),
                initial_shared_tags: Vec::new(),
                discipline_uuid,
                subject_uuid,
                topic_uuid: None,
                subtopic_uuid: None,
                question_license: QuestionLicense::CcBy4_0,
                question_revision_reason: QuestionRevisionReason::new(
                    "Publish a fork of the watched Question".to_owned(),
                )
                .expect("fork publication reason"),
                question_ownership_event_id: id(fixture_id_base + 6),
                question_publication_event_id: id(fixture_id_base + 7),
                question_availability_event_id: id(fixture_id_base + 8),
            },
        )
        .await
        .expect("publish the Question fork");
    fork_question_id
}

async fn create_watchable_pool(
    application: &sqlx::PgPool,
    pool_identifier: &str,
) -> question_model::QuestionPoolId {
    let question_id: question_model::PublishedQuestionId =
        QUESTION.parse().expect("fixture Question ID");
    let question_revision_tuple = question_model::PublishedQuestionRevisionTuple {
        published_question_id: question_id,
        revision_number: QuestionRevisionNumber::new(1).expect("source Revision 1"),
    };
    let question_pool_id = question_model::QuestionPoolId::from_random_identifier(pool_identifier)
        .expect("Question Pool ID");
    let created = PostgresQuestionPoolCreationStore::new(application.clone())
        .create_question_pool(
            token(),
            CreateQuestionPoolInput {
                title: "M14 Watch acceptance Pool".to_owned(),
                description: "Pool for connected Watch acceptance".to_owned(),
                question_pool_id: question_pool_id.clone(),
                members: vec![question_revision_tuple],
                tags: Vec::new(),
            },
        )
        .await
        .expect("create ordinary Question Pool");
    assert_eq!(created.question_pool_id, question_pool_id);
    assert_eq!(created.edit_number, 1);
    question_pool_id
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn publishing_a_question_fork_reaches_its_source_watcher() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let source_question_id: question_model::PublishedQuestionId =
        QUESTION.parse().expect("fixture Question ID");
    let watches = PostgresQuestionWatchStore::new(application.clone());
    let watch = watches
        .set_current_question_watch(token(), &source_question_id, true)
        .await
        .expect("Question Watch");
    assert!(watch.watching);

    let fork_question_id = publish_question_fork(&admin, &application, 0x1400).await;

    let inbox = PostgresLibraryWatchNotificationStore::new(application.clone());
    let notifications = inbox
        .library_watch_notifications(token(), 20)
        .await
        .expect("Watch inbox");
    assert!(notifications.iter().any(|notification| {
        notification.target_public_id.as_str() == source_question_id.as_str()
            && matches!(
                &notification.activity,
                LibraryWatchActivity::QuestionFork {
                    source_question_revision_number,
                    forked_public_id,
                } if source_question_revision_number.get() == 1
                    && forked_public_id.as_str() == fork_question_id.as_str()
            )
    }));
    application.close().await;
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn changing_question_pool_members_reaches_its_watcher() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    let application = lazy_pool(&std::env::var("DATABASE_URL").expect("application database URL"))
        .expect("application pool");
    let question_pool_id = create_watchable_pool(&application, "M14P001").await;
    let pool_watches = PostgresQuestionPoolStewardshipStore::new(application.clone());
    let watch = pool_watches
        .set_current_question_pool_watch_projection(token(), &question_pool_id, true)
        .await
        .expect("Question Pool Watch");
    assert!(watch.watching);
    let added_question_id = publish_question_fork(&admin, &application, 0x1410).await;

    let saved = PostgresQuestionPoolLibraryStore::new(application.clone())
        .save_question_pool_members(
            token(),
            SaveQuestionPoolMembersRequest {
                question_pool_id: question_pool_id.clone(),
                expected_question_pool_edit_number: QuestionPoolEditNumber::new(1)
                    .expect("initial Pool Edit Number"),
                members: vec![PublishedQuestionRevisionTuple {
                    published_question_id: added_question_id,
                    revision_number: QuestionRevisionNumber::new(1)
                        .expect("added Question Revision 1"),
                }],
            },
        )
        .await
        .expect("save changed exact Pool member tuple set through the application Store");
    assert_eq!(saved.question_pool_edit_number.get(), 2);

    let notifications = PostgresLibraryWatchNotificationStore::new(application.clone())
        .library_watch_notifications(token(), 20)
        .await
        .expect("Watch inbox");
    assert!(notifications.iter().any(|notification| {
        notification.target_kind == LibraryWatchTargetKind::QuestionPool
            && notification.target_public_id.as_str() == question_pool_id.as_str()
            && matches!(
                &notification.activity,
                LibraryWatchActivity::MembersChanged {
                    question_pool_edit_number,
                } if question_pool_edit_number.get() == 2
            )
    }));
    application.close().await;
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn forking_a_question_pool_reaches_its_source_watcher() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    let application = lazy_pool(&std::env::var("DATABASE_URL").expect("application database URL"))
        .expect("application pool");
    let source_pool_id = create_watchable_pool(&application, "M14P002").await;
    let pool_watches = PostgresQuestionPoolStewardshipStore::new(application.clone());
    let watch = pool_watches
        .set_current_question_pool_watch_projection(token(), &source_pool_id, true)
        .await
        .expect("source Question Pool Watch");
    assert!(watch.watching);
    let forked_pool_id = question_model::QuestionPoolId::from_random_identifier("M14P003")
        .expect("forked Question Pool ID");

    let forked = PostgresQuestionPoolForkStore::new(application.clone())
        .fork_question_pool(token(), forked_pool_id.clone(), source_pool_id.clone())
        .await
        .expect("create ordinary source-linked Question Pool fork");
    assert_eq!(forked.question_pool_id, forked_pool_id);
    assert_eq!(forked.edit_number, 1);

    let expected_forked_id: question_model::LibraryObjectId = forked_pool_id
        .as_str()
        .parse()
        .expect("Pool fork ID uses the public Library ID shape");
    let notifications = PostgresLibraryWatchNotificationStore::new(application.clone())
        .library_watch_notifications(token(), 20)
        .await
        .expect("Watch inbox");
    assert!(notifications.iter().any(|notification| {
        notification.target_kind == LibraryWatchTargetKind::QuestionPool
            && notification.target_public_id.as_str() == source_pool_id.as_str()
            && matches!(
                &notification.activity,
                LibraryWatchActivity::QuestionPoolFork {
                    source_question_pool_edit_number,
                    forked_public_id,
                } if source_question_pool_edit_number.get() == 1
                    && forked_public_id == &expected_forked_id
            )
    }));
    application.close().await;
    admin.close().await;
}
