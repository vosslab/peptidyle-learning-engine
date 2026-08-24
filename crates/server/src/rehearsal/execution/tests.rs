use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use learning_data_access::in_memory::{MemorySealedPrivateExecutionStore, MemoryStore};
use learning_data_access::{
    AssignmentRecord, CatalogStore, ClaimRehearsalSubmissionRouteCommand, CourseRecord,
    CreateAssignmentCommand, CreateCourseCommand, DraftRecord, NavigationReferenceStore,
    PublishDraftCommand, PutAssignmentTeachingSettingsCommand, RehearsalDeliveryClaimResult,
    RehearsalDeliveryDispatchResult, RehearsalDeliveryRequest, RehearsalIdempotencyKey,
    RehearsalOperationDigest, RehearsalOperationStore, RehearsalRouteIdentity,
    RehearsalRouteMutationStore, RehearsalStore, SealedRehearsalDeliveryExecutionStore,
    SealedRehearsalSubmissionExecutionStore, Store, TenantContext,
};
use question_model::answer::SelectionCardinality;
use question_model::capability::Capability;
use question_model::envelope::ContentBlock;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId,
    AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, BackendCapabilities, BaseAssignmentPolicy, CourseId,
    CourseLocalDateTime, CourseTerm, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, IanaTimeZone, PointValue, PolicyModificationModeView, PolicyPatchView,
    PreviewSelectedMoment, PreviewSyntheticGroupReferences, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, RehearsalSubjectStart,
    RehearsalSyntheticSubjectRequest, SyntheticPreviewModifiers, TeachingAttemptLimitFieldPatch,
    TeachingLimitFieldPatch, TeachingTimeFieldPatch, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

use super::{
    RehearsalExecutionCoordinator, RehearsalExecutionError, RehearsalGradeBackend,
    RehearsalIssueBackend,
};
use crate::native_backend::{NativeBackend, graded_rehearsal_receipt};
use crate::run::RunBackendError;

struct CountingIssueBackend {
    native: NativeBackend<MemoryStore>,
    calls: AtomicUsize,
}

struct UnsupportedIssueBackend;

#[async_trait]
impl RehearsalIssueBackend for UnsupportedIssueBackend {
    async fn issue_frozen_rehearsal(
        &self,
        _work: &learning_data_access::SealedRehearsalDeliveryIssueWork,
    ) -> Result<learning_data_access::RehearsalIssuedExecutionArtifactV1, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "fixture refuses this frozen family".into(),
        ))
    }
}

#[async_trait]
impl RehearsalIssueBackend for CountingIssueBackend {
    async fn issue_frozen_rehearsal(
        &self,
        work: &learning_data_access::SealedRehearsalDeliveryIssueWork,
    ) -> Result<learning_data_access::RehearsalIssuedExecutionArtifactV1, RunBackendError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.native.issue_frozen_rehearsal(work).await
    }
}

#[async_trait]
impl RehearsalGradeBackend for CountingIssueBackend {
    async fn grade_frozen_rehearsal(
        &self,
        work: learning_data_access::SealedRehearsalGradingParts,
    ) -> Result<crate::run::GradeReceipt, RunBackendError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.native.grade_frozen_rehearsal(work).await
    }
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn choice(value: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(value),
        body: vec![ContentBlock::Text {
            markdown: value.into(),
        }],
    }
}

fn rehearsal_subject() -> RehearsalSubjectStart {
    RehearsalSubjectStart::Synthetic {
        request: RehearsalSyntheticSubjectRequest {
            selected_moment: PreviewSelectedMoment {
                value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
            },
            groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("groups"),
            modifiers: SyntheticPreviewModifiers {
                mode: PolicyModificationModeView::ExtendOnly,
                patch: PolicyPatchView {
                    available_at: TeachingTimeFieldPatch::Inherit,
                    due_at: TeachingTimeFieldPatch::Inherit,
                    closes_at: TeachingTimeFieldPatch::Inherit,
                    time_limit_seconds: TeachingLimitFieldPatch::Inherit,
                    attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
                },
            },
        },
    }
}

