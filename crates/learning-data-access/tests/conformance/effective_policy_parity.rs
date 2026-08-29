//! Backend-neutral S5-plus-S3 resolution parity for Store conformance.

use super::sysadmin_course_creation_authority;
use domain::effective_assignment_policy::{
    AuthorizationGate, BaseAssignmentPolicy, EffectivePolicyDecision, IndividualPolicyException,
    PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource,
};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateAssignmentCommand,
    CreateCourseCommand, DraftRecord, IssueQuestionAttemptCommand, PresentationCapability,
    PublishDraftCommand, PutAssignmentTeachingSettingsCommand, PutIndividualPolicyExceptionCommand,
    ResolveEffectivePolicyCommand, SessionStore, Store, StoreError,
    StoredIndividualPolicyException, StudentWorkRoutingBinding, SubmissionIdempotencyKey,
    SubmitQuestionAttemptCommand, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentScoringMode, AttemptProvenance, AttemptResult,
    BackendCapabilities, Capability, CourseId, CourseTerm, DraftQuestionDefinition,
    DraftQuestionSource, FeedbackContent, GradingDefinition, ImplementationVersion,
    LateSubmissionPolicy, MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
    PointValue, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttemptId,
    QuestionDefinition, QuestionMetadata, QuestionSource, ResponseDefinition, StudentResponse,
    TenantId, UserId, VersionId, WorkspaceId,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

async fn published_reference<S>(
    store: &S,
    context: TenantContext,
    tenant: TenantId,
    author: UserId,
) -> (
    ProblemVersionRef,
    learning_data_access::IssuedQuestionSnapshotV1,
)
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
    let question = QuestionDefinition::from_draft(
        draft.question.clone(),
        reference.problem,
        reference.version,
        QuestionSource::Native {
            family: "molar_mass".to_string(),
        },
    );
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct parity native question snapshot");
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
    (reference, issued_question_snapshot)
}

