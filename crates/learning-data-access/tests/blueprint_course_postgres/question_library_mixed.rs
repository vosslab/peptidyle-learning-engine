//! Connected matrix for mixed Published Question and Pool Library search.

use super::*;
use std::collections::HashSet;

struct MixedFixture {
    pool_id: String,
    second_pool_id: String,
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
         content_discipline_id, content_subject_id, tags, source_question_pool_id, created_at, \
         bloom_cognitive_process, bloom_knowledge_dimension) \
         VALUES ($1, $2, 1, 1, 'multipleChoice', 'ple', 'CC-BY-4.0', $3, \
                 'Pool own tied description', $4, $5, ARRAY['p1 metadata tag'], $6, \
                 '2026-09-25 12:00:00+00'::timestamptz, \
                 $7::ple_data.bloom_cognitive_process, \
                 $8::ple_data.bloom_knowledge_dimension)",
    )
    .bind(pool_id)
    .bind(instructor_account_id())
    .bind(FIXTURE_TITLE)
    .bind(id(0xcc01))
    .bind(id(0xcc02))
    .bind(source_pool_id)
    .bind(question_id.map(|_| "Remember"))
    .bind(question_id.map(|_| "Factual Knowledge"))
    .execute(&mut **transaction)
    .await
    .expect("mixed Pool");
    if let Some(question_id) = question_id {
        sqlx::query(
            "INSERT INTO ple_data.question_pool_member (question_pool_id, \
             published_question_id, question_revision_number, created_at) \
             VALUES ($1, $2, 1, '2026-09-25 12:00:00+00'::timestamptz)",
        )
        .bind(pool_id)
        .bind(question_id)
        .execute(&mut **transaction)
        .await
        .expect("mixed Pool member");
    }
}

async fn insert_mixed_fixture(admin: &sqlx::postgres::PgPool) -> MixedFixture {
    let pool_id = question_model::QuestionPoolId::from_random_identifier("P100001")
        .expect("Pool fixture ID")
        .to_string();
    let second_pool_id = question_model::QuestionPoolId::from_random_identifier("P100002")
        .expect("second Pool fixture ID")
        .to_string();
    let fork_pool_id = question_model::QuestionPoolId::from_random_identifier("P100003")
        .expect("fork Pool fixture ID")
        .to_string();
    let questions = fixture_question_ids();
    let mut transaction = admin.begin().await.expect("mixed Pool transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Pool data owner");
    insert_pool(&mut transaction, &pool_id, None, Some(&questions[0])).await;
    insert_pool(&mut transaction, &second_pool_id, None, Some(&questions[1])).await;
    insert_pool(
        &mut transaction,
        &fork_pool_id,
        Some(&pool_id),
        Some(&questions[2]),
    )
    .await;
    transaction.commit().await.expect("mixed Pool commit");
    MixedFixture {
        pool_id,
        second_pool_id,
        fork_pool_id,
    }
}

