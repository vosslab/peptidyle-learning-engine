//! Connected matrix for mixed Published Question and Pool Library search.

use super::*;
use std::collections::HashSet;

struct MixedFixture {
    pool_id: String,
    fork_pool_id: String,
}

async fn insert_pool(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pool_id: &str,
    source_pool_id: Option<&str>,
    question_id: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO ple_data.question_pool (question_pool_id, owner_account_id, question_pool_edit_number, \
         question_pool_metadata_edit_number, question_type, backend, license, title, description, \
         content_discipline_id, content_subject_id, tags, source_question_pool_id, \
         interchangeability_attested_by_account_id, interchangeability_attested_at, created_at) \
         VALUES ($1, $2, 1, 1, 'multipleChoice', 'ple', 'CC-BY-4.0', $3, \
                 'Pool own tied description', $4, $5, ARRAY['p1 metadata tag'], $6, $2, \
                 '2026-09-25 12:00:00+00'::timestamptz, '2026-09-25 12:00:00+00'::timestamptz)",
    )
    .bind(pool_id)
    .bind(instructor_account_id())
    .bind(FIXTURE_TITLE)
    .bind(id(0xcc01))
    .bind(id(0xcc02))
    .bind(source_pool_id)
    .execute(&mut **transaction)
    .await
    .expect("mixed Pool");
    if let Some(question_id) = question_id {
        sqlx::query(
            "INSERT INTO ple_data.question_pool_member (question_pool_id, member_position, \
             published_question_id, question_revision_number, created_at) \
             VALUES ($1, 1, $2, 1, '2026-09-25 12:00:00+00'::timestamptz)",
        )
        .bind(pool_id)
        .bind(question_id)
        .execute(&mut **transaction)
        .await
        .expect("mixed Pool member");
        sqlx::query(
            "INSERT INTO ple_data.question_pool_bloom (question_pool_id, cognitive_process, \
             knowledge_dimension, classification_edit_number) \
             VALUES ($1, 'Remember', 'Factual Knowledge', 1)",
        )
        .bind(pool_id)
        .execute(&mut **transaction)
        .await
        .expect("mixed Pool Bloom");
    }
}