/// Runs the same supplied-S5-input resolution contract against every Store.
/// It intentionally owns no SQL, browser, clock, or receipt-history behavior.
pub(crate) async fn exercise_effective_policy_resolution_parity<S>(store: &S)
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    let tenant = TenantId::from_uuid(id(99_500));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(99_501));
    let learner = UserId::from_uuid(id(99_502));
    let course = CourseId::from_uuid(id(99_503));
    let assignment = AssignmentId::from_uuid(id(99_504));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
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
                authority: course_creation_authority,
            },
        )
        .await
        .expect("create parity course");
    store
        .upsert_course_member(
            context,
            instructor,
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
    let (reference, issued_question_snapshot) =
        published_reference(store, context, tenant, instructor).await;
    let assignment_record = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "Parity assignment".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
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
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: question_model::RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: question_model::GradePolicy::Highest,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    for (offset, lifecycle) in [
        question_model::AssignmentLifecycle::Published,
        question_model::AssignmentLifecycle::Closed,
        question_model::AssignmentLifecycle::Archived,
    ]
    .into_iter()
    .enumerate()
    {
        let rejected_assignment = AssignmentId::from_uuid(id(99_600 + offset as u128));
        let rejected_record = AssignmentRecord {
            id: rejected_assignment,
            lifecycle,
            ..assignment_record.clone()
        };
        assert!(matches!(
            store
                .create_assignment(
                    context,
                    CreateAssignmentCommand {
                        actor: instructor,
                        assignment: rejected_record,
                        base_policy: BaseAssignmentPolicy::default(),
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(
            store
                .get_assignment(context, rejected_assignment)
                .await
                .expect("read rejected lifecycle assignment")
                .is_none()
        );
        assert!(
            store
                .get_base_assignment_policy(context, rejected_assignment)
                .await
                .expect("read rejected lifecycle policy")
                .is_none()
        );
    }
    assert!(matches!(
        store
            .create_assignment(
                context,
                CreateAssignmentCommand {
                    actor: instructor,
                    assignment: assignment_record.clone(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: Some(ActivityTimestamp::from_unix_millis(0)),
                        ..BaseAssignmentPolicy::default()
                    },
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .get_assignment(context, assignment)
            .await
            .expect("read rejected assignment")
            .is_none()
    );
    let created = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                assignment: assignment_record,
                base_policy: BaseAssignmentPolicy::default(),
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
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: None,
                        due_at: None,
                        closes_at: None,
                        time_limit_seconds: Some(
                            NonZeroU32::new(120).expect("positive base limit"),
                        ),
                        attempt_limit: NonZeroU32::new(1),
                        late_submission: LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
                },
            },
        )
        .await
        .expect("store parity M1 record");
    let current = store
        .get_base_assignment_policy(context, assignment)
        .await
        .expect("read persisted parity policy")
        .expect("parity policy exists");
    for policy in [
        BaseAssignmentPolicy {
            available_at: Some(ActivityTimestamp::from_unix_millis(0)),
            ..current.policy
        },
        BaseAssignmentPolicy {
            available_at: Some(ActivityTimestamp::from_unix_millis(2)),
            due_at: Some(ActivityTimestamp::from_unix_millis(1)),
            ..current.policy
        },
        BaseAssignmentPolicy {
            time_limit_seconds: NonZeroU32::new(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS + 1),
            ..current.policy
        },
        BaseAssignmentPolicy {
            attempt_limit: NonZeroU32::new(MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1),
            ..current.policy
        },
    ] {
        let result = store
            .put_assignment_teaching_settings(
                context,
                PutAssignmentTeachingSettingsCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: current.revision,
                    settings: question_model::AssignmentTeachingSettings {
                        lifecycle: question_model::AssignmentLifecycle::Published,
                        instructions: question_model::AssignmentInstructions::default(),
                        base_policy: policy,
                    },
                },
            )
            .await;
        assert!(matches!(result, Err(StoreError::InvalidRecord(_))));
        let after = store
            .get_base_assignment_policy(context, assignment)
            .await
            .expect("read policy after rejected edit")
            .expect("rejected edit retains policy");
        assert_eq!(after, current);
    }
    let postgres_boundary = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: current.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        time_limit_seconds: NonZeroU32::new(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS),
                        ..current.policy
                    },
                },
            },
        )
        .await
        .expect("accept PostgreSQL integer boundaries");
    assert_eq!(
        postgres_boundary.policy.time_limit_seconds,
        NonZeroU32::new(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
    );
    let entitlement = store
        .evaluate_assignment_entitlement(context, learner, course, assignment)
        .await
        .expect("evaluate parity S5 grant");
    let resolved = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment,
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

    let active = store
        .start_or_resume_run(
            context,
            learner,
            StudentWorkRoutingBinding::new(course, assignment),
            question_model::RunId::from_uuid(id(99_514)),
        )
        .await
        .expect("first limited run starts");
    let active_list = store
        .list_learner_entitled_assignments(
            context,
            learner,
            course,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(10).expect("bounded page"),
            ),
        )
        .await
        .expect("active limited assignment remains listed");
    assert!(
        active_list
            .items
            .iter()
            .any(|record| record.id == assignment)
    );
    let resumed = store
        .start_or_resume_run(
            context,
            learner,
            StudentWorkRoutingBinding::new(course, assignment),
            question_model::RunId::from_uuid(id(99_515)),
        )
        .await
        .expect("active final-allowed run resumes");
    assert_eq!(resumed, active);
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: learner,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(id(99_516)),
                run: active.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot,
                seed: 9,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: "limited-run-parity".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("limited-run-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("limited-run-grading"),
                    rendered_question_sha256: "limited-run-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue the only run item");
    let completed = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: learner,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: attempt.id,
                response: StudentResponse::Numeric { value: 1.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("limited-run-completion")
                    .expect("valid idempotency key"),
            },
        )
        .await
        .expect("complete the one allowed run");
    assert_eq!(completed.run.id, active.id);
    assert!(completed.run.completed_at.is_some());
    let exhausted_list = store
        .list_learner_entitled_assignments(
            context,
            learner,
            course,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(10).expect("bounded page"),
            ),
        )
        .await
        .expect("list after limited run completion");
    assert!(
        !exhausted_list
            .items
            .iter()
            .any(|record| record.id == assignment)
    );
    assert!(matches!(
        store
            .start_or_resume_run(
                context,
                learner,
                StudentWorkRoutingBinding::new(course, assignment),
                question_model::RunId::from_uuid(id(99_517)),
            )
            .await,
        Err(StoreError::NotFound)
    ));
}
