//! Backend-neutral S5-plus-S3 resolution parity for Store conformance.

use domain::effective_assignment_policy::{
    AssignmentLifecycleGate, AuthorizationGate, BaseAssignmentPolicy, EffectivePolicyDecision,
    IndividualPolicyException, PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource,
};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand,
    DraftRecord, PublishDraftCommand, PutBaseAssignmentPolicyCommand,
    PutIndividualPolicyExceptionCommand, ResolveEffectivePolicyCommand, Store,
    StoredIndividualPolicyException, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentScoringMode, BackendCapabilities, Capability, CourseId,
    CourseTerm, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
    LateSubmissionPolicy, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, VersionId, WorkspaceId,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn published_reference<S>(
    store: &S,
    context: TenantContext,
    tenant: TenantId,
    author: UserId,
) -> ProblemVersionRef
where
    S: Store + CatalogStore,
{
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(99_510)),
        version: VersionId::from_uuid(id(99_511)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id(99_512)),
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Parity fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, author, None, draft.clone())
        .await
        .expect("save parity draft");
    store
        .publish_draft(
            context,
            author,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: author,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid byline"),
                ])
                .expect("valid byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish parity draft");
    reference
}

/// Runs the same supplied-S5-input resolution contract against every Store.
/// It intentionally owns no SQL, browser, clock, or receipt-history behavior.
pub(crate) async fn exercise_effective_policy_resolution_parity<S>(store: &S)
where
    S: Store + CatalogStore + CourseRosterStore,
{
    let tenant = TenantId::from_uuid(id(99_500));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(99_501));
    let learner = UserId::from_uuid(id(99_502));
    let course = CourseId::from_uuid(id(99_503));
    let assignment = AssignmentId::from_uuid(id(99_504));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Effective policy parity".to_string(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("valid parity term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("create parity course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "Parity learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create parity learner");
    let student = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("read parity learner")
        .expect("parity learner exists")
        .student
        .expect("parity learner identity");
    let reference = published_reference(store, context, tenant, instructor).await;
    let created = store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Parity assignment".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(99_513)),
                    reference,
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: question_model::RunPolicies {
                    completion: question_model::CompletionRequirement::AnswerAll,
                    grade: question_model::GradePolicy::Highest,
                    continued_practice: question_model::ContinuedPractice::Unlimited,
                    variation: question_model::VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("create parity assignment");
    let revised = store
        .put_individual_policy_exception(
            context,
            PutIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(id(99_505)),
                    exception: IndividualPolicyException {
                        student,
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet {
                            time_limit_seconds: PolicyPatch::Set(
                                NonZeroU32::new(300).expect("positive parity limit"),
                            ),
                            ..PolicyPatchSet::INHERIT
                        },
                    },
                },
            },
        )
        .await
        .expect("store parity M4 record");
    store
        .put_base_assignment_policy(
            context,
            PutBaseAssignmentPolicyCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                policy: BaseAssignmentPolicy {
                    available_at: None,
                    due_at: None,
                    closes_at: None,
                    time_limit_seconds: Some(NonZeroU32::new(120).expect("positive base limit")),
                    attempt_limit: None,
                    late_submission: LateSubmissionPolicy::Accept,
                    deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                },
            },
        )
        .await
        .expect("store parity M1 record");
    let entitlement = store
        .evaluate_assignment_entitlement(context, learner, course, assignment)
        .await
        .expect("evaluate parity S5 grant");
    let resolved = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment,
                lifecycle: AssignmentLifecycleGate::Open,
                entitlement,
                authorization: AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(0),
                prior_run_count: 0,
            },
        )
        .await
        .expect("resolve parity policy")
        .expect("parity assignment exists");
    let EffectivePolicyDecision::Allowed { policy, .. } = resolved.decision else {
        panic!("granted parity learner resolves an allowed policy");
    };
    assert_eq!(
        policy.time_limit_seconds.value,
        Some(NonZeroU32::new(300).expect("positive M4 limit"))
    );
    assert_eq!(
        policy.time_limit_seconds.source,
        PolicySource::IndividualException(student)
    );
}
