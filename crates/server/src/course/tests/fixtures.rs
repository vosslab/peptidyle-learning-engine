use super::*;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CatalogStore, DraftRecord, PublishDraftCommand, PutAssignmentTeachingSettingsCommand,
    SessionLifetime, SessionSubject, Store, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, TimingPolicy,
    VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentId, AssignmentTeachingSettings, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ProblemId, PublicationScope,
    QuestionMetadata, QuestionSource, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

pub(super) fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
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
            crate::auth::CookieTransport::FirstPartyHttps,
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

pub(crate) async fn publish_fixture(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> ProblemVersionRef {
    publish_fixture_with_identity(store, context, tenant, publisher, 20).await
}

pub(super) async fn publish_fixture_with_identity(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
    identity: u128,
) -> ProblemVersionRef {
    let problem = ProblemId::from_uuid(id(identity));
    let version = VersionId::from_uuid(id(identity + 1));
    let workspace = WorkspaceId::from_uuid(id(identity + 2));
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
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: format!("Peptide bond fixture {identity}"),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
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
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("fixture publication");
    ProblemVersionRef { problem, version }
}

pub(crate) fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

/// Publishes a freshly created draft through the public revision-checked Store
/// command. Test fixtures use this rather than constructing published rows.
pub(crate) async fn publish_assignment(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    settings: AssignmentTeachingSettings,
) {
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("fixture assignment read")
        .expect("fixture assignment exists");
    assert_eq!(
        stored.record.lifecycle,
        question_model::AssignmentLifecycle::Draft
    );
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor,
                course,
                assignment,
                expected_revision: stored.revision,
                settings,
            },
        )
        .await
        .expect("fixture assignment publish");
}
