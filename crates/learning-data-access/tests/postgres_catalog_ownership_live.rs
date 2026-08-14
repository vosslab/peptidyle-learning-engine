#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for tenant-qualified public catalog ownership.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CatalogTransition, CourseRecord, CourseRosterStore,
    DraftRecord, OwnerCorrectionAuthority, OwnerCorrectionStore, PublishDraftCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::{License, Tag};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AssignmentSelectionCandidate, AssignmentSelectionGroup, AssignmentSelectionGroupId,
    BackendCapabilities, Capability, CatalogLifecycle, CatalogSearchQuery, CourseId,
    CourseMembership, CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, PointValue, ProblemDisplayRef, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, ResponseDefinition, RunId,
    SelectionOrdering, TenantId, UserId, VersionId, WorkspaceId,
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

fn institution_publication(
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
        scope: PublicationScope::Institution,
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

async fn begin_student_tenant(
    pool: &sqlx::PgPool,
    tenant: TenantId,
) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut transaction = pool.begin().await.expect("begin direct RLS transaction");
    sqlx::query("SET LOCAL ROLE ple_student")
        .execute(&mut *transaction)
        .await
        .expect("assume student role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("set direct RLS tenant");
    transaction
}

async fn begin_grader_tenant(
    pool: &sqlx::PgPool,
    tenant: TenantId,
) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut transaction = pool.begin().await.expect("begin direct RLS transaction");
    sqlx::query("SET LOCAL ROLE ple_grader")
        .execute(&mut *transaction)
        .await
        .expect("assume grader role");
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
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let owner_tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let owner_context = TenantContext::from_authenticated_session(owner_tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let shared_user = UserId::from_uuid(id());
    let owner_session = SessionTokenHash::compute(b"postgres-owner-correction-session");
    store
        .create_session(
            owner_session,
            SessionSubject::new(
                owner_tenant,
                shared_user,
                "Catalog owner",
                vec![question_model::UserRole::Instructor],
            )
            .expect("owner correction session subject is valid"),
            SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
        )
        .await
        .expect("persist owner correction session");
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
        Err(StoreError::NotFound),
        "foreign tenants must not learn whether an owned catalog version exists"
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

    let mut foreign_grant = begin_app_tenant(&pool, foreign_tenant).await;
    let error = sqlx::query(
        "INSERT INTO catalog_tenant_grant (tenant_id, problem_id, version_id) VALUES ($1, $2, $3)",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .execute(&mut *foreign_grant)
    .await
    .expect_err("a visible foreign problem must not accept a self-grant");
    let error_code = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned());
    assert_eq!(error_code.as_deref(), Some("42501"));
    foreign_grant
        .rollback()
        .await
        .expect("rollback direct foreign grant probe");

    let mut foreign_payload = begin_app_tenant(&pool, foreign_tenant).await;
    let error = sqlx::query(
        "INSERT INTO problem_version_payload (problem_id, version_id, payload, payload_sha256) \
         VALUES ($1, $2, '{}'::jsonb, repeat('a', 64))",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .execute(&mut *foreign_payload)
    .await
    .expect_err("a visible foreign problem must not accept an immutable payload append");
    let error_code = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned());
    assert_eq!(error_code.as_deref(), Some("42501"));
    foreign_payload
        .rollback()
        .await
        .expect("rollback direct foreign payload probe");

    let mut foreign_artifact = begin_app_tenant(&pool, foreign_tenant).await;
    let error = sqlx::query(
        "INSERT INTO published_source_artifact \
         (problem_id, version_id, backend, object_id, payload, payload_sha256) \
         VALUES ($1, $2, 'native', $3, '{}'::jsonb, repeat('a', 64))",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .bind(id())
    .execute(&mut *foreign_artifact)
    .await
    .expect_err("a visible foreign problem must not accept a source artifact append");
    let error_code = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned());
    assert_eq!(error_code.as_deref(), Some("42501"));
    foreign_artifact
        .rollback()
        .await
        .expect("rollback direct foreign artifact probe");

    let mut owner_append = begin_app_tenant(&pool, owner_tenant).await;
    let owner_grant = sqlx::query(
        "INSERT INTO catalog_tenant_grant (tenant_id, problem_id, version_id) VALUES ($1, $2, $3)",
    )
    .bind(owner_tenant.as_uuid())
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .execute(&mut *owner_append)
    .await
    .expect("owner grant insert remains permitted");
    assert_eq!(owner_grant.rows_affected(), 1);
    let owner_artifact = sqlx::query(
        "INSERT INTO published_source_artifact \
         (problem_id, version_id, backend, object_id, payload, payload_sha256) \
         VALUES ($1, $2, 'native', $3, '{}'::jsonb, repeat('a', 64))",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .bind(id())
    .execute(&mut *owner_append)
    .await
    .expect("owner source artifact insert remains permitted");
    assert_eq!(owner_artifact.rows_affected(), 1);
    owner_append
        .rollback()
        .await
        .expect("rollback direct owner append probe");

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

    let owner_missing_session_version = VersionId::from_uuid(id());
    let mut owner_missing_session = begin_app_tenant(&pool, owner_tenant).await;
    let error = sqlx::query(
        "INSERT INTO problem_version ( \
             problem_id, version_id, version_number, content_sha256, workspace_id, title, \
             backend, capabilities, metadata, publication_scope, lifecycle, authors, \
             previous_version_id \
         ) \
         SELECT problem_id, $3, 2, repeat('e', 64), $4, 'owner successor without session', backend, \
                capabilities, metadata, publication_scope, 'published', authors, version_id \
           FROM problem_version \
          WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .bind(owner_missing_session_version.as_uuid())
    .bind(WorkspaceId::from_uuid(id()).as_uuid())
    .execute(&mut *owner_missing_session)
    .await
    .expect_err("an owner successor without the authenticated session capability must be denied");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("42501")
    );
    owner_missing_session
        .rollback()
        .await
        .expect("rollback owner missing-session insert probe");

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
        Err(StoreError::Forbidden)
    );

    let foreign_course = CourseId::from_uuid(id());
    let foreign_instructor = UserId::from_uuid(id());
    let foreign_student = UserId::from_uuid(id());
    let foreign_assignment = AssignmentId::from_uuid(id());
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign public-question course".to_string(),
                members: vec![CourseMembership {
                    user: foreign_instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("foreign course should accept public catalog use");
    store
        .create_untimed_assignment(
            foreign_context,
            AssignmentRecord {
                id: foreign_assignment,
                tenant: foreign_tenant,
                course_id: foreign_course,
                title: "Foreign future-definition fixture".to_string(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(id()),
                    reference,
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: vec![AssignmentSelectionGroup {
                    id: AssignmentSelectionGroupId::from_uuid(id()),
                    position: 1,
                    draw_count: 1,
                    points_per_item: PointValue::from_whole(1),
                    ordering: SelectionOrdering::CandidateOrder,
                    algorithm_version: 1,
                    candidates: vec![AssignmentSelectionCandidate {
                        id: AssignmentItemId::from_uuid(id()),
                        position: 0,
                        reference,
                        delivery_state: AssignmentDeliveryState::Active,
                    }],
                }],
                policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("foreign instructor can assign a public predecessor");
    store
        .upsert_course_member(
            foreign_context,
            UpsertCourseMember {
                course: foreign_course,
                user: foreign_student,
                display_name: "Foreign issued-evidence student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("foreign student enrollment should derive from the assignment");
    let foreign_run = store
        .start_or_resume_run(
            foreign_context,
            foreign_student,
            foreign_assignment,
            RunId::from_uuid(id()),
        )
        .await
        .expect("foreign run should snapshot the predecessor before correction");

    let mut held_lock = begin_app_tenant(&pool, foreign_tenant).await;
    let locked_lifecycle: Option<String> =
        sqlx::query_scalar("SELECT public.ple_lock_assignable_problem_version($1, $2)")
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .fetch_one(&mut *held_lock)
            .await
            .expect("foreign public assignment path obtains a broker-held share lock");
    assert_eq!(locked_lifecycle.as_deref(), Some("published"));
    let mut competing_lock = begin_app_tenant(&pool, owner_tenant).await;
    let contention = sqlx::query(
        "SELECT 1 FROM problem_version \
         WHERE problem_id = $1 AND version_id = $2 FOR UPDATE NOWAIT",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .execute(&mut *competing_lock)
    .await
    .expect_err("broker-held share lock blocks a concurrent version update without waiting");
    assert_eq!(
        contention
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("55P03"),
        "NOWAIT observes the broker-held FOR SHARE lock"
    );
    competing_lock
        .rollback()
        .await
        .expect("rollback competing lock probe");
    held_lock
        .rollback()
        .await
        .expect("release broker-held share lock before correction");

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
        .publish_owner_correction(
            owner_context,
            OwnerCorrectionAuthority {
                actor: shared_user,
                session: owner_session,
            },
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

    let mut foreign_evidence = begin_app_tenant(&pool, foreign_tenant).await;
    let assignment_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(foreign_assignment.as_uuid())
    .fetch_one(&mut *foreign_evidence)
    .await
    .expect("read foreign assignment revision after correction");
    let item_version: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM assignment_item WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(foreign_assignment.as_uuid())
    .fetch_one(&mut *foreign_evidence)
    .await
    .expect("read foreign fixed item after correction");
    let candidate_version: Uuid = sqlx::query_scalar(
        "SELECT candidate.version_id FROM assignment_selection_candidate AS candidate \
         JOIN assignment_selection_group AS group_row \
           ON group_row.tenant_id = candidate.tenant_id \
          AND group_row.selection_group_id = candidate.selection_group_id \
         WHERE candidate.tenant_id = $1 AND group_row.assignment_id = $2",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(foreign_assignment.as_uuid())
    .fetch_one(&mut *foreign_evidence)
    .await
    .expect("read foreign selection candidate after correction");
    let issued_predecessor_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND problem_id = $3 AND version_id = $4",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(foreign_run.id.as_uuid())
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .fetch_one(&mut *foreign_evidence)
    .await
    .expect("read immutable foreign run evidence after correction");
    let foreign_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event \
         WHERE tenant_id = $1 AND action = 'catalog.ownerCorrectionPropagated' \
           AND target_kind = 'assignment' AND target_id = $2 AND actor_id = $3 \
           AND payload ->> 'predecessorVersionId' = $4 \
           AND payload ->> 'successorVersionId' = $5 \
           AND payload ->> 'questionId' = $6 AND payload ->> 'assignmentId' = $2::text",
    )
    .bind(foreign_tenant.as_uuid())
    .bind(foreign_assignment.as_uuid())
    .bind(shared_user.as_uuid())
    .bind(version.as_uuid().to_string())
    .bind(owner_successor_version.as_uuid().to_string())
    .bind(owner_successor.question_id.compact())
    .fetch_one(&mut *foreign_evidence)
    .await
    .expect("read foreign correction audit");
    foreign_evidence
        .commit()
        .await
        .expect("commit read-only foreign correction evidence");
    assert_eq!(
        assignment_revision, 2,
        "one correction advances assignment revision once"
    );
    assert_eq!(item_version, owner_successor_version.as_uuid());
    assert_eq!(candidate_version, owner_successor_version.as_uuid());
    assert_eq!(
        issued_predecessor_rows, 2,
        "issued run items retain predecessor evidence"
    );
    assert_eq!(
        foreign_audits, 1,
        "affected assignment receives one correction audit"
    );
    assert!(
        store
            .get_catalog_problem(
                foreign_context,
                ProblemVersionRef {
                    problem,
                    version: owner_successor_version,
                },
            )
            .await
            .expect("foreign successor visibility lookup")
            .is_some(),
        "public successor remains visible to the foreign course tenant"
    );
    let mut owner_audit = begin_app_tenant(&pool, owner_tenant).await;
    let owner_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event \
         WHERE tenant_id = $1 AND action = 'catalog.ownerCorrectionPublished' \
           AND target_kind = 'problemVersion' AND target_id = $2 AND actor_id = $3 \
           AND payload ->> 'predecessorVersionId' = $4 \
           AND payload ->> 'successorVersionId' = $5 \
           AND payload ->> 'questionId' = $6",
    )
    .bind(owner_tenant.as_uuid())
    .bind(owner_successor_version.as_uuid())
    .bind(shared_user.as_uuid())
    .bind(version.as_uuid().to_string())
    .bind(owner_successor_version.as_uuid().to_string())
    .bind(owner_successor.question_id.compact())
    .fetch_one(&mut *owner_audit)
    .await
    .expect("read owner correction audit");
    owner_audit
        .commit()
        .await
        .expect("commit owner correction audit read");
    assert_eq!(
        owner_audits, 1,
        "owner correction writes durable source audit evidence"
    );

    let corrected_predecessor = store
        .get_catalog_problem(owner_context, reference)
        .await
        .expect("read owner correction predecessor")
        .expect("owner correction predecessor remains addressable");
    assert!(matches!(
        corrected_predecessor.lifecycle,
        CatalogLifecycle::Archived { .. }
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

    let lifecycle_problem = ProblemId::from_uuid(id());
    let lifecycle_version = VersionId::from_uuid(id());
    let lifecycle_reference = ProblemVersionRef {
        problem: lifecycle_problem,
        version: lifecycle_version,
    };
    let lifecycle_draft = draft(owner_tenant, WorkspaceId::from_uuid(id()), None);
    let saved_lifecycle = store
        .upsert_draft(owner_context, shared_user, None, lifecycle_draft.clone())
        .await
        .expect("save independent lifecycle draft");
    store
        .publish_draft(
            owner_context,
            shared_user,
            publication(
                lifecycle_draft,
                saved_lifecycle.revision,
                lifecycle_reference,
                shared_user,
            ),
        )
        .await
        .expect("publish independent lifecycle version");
    let deprecated = store
        .transition_catalog_problem(
            owner_context,
            shared_user,
            lifecycle_reference,
            CatalogTransition::Deprecate {
                reason: "owner lifecycle review".to_string(),
            },
        )
        .await
        .expect("owner tenant deprecates a version without a successor");
    assert!(matches!(
        deprecated.lifecycle,
        CatalogLifecycle::Deprecated { .. }
    ));
    let archived = store
        .transition_catalog_problem(
            owner_context,
            shared_user,
            lifecycle_reference,
            CatalogTransition::Archive,
        )
        .await
        .expect("owner tenant archives its independently deprecated version");
    assert!(matches!(
        archived.lifecycle,
        CatalogLifecycle::Archived { .. }
    ));
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_resolver_hides_foreign_institution_question_id() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let owner_tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let owner_context = TenantContext::from_authenticated_session(owner_tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(id());
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let owner_draft = draft(owner_tenant, WorkspaceId::from_uuid(id()), None);
    let saved_owner = store
        .upsert_draft(owner_context, publisher, None, owner_draft.clone())
        .await
        .expect("save institution-only draft");
    let published = store
        .publish_draft(
            owner_context,
            publisher,
            institution_publication(owner_draft, saved_owner.revision, reference, publisher),
        )
        .await
        .expect("publish institution-only problem");
    let display_reference = ProblemDisplayRef {
        question_id: published.question_id,
    };

    let mut student_read = begin_student_tenant(&pool, owner_tenant).await;
    let student_visible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM problem_version WHERE problem_id = $1 AND version_id = $2)",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .fetch_one(&mut *student_read)
    .await
    .expect("institution grant keeps the version visible to students");
    assert!(student_visible);
    student_read
        .rollback()
        .await
        .expect("rollback student visibility probe");

    let mut grader_read = begin_grader_tenant(&pool, owner_tenant).await;
    let grader_grant_visible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM catalog_tenant_grant \
         WHERE tenant_id = $1 AND problem_id = $2 AND version_id = $3)",
    )
    .bind(owner_tenant.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .fetch_one(&mut *grader_read)
    .await
    .expect("institution grant remains available to the grader visibility policy");
    assert!(grader_grant_visible);
    let grader_visible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM problem_version WHERE problem_id = $1 AND version_id = $2)",
    )
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .fetch_one(&mut *grader_read)
    .await
    .expect("institution grant keeps the version visible to graders");
    assert!(grader_visible);
    grader_read
        .rollback()
        .await
        .expect("rollback grader visibility probe");

    assert_eq!(
        store
            .resolve_catalog_problem(owner_context, display_reference.clone())
            .await
            .expect("owner Question ID lookup should run")
            .map(|record| (record.problem, record.version)),
        Some((reference.problem, reference.version)),
        "one Question ID resolves the current published question"
    );
    assert_eq!(
        store
            .resolve_catalog_problem(foreign_context, display_reference)
            .await
            .expect("foreign Question ID lookup should run"),
        None,
        "a foreign tenant must not discover institution-only content through its Question ID"
    );
    let mut foreign_lock = begin_app_tenant(&pool, foreign_tenant).await;
    let denied_lifecycle: Option<String> =
        sqlx::query_scalar("SELECT public.ple_lock_assignable_problem_version($1, $2)")
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .fetch_one(&mut *foreign_lock)
            .await
            .expect("lock capability query should fail closed for hidden institution content");
    foreign_lock
        .rollback()
        .await
        .expect("rollback foreign lock denial probe");
    assert_eq!(denied_lifecycle, None);
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_search_finds_exact_question_id() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool, [0x42; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id());
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let source = draft(tenant, WorkspaceId::from_uuid(id()), None);
    let saved = store
        .upsert_draft(context, publisher, None, source.clone())
        .await
        .expect("save exact-search draft");
    let published = store
        .publish_draft(
            context,
            publisher,
            publication(source, saved.revision, reference, publisher),
        )
        .await
        .expect("publish exact-search problem");
    let text_query = CatalogSearchQuery {
        text: Some("molar".to_string()),
        ..CatalogSearchQuery::default()
    };
    let text_page = store
        .search_catalog(context, text_query)
        .await
        .expect("ordinary catalog text search must execute");
    assert!(
        text_page
            .items
            .iter()
            .any(|item| item.problem == reference.problem && item.version == reference.version),
        "ordinary text search must include the row published by this test"
    );

    let exact_query = CatalogSearchQuery {
        text: Some(published.question_id.to_string()),
        ..CatalogSearchQuery::default()
    };

    let page = store
        .search_catalog(context, exact_query)
        .await
        .expect("exact human catalog search must execute");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].problem, reference.problem);
    assert_eq!(page.items[0].version, reference.version);
}
