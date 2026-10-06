//! Connected cleanup of a large Question import through shipped Library search and bulk metadata.

use learning_data_access::postgres::PostgresBulkPublishedQuestionMetadataStore;
use learning_data_access::{
    BulkPublishedQuestionMetadataInput, BulkPublishedQuestionMetadataPatch,
    BulkPublishedQuestionMetadataSelection, BulkPublishedQuestionMetadataStore,
};
use question_model::MAX_BULK_QUESTION_METADATA_ITEMS;

use super::*;

const IMPORT_ROWS: usize = 13_000;
const DECOY_ROWS: usize = 25;
const IMPORT_TAG: &str = "bulk import tag";
const CLEANED_TAG: &str = "cleaned import tag";
const DECOY_TAG: &str = "decoy import tag";
const IMPORT_TEXT: &str = "bulk import enzyme";

fn import_question_id(index: usize) -> String {
    question_model::PublishedQuestionId::from_random_identifier(format!("A{index:06}"))
        .expect("import Question ID")
        .to_string()
}

fn import_request(
    tag: &str,
    after: Option<QuestionLibrarySearchCursorPosition>,
    exact_question_id: Option<question_model::PublishedQuestionId>,
) -> QuestionLibrarySearchRequest {
    QuestionLibrarySearchRequest {
        kind: question_model::LibrarySearchKind::Questions,
        membership: question_model::LibraryQuestionMembership::All,
        owner_account_id: None,
        has_capability_filter: false,
        exact_question_id,
        text_terms: vec![QuestionLibraryTextTerm {
            field: QuestionLibraryTextField::Any,
            value: IMPORT_TEXT.to_owned(),
            excluded: false,
        }],
        author_names: Vec::new(),
        backends: QuestionLibraryBackendRestriction::Any,
        tags: vec![tag.to_owned()],
        subjects: Vec::new(),
        topics: Vec::new(),
        discipline_uuid: None,
        subject_uuid: None,
        topic_uuid: None,
        subtopic_uuid: None,
        cross_discipline: false,
        bloom_cognitive_process: None,
        bloom_knowledge_dimension: None,
        question_types: vec![question_model::QuestionType::MultipleChoice],
        question_licenses: vec![question_model::QuestionLicense::CcBy4_0],
        authored_by_current_account: false,
        sort: QuestionLibrarySearchSort::TitleAscending,
        page_size: question_model::MAX_DISCOVERY_PAGE_SIZE as u16,
        after,
    }
}

