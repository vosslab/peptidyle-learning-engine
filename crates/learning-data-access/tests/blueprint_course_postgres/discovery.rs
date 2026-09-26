//! Connected discovery-page bounds using the existing Blueprint fixture authority.

use super::*;

const DISCOVERY_PAGE_SIZE: u16 = 250;
const POOL_ROWS: usize = 250;
const BLUEPRINT_ROWS: usize = 251;
const POOL_FIXTURE_TEXT: &str = "p0 discovery pool fixture";
const BLUEPRINT_FIXTURE_TEXT: &str = "P0 Discovery Blueprint Fixture";

fn discovery_request(
    size: DiscoveryPageSize,
    after: Option<learning_data_access::Cursor>,
) -> learning_data_access::BlueprintCourseListRequest {
    learning_data_access::BlueprintCourseListRequest {
        page: DiscoveryPageRequest { after, size },
        sort: learning_data_access::BlueprintCourseListSort::Name,
        query: String::new(),
        include_archived: false,
        public_only: false,
        promoted_only: false,
        discipline_uuid: None,
        subject_uuid: None,
        topic_uuid: None,
        subtopic_uuid: None,
        cross_discipline: false,
    }
}

fn discovery_request_with_sort(
    size: DiscoveryPageSize,
    after: Option<learning_data_access::Cursor>,
    sort: learning_data_access::BlueprintCourseListSort,
) -> learning_data_access::BlueprintCourseListRequest {
    learning_data_access::BlueprintCourseListRequest {
        sort,
        ..discovery_request(size, after)
    }
}

