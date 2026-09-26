//! Connected PostgreSQL oracle for bounded Question Library search.

use super::*;

const FIXTURE_ROWS: usize = 65;
const FIXTURE_TITLE: &str = "P1 tied Question title";

fn request() -> QuestionLibrarySearchRequest {
    QuestionLibrarySearchRequest {
        exact_question_id: None,
        text_terms: vec![QuestionLibraryTextTerm {
            field: QuestionLibraryTextField::Any,
            value: "p1 tied".to_owned(),
            excluded: false,
        }],
        author_names: vec!["p1 fixture author".to_owned()],
        backends: QuestionLibraryBackendRestriction::Any,
        tags: vec!["p1 metadata tag".to_owned()],
        subjects: vec!["blueprint fixture subject".to_owned()],
        topics: Vec::new(),
        discipline_uuid: Some(id(0xcc01)),
        subject_uuid: Some(id(0xcc02)),
        topic_uuid: None,
        subtopic_uuid: None,
        cross_discipline: false,
        bloom_cognitive_process: None,
        bloom_knowledge_dimension: None,
        question_types: vec![question_model::QuestionType::MultipleChoice],
        question_licenses: vec![question_model::QuestionLicense::CcBy4_0],
        used_in_current_account_courses: false,
        authored_by_current_account: true,
        sort: QuestionLibrarySearchSort::TitleAscending,
        page_size: 50,
        after: None,
    }
}

fn fixture_question_ids() -> Vec<String> {
    let mut question_ids = (0..FIXTURE_ROWS)
        .map(|index| {
            question_model::PublishedQuestionId::from_random_identifier(format!("Q{index:06}"))
                .expect("Question Library fixture ID")
                .to_string()
        })
        .collect::<Vec<_>>();
    question_ids.sort();
    question_ids
}

