use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CatalogStore, DraftRecord, PublishDraftCommand, ReplaceAlphaCourseCommand,
    ReplaceBlueprintCommand, ReusableCurriculumStore, SessionLifetime, SessionStore,
    SessionSubject, SessionTokenHash, Store, TenantContext,
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
    LateSubmissionPolicy, LearnerDisclosurePolicy, PointValue, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, ResponseDefinition,
    ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput, ReusableAssignmentEntryInput,
    ReusableFixedQuestionInput, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "../support/acceptance_runtime.rs"]
mod acceptance_runtime;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn account(pool: &PgPool, user: UserId, name: &str, approved: bool) {
    let email = format!("b2-{}@example.test", user.as_uuid().simple());
    sqlx::query(
        "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
         VALUES ($1, $2, $2, $3)",
    )
    .bind(user.as_uuid())
    .bind(email)
    .bind(name)
    .execute(pool)
    .await
    .expect("B2 fixture account");
    if approved {
        sqlx::query(
            "INSERT INTO instructor_approval (user_id, approved_by, approved_at, revision) \
             VALUES ($1, $1, transaction_timestamp(), 1)",
        )
        .bind(user.as_uuid())
        .execute(pool)
        .await
        .expect("B2 fixture approval");
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
        .expect("B2 fixture session");
    token
}

pub(super) fn definition(
    question_id: question_model::QuestionId,
    title: &str,
) -> BlueprintDefinitionInput {
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
                learner_disclosure: LearnerDisclosurePolicy::default(),
            },
            schedule: Default::default(),
        },
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    author: UserId,
) -> learning_data_access::PublishedProblemRecord {
    let draft = DraftRecord {
        tenant: context.tenant_id(),
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native {
                family: "b2_live".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "B2 public fixture".into(),
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
                title: "B2 public fixture".into(),
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
        .expect("B2 fixture draft");
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
                    family: "b2_live".into(),
                },
                publisher: author,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("B2 fixture".into())
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
        .expect("B2 fixture publication")
}

pub(super) struct AdoptionFixture {
    pub(super) pool: PgPool,
    pub(super) store: PostgresStore,
    pub(super) tenant: TenantId,
    pub(super) context: TenantContext,
    pub(super) instructor: UserId,
    pub(super) instructor_session: SessionTokenHash,
    pub(super) foreign_tenant: TenantId,
    pub(super) foreign_context: TenantContext,
    pub(super) foreign_instructor: UserId,
    pub(super) foreign_instructor_session: SessionTokenHash,
    pub(super) learner_session: SessionTokenHash,
    pub(super) sysadmin_session: SessionTokenHash,
    pub(super) public_question: question_model::QuestionId,
    pub(super) alpha: question_model::ObservedAlphaSource,
    pub(super) blueprint: question_model::ObservedBlueprintSource,
}

impl AdoptionFixture {
    pub(super) fn reloaded_store(&self) -> PostgresStore {
        PostgresStore::with_question_id_secret(self.pool.clone(), [0xB2; 32])
    }

    pub(super) async fn bootstrap() -> Self {
        let runtime = acceptance_runtime::load();
        let pool = lazy_pool(runtime.admin_url().expose()).expect("live PostgreSQL URL");
        verify_application_schema(&pool)
            .await
            .expect("fully migrated B2 schema");
        let store = PostgresStore::with_question_id_secret(pool.clone(), [0xB2; 32]);
        let tenant = TenantId::from_uuid(id());
        let foreign_tenant = TenantId::from_uuid(id());
        let context = TenantContext::from_authenticated_session(tenant);
        let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
        let instructor = UserId::from_uuid(id());
        let foreign_instructor = UserId::from_uuid(id());
        let learner = UserId::from_uuid(id());
        let sysadmin = UserId::from_uuid(id());
        account(&pool, instructor, "B2 Source Instructor", true).await;
        account(&pool, foreign_instructor, "B2 Destination Instructor", true).await;
        account(&pool, learner, "B2 Learner", false).await;
        account(&pool, sysadmin, "B2 Unrelated Sysadmin", false).await;
        let instructor_session = session(
            &store,
            tenant,
            instructor,
            "B2 Source Instructor",
            vec![UserRole::Instructor],
        )
        .await;
        let foreign_instructor_session = session(
            &store,
            foreign_tenant,
            foreign_instructor,
            "B2 Destination Instructor",
            vec![UserRole::Instructor],
        )
        .await;
        let learner_session = session(
            &store,
            tenant,
            learner,
            "B2 Learner",
            vec![UserRole::Student],
        )
        .await;
        let sysadmin_session = session(
            &store,
            tenant,
            sysadmin,
            "B2 Unrelated Sysadmin",
            vec![UserRole::Sysadmin],
        )
        .await;
        let public = publish(&store, context, instructor).await;
        let public_question = public.question_id.clone();
        let blueprint = store
            .replace_blueprint(
                context,
                instructor_session,
                ReplaceBlueprintCommand {
                    reference: None,
                    expected_revision: None,
                    definition: definition(public_question.clone(), "B2 reusable Blueprint"),
                },
            )
            .await
            .expect("B2 Blueprint source");
        let alpha = store
            .replace_alpha_course(
                context,
                instructor_session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: AlphaCourseDefinitionInput {
                        title: "B2 public Alpha".into(),
                        modules: vec![AlphaCourseModuleInput {
                            label: "B2 module".into(),
                            definitions: vec![
                                definition(public_question.clone(), "B2 reusable Alpha assignment")
                                    .definition,
                            ],
                        }],
                    },
                },
            )
            .await
            .expect("B2 Alpha source");
        Self {
            pool,
            store,
            tenant,
            context,
            instructor,
            instructor_session,
            foreign_tenant,
            foreign_context,
            foreign_instructor,
            foreign_instructor_session,
            learner_session,
            sysadmin_session,
            public_question,
            alpha: question_model::ObservedAlphaSource {
                reference: alpha.reference,
                revision: alpha.revision,
            },
            blueprint: question_model::ObservedBlueprintSource {
                reference: blueprint.reference,
                revision: blueprint.revision,
            },
        }
    }
}