async fn insert_mixed_fixture(
    admin: &sqlx::postgres::PgPool,
    application: &sqlx::postgres::PgPool,
) -> MixedFixture {
    let pool_id = question_model::QuestionPoolId::from_random_identifier("P100001")
        .expect("Pool fixture ID")
        .to_string();
    let fork_pool_id = question_model::QuestionPoolId::from_random_identifier("P100002")
        .expect("fork Pool fixture ID")
        .to_string();
    let questions = fixture_question_ids();
    let mut transaction = admin.begin().await.expect("mixed Pool transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Pool data owner");
    insert_pool(&mut transaction, &pool_id, None, Some(&questions[0])).await;
    transaction.commit().await.expect("mixed Pool commit");
    insert_assessment_pool_fork(application, &pool_id, &fork_pool_id).await;
    let mut transaction = admin.begin().await.expect("fork sort-time transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("fork sort-time owner");
    sqlx::query(
        "UPDATE ple_data.question_pool_member SET published_question_id = $2 \
         WHERE question_pool_id = $1",
    )
    .bind(&fork_pool_id)
    .bind(&questions[1])
    .execute(&mut *transaction)
    .await
    .expect("Question whose only membership is the Assessment fork");
    transaction.commit().await.expect("fork sort-time commit");
    MixedFixture {
        pool_id,
        fork_pool_id,
    }
}

async fn insert_assessment_pool_fork(
    application: &sqlx::postgres::PgPool,
    source_pool_id: &str,
    fork_pool_id: &str,
) {
    let mut transaction = application
        .begin()
        .await
        .expect("Assessment Pool fixture transaction");
    authenticate_application_transaction(&mut transaction).await;
    let course_id = "CI0000000Y";
    let course_id = sqlx::query_scalar::<_, String>(
        "SELECT course_instance_id FROM ple_api.create_course_instance(         $1, $2, $3, $4, 'empty', NULL, NULL, 'M10-C', 'M10 Pool Course',          current_date, current_date + 1, NULL, '[]'::jsonb,          '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
    )
    .bind(course_id)
    .bind(id(0xa101))
    .bind(id(0xa102))
    .bind(id(0xa103))
    .fetch_one(&mut *transaction)
    .await
    .expect("Assessment Pool fixture Course");
    transaction.commit().await.expect("Course fixture commit");
    let mut transaction = application
        .begin()
        .await
        .expect("Assessment fixture transaction");
    authenticate_application_transaction(&mut transaction).await;
    let assessment_id = question_model::AssessmentId::from_debug_serial(0xa105).to_string();
    sqlx::query(
        "SELECT assessment_id FROM ple_api.create_assessment($1, $2,          'regular_assignment', 'M10 Pool Assessment', 'Fixture instructions')",
    )
    .bind(&assessment_id)
    .bind(&course_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("Assessment Pool fixture Assessment");
    sqlx::query(
        "SELECT assessment_entry_id FROM ple_api.import_assessment_question_pool_fork_for_ids(         $1, $2, $3, 1, $4, $5, 1, 0, 1, 1::numeric, 'question_pool_order', 'normal')",
    )
    .bind(&course_id)
    .bind(&assessment_id)
    .bind(id(0xa104))
    .bind(fork_pool_id)
    .bind(source_pool_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("actual Assessment-owned Pool fork");
    transaction
        .commit()
        .await
        .expect("Assessment Pool fixture commit");
}

fn mixed_request() -> QuestionLibrarySearchRequest {
    QuestionLibrarySearchRequest {
        kind: question_model::LibrarySearchKind::Both,
        membership: question_model::LibraryQuestionMembership::All,
        text_terms: vec![QuestionLibraryTextTerm {
            field: QuestionLibraryTextField::Any,
            value: "p1 tied".to_owned(),
            excluded: false,
        }],
        author_names: Vec::new(),
        authored_by_current_account: false,
        tags: Vec::new(),
        page_size: 10,
        ..request()
    }
}

async fn collect_ids(
    store: &PostgresQuestionLibraryStore,
    mut request: QuestionLibrarySearchRequest,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for _ in 0..20 {
        let page = store
            .search_published_question_library_entries(token(), request.clone())
            .await
            .expect("mixed Library page");
        for item in &page.items {
            assert!(
                seen.insert(item.public_id().to_owned()),
                "mixed cursor repeated {}",
                item.public_id()
            );
            result.push(item.public_id().to_owned());
        }
        let Some(after) = page.next_position else {
            return result;
        };
        request.after = Some(after);
    }
    panic!("mixed Library cursor did not exhaust within 20 pages");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_library_mixed_matrix_in_postgresql() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    insert_question_library_rows(&admin).await;
    let application = lazy_pool(&std::env::var("DATABASE_URL").expect("application database URL"))
        .expect("application pool");
    let fixture = insert_mixed_fixture(&admin, &application).await;
    let store = PostgresQuestionLibraryStore::new(application.clone());
    let question_ids = fixture_question_ids();

    let mixed_ids = collect_ids(&store, mixed_request()).await;
    assert!(mixed_ids.contains(&question_ids[0]));
    assert!(mixed_ids.contains(&fixture.pool_id));
    assert!(mixed_ids.contains(&fixture.fork_pool_id));

    let no_pool_ids = collect_ids(
        &store,
        QuestionLibrarySearchRequest {
            membership: question_model::LibraryQuestionMembership::NoPool,
            ..mixed_request()
        },
    )
    .await;
    assert!(no_pool_ids.contains(&fixture.pool_id));
    assert!(!no_pool_ids.contains(&question_ids[0]));

    let pools_only = collect_ids(
        &store,
        QuestionLibrarySearchRequest {
            kind: question_model::LibrarySearchKind::Pools,
            ..mixed_request()
        },
    )
    .await;
    assert_eq!(
        pools_only,
        vec![fixture.pool_id.clone(), fixture.fork_pool_id.clone()]
    );

    let structured_type = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: Vec::new(),
                question_types: vec![question_model::QuestionType::MultipleChoice],
                ..mixed_request()
            },
        )
        .await
        .expect("structured Question Type search");
    assert!(
        structured_type
            .items
            .iter()
            .any(|item| item.public_id() == fixture.pool_id)
    );

    let type_text = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::QuestionType,
                    value: "multiple choice".to_owned(),
                    excluded: false,
                }],
                ..mixed_request()
            },
        )
        .await
        .expect("Question Type text search");
    assert!(type_text.items.iter().all(|item| matches!(
        item,
        learning_data_access::LibrarySearchEntry::Question { .. }
    )));

    assert_eq!(
        store
            .load_library_object_kind(
                token(),
                &question_ids[0].parse().expect("Question Library object ID"),
            )
            .await
            .expect("Question kind"),
        question_model::LibraryObjectKind::Question
    );
    assert_eq!(
        store
            .load_library_object_kind(
                token(),
                &fixture.pool_id.parse().expect("Pool Library object ID"),
            )
            .await
            .expect("Pool kind"),
        question_model::LibraryObjectKind::QuestionPool
    );
    assert!(matches!(
        store
            .load_library_object_kind(
                SessionTokenHash::compute(b"mixed Library unauthorized"),
                &question_ids[0].parse().expect("Question Library object ID"),
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    application.close().await;
    admin.close().await;
}