async fn insert_question_library_rows(admin: &sqlx::postgres::PgPool) {
    let question_ids = fixture_question_ids();
    let object_ids = (0..FIXTURE_ROWS)
        .map(|index| id(0xd100 + index as u128))
        .collect::<Vec<_>>();
    let mut transaction = admin
        .begin()
        .await
        .expect("Question Library fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question Library data owner");
    sqlx::query(
        "INSERT INTO ple_data.published_question (published_question_id, created_at) \
         SELECT question_id, clock_timestamp() FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .execute(&mut *transaction)
    .await
    .expect("Question Library lineages");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (published_question_id, revision_number, backend, question_type, published_at) \
         SELECT question_id, 1, 'ple', 'multipleChoice', '2026-09-25 12:00:00+00'::timestamptz \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .execute(&mut *transaction)
    .await
    .expect("Question Library revisions");
    sqlx::query(
        "INSERT INTO ple_data.published_question_metadata (\
             published_question_id, question_title, question_description, language, tags, \
             content_discipline_id, content_subject_id, created_at, updated_at\
         ) SELECT question_id, $2, 'P1 literal %_\\ fixture description', 'en', ARRAY[$3], \
                  $4, $5, statement_timestamp(), statement_timestamp() \
             FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .bind(FIXTURE_TITLE)
    .bind("p1 metadata tag")
    .bind(id(0xcc01))
    .bind(id(0xcc02))
    .execute(&mut *transaction)
    .await
    .expect("Question Library metadata");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question Library private owner");
    sqlx::query(
        "INSERT INTO ple_private.object_record (\
             object_record_id, object_address, object_storage_area, object_data_class, \
             sha256, size_bytes, media_type, created_at, updated_at\
         ) SELECT object_id, jsonb_build_object(\
                    'kind', 'questionSource', \
                    'publishedQuestionRevisionTuple', jsonb_build_object(\
                        'publishedQuestionId', question_id, 'revisionNumber', 1), \
                    'objectId', object_id\
                ), 'private-content', 'question-source', decode(repeat('a1', 32), 'hex'), \
                1, 'application/json', statement_timestamp(), statement_timestamp() \
           FROM unnest($1::text[], $2::uuid[]) row(question_id, object_id)",
    )
    .bind(&question_ids)
    .bind(&object_ids)
    .execute(&mut *transaction)
    .await
    .expect("Question Library source records");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding (\
             published_question_id, revision_number, backend, question_format, \
             source_object_record_id, source_object_checksum, created_at\
         ) SELECT question_id, 1, 'ple', 'pleQuestionJson', object_id, repeat('a1', 32), \
                  '2026-09-25 12:00:00+00'::timestamptz \
           FROM unnest($1::text[], $2::uuid[]) row(question_id, object_id)",
    )
    .bind(&question_ids)
    .bind(&object_ids)
    .execute(&mut *transaction)
    .await
    .expect("Question Library source bindings");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Question Library data owner");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_acceptance (\
             published_question_id, revision_number, parent_revision_number, editor_account_id, \
             accepted_by_account_id, accepted_at, reason_for_edit\
         ) SELECT question_id, 1, NULL, $2, $2, '2026-09-25 12:00:00+00'::timestamptz, 'P1 fixture' \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("Question Library acceptances");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_authorship (\
             published_question_id, revision_number, author_position, author_display_name, author_account_id\
         ) SELECT question_id, 1, 1, 'P1  fixture   Author', $2 \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("Question Library authorship");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_license \
         (published_question_id, revision_number, spdx_expression) \
         SELECT question_id, 1, 'CC-BY-4.0' FROM unnest($1::text[]) question(question_id)",
    )
    .bind(&question_ids)
    .execute(&mut *transaction)
    .await
    .expect("Question Library licenses");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_bloom (\
             published_question_id, revision_number, cognitive_process, knowledge_dimension, \
             classification_edit_number\
         ) VALUES ($1, 1, 'Remember', 'Factual Knowledge', 1), \
                  ($2, 1, 'Create', 'Metacognitive Knowledge', 1)",
    )
    .bind(&question_ids[0])
    .bind(&question_ids[FIXTURE_ROWS - 1])
    .execute(&mut *transaction)
    .await
    .expect("Question Library Bloom classifications");
    transaction
        .commit()
        .await
        .expect("Question Library fixture commit");
}