async fn insert_questions(
    admin: &sqlx::postgres::PgPool,
    question_ids: &[String],
    title_pattern: &str,
    tag: &str,
    object_base: u128,
) {
    let object_ids = question_ids
        .iter()
        .enumerate()
        .map(|(index, _)| Uuid::from_u128(object_base + index as u128))
        .collect::<Vec<_>>();
    let ownership_event_ids = question_ids
        .iter()
        .enumerate()
        .map(|(index, _)| Uuid::from_u128(object_base + 0x0100_0000 + index as u128))
        .collect::<Vec<_>>();
    let mut transaction = admin.begin().await.expect("import fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("import data owner");
    sqlx::query(
        "INSERT INTO ple_data.published_question (published_question_id, created_at) \
         SELECT question_id, clock_timestamp() FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .execute(&mut *transaction)
    .await
    .expect("import lineages");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (published_question_id, revision_number, backend, question_type, published_at) \
         SELECT question_id, 1, 'ple', 'multipleChoice', '2026-09-25 12:00:00+00'::timestamptz \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .execute(&mut *transaction)
    .await
    .expect("import revisions");
    sqlx::query(
        "INSERT INTO ple_data.published_question_metadata (\
             published_question_id, question_title, question_description, language, tags, \
             content_discipline_id, content_subject_id, created_at, updated_at\
         ) SELECT question_id, format($2, question_id), 'Imported question for library cleanup', \
                  'en', ARRAY[$3], $4, $5, statement_timestamp(), statement_timestamp() \
             FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .bind(title_pattern)
    .bind(tag)
    .bind(id(0xcc01))
    .bind(id(0xcc02))
    .execute(&mut *transaction)
    .await
    .expect("import metadata");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("import private owner");
    sqlx::query(
        "INSERT INTO ple_private.object_record (\
             object_record_id, object_address, object_storage_area, object_data_class, \
             sha256, size_bytes, media_type, created_at, updated_at\
         ) SELECT object_id, jsonb_build_object(\
                    'kind', 'questionSource', \
                    'publishedQuestionRevisionTuple', jsonb_build_object(\
                        'publishedQuestionId', question_id, 'revisionNumber', 1), \
                    'objectId', object_id\
                ), 'private-content', 'question-source', decode(repeat('ab', 32), 'hex'), \
                1, 'application/json', statement_timestamp(), statement_timestamp() \
           FROM unnest($1::text[], $2::uuid[]) row(question_id, object_id)",
    )
    .bind(question_ids)
    .bind(&object_ids)
    .execute(&mut *transaction)
    .await
    .expect("import source records");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding (\
             published_question_id, revision_number, backend, question_format, \
             source_object_record_id, source_object_checksum, created_at\
         ) SELECT question_id, 1, 'ple', 'pleQuestionJson', object_id, repeat('ab', 32), \
                  '2026-09-25 12:00:00+00'::timestamptz \
           FROM unnest($1::text[], $2::uuid[]) row(question_id, object_id)",
    )
    .bind(question_ids)
    .bind(&object_ids)
    .execute(&mut *transaction)
    .await
    .expect("import source bindings");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("import acceptance owner");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_acceptance (\
             published_question_id, revision_number, parent_revision_number, editor_account_id, \
             accepted_by_account_id, accepted_at, reason_for_edit\
         ) SELECT question_id, 1, NULL, $2, $2, '2026-09-25 12:00:00+00'::timestamptz, \
                  'Bulk import fixture' \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("import acceptances");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_authorship (\
             published_question_id, revision_number, author_position, author_display_name, \
             author_account_id\
         ) SELECT question_id, 1, 1, 'Bulk import Author', $2 \
           FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("import authorship");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_license \
         (published_question_id, revision_number, spdx_expression) \
         SELECT question_id, 1, 'CC-BY-4.0' FROM unnest($1::text[]) question(question_id)",
    )
    .bind(question_ids)
    .execute(&mut *transaction)
    .await
    .expect("import licenses");
    sqlx::query(
        "INSERT INTO ple_data.question_ownership_event (\
             question_ownership_event_id, published_question_id, owner_account_id, \
             recorded_by_account_id, event_kind, occurred_at\
         ) SELECT event_id, question_id, $3, $3, 'initial', \
                  '2026-09-25 12:00:00+00'::timestamptz \
           FROM unnest($1::uuid[], $2::text[]) row(event_id, question_id)",
    )
    .bind(&ownership_event_ids)
    .bind(question_ids)
    .bind(instructor_account_id())
    .execute(&mut *transaction)
    .await
    .expect("import initial ownership");
    transaction.commit().await.expect("import fixture commit");
}

async fn vet_seeded_instructor(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("vetting transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("vetting private owner");
    let sysadmin_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'sysadmin', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("vetting Sysadmin");
    sqlx::query("INSERT INTO ple_private.instructor_profile (account_id, first_name, last_name, affiliation) VALUES ($1, 'Bulk', 'Instructor', 'Test University')")
        .bind(instructor_account_id())
        .execute(&mut *transaction)
        .await
        .expect("Instructor Profile");
    sqlx::query("SET LOCAL ROLE ple_audit_owner")
        .execute(&mut *transaction)
        .await
        .expect("vetting audit owner");
    sqlx::query("SELECT ple_audit.record_instructor_account_creation_event($1, $2)")
        .bind(instructor_account_id())
        .bind(&sysadmin_id)
        .execute(&mut *transaction)
        .await
        .expect("Instructor creation evidence");
    transaction
        .commit()
        .await
        .expect("Instructor fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_library_search_filters_sort_and_bulk_edit_clean_a_large_import() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    vet_seeded_instructor(&admin).await;
    let import_ids = (0..IMPORT_ROWS).map(import_question_id).collect::<Vec<_>>();
    insert_questions(
        &admin,
        &import_ids,
        "Bulk import enzyme %s",
        IMPORT_TAG,
        0xA000_0000_0000,
    )
    .await;
    let decoy_ids = (IMPORT_ROWS..IMPORT_ROWS + DECOY_ROWS)
        .map(import_question_id)
        .collect::<Vec<_>>();
    insert_questions(
        &admin,
        &decoy_ids,
        "Control specimen %s",
        DECOY_TAG,
        0xB000_0000_0000,
    )
    .await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let library = PostgresQuestionLibraryStore::new(application.clone());
    let mut matched = Vec::new();
    let mut after = None;
    let mut import_count = None;
    while matched.len() <= MAX_BULK_QUESTION_METADATA_ITEMS {
        let page = library
            .search_published_question_library_entries(
                token(),
                import_request(IMPORT_TAG, after, None),
            )
            .await
            .expect("import search page");
        if import_count.is_none() {
            import_count = Some(
                page.facets
                    .tags
                    .iter()
                    .find(|facet| facet.tag.as_str() == IMPORT_TAG)
                    .map(|facet| facet.count)
                    .expect("import tag facet"),
            );
        }
        let page_len = page.items.len();
        after = page.next_position;
        matched.extend(page.items.into_iter().map(|item| match item {
            learning_data_access::LibrarySearchEntry::Question { entry, .. } => *entry,
            learning_data_access::LibrarySearchEntry::Pool { .. } => {
                panic!("Question-only import query returned Pool")
            }
        }));
        if after.is_none() || page_len == 0 {
            break;
        }
    }
    assert_eq!(
        import_count,
        Some(IMPORT_ROWS as u64),
        "search text, tag, type, and license together count all 13,000 imported Questions"
    );
    assert!(
        matched.len() > MAX_BULK_QUESTION_METADATA_ITEMS,
        "title sort reaches past one bulk command"
    );
    let titles = matched
        .iter()
        .take(MAX_BULK_QUESTION_METADATA_ITEMS)
        .map(|item| item.question_title.clone())
        .collect::<Vec<_>>();
    let mut title_order = titles.clone();
    title_order.sort();
    assert_eq!(
        titles, title_order,
        "title sort keeps the import page order"
    );
    let selected = &matched[..MAX_BULK_QUESTION_METADATA_ITEMS];
    let untouched = &matched[MAX_BULK_QUESTION_METADATA_ITEMS];
    let bulk = PostgresBulkPublishedQuestionMetadataStore::new(application);
    let results = bulk
        .bulk_replace_published_question_metadata(
            token(),
            BulkPublishedQuestionMetadataInput {
                selection: selected
                    .iter()
                    .map(|item| BulkPublishedQuestionMetadataSelection {
                        question_id: item
                            .published_question_revision_tuple
                            .published_question_id
                            .clone(),
                        metadata_edit_number: item.shared_metadata.metadata_edit_number,
                        question_title: None,
                        question_description: None,
                    })
                    .collect(),
                patch: BulkPublishedQuestionMetadataPatch {
                    tags: Some(vec![CLEANED_TAG.to_owned()]),
                    ..BulkPublishedQuestionMetadataPatch::default()
                },
            },
        )
        .await
        .expect("bulk cleanup of the first sorted thousand");
    assert_eq!(results.len(), MAX_BULK_QUESTION_METADATA_ITEMS);
    assert!(
        results
            .iter()
            .all(|result| result.metadata_edit_number == 2)
    );

    let cleaned_id = selected[0]
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let untouched_id = untouched
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let cleaned = library
        .search_published_question_library_entries(token(), import_request(CLEANED_TAG, None, None))
        .await
        .expect("cleaned Question search");
    assert_eq!(
        cleaned
            .facets
            .tags
            .iter()
            .find(|facet| facet.tag.as_str() == CLEANED_TAG)
            .map(|facet| facet.count),
        Some(MAX_BULK_QUESTION_METADATA_ITEMS as u64),
        "the bulk command retags one thousand imported Questions"
    );
    let cleaned_one = library
        .search_published_question_library_entries(
            token(),
            import_request(CLEANED_TAG, None, Some(cleaned_id.clone())),
        )
        .await
        .expect("one cleaned Question");
    assert_eq!(cleaned_one.items.len(), 1);
    let still_imported = library
        .search_published_question_library_entries(token(), import_request(IMPORT_TAG, None, None))
        .await
        .expect("remaining import search");
    assert_eq!(
        still_imported
            .facets
            .tags
            .iter()
            .find(|facet| facet.tag.as_str() == IMPORT_TAG)
            .map(|facet| facet.count),
        Some((IMPORT_ROWS - MAX_BULK_QUESTION_METADATA_ITEMS) as u64),
        "the Questions past the bulk cap keep the imported tag"
    );
    let untouched_one = library
        .search_published_question_library_entries(
            token(),
            import_request(IMPORT_TAG, None, Some(untouched_id)),
        )
        .await
        .expect("one untouched imported Question");
    assert_eq!(untouched_one.items.len(), 1);
    let cleaned_removed = library
        .search_published_question_library_entries(
            token(),
            import_request(IMPORT_TAG, None, Some(cleaned_id)),
        )
        .await
        .expect("cleaned Question no longer matches the import tag");
    assert!(cleaned_removed.items.is_empty());
    let decoys = library
        .search_published_question_library_entries(
            token(),
            QuestionLibrarySearchRequest {
                text_terms: Vec::new(),
                tags: vec![DECOY_TAG.to_owned()],
                question_types: Vec::new(),
                question_licenses: Vec::new(),
                ..import_request(DECOY_TAG, None, None)
            },
        )
        .await
        .expect("decoy search");
    assert_eq!(
        decoys
            .facets
            .tags
            .iter()
            .find(|facet| facet.tag.as_str() == DECOY_TAG)
            .map(|facet| facet.count),
        Some(DECOY_ROWS as u64),
        "Questions outside the import filter stay unchanged"
    );
}

const SYNTAX_KEPT_ROWS: usize = 6_000;
const SYNTAX_EXCLUDED_ROWS: usize = 6_000;
const SYNTAX_TAG: &str = "syntax corpus";

fn syntax_request(exclude_inhibitor: bool) -> QuestionLibrarySearchRequest {
    let mut text_terms = vec![QuestionLibraryTextTerm {
        field: QuestionLibraryTextField::Any,
        value: "enzyme".to_owned(),
        excluded: false,
    }];
    if exclude_inhibitor {
        text_terms.push(QuestionLibraryTextTerm {
            field: QuestionLibraryTextField::Any,
            value: "inhibitor".to_owned(),
            excluded: true,
        });
    }
    QuestionLibrarySearchRequest {
        text_terms,
        tags: vec![SYNTAX_TAG.to_owned()],
        question_types: vec![question_model::QuestionType::MultipleChoice],
        question_licenses: vec![question_model::QuestionLicense::CcBy4_0],
        page_size: 50,
        ..import_request(SYNTAX_TAG, None, None)
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_library_syntax_narrows_a_large_library() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    vet_seeded_instructor(&admin).await;
    let kept_ids = (20_000..20_000 + SYNTAX_KEPT_ROWS)
        .map(import_question_id)
        .collect::<Vec<_>>();
    insert_questions(
        &admin,
        &kept_ids,
        "Library enzyme kinetics %s",
        SYNTAX_TAG,
        0xC000_0000_0000,
    )
    .await;
    let excluded_ids = (20_000 + SYNTAX_KEPT_ROWS
        ..20_000 + SYNTAX_KEPT_ROWS + SYNTAX_EXCLUDED_ROWS)
        .map(import_question_id)
        .collect::<Vec<_>>();
    insert_questions(
        &admin,
        &excluded_ids,
        "Library enzyme inhibitor %s",
        SYNTAX_TAG,
        0xD000_0000_0000,
    )
    .await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let library =
        PostgresQuestionLibraryStore::new(lazy_pool(&application_url).expect("application pool"));
    let corpus = library
        .search_published_question_library_entries(token(), syntax_request(false))
        .await
        .expect("large library search");
    let corpus_count = corpus
        .facets
        .tags
        .iter()
        .find(|facet| facet.tag.as_str() == SYNTAX_TAG)
        .map(|facet| facet.count);
    assert_eq!(
        corpus_count,
        Some((SYNTAX_KEPT_ROWS + SYNTAX_EXCLUDED_ROWS) as u64)
    );

    let started = std::time::Instant::now();
    let narrowed = library
        .search_published_question_library_entries(token(), syntax_request(true))
        .await
        .expect("expert syntax search");
    let elapsed = started.elapsed();
    let narrowed_count = narrowed
        .facets
        .tags
        .iter()
        .find(|facet| facet.tag.as_str() == SYNTAX_TAG)
        .map(|facet| facet.count);
    assert_eq!(narrowed_count, Some(SYNTAX_KEPT_ROWS as u64));
    assert_eq!(narrowed.items.len(), 50);
    assert!(narrowed.next_position.is_some());
    assert!(narrowed.items.iter().all(|item| {
        let title = item.title().to_ascii_lowercase();
        title.contains("enzyme") && !title.contains("inhibitor")
    }));
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "expert syntax took {elapsed:?} for {} Questions",
        SYNTAX_KEPT_ROWS + SYNTAX_EXCLUDED_ROWS
    );
}