async fn dispatched_fixture() -> (
    Arc<MemoryStore>,
    TenantContext,
    learning_data_access::DispatchedRehearsalDelivery,
    RehearsalRouteIdentity,
) {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("clock");
    let tenant = TenantId::from_uuid(id(73_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(73_002));
    let course = CourseId::from_uuid(id(73_003));
    let assignment = AssignmentId::from_uuid(id(73_004));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(73_005)),
        version: VersionId::from_uuid(id(73_006)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id(73_007)),
            source: DraftQuestionSource::Native {
                family: adapter_native::peptide_bond_geometry::FAMILY_ID.into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "In {{residue}}, which linkage is planar?".into(),
            }],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![choice("ester"), choice("amide")],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: adapter_native::peptide_bond_geometry::GENERATOR_ID.into(),
                    version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.into(),
                },
                parameters: BTreeMap::from([(
                    "residue".into(),
                    ParameterSpec::Choice {
                        options: vec!["alanine".into(), "glycine".into()],
                    },
                )]),
            },
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide rehearsal".into(),
                tags: vec![],
                taxonomy: vec![],
                license: License::CcBy,
                language: "en-US".into(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: adapter_native::peptide_bond_geometry::FAMILY_ID.into(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE".into()).expect("author"),
                ])
                .expect("byline"),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("published");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Rehearsal course".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("course");
    store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Live rehearsal assignment".into(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: AssignmentInstructions::default(),
                    audience: AssignmentAudience::CourseWide,
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id(73_008)),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: vec![],
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("assignment record");
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: stored.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions: AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy::default(),
                },
            },
        )
        .await
        .expect("publish assignment");
    let assignment_reference = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment reference")
        .expect("assignment exists");
    let revision = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("assignment record")
        .revision;
    let started = store
        .start_rehearsal_from_route(
            context,
            learning_data_access::StartRehearsalRouteCommand {
                actor: instructor,
                course,
                assignment: assignment_reference,
                expected_revision: question_model::TeachingOperationRevision::new(revision.value())
                    .expect("revision"),
                subject: rehearsal_subject(),
                start_new_after_completion: false,
                idempotency_key: RehearsalIdempotencyKey::new("issue-executor-start".into())
                    .expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([7; 32]),
            },
        )
        .await
        .expect("start rehearsal");
    let locator = learning_data_access::RehearsalLocator {
        actor: instructor,
        course,
        assignment: assignment_reference,
        revision: question_model::TeachingOperationRevision::new(revision.value())
            .expect("revision"),
        rehearsal: started.receipt.rehearsal,
    };
    let claimed = store
        .claim_rehearsal_delivery(
            context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("issue-executor-delivery".into())
                    .expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([8; 32]),
            },
        )
        .await
        .expect("claim delivery");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = claimed else {
        panic!("fresh rehearsal is prepared")
    };
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(context, prepared)
        .await
        .expect("dispatch delivery")
    else {
        panic!("untimed rehearsal dispatches")
    };
    (
        store,
        context,
        dispatched,
        RehearsalRouteIdentity {
            actor: locator.actor,
            course: locator.course,
            assignment: locator.assignment,
            rehearsal: locator.rehearsal,
            expected_revision: locator.revision,
        },
    )
}

#[tokio::test]
async fn first_issue_commits_once_and_crash_replay_projects_without_reissue() {
    let (store, context, dispatched, _) = dispatched_fixture().await;
    let backend = Arc::new(CountingIssueBackend {
        native: NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        ),
        calls: AtomicUsize::new(0),
    });
    let sealed: Arc<dyn SealedRehearsalDeliveryExecutionStore> =
        Arc::new(MemorySealedPrivateExecutionStore::new(Arc::clone(&store)));
    let coordinator = RehearsalExecutionCoordinator::new(Arc::clone(&backend), sealed);
    // This opaque comparison covers ordinary learner runs, attempts,
    // submissions, scores, gradebook projections, and jobs. Rehearsal owns
    // only its separate immutable execution evidence.
    let learner_effects_before = store
        .rehearsal_state_effect_fingerprint()
        .expect("learner-state fingerprint before issue");
    let first = coordinator
        .issue_or_resume(context, &dispatched)
        .await
        .expect("first issue");
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    let replay = coordinator
        .issue_or_resume(context, &dispatched)
        .await
        .expect("crash replay");
    assert_eq!(replay, first);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("learner-state fingerprint after replay")
            .has_no_ordinary_effects_from(&learner_effects_before)
    );
}

