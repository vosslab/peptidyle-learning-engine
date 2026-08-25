use learning_data_access::postgres::PostgresStore;
use learning_data_access::{
    CatalogStore, DraftRecord, PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject,
    SessionTokenHash, Store, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    BackendCapabilities, Capability, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ProblemId, ProblemVersionRef, PublicationScope, QuestionId,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, UserRole, VersionId,
    WorkspaceId,
};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

pub(super) struct Fixture {
    pub(super) pool: PgPool,
    pub(super) store: PostgresStore,
    pub(super) tenant: TenantId,
    pub(super) context: TenantContext,
    pub(super) other_context: TenantContext,
    pub(super) elena: UserId,
    pub(super) morgan: UserId,
    pub(super) ada: UserId,
    pub(super) elena_session: SessionTokenHash,
    pub(super) morgan_session: SessionTokenHash,
    pub(super) ada_session: SessionTokenHash,
    pub(super) public_questions: Vec<QuestionId>,
    pub(super) bulk_public_questions: Vec<QuestionId>,
    pub(super) institution_question: QuestionId,
    pub(super) institution_reference: ProblemVersionRef,
}

impl Fixture {
    pub(super) async fn new(pool: PgPool) -> Self {
        let store = PostgresStore::with_question_id_secret(pool.clone(), [0xD2; 32]);
        let tenant = TenantId::from_uuid(id());
        let other_tenant = TenantId::from_uuid(id());
        let context = TenantContext::from_authenticated_session(tenant);
        let other_context = TenantContext::from_authenticated_session(other_tenant);
        let elena = UserId::from_uuid(id());
        let morgan = UserId::from_uuid(id());
        let ada = UserId::from_uuid(id());
        for (user, label, approved) in [
            (elena, "Elena Instructor", true),
            (morgan, "Morgan Sysadmin", false),
            (ada, "Ada Instructor", true),
        ] {
            let email = format!("d2-{}@example.test", user.as_uuid().simple());
            sqlx::query(
                "INSERT INTO ple_account (user_id, normalized_email, delivery_email, display_name) \
                 VALUES ($1, $2, $2, $3)",
            )
            .bind(user.as_uuid())
            .bind(email)
            .bind(label)
            .execute(&pool)
            .await
            .expect("D2 fixture account");
            if approved {
                sqlx::query(
                    "INSERT INTO instructor_approval (user_id, approved_by, approved_at, revision) \
                     VALUES ($1, $1, transaction_timestamp(), 1)",
                )
                .bind(user.as_uuid())
                .execute(&pool)
                .await
                .expect("D2 fixture instructor approval");
            }
        }
        let elena_session = session(
            &store,
            tenant,
            elena,
            "Elena Instructor",
            vec![UserRole::Instructor, UserRole::Sysadmin],
        )
        .await;
        let morgan_session = session(
            &store,
            tenant,
            morgan,
            "Morgan Sysadmin",
            vec![UserRole::Instructor, UserRole::Sysadmin],
        )
        .await;
        let ada_session = session(
            &store,
            tenant,
            ada,
            "Ada Instructor",
            vec![UserRole::Instructor],
        )
        .await;
        let public_one = publish(
            &store,
            context,
            elena,
            "D2 public enzyme kinetics",
            PublicationScope::Public,
        )
        .await;
        let public_two = publish(
            &store,
            context,
            elena,
            "D2 public protein folding",
            PublicationScope::Public,
        )
        .await;
        let institution = publish(
            &store,
            context,
            elena,
            "D2 institution-only spectroscopy",
            PublicationScope::Institution,
        )
        .await;
        let mut bulk_public_questions = Vec::with_capacity(200);
        for index in 0..200 {
            let published = publish(
                &store,
                context,
                elena,
                &format!("D2 bulk public question {index}"),
                PublicationScope::Public,
            )
            .await;
            bulk_public_questions.push(published.question_id.clone());
        }
        Self {
            pool,
            store,
            tenant,
            context,
            other_context,
            elena,
            morgan,
            ada,
            elena_session,
            morgan_session,
            ada_session,
            public_questions: vec![
                public_one.question_id.clone(),
                public_two.question_id.clone(),
            ],
            bulk_public_questions,
            institution_question: institution.question_id.clone(),
            institution_reference: ProblemVersionRef {
                problem: institution.problem,
                version: institution.version,
            },
        }
    }

    pub(super) async fn cleanup(&self) {
        // Mutable curation and session state is removed explicitly. Published
        // questions and their account provenance remain immutable until the
        // acceptance owner destroys this oracle's disposable database volume.
        sqlx::query("DELETE FROM saved_problem_search WHERE owner_tenant_id=$1")
            .bind(self.tenant.as_uuid())
            .execute(&self.pool)
            .await
            .expect("D2 saved-search cleanup");
        sqlx::query("DELETE FROM problem_collection WHERE owner_tenant_id=$1")
            .bind(self.tenant.as_uuid())
            .execute(&self.pool)
            .await
            .expect("D2 collection cleanup");
        sqlx::query("DELETE FROM workspace_draft WHERE tenant_id=$1")
            .bind(self.tenant.as_uuid())
            .execute(&self.pool)
            .await
            .expect("D2 draft cleanup");
        for user in [self.elena, self.morgan, self.ada] {
            sqlx::query("DELETE FROM auth_session WHERE tenant_id=$1 AND user_id=$2")
                .bind(self.tenant.as_uuid())
                .bind(user.as_uuid())
                .execute(&self.pool)
                .await
                .expect("D2 session cleanup");
            sqlx::query("DELETE FROM instructor_approval WHERE user_id=$1")
                .bind(user.as_uuid())
                .execute(&self.pool)
                .await
                .expect("D2 approval cleanup");
        }
        let remaining: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM problem_collection WHERE owner_tenant_id=$1) + \
             (SELECT count(*) FROM saved_problem_search WHERE owner_tenant_id=$1) + \
             (SELECT count(*) FROM auth_session WHERE tenant_id=$1 AND user_id=ANY($2))",
        )
        .bind(self.tenant.as_uuid())
        .bind(vec![
            self.elena.as_uuid(),
            self.morgan.as_uuid(),
            self.ada.as_uuid(),
        ])
        .fetch_one(&self.pool)
        .await
        .expect("D2 cleanup count");
        assert_eq!(
            remaining, 0,
            "D2 live fixture leaves no curation/session state"
        );
    }
}

async fn session(
    store: &PostgresStore,
    tenant: TenantId,
    user: UserId,
    label: &str,
    roles: Vec<UserRole>,
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, label, roles).expect("D2 session subject"),
            SessionLifetime::from_seconds(3_600).expect("D2 session lifetime"),
        )
        .await
        .expect("D2 session");
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
                family: "d2_live".into(),
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
                tags: vec![Tag::new("d2")],
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".into(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, author, None, draft.clone())
        .await
        .expect("D2 draft");
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
                    family: "d2_live".into(),
                },
                publisher: author,
                scope,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("D2 fixture".into()).expect("byline"),
                ])
                .expect("byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("D2 publication")
}