fn mixed_request() -> QuestionLibrarySearchRequest {
    QuestionLibrarySearchRequest {
        kind: question_model::LibrarySearchKind::Both,
        questions: question_model::PublishedQuestionFilter::All,
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

pub(super) async fn assert_mixed_search_matrix(
    admin: &sqlx::postgres::PgPool,
    store: &PostgresQuestionLibraryStore,
) {
    let fixture = insert_mixed_fixture(admin).await;
    let question_ids = fixture_question_ids();

    let mixed_ids = collect_ids(store, mixed_request()).await;
    assert!(mixed_ids.contains(&question_ids[0]));
    assert!(mixed_ids.contains(&question_ids[1]));
    assert!(mixed_ids.contains(&question_ids[2]));
    assert!(mixed_ids.contains(&fixture.pool_id));
    assert!(mixed_ids.contains(&fixture.second_pool_id));
    assert!(mixed_ids.contains(&fixture.fork_pool_id));

    let mixed_page = store
        .search_published_question_library_entries(token(), mixed_request())
        .await
        .expect("mixed categories and facets");
    assert_eq!(
        mixed_page.facets.categories,
        question_model::LibrarySearchCategoryCounts {
            questions_in_no_pool: (FIXTURE_ROWS - 3) as u64,
            questions_in_pool: 3,
            pools: 3,
        },
        "category counts apply domain filters before kind, Questions in no Pool, and paging"
    );
    assert_eq!(
        mixed_page.facets.question_types[0].count,
        (FIXTURE_ROWS + 3) as u64,
        "Type facets count both Library Object kinds before cursor paging"
    );

    let no_pool_ids = collect_ids(
        store,
        QuestionLibrarySearchRequest {
            questions: question_model::PublishedQuestionFilter::InNoPool,
            ..mixed_request()
        },
    )
    .await;
    assert!(no_pool_ids.contains(&fixture.pool_id));
    assert!(no_pool_ids.contains(&fixture.second_pool_id));
    assert!(no_pool_ids.contains(&fixture.fork_pool_id));
    assert!(!no_pool_ids.contains(&question_ids[0]));
    assert!(!no_pool_ids.contains(&question_ids[1]));
    assert!(!no_pool_ids.contains(&question_ids[2]));

    let no_pool_page = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                questions: question_model::PublishedQuestionFilter::InNoPool,
                ..mixed_request()
            },
        )
        .await
        .expect("Questions in no Pool page");
    assert_eq!(
        no_pool_page.facets.categories, mixed_page.facets.categories,
        "category counts stay before Questions in no Pool selection"
    );
    assert_eq!(
        no_pool_page.facets.question_types[0].count,
        (FIXTURE_ROWS) as u64,
        "facets count filtered rows including Pools, before cursor paging"
    );

    let pools_only = collect_ids(
        store,
        QuestionLibrarySearchRequest {
            kind: question_model::LibrarySearchKind::Pools,
            ..mixed_request()
        },
    )
    .await;
    let mut expected_pools = vec![
        fixture.pool_id.clone(),
        fixture.second_pool_id.clone(),
        fixture.fork_pool_id.clone(),
    ];
    expected_pools.sort();
    assert_eq!(
        pools_only, expected_pools,
        "same-title Pools retain global ID ordering, including the fork"
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
    assert!(type_text.items.iter().any(|item| matches!(
        item,
        learning_data_access::LibrarySearchEntry::Question { .. }
    )));
    assert!(
        type_text
            .items
            .iter()
            .any(|item| matches!(item, learning_data_access::LibrarySearchEntry::Pool { .. }))
    );

    let negative_author_ids = collect_ids(
        store,
        QuestionLibrarySearchRequest {
            text_terms: vec![QuestionLibraryTextTerm {
                field: QuestionLibraryTextField::Author,
                value: "Nobody in this fixture".to_owned(),
                excluded: true,
            }],
            ..mixed_request()
        },
    )
    .await;
    assert!(negative_author_ids.contains(&fixture.pool_id));
    assert!(negative_author_ids.contains(&fixture.second_pool_id));
    assert!(negative_author_ids.contains(&fixture.fork_pool_id));

    let positive_author_ids = collect_ids(
        store,
        QuestionLibrarySearchRequest {
            text_terms: vec![QuestionLibraryTextTerm {
                field: QuestionLibraryTextField::Author,
                value: "Nobody in this fixture".to_owned(),
                excluded: false,
            }],
            ..mixed_request()
        },
    )
    .await;
    assert!(!positive_author_ids.contains(&fixture.pool_id));
    assert!(!positive_author_ids.contains(&fixture.second_pool_id));
    assert!(!positive_author_ids.contains(&fixture.fork_pool_id));

    let common_backend = collect_ids(
        store,
        QuestionLibrarySearchRequest {
            backends: QuestionLibraryBackendRestriction::Only(vec![
                question_model::QuestionBackend::Ple,
            ]),
            ..mixed_request()
        },
    )
    .await;
    assert!(common_backend.contains(&question_ids[0]));
    assert!(common_backend.contains(&fixture.pool_id));
    assert!(common_backend.contains(&fixture.fork_pool_id));

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
}
