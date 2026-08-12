use super::*;
use axum::body::to_bytes;
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CatalogStore, DraftRecord, PublishDraftCommand, SessionLifetime, SessionSubject, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    BackendCapabilities, Capability, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ProblemId, PublicationScope, QuestionMetadata, QuestionSource, TenantId,
    UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

pub(super) fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

pub(super) async fn issued_cookie(
    store: &MemoryStore,
    roles: Vec<UserRole>,
    user: UserId,
) -> String {
    issued_cookie_for_tenant(store, TenantId::from_uuid(id(1)), roles, user).await
}

pub(super) async fn issued_cookie_for_tenant(
    store: &MemoryStore,
    tenant: TenantId,
    roles: Vec<UserRole>,
    user: UserId,
) -> String {
    let subject =
        SessionSubject::new(tenant, user, "Course Fixture", roles).expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

pub(super) async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

pub(super) async fn publish_fixture(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> ProblemVersionRef {
    let problem = ProblemId::from_uuid(id(20));
    let version = VersionId::from_uuid(id(21));
    let workspace = WorkspaceId::from_uuid(id(22));
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "course-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is a peptide bond?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateFull,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide bond fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft save");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: question_model::ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: "course-fixture".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("fixture publication");
    ProblemVersionRef { problem, version }
}

pub(super) fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}
