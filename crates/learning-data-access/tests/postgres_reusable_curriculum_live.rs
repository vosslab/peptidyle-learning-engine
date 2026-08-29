#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for the B1 reusable-curriculum broker.
//!
//! This ignored test uses the application Store for the behavior under test.
//! It inspects only the migration-owned authority boundary directly.

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;

use acceptance_runtime::load as load_acceptance_runtime;
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CatalogStore, CatalogTransition, DraftRecord, PageRequest, PageSize, PublishDraftCommand,
    ReplaceAlphaCourseCommand, ReplaceBlueprintCommand, ReusableCurriculumCapability,
    ReusableCurriculumStore, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    Store, StoreError, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AssignmentDeadlineBehavior,
    AssignmentInstructions, AssignmentScoringMode, BackendCapabilities, BlueprintDefinitionInput,
    Capability, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
    LateSubmissionPolicy, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentEntryInput, ReusableFixedQuestionInput,
    StudentDisclosurePolicy, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn definition(question_id: question_model::QuestionId, title: &str) -> BlueprintDefinitionInput {
    BlueprintDefinitionInput {
        definition: ReusableAssignmentDefinitionInput {
            title: title.to_string(),
            instructions: AssignmentInstructions::try_new("Explain your reasoning.".to_string())
                .expect("valid fixture instructions"),
            entries: vec![ReusableAssignmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    question_id,
                    points_possible: PointValue::from_whole(3),
                    scoring_mode: AssignmentScoringMode::Normal,
                },
            )],
            defaults: ReusableAssignmentDefaults {
                time_limit_seconds: None,
                attempt_limit: None,
                late_submission: LateSubmissionPolicy::Accept,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                run_policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
                student_disclosure: StudentDisclosurePolicy::default(),
            },
            schedule: Default::default(),
        },
    }
}

async fn account(pool: &PgPool, user: UserId, name: &str, approved: bool) {
    let email = format!("b1-{}@example.test", user.as_uuid().simple());
    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, $3)",
    )
    .bind(user.as_uuid())
    .bind(email)
    .bind(name)
    .execute(pool)
    .await
    .expect("B1 fixture account");
    if approved {
        sqlx::query(
            "INSERT INTO instructor_approval (user_id, approved_by, approved_at, revision) \
             VALUES ($1, $1, transaction_timestamp(), 1)",
        )
        .bind(user.as_uuid())
        .execute(pool)
        .await
        .expect("B1 fixture approval");
    }
}

