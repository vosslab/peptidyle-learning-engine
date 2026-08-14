#![cfg(feature = "postgres")]

//! Disposable public-Store oracle for ranked catalog discovery behavior.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CatalogStore, CatalogTransition, DraftRecord, PublishDraftCommand, Store, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    BackendCapabilities, Capability, CatalogSearchQuery, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

async fn cross_statistics_disclosure_threshold(pool: &sqlx::PgPool, reference: ProblemVersionRef) {
    let mut broker = pool
        .begin()
        .await
        .expect("begin statistics broker threshold transaction");
    sqlx::query("SET LOCAL ROLE ple_statistics_broker")
        .execute(&mut *broker)
        .await
        .expect("assume statistics broker role");
    sqlx::query(
        "INSERT INTO question_statistics_aggregate (problem_id, version_id, cohort_size, score_sum, attempts_sum, duration_histogram_version, duration_histogram, scored_cohort_size, score_mean, rest_score_mean, score_m2, rest_score_m2, score_rest_co_moment) \
         VALUES ($1, $2, 5, 3, 5, 1, ARRAY[5,0,0,0,0,0,0,0,0,0]::bigint[], 5, 0.6, 0.6, 0, 0, 0)",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .execute(&mut *broker)
    .await
    .expect("statistics broker crosses disclosure threshold");
    broker
        .commit()
        .await
        .expect("commit statistics disclosure threshold");
}

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn draft(tenant: TenantId, workspace: WorkspaceId, title: &str) -> DraftRecord {
    DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateFull,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: title.to_string(),
                tags: vec![Tag::new("biochemistry")],
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        revises: None,
        derived_from: None,
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    publisher: UserId,
    title: &str,
) -> learning_data_access::PublishedProblemRecord {
    let source = draft(context.tenant_id(), WorkspaceId::from_uuid(id()), title);
    let saved = store
        .upsert_draft(context, publisher, None, source.clone())
        .await
        .expect("save catalog search fixture");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: source,
                expected_revision: saved.revision,
                publication: ProblemVersionRef {
                    problem: ProblemId::from_uuid(id()),
                    version: VersionId::from_uuid(id()),
                },
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher,
                scope: PublicationScope::Public,
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish catalog search fixture")
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_search_store_preserves_ranked_cursor_behavior() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool, [0x42; 32]);
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id()));
    let publisher = UserId::from_uuid(id());
    let exact = publish(&store, context, publisher, "Molecular peptide catalyst").await;
    publish(&store, context, publisher, "Molecular peptide folding").await;
    publish(&store, context, publisher, "Molecular peptide folding").await;

    let exact_page = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some(exact.question_id.to_string()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("exact Question ID search");
    assert_eq!(
        exact_page
            .items
            .iter()
            .map(|item| item.problem)
            .collect::<Vec<_>>(),
        vec![exact.problem]
    );
    let first = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("molecular peptide".to_string()),
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("broad lexical search");
    assert!(!first.items.is_empty());
    let typo = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("moleculr peptide".to_string()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("deliberate typo search");
    assert!(!typo.items.is_empty());
    let cursor = first
        .next_cursor
        .clone()
        .expect("bounded first search page");
    let second = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("molecular peptide".to_string()),
                page_size: Some(1),
                cursor: Some(cursor.clone()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("second keyset page");
    assert_ne!(first.items[0].problem, second.items[0].problem);
    assert!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("different query".to_string()),
                    cursor: Some(cursor.clone()),
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .is_err()
    );
    let mut forged = cursor.into_bytes();
    forged[0] ^= 1;
    assert!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("molecular peptide".to_string()),
                    cursor: Some(String::from_utf8(forged).expect("base64url cursor is UTF-8")),
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .is_err()
    );
    let later = publish(
        &store,
        context,
        publisher,
        "Molecular peptide later publication",
    )
    .await;
    let continued = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("molecular peptide".to_string()),
                page_size: Some(10),
                cursor: Some(first.next_cursor.expect("first cursor remains available")),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("snapshot continuation");
    assert!(
        continued
            .items
            .iter()
            .all(|item| item.problem != later.problem)
    );
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_search_continuation_preserves_snapshot_visibility_boundaries() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x43; 32]);
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id()));
    let publisher = UserId::from_uuid(id());
    let query_text = format!("catalog snapshot {}", id().simple());
    let records = [
        publish(&store, context, publisher, &format!("{query_text} alpha")).await,
        publish(&store, context, publisher, &format!("{query_text} beta")).await,
        publish(&store, context, publisher, &format!("{query_text} gamma")).await,
    ];
    let query = CatalogSearchQuery {
        text: Some(query_text.clone()),
        page_size: Some(1),
        ..CatalogSearchQuery::default()
    };
    let first = store
        .search_catalog(context, query.clone())
        .await
        .expect("first catalog snapshot page");
    let first_problem = first.items[0].problem;
    let disclosure_target = records
        .iter()
        .find(|record| record.problem != first_problem)
        .expect("a remaining record can cross the disclosure threshold");
    let lifecycle_target = records
        .iter()
        .find(|record| {
            record.problem != first_problem && record.problem != disclosure_target.problem
        })
        .expect("a different remaining record can become lifecycle-hidden");

    cross_statistics_disclosure_threshold(
        &pool,
        ProblemVersionRef {
            problem: disclosure_target.problem,
            version: disclosure_target.version,
        },
    )
    .await;
    store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: lifecycle_target.problem,
                version: lifecycle_target.version,
            },
            CatalogTransition::Deprecate {
                reason: "catalog snapshot review".to_string(),
            },
        )
        .await
        .expect("author lifecycle transition hides remaining record");
    let later = publish(&store, context, publisher, &format!("{query_text} later")).await;
    cross_statistics_disclosure_threshold(
        &pool,
        ProblemVersionRef {
            problem: later.problem,
            version: later.version,
        },
    )
    .await;

    let continuation = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                cursor: first.next_cursor.clone(),
                ..query.clone()
            },
        )
        .await
        .expect("snapshot continuation with current safe visibility");
    let continuation_total =
        continuation.facets.statistics.available + continuation.facets.statistics.unavailable;
    assert!(
        continuation
            .items
            .iter()
            .any(|item| item.problem == disclosure_target.problem)
    );
    assert!(
        continuation
            .items
            .iter()
            .all(|item| item.problem != lifecycle_target.problem && item.problem != later.problem)
    );
    assert_eq!(continuation.facets.statistics.available, 0);
    assert!(continuation_total > continuation.items.len() as u64);

    let fresh = store
        .search_catalog(context, query)
        .await
        .expect("fresh catalog visibility after transitions");
    let fresh_total = fresh.facets.statistics.available + fresh.facets.statistics.unavailable;
    assert!(fresh.facets.statistics.available > continuation.facets.statistics.available);
    assert!(fresh_total > continuation_total);
}
