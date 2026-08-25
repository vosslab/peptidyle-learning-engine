#![cfg(feature = "postgres")]

//! Disposable public-Store oracle for ranked catalog discovery behavior.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CatalogStore, CatalogTransition, CourseRecord, CreateCourseCommand, DraftRecord,
    PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
    TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    BackendCapabilities, Capability, CatalogAuthorship, CatalogLicenseValue, CatalogResponseFamily,
    CatalogSearchQuery, CatalogUsedInMyCourses, CourseId, CourseTerm, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionBackend, QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId,
    UserRole, VersionId, WorkspaceId,
};
use uuid::Uuid;

/// Adds one immutable D1 evidence event after the page-one cursor boundary.
///
/// The production evidence oracle exercises `ple_record_question_statistics`.
/// This cursor fixture writes the already-validated anonymous projection under
/// its broker role so it can isolate evidence-event snapshot behavior.
async fn append_disclosed_evidence_revision(pool: &sqlx::PgPool, reference: ProblemVersionRef) {
    let mut broker = pool
        .begin()
        .await
        .expect("begin evidence revision broker transaction");
    sqlx::query("SET LOCAL ROLE ple_statistics_broker")
        .execute(&mut *broker)
        .await
        .expect("assume evidence statistics broker role");
    sqlx::query(
        "INSERT INTO catalog_discovery_evidence_revision (\
             problem_id, version_id, evidence_sequence, formula_version, response_family, \
             course_count, first_attempt_count, difficulty_index, attempts_mean, \
             time_median_seconds_estimate, discrimination_index, quality_signal, evidence_at\
         ) \
         SELECT $1, $2, nextval('catalog_search_publication_sequence'), 1, \
                publication.response_family, 2, 5, 0.6, 1.0, 60, NULL, \
                round((ln(3::double precision) + ln(6::double precision))::numeric, 6), \
                transaction_timestamp() \
           FROM problem_version AS publication \
          WHERE publication.problem_id = $1 AND publication.version_id = $2",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .execute(&mut *broker)
    .await
    .expect("statistics broker appends disclosed independent-learner evidence");
    sqlx::query(
        "UPDATE catalog_search_document AS document \
            SET quality_signal = revision.quality_signal, updated_at = transaction_timestamp() \
           FROM catalog_discovery_evidence_revision AS revision \
          WHERE document.problem_id = $1 AND document.version_id = $2 \
            AND revision.problem_id = document.problem_id \
            AND revision.version_id = document.version_id \
            AND revision.evidence_sequence = ( \
                SELECT max(latest.evidence_sequence) \
                  FROM catalog_discovery_evidence_revision AS latest \
                 WHERE latest.problem_id = $1 AND latest.version_id = $2\
            )",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .execute(&mut *broker)
    .await
    .expect("evidence revision refreshes the search quality cache");
    broker
        .commit()
        .await
        .expect("commit disclosed evidence revision");
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
            attempt_policy: AttemptPolicy { max_attempts: None },
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
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish catalog search fixture")
}

/// Persists the instructor session and direct course membership required for
/// the actor-bound usage snapshot that accompanies every catalog cursor.
async fn seeded_instructor_session(
    pool: &sqlx::PgPool,
    store: &PostgresStore,
    context: TenantContext,
    instructor: UserId,
) -> SessionTokenHash {
    let email = format!(
        "catalog-discovery-{}@example.test",
        instructor.as_uuid().simple()
    );
    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, 'Catalog discovery live instructor')",
    )
    .bind(instructor.as_uuid())
    .bind(&email)
    .execute(pool)
    .await
    .expect("persist catalog discovery instructor account");
    sqlx::query(
        "INSERT INTO instructor_approval (user_id, approved_by, approved_at, revision) \
         VALUES ($1, $1, transaction_timestamp(), 1)",
    )
    .bind(instructor.as_uuid())
    .execute(pool)
    .await
    .expect("approve catalog discovery instructor");
    let session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            session,
            SessionSubject::new(
                context.tenant_id(),
                instructor,
                "Catalog discovery live instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid instructor session"),
            SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
        )
        .await
        .expect("persist catalog discovery instructor session");
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant: context.tenant_id(),
                    title: "Catalog discovery live course".to_string(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("valid catalog discovery course term"),
                },
                authority: sysadmin_course_creation_authority(
                    store,
                    context.tenant_id(),
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("course creation grants the instructor direct active membership");
    session
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_search_store_preserves_ranked_cursor_behavior() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id()));
    let publisher = UserId::from_uuid(id());
    let session = seeded_instructor_session(&pool, &store, context, publisher).await;
    let exact = publish(&store, context, publisher, "Molecular peptide catalyst").await;
    publish(&store, context, publisher, "Molecular peptide folding").await;
    publish(&store, context, publisher, "Molecular peptide folding").await;

    let exact_page = store
        .search_catalog(
            context,
            session,
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
            .map(|item| item.summary.question_id.clone())
            .collect::<Vec<_>>(),
        vec![exact.question_id]
    );
    let metadata_page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                bylines: vec!["ple fixture".to_string()],
                backends: vec![QuestionBackend::Native, QuestionBackend::Qti],
                tags: vec!["biochemistry".to_string()],
                response_families: vec![
                    CatalogResponseFamily::Numeric,
                    CatalogResponseFamily::ShortText,
                ],
                licenses: vec![CatalogLicenseValue::CcBySa, CatalogLicenseValue::Cc0],
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("repeated metadata filters search");
    assert_eq!(metadata_page.items.len(), 3);
    assert!(
        metadata_page
            .items
            .iter()
            .all(|item| item.summary.response_family == CatalogResponseFamily::Numeric)
    );
    assert!(
        metadata_page
            .facets
            .backends
            .iter()
            .any(|facet| facet.backend == QuestionBackend::Native && facet.count == 3)
    );
    assert!(
        metadata_page
            .facets
            .response_families
            .iter()
            .any(
                |facet| facet.response_family == CatalogResponseFamily::Numeric && facet.count == 3
            )
    );
    assert_eq!(metadata_page.facets.used_in_my_courses.used, 0);
    let authored_page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                authorship: CatalogAuthorship::AuthoredByCurrentActor,
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("session-derived authorship search");
    assert_eq!(authored_page.items.len(), 3);
    assert!(
        authored_page
            .items
            .iter()
            .all(|item| item.summary.byline.names[0].as_str() == "PLE fixture")
    );
    let used_page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                used_in_my_courses: CatalogUsedInMyCourses::Used,
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("actor-bound used-in-my-courses search");
    assert!(used_page.items.is_empty());
    assert_eq!(used_page.facets.used_in_my_courses.used, 0);
    let first = store
        .search_catalog(
            context,
            session,
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
            session,
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
            session,
            CatalogSearchQuery {
                text: Some("molecular peptide".to_string()),
                page_size: Some(1),
                cursor: Some(cursor.clone()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("second keyset page");
    assert_ne!(
        first.items[0].summary.question_id,
        second.items[0].summary.question_id
    );
    assert!(
        store
            .search_catalog(
                context,
                session,
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
                session,
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
            session,
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
            .all(|item| item.summary.question_id != later.question_id)
    );
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_search_continuation_preserves_snapshot_visibility_boundaries() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x43; 32]);
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id()));
    let publisher = UserId::from_uuid(id());
    let session = seeded_instructor_session(&pool, &store, context, publisher).await;
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
        .search_catalog(context, session, query.clone())
        .await
        .expect("first catalog snapshot page");
    let first_question_id = first.items[0].summary.question_id.clone();
    let disclosure_target = records
        .iter()
        .find(|record| record.question_id != first_question_id)
        .expect("a remaining record can cross the disclosure threshold");
    let lifecycle_target = records
        .iter()
        .find(|record| {
            record.question_id != first_question_id
                && record.question_id != disclosure_target.question_id
        })
        .expect("a different remaining record can become lifecycle-hidden");

    append_disclosed_evidence_revision(
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
    append_disclosed_evidence_revision(
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
            session,
            CatalogSearchQuery {
                cursor: first.next_cursor.clone(),
                ..query.clone()
            },
        )
        .await
        .expect("snapshot continuation with current safe visibility");
    let continuation_total =
        continuation.facets.evidence.available + continuation.facets.evidence.unavailable;
    assert!(
        continuation
            .items
            .iter()
            .any(|item| item.summary.question_id == disclosure_target.question_id)
    );
    assert!(continuation.items.iter().all(|item| {
        item.summary.question_id != lifecycle_target.question_id
            && item.summary.question_id != later.question_id
    }));
    assert_eq!(continuation.facets.evidence.available, 0);
    assert!(continuation_total > continuation.items.len() as u64);

    let fresh = store
        .search_catalog(context, session, query)
        .await
        .expect("fresh catalog visibility after transitions");
    let fresh_total = fresh.facets.evidence.available + fresh.facets.evidence.unavailable;
    assert!(fresh.facets.evidence.available > continuation.facets.evidence.available);
    assert!(fresh_total > continuation_total);
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
