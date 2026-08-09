#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for tenant-qualified public catalog ownership.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CatalogStore, CatalogTransition, DraftRecord, PublishDraftCommand, Store, StoreError,
    TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    BackendCapabilities, Capability, CatalogLifecycle, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn draft(
    tenant: TenantId,
    workspace: WorkspaceId,
    revises: Option<ProblemVersionRef>,
) -> DraftRecord {
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
                title: "Molar mass".to_string(),
                tags: vec![Tag::new("biochemistry")],
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        revises,
        derived_from: None,
    }
}

fn publication(
    draft: DraftRecord,
    revision: learning_data_access::WorkspaceDraftRevision,
    reference: ProblemVersionRef,
    publisher: UserId,
) -> PublishDraftCommand {
    PublishDraftCommand {
        expected_draft: draft,
        expected_revision: revision,
        publication: reference,
        published_source: QuestionSource::Native {
            family: "molar_mass".to_string(),
        },
        publisher,
        scope: PublicationScope::Public,
        source_artifact: None,
        qti_promotion: None,
        flat_question_promotion: None,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
    }
}

async fn begin_app_tenant(
    pool: &sqlx::PgPool,
    tenant: TenantId,
) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut transaction = pool.begin().await.expect("begin direct RLS transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("set direct RLS tenant");
    transaction
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_public_catalog_writes_require_owner_tenant() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let owner_tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let owner_context = TenantContext::from_authenticated_session(owner_tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let shared_user = UserId::from_uuid(id());
    let problem = ProblemId::from_uuid(id());
    let version = VersionId::from_uuid(id());
    let reference = ProblemVersionRef { problem, version };

    let owner_draft = draft(owner_tenant, WorkspaceId::from_uuid(id()), None);
    let saved_owner = store
        .upsert_draft(owner_context, shared_user, None, owner_draft.clone())
        .await
        .expect("save owner draft");
    store
        .publish_draft(
            owner_context,
            shared_user,
            publication(owner_draft, saved_owner.revision, reference, shared_user),
        )
        .await
        .expect("publish owner public problem");

    assert_eq!(
        store
            .transition_catalog_problem(
                foreign_context,
                shared_user,
                reference,
                CatalogTransition::Deprecate {
                    reason: "foreign tenant".to_string(),
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert!(matches!(
        store
            .get_catalog_problem(owner_context, reference)
            .await
            .expect("read after refused foreign transition")
            .expect("owner problem remains present")
            .lifecycle,
        CatalogLifecycle::Published
    ));

    let mut foreign_update = begin_app_tenant(&pool, foreign_tenant).await;
    let update = sqlx::query(
        "UPDATE problem_version SET lifecycle = 'deprecated', lifecycle_reason = 'foreign RLS' \
         WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .execute(&mut *foreign_update)
    .await
    .expect("RLS filters the foreign public-row update");
    assert_eq!(update.rows_affected(), 0);
    foreign_update
        .rollback()
        .await
        .expect("rollback direct foreign update probe");

    let foreign_successor_version = VersionId::from_uuid(id());
    let foreign_successor_workspace = WorkspaceId::from_uuid(id());
    let mut foreign_insert = begin_app_tenant(&pool, foreign_tenant).await;
    let error = sqlx::query(
        "INSERT INTO problem_version ( \
             problem_id, version_id, version_number, content_sha256, workspace_id, title, \
             backend, capabilities, metadata, publication_scope, lifecycle, authors, \
             previous_version_id \
         ) \
         SELECT problem_id, $3, 2, repeat('f', 64), $4, 'foreign successor', backend, \
                capabilities, metadata, publication_scope, 'published', authors, version_id \
           FROM problem_version \
          WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .bind(foreign_successor_version.as_uuid())
    .bind(foreign_successor_workspace.as_uuid())
    .execute(&mut *foreign_insert)
    .await
    .expect_err("RLS must reject a foreign successor insert");
    let error_code = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned());
    assert_eq!(error_code.as_deref(), Some("42501"));
    foreign_insert
        .rollback()
        .await
        .expect("rollback direct foreign insert probe");

    let foreign_draft = draft(
        foreign_tenant,
        WorkspaceId::from_uuid(id()),
        Some(reference),
    );
    let saved_foreign = store
        .upsert_draft(foreign_context, shared_user, None, foreign_draft.clone())
        .await
        .expect("same user ID owns a separate foreign-tenant draft");
    assert_eq!(
        store
            .publish_draft(
                foreign_context,
                shared_user,
                publication(
                    foreign_draft,
                    saved_foreign.revision,
                    ProblemVersionRef {
                        problem,
                        version: foreign_successor_version,
                    },
                    shared_user,
                ),
            )
            .await,
        Err(StoreError::NotFound)
    );

    let owner_successor_version = VersionId::from_uuid(id());
    let owner_revision_draft = draft(owner_tenant, WorkspaceId::from_uuid(id()), Some(reference));
    let saved_owner_revision = store
        .upsert_draft(
            owner_context,
            shared_user,
            None,
            owner_revision_draft.clone(),
        )
        .await
        .expect("save owner revision draft");
    let owner_successor = store
        .publish_draft(
            owner_context,
            shared_user,
            publication(
                owner_revision_draft,
                saved_owner_revision.revision,
                ProblemVersionRef {
                    problem,
                    version: owner_successor_version,
                },
                shared_user,
            ),
        )
        .await
        .expect("owner tenant publishes a successor");
    assert_eq!(owner_successor.previous_version, Some(version));

    let deprecated = store
        .transition_catalog_problem(
            owner_context,
            shared_user,
            reference,
            CatalogTransition::Deprecate {
                reason: "owner correction".to_string(),
            },
        )
        .await
        .expect("owner tenant deprecates its public version");
    assert!(matches!(
        deprecated.lifecycle,
        CatalogLifecycle::Deprecated { .. }
    ));
    assert_eq!(
        store
            .transition_catalog_problem(
                foreign_context,
                shared_user,
                reference,
                CatalogTransition::Archive,
            )
            .await,
        Err(StoreError::NotFound)
    );
    let archived = store
        .transition_catalog_problem(
            owner_context,
            shared_user,
            reference,
            CatalogTransition::Archive,
        )
        .await
        .expect("owner tenant archives its deprecated version");
    assert!(matches!(
        archived.lifecycle,
        CatalogLifecycle::Archived { .. }
    ));
}