async fn add_facet_bounds(admin: &sqlx::postgres::PgPool) {
    let question_ids = fixture_question_ids();
    let subject_ids = (0..FIXTURE_ROWS)
        .map(|index| id(0xe100 + index as u128))
        .collect::<Vec<_>>();
    let topic_ids = (0..FIXTURE_ROWS)
        .map(|index| id(0xe200 + index as u128))
        .collect::<Vec<_>>();
    let mut transaction = admin.begin().await.expect("facet bounds transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("facet bounds data owner");
    sqlx::query(
        "INSERT INTO ple_data.content_subject (content_subject_id, name) \
         SELECT subject_id, format('P2 facet Subject %s', ordinal) \
           FROM unnest($1::uuid[]) WITH ORDINALITY AS row(subject_id, ordinal)",
    )
    .bind(&subject_ids)
    .execute(&mut *transaction)
    .await
    .expect("bounded facet Subjects");
    sqlx::query(
        "INSERT INTO ple_data.content_subject_discipline \
         (content_subject_id, content_discipline_id) \
         SELECT subject_id, $2 FROM unnest($1::uuid[]) AS row(subject_id)",
    )
    .bind(&subject_ids)
    .bind(id(0xcc01))
    .execute(&mut *transaction)
    .await
    .expect("bounded facet Subject Disciplines");
    sqlx::query(
        "INSERT INTO ple_data.content_topic (content_topic_id, content_subject_id, name) \
         SELECT topic_id, subject_id, format('P2 facet Topic %s', ordinal) \
           FROM unnest($1::uuid[], $2::uuid[]) WITH ORDINALITY \
                AS row(topic_id, subject_id, ordinal)",
    )
    .bind(&topic_ids)
    .bind(&subject_ids)
    .execute(&mut *transaction)
    .await
    .expect("bounded facet Topics");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_authorship \
         (published_question_id, revision_number, author_position, author_display_name) \
         SELECT question_id, 1, 2, format('P2 facet Author %s', ordinal) \
           FROM unnest($1::text[]) WITH ORDINALITY AS row(question_id, ordinal)",
    )
    .bind(&question_ids)
    .execute(&mut *transaction)
    .await
    .expect("bounded facet authors");
    sqlx::query(
        "WITH fixture AS ( \
             SELECT question_id, subject_id, topic_id, ordinal \
               FROM unnest($1::text[], $2::uuid[], $3::uuid[]) WITH ORDINALITY \
                    AS row(question_id, subject_id, topic_id, ordinal) \
         ) \
         UPDATE ple_data.published_question_metadata AS metadata \
            SET tags = ARRAY['p1 metadata tag', format('P2 facet Tag %s', fixture.ordinal)], \
                content_subject_id = fixture.subject_id, \
                content_topic_id = fixture.topic_id, \
                updated_at = statement_timestamp() \
           FROM fixture \
          WHERE metadata.published_question_id = fixture.question_id",
    )
    .bind(&question_ids)
    .bind(&subject_ids)
    .bind(&topic_ids)
    .execute(&mut *transaction)
    .await
    .expect("bounded facet metadata");
    transaction.commit().await.expect("facet bounds commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn question_library_search_filters_and_pages_in_postgresql() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    insert_question_library_rows(&admin).await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let store = PostgresQuestionLibraryStore::new(application.clone());
    let first = store
        .search_published_question_library_entries(token(), request())
        .await
        .expect("first page");
    assert_eq!(
        first.items.len(),
        50,
        "PostgreSQL returns one bounded first page"
    );
    assert_eq!(
        first
            .facets
            .author_names
            .iter()
            .map(|facet| (facet.author_name.as_str(), facet.count))
            .collect::<Vec<_>>(),
        vec![("P1 fixture Author", FIXTURE_ROWS as u64)],
        "facet labels normalize stored whitespace and deduplicate one Question's values"
    );
    assert_eq!(
        first
            .facets
            .tags
            .iter()
            .map(|facet| (facet.tag.as_str(), facet.count))
            .collect::<Vec<_>>(),
        vec![("p1 metadata tag", FIXTURE_ROWS as u64)],
        "text facets count the complete query, not the returned page"
    );
    assert_eq!(
        first
            .facets
            .bloom_cognitive_processes
            .iter()
            .map(|facet| (facet.cognitive_process, facet.count))
            .collect::<Vec<_>>(),
        vec![
            (question_model::BloomCognitiveProcess::Remember, 1),
            (question_model::BloomCognitiveProcess::Understand, 0),
            (question_model::BloomCognitiveProcess::Apply, 0),
            (question_model::BloomCognitiveProcess::Analyze, 0),
            (question_model::BloomCognitiveProcess::Evaluate, 0),
            (question_model::BloomCognitiveProcess::Create, 1),
        ],
        "Bloom facets include the matching Question beyond the first page and zero-count categories"
    );
    let expected_ids = fixture_question_ids();
    let position = first
        .next_position
        .expect("51st matching row supplies a continuation");
    assert!(matches!(
        position,
        QuestionLibrarySearchCursorPosition::TitleAscending { .. }
    ));
    let second = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                after: Some(position),
                ..request()
            },
        )
        .await
        .expect("tied title continuation");
    assert_eq!(
        second.items.len(),
        FIXTURE_ROWS - 50,
        "tied title/date continuation reaches the remaining matching rows"
    );
    let title_ids = first
        .items
        .iter()
        .chain(&second.items)
        .map(|item| {
            item.published_question_revision_tuple
                .published_question_id
                .as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        title_ids,
        expected_ids.iter().map(String::as_str).collect::<Vec<_>>(),
        "tied-title pages retain every Question exactly once in ID order"
    );
    assert!(second.next_position.is_none());
    let empty = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                after: Some(QuestionLibrarySearchCursorPosition::TitleAscending {
                    title: FIXTURE_TITLE.to_owned(),
                    question_id: expected_ids[FIXTURE_ROWS - 1]
                        .parse()
                        .expect("fixture Question ID"),
                }),
                ..request()
            },
        )
        .await
        .expect("empty continuation page");
    assert!(empty.items.is_empty());
    assert_eq!(
        empty.facets.bloom_cognitive_processes, first.facets.bloom_cognitive_processes,
        "an empty cursor page retains the full-query Bloom facets"
    );
    let newest_first = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                sort: QuestionLibrarySearchSort::PublishedNewest,
                ..request()
            },
        )
        .await
        .expect("tied date first page");
    let newest_position = newest_first
        .next_position
        .expect("51st matching row supplies a date continuation");
    let newest_second = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                sort: QuestionLibrarySearchSort::PublishedNewest,
                after: Some(newest_position),
                ..request()
            },
        )
        .await
        .expect("tied date continuation");
    assert_eq!(
        newest_second.items.len(),
        FIXTURE_ROWS - 50,
        "tied date reaches the remaining matching rows"
    );
    assert!(newest_second.next_position.is_none());
    let newest_ids = newest_first
        .items
        .iter()
        .chain(&newest_second.items)
        .map(|item| {
            item.published_question_revision_tuple
                .published_question_id
                .as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        newest_ids,
        expected_ids.iter().map(String::as_str).collect::<Vec<_>>(),
        "tied-date pages retain every Question exactly once in ID order"
    );

    let positive = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::Any,
                    value: "%_\\".to_owned(),
                    excluded: false,
                }],
                author_names: Vec::new(),
                tags: Vec::new(),
                subjects: Vec::new(),
                discipline_uuid: None,
                subject_uuid: None,
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                authored_by_current_account: false,
                ..request()
            },
        )
        .await
        .expect("literal positive term");
    assert_eq!(
        positive.items.len(),
        50,
        "literal wildcard characters match their stored bytes"
    );
    let excluded = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::Any,
                    value: "p1 tied".to_owned(),
                    excluded: true,
                }],
                author_names: Vec::new(),
                tags: Vec::new(),
                subjects: Vec::new(),
                discipline_uuid: None,
                subject_uuid: None,
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                authored_by_current_account: false,
                ..request()
            },
        )
        .await
        .expect("excluded matching term");
    assert!(
        excluded.items.iter().all(|item| {
            expected_ids.iter().all(|fixture_id| {
                fixture_id
                    != item
                        .published_question_revision_tuple
                        .published_question_id
                        .as_str()
            })
        }),
        "excluded matching term removes every matching fixture row"
    );
    let excluded_empty = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::QuestionType,
                    value: String::new(),
                    excluded: true,
                }],
                author_names: Vec::new(),
                tags: Vec::new(),
                subjects: Vec::new(),
                discipline_uuid: None,
                subject_uuid: None,
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                authored_by_current_account: false,
                ..request()
            },
        )
        .await
        .expect("excluded empty term");
    assert!(
        excluded_empty.items.is_empty(),
        "excluded empty term rejects every row like the Rust matcher"
    );
    let positive_empty = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::QuestionType,
                    value: String::new(),
                    excluded: false,
                }],
                author_names: Vec::new(),
                tags: Vec::new(),
                subjects: Vec::new(),
                discipline_uuid: None,
                subject_uuid: None,
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                authored_by_current_account: false,
                ..request()
            },
        )
        .await
        .expect("positive empty term");
    assert!(
        positive_empty.items.is_empty(),
        "positive empty term rejects every row like the Rust matcher"
    );
    let impossible_backend = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                backends: QuestionLibraryBackendRestriction::Only(Vec::new()),
                ..request()
            },
        )
        .await
        .expect("empty backend restriction");
    assert!(
        impossible_backend.items.is_empty(),
        "Only([]) differs from unrestricted Any"
    );
    assert!(
        impossible_backend.facets.author_names.is_empty()
            && impossible_backend.facets.backends.is_empty()
            && impossible_backend.facets.tags.is_empty()
            && impossible_backend.facets.subjects.is_empty()
            && impossible_backend.facets.topics.is_empty()
            && impossible_backend.facets.question_types.is_empty()
            && impossible_backend.facets.question_licenses.is_empty(),
        "Only([]) retains meaningful all-zero facets"
    );
    assert_eq!(impossible_backend.facets.used_in_my_courses.used, 0);
    assert_eq!(
        impossible_backend
            .facets
            .bloom_cognitive_processes
            .iter()
            .map(|facet| (facet.cognitive_process, facet.count))
            .collect::<Vec<_>>(),
        vec![
            (question_model::BloomCognitiveProcess::Remember, 0),
            (question_model::BloomCognitiveProcess::Understand, 0),
            (question_model::BloomCognitiveProcess::Apply, 0),
            (question_model::BloomCognitiveProcess::Analyze, 0),
            (question_model::BloomCognitiveProcess::Evaluate, 0),
            (question_model::BloomCognitiveProcess::Create, 0),
        ],
        "Only([]) retains all closed Bloom Cognitive Process categories"
    );
    assert_eq!(
        impossible_backend
            .facets
            .bloom_knowledge_dimensions
            .iter()
            .map(|facet| (facet.knowledge_dimension, facet.count))
            .collect::<Vec<_>>(),
        vec![
            (question_model::BloomKnowledgeDimension::FactualKnowledge, 0),
            (
                question_model::BloomKnowledgeDimension::ConceptualKnowledge,
                0
            ),
            (
                question_model::BloomKnowledgeDimension::ProceduralKnowledge,
                0
            ),
            (
                question_model::BloomKnowledgeDimension::MetacognitiveKnowledge,
                0
            ),
        ],
        "Only([]) retains all closed Bloom Knowledge Dimension categories"
    );
    let discipline_only = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: vec![QuestionLibraryTextTerm {
                    field: QuestionLibraryTextField::Any,
                    value: "course fixture discipline".to_owned(),
                    excluded: false,
                }],
                author_names: Vec::new(),
                tags: Vec::new(),
                subjects: Vec::new(),
                discipline_uuid: None,
                subject_uuid: None,
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                authored_by_current_account: false,
                ..request()
            },
        )
        .await
        .expect("unqualified text search");
    assert!(
        discipline_only.items.is_empty(),
        "unqualified text does not search Discipline"
    );
    add_facet_bounds(&admin).await;
    let bounded_facets = store
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: Vec::new(),
                subjects: Vec::new(),
                topics: Vec::new(),
                subject_uuid: None,
                ..request()
            },
        )
        .await
        .expect("bounded facet query");
    for (values, truncated, facet_name) in [
        (
            bounded_facets.facets.author_names.len(),
            bounded_facets.facets.author_names_truncated,
            "authors",
        ),
        (
            bounded_facets.facets.tags.len(),
            bounded_facets.facets.tags_truncated,
            "tags",
        ),
        (
            bounded_facets.facets.subjects.len(),
            bounded_facets.facets.subjects_truncated,
            "subjects",
        ),
        (
            bounded_facets.facets.topics.len(),
            bounded_facets.facets.topics_truncated,
            "topics",
        ),
    ] {
        assert_eq!(
            values, 64,
            "{facet_name} facet values use the documented bound"
        );
        assert!(truncated, "{facet_name} reports values beyond the bound");
    }
    assert!(matches!(
        store
            .search_published_question_library_entries(
                SessionTokenHash::compute(b"P1 unauthorized"),
                request(),
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    application.close().await;
    admin.close().await;
}