async fn session(
    store: &PostgresStore,
    tenant: TenantId,
    user: UserId,
    name: &str,
    roles: Vec<UserRole>,
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, name, roles).expect("fixture session subject"),
            SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
        )
        .await
        .expect("B1 fixture session");
    token
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    author: UserId,
    title: &str,
    scope: PublicationScope,
) -> learning_data_access::PublishedProblemRecord {
    let draft = DraftRecord {
        tenant: context.tenant_id(),
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native {
                family: "b1_live".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: title.into(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: title.into(),
                tags: vec![],
                taxonomy: vec![],
                license: question_model::taxonomy::License::CcBy,
                language: "en-US".into(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, author, None, draft.clone())
        .await
        .expect("B1 fixture draft");
    store
        .publish_draft(
            context,
            author,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef {
                    problem: ProblemId::from_uuid(id()),
                    version: VersionId::from_uuid(id()),
                },
                published_source: QuestionSource::Native {
                    family: "b1_live".into(),
                },
                publisher: author,
                scope,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("B1 fixture".into())
                        .expect("fixture byline"),
                ])
                .expect("fixture byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("B1 fixture publication")
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_reusable_curriculum_live_oracle_is_brokered_and_atomic() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("fully migrated B1 schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0xB1; 32]);
    let tenant = TenantId::from_uuid(id());
    let reader_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let reader_context = TenantContext::from_authenticated_session(reader_tenant);
    let elena = UserId::from_uuid(id());
    let ada = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    account(&pool, elena, "Elena Instructor", true).await;
    account(&pool, ada, "Ada Instructor", true).await;
    account(&pool, learner, "Learner", false).await;
    let elena_session = session(
        &store,
        tenant,
        elena,
        "Elena Instructor",
        vec![UserRole::Instructor],
    )
    .await;
    let ada_session = session(
        &store,
        reader_tenant,
        ada,
        "Ada Instructor",
        vec![UserRole::Instructor],
    )
    .await;
    let learner_session =
        session(&store, tenant, learner, "Learner", vec![UserRole::Student]).await;
    let public = publish(
        &store,
        context,
        elena,
        "B1 public question",
        PublicationScope::Public,
    )
    .await;
    let institution = publish(
        &store,
        context,
        elena,
        "B1 institution question",
        PublicationScope::Institution,
    )
    .await;

    let created = store
        .replace_blueprint(
            context,
            elena_session,
            ReplaceBlueprintCommand {
                reference: None,
                expected_revision: None,
                definition: definition(public.question_id.clone(), "Protein blueprint"),
            },
        )
        .await
        .expect("Blueprint create");
    assert_eq!(created.revision.value(), 1);
    assert!(
        !serde_json::to_string(&created)
            .expect("safe view JSON")
            .contains("problemId")
    );
    let pin =
        sqlx::query("SELECT problem_id, version_id FROM course_blueprint_fixed WHERE tenant_id=$1")
            .bind(tenant.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("stored exact pin");
    let pin_problem: Uuid = pin.try_get("problem_id").expect("stored problem pin");
    let pin_version: Uuid = pin.try_get("version_id").expect("stored version pin");
    assert_eq!(pin_problem, public.problem.as_uuid());
    assert_eq!(pin_version, public.version.as_uuid());
    let no_op = store
        .replace_blueprint(
            context,
            elena_session,
            ReplaceBlueprintCommand {
                reference: Some(created.reference),
                expected_revision: Some(created.revision),
                definition: definition(public.question_id.clone(), "Protein blueprint"),
            },
        )
        .await
        .expect("semantic no-op");
    assert_eq!(no_op.revision, created.revision);
    let updated = store
        .replace_blueprint(
            context,
            elena_session,
            ReplaceBlueprintCommand {
                reference: Some(created.reference),
                expected_revision: Some(created.revision),
                definition: definition(public.question_id.clone(), "Revised protein blueprint"),
            },
        )
        .await
        .expect("revisioned update");
    assert_eq!(updated.revision.value(), 2);
    assert_eq!(
        store
            .replace_blueprint(
                context,
                elena_session,
                ReplaceBlueprintCommand {
                    reference: Some(created.reference),
                    expected_revision: Some(created.revision),
                    definition: definition(public.question_id.clone(), "Stale"),
                }
            )
            .await,
        Err(StoreError::Conflict)
    );

    let alpha = AlphaCourseDefinitionInput {
        title: "Public biochemistry sequence".to_string(),
        modules: vec![AlphaCourseModuleInput {
            label: "Week one".to_string(),
            definitions: vec![definition(public.question_id.clone(), "Catalysis").definition],
        }],
    };
    let alpha = store
        .replace_alpha_course(
            context,
            elena_session,
            ReplaceAlphaCourseCommand {
                reference: None,
                expected_revision: None,
                definition: alpha,
            },
        )
        .await
        .expect("public Alpha create");
    assert_eq!(
        store
            .list_alpha_courses(
                reader_context,
                ada_session,
                PageRequest::first(PageSize::new(10).expect("page size"))
            )
            .await
            .expect("cross-tenant instructor Alpha read")
            .items[0]
            .reference,
        alpha.reference
    );
    assert_eq!(
        store
            .replace_alpha_course(
                reader_context,
                ada_session,
                ReplaceAlphaCourseCommand {
                    reference: Some(alpha.reference),
                    expected_revision: Some(alpha.revision),
                    definition: AlphaCourseDefinitionInput {
                        title: "Attempted foreign replacement".to_string(),
                        modules: vec![AlphaCourseModuleInput {
                            label: "Week one".to_string(),
                            definitions: vec![
                                definition(public.question_id.clone(), "Catalysis").definition
                            ],
                        }],
                    },
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    assert_eq!(
        store
            .preflight_reusable_curriculum(
                context,
                learner_session,
                ReusableCurriculumCapability::AlphaRead
            )
            .await,
        Err(StoreError::Forbidden)
    );
    let rejected = AlphaCourseDefinitionInput {
        title: "Invalid private Alpha".to_string(),
        modules: vec![AlphaCourseModuleInput {
            label: "Week one".to_string(),
            definitions: vec![definition(institution.question_id, "Private").definition],
        }],
    };
    assert!(matches!(
        store
            .replace_alpha_course(
                context,
                elena_session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: rejected,
                }
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_alpha_course(context, elena_session, alpha.reference)
            .await
            .expect("Alpha read after rejected replacement")
            .expect("created Alpha remains after rejected replacement")
            .revision,
        alpha.revision
    );
    store
        .transition_catalog_problem(
            context,
            elena,
            ProblemVersionRef {
                problem: public.problem,
                version: public.version,
            },
            CatalogTransition::Deprecate {
                reason: "Superseded by a corrected question".to_string(),
            },
        )
        .await
        .expect("deprecate retained exact publication");
    let retained = store
        .get_blueprint(context, elena_session, created.reference)
        .await
        .expect("retained Blueprint read")
        .expect("Blueprint remains readable");
    let question = match &retained.definition.entries[0] {
        question_model::ReusableAssignmentEntryView::Fixed { question, .. } => question,
        question_model::ReusableAssignmentEntryView::Pool(_) => panic!("fixture has a fixed item"),
    };
    assert_eq!(
        question.selection_availability,
        question_model::ReusableSelectionAvailability::Retained,
        "exact retained pin stays inspectable after deprecation"
    );

    let bypass: bool = sqlx::query_scalar(
        "SELECT rolbypassrls FROM pg_roles WHERE rolname='ple_reusable_curriculum_broker'",
    )
    .fetch_one(&pool)
    .await
    .expect("broker flags");
    assert!(!bypass, "broker remains non-bypassing");
    let forced: bool = sqlx::query_scalar(
        "SELECT relforcerowsecurity FROM pg_class WHERE oid='public.course_blueprint'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("forced RLS flag");
    assert!(forced, "Blueprint relation forces RLS");
    let app_can_call_broker: bool = sqlx::query_scalar(
        "SELECT has_function_privilege('ple_app', \
         'public.ple_get_curriculum_blueprint_v1(uuid,character,integer)', 'EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("application broker capability");
    assert!(
        app_can_call_broker,
        "application role has the fixed broker capability"
    );
    let mut transaction = pool.begin().await.expect("direct-DML probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application role");
    assert!(
        sqlx::query("SELECT * FROM public.course_blueprint")
            .fetch_all(&mut *transaction)
            .await
            .is_err(),
        "application role has no direct aggregate table read"
    );
    assert!(
        sqlx::query("UPDATE public.course_blueprint SET revision = revision")
            .execute(&mut *transaction)
            .await
            .is_err(),
        "application role has no direct aggregate table mutation"
    );
    transaction.rollback().await.expect("probe rollback");
}