#[tokio::test]
async fn unsupported_frozen_execution_fails_closed_before_any_screen_projection() {
    let (store, context, dispatched, _) = dispatched_fixture().await;
    let sealed: Arc<dyn SealedRehearsalDeliveryExecutionStore> =
        Arc::new(MemorySealedPrivateExecutionStore::new(store));
    let coordinator = RehearsalExecutionCoordinator::new(Arc::new(UnsupportedIssueBackend), sealed);
    let error = coordinator
        .issue_or_resume(context, &dispatched)
        .await
        .expect_err("unsupported frozen execution must not create a screen");
    assert!(matches!(error, RehearsalExecutionError::Unsupported(_)));
}

#[tokio::test]
async fn native_submission_recovers_pending_claim_and_replays_without_second_grade() {
    let (store, context, dispatched, route) = dispatched_fixture().await;
    let backend = Arc::new(CountingIssueBackend {
        native: NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        ),
        calls: AtomicUsize::new(0),
    });
    let sealed_memory = Arc::new(MemorySealedPrivateExecutionStore::new(Arc::clone(&store)));
    let delivery: Arc<dyn SealedRehearsalDeliveryExecutionStore> = sealed_memory.clone();
    let submission: Arc<dyn SealedRehearsalSubmissionExecutionStore> = sealed_memory;
    let route_store: Arc<dyn RehearsalRouteMutationStore> = store.clone();
    let coordinator = RehearsalExecutionCoordinator::with_submission_execution(
        Arc::clone(&backend),
        delivery,
        submission,
        route_store,
    );
    let screen = coordinator
        .issue_or_resume(context, &dispatched)
        .await
        .expect("issue screen");
    let digest = screen
        .commitment()
        .expect("issued screen commitment")
        .public_token();
    let rendered_choice = match &screen.presentation.response {
        question_model::RehearsalResponseSchemaV1::SingleChoice { choices } => ChoiceId::new(
            choices
                .first()
                .expect("fixture has a visible choice")
                .id
                .as_str(),
        ),
        _ => panic!("fixture renders one visible choice response"),
    };
    store
        .complete_rehearsal_delivery(
            context,
            learning_data_access::RehearsalDeliveryCompletionCommand { dispatched, screen },
        )
        .await
        .expect("complete screen");
    let key = RehearsalIdempotencyKey::new("native-grade-recovery".into()).expect("key");
    store
        .claim_rehearsal_submission_from_route(
            context,
            ClaimRehearsalSubmissionRouteCommand {
                route,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![rendered_choice],
                },
                presentation_digest: digest,
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("durably prepare submission");
    let ordinary_before = store
        .rehearsal_state_effect_fingerprint()
        .expect("ordinary-state fingerprint");
    let first = coordinator
        .grade_or_resume_submission(context, route, key.clone())
        .await
        .expect("recover pending and grade");
    assert!(!first.replayed);
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "one issue and one grade"
    );
    let replay = coordinator
        .grade_or_resume_submission(context, route, key)
        .await
        .expect("exact receipt replay");
    assert!(replay.replayed);
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "replay cannot grade again"
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("ordinary-state fingerprint after grade")
            .has_no_ordinary_effects_from(&ordinary_before)
    );
}

#[test]
fn manual_or_ungraded_backend_outcomes_have_no_rehearsal_fallback() {
    for outcome in [
        grading::GradeOutcome::NeedsManualGrading,
        grading::GradeOutcome::Ungraded,
    ] {
        let Err(error) =
            graded_rehearsal_receipt(outcome, question_model::FeedbackContent::default(), "test")
        else {
            panic!("rehearsal requires deterministic server grading");
        };
        assert!(matches!(error, RunBackendError::Unsupported(_)));
    }
}