async fn insert_pool_discovery_rows(admin: &sqlx::postgres::PgPool) {
    let pool_ids = (0..POOL_ROWS)
        .map(|index| {
            question_model::QuestionPoolId::from_random_identifier(format!("AA{index:05}"))
                .expect("fixture Pool ID")
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut transaction = admin
        .begin()
        .await
        .expect("Pool discovery fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("Pool discovery data owner");
    // ASVS 1.2.4: the generated identifiers and fixture identity remain bound values.
    sqlx::query(
        "INSERT INTO ple_data.question_pool (\
             question_pool_id, question_pool_edit_number, created_at, title, description, \
             content_discipline_id, content_subject_id, interchangeability_attested_by_account_id, \
             interchangeability_attested_at\
         ) \
         SELECT pool_id, 1, clock_timestamp(), 'P0 Discovery Pool Fixture ' || ordinal, \
                'P0 Discovery Pool Fixture', metadata.content_discipline_id, \
                metadata.content_subject_id, $2, clock_timestamp() \
           FROM unnest($1::text[]) WITH ORDINALITY AS pools(pool_id, ordinal) \
           JOIN ple_data.published_question_metadata AS metadata ON metadata.published_question_id = $3",
    )
    .bind(&pool_ids)
    .bind(instructor_account_id())
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("250 Pool discovery fixture rows");
    sqlx::query(
        "INSERT INTO ple_data.question_pool_member (\
             question_pool_id, member_position, published_question_id, question_revision_number, \
             created_at, updated_at\
         ) \
         SELECT pool_id, 1, $2, 1, statement_timestamp(), statement_timestamp() \
           FROM unnest($1::text[]) AS pools(pool_id)",
    )
    .bind(&pool_ids)
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Pool discovery fixture members");
    transaction
        .commit()
        .await
        .expect("Pool discovery fixture commit");
}

async fn assert_tied_count_continuation(
    blueprint_store: &PostgresBlueprintCourseStore,
    size: DiscoveryPageSize,
    sort: learning_data_access::BlueprintCourseListSort,
    mismatched_sort: learning_data_access::BlueprintCourseListSort,
    expected_second_long_name: String,
) {
    let first = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                ..discovery_request_with_sort(size, None, sort)
            },
        )
        .await
        .expect("count-ranked Blueprint page");
    let next = first
        .next_cursor
        .clone()
        .expect("count-ranked page carries deterministic continuation");
    assert!(
        blueprint_store
            .list_blueprint_courses(
                token(),
                learning_data_access::BlueprintCourseListRequest {
                    query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                    ..discovery_request_with_sort(size, Some(next.clone()), mismatched_sort)
                },
            )
            .await
            .is_err(),
        "a continuation cursor remains bound to its selected count sort"
    );
    let second = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                ..discovery_request_with_sort(size, Some(next), sort)
            },
        )
        .await
        .expect("count-ranked continuation");
    assert_eq!(first.items.len(), usize::from(DISCOVERY_PAGE_SIZE));
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    let first_ids = first
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let second_ids = second
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        first_ids.is_disjoint(&second_ids),
        "a tied cursor continuation has disjoint Blueprint identities"
    );
    assert_eq!(
        second.items[0].long_name, expected_second_long_name,
        "the exact next tied name follows the first 250 name/ID positions"
    );
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn discovery_pages_return_250_rows_and_one_blueprint_lookahead() {
    assert!(
        DiscoveryPageSize::new(251).is_err(),
        "251 cannot enter the Store request"
    );
    let size = DiscoveryPageSize::new(DISCOVERY_PAGE_SIZE).expect("250 discovery rows");
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    insert_pool_discovery_rows(&admin).await;
    admin.close().await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let pool_store = PostgresQuestionPoolLibraryStore::new(application.clone());
    let pools = pool_store
        .list_published_question_pools(
            token(),
            DiscoveryPageRequest::first(size),
            QuestionPoolDiscoveryFilter::default(),
            QuestionPoolTextFilter {
                text: Some(POOL_FIXTURE_TEXT.to_owned()),
                terms: vec![QuestionPoolTextTerm {
                    field: QuestionPoolTextField::Any,
                    value: POOL_FIXTURE_TEXT.to_owned(),
                    excluded: false,
                }],
                tags: Vec::new(),
            },
        )
        .await
        .expect("authorized 250 Pool discovery rows");
    assert_eq!(pools.items.len(), usize::from(DISCOVERY_PAGE_SIZE));
    assert!(
        pools.next_cursor.is_none(),
        "all 250 Pools fit on the requested page"
    );

    let blueprint_store = PostgresBlueprintCourseStore::new(application.clone());
    let mut featured = None;
    for index in 0..BLUEPRINT_ROWS {
        let mut input = content_input(&format!("Discovery Blueprint {index:03}"));
        input.short_name = format!("P0-DISC-{index:03}");
        input.long_name = format!("{BLUEPRINT_FIXTURE_TEXT} {index:03}");
        input.modules[0].assessments[0]
            .entries
            .retain(|entry| matches!(entry, BlueprintAssessmentEntryInput::Fixed(_)));
        let mut request_checksum = [0xa5; 32];
        request_checksum[..2].copy_from_slice(&(index as u16).to_be_bytes());
        let created = blueprint_store
            .create_blueprint_course(
                token(),
                RequestChecksum::from_bytes(request_checksum),
                input,
                Default::default(),
            )
            .await
            .expect("Blueprint discovery fixture row");
        if index + 1 == BLUEPRINT_ROWS {
            featured = Some(created.blueprint_revision_tuple);
        }
    }
    let featured = featured.expect("featured Blueprint fixture");
    let featured_metadata = blueprint_store
        .load_blueprint_course(token(), featured.blueprint_course_id.clone())
        .await
        .expect("featured Blueprint metadata");
    transition_blueprint_availability(
        &application_url,
        &featured.blueprint_course_id.as_string(),
        featured_metadata.blueprint_edit_number.as_i64(),
        "public",
        None,
    )
    .await
    .expect("publish featured Blueprint before adoption");
    let courses = PostgresCourseInstanceStore::new(application.clone());
    let term = near_now_term(&application_url).await;
    for index in 0..2 {
        courses
            .create_course_instance(
                token(),
                CreateCourseInstanceInput {
                    classification: question_model::CourseClassification {
                        discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                        subject_uuid: None,
                        topic_uuid: None,
                        subtopic_uuid: None,
                        tags: Vec::new(),
                    },
                    source: CourseInstanceCreationSource::Adopted {
                        blueprint_revision_tuple: featured.clone(),
                    },
                    short_name: format!("P4-{index}"),
                    long_name: format!("Blueprint sort fixture Course {index}"),
                    term: term.clone(),
                    assigned_instructor_account_id: None,
                },
                Default::default(),
            )
            .await
            .expect("featured Blueprint adoption");
    }
    let ranked = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                ..discovery_request_with_sort(
                    DiscoveryPageSize::new(1).expect("single rank"),
                    None,
                    learning_data_access::BlueprintCourseListSort::Adoptions,
                )
            },
        )
        .await
        .expect("adoption-ranked Blueprint page");
    assert_eq!(ranked.items[0].id, featured.blueprint_course_id);
    assert_eq!(ranked.items[0].total_adoptions, 2);
    let empty_ranked = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: "no matching Blueprint sort fixture".to_owned(),
                ..discovery_request_with_sort(
                    DiscoveryPageSize::new(1).expect("single empty rank"),
                    None,
                    learning_data_access::BlueprintCourseListSort::Students,
                )
            },
        )
        .await
        .expect("empty student-ranked first page");
    assert!(empty_ranked.items.is_empty());
    assert!(empty_ranked.next_cursor.is_none());
    let first = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                ..discovery_request(size, None)
            },
        )
        .await
        .expect("first Blueprint discovery page");
    assert_eq!(first.items.len(), usize::from(DISCOVERY_PAGE_SIZE));
    let next = first
        .next_cursor
        .expect("251st Blueprint produces one lookahead cursor");
    let second = blueprint_store
        .list_blueprint_courses(
            token(),
            learning_data_access::BlueprintCourseListRequest {
                query: BLUEPRINT_FIXTURE_TEXT.to_owned(),
                ..discovery_request(size, Some(next))
            },
        )
        .await
        .expect("second Blueprint discovery page");
    assert_eq!(
        second.items.len(),
        1,
        "the first page read only one extra Blueprint"
    );
    assert!(second.next_cursor.is_none());
    assert_tied_count_continuation(
        &blueprint_store,
        size,
        learning_data_access::BlueprintCourseListSort::Adoptions,
        learning_data_access::BlueprintCourseListSort::Students,
        format!("{BLUEPRINT_FIXTURE_TEXT} 249"),
    )
    .await;
    assert_tied_count_continuation(
        &blueprint_store,
        size,
        learning_data_access::BlueprintCourseListSort::Students,
        learning_data_access::BlueprintCourseListSort::Adoptions,
        format!("{BLUEPRINT_FIXTURE_TEXT} 250"),
    )
    .await;
    application.close().await;
}
