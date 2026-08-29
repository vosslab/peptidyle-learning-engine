//! Ordinary Store fixture for the connected Student-work inspection oracle.

use super::*;
use learning_data_access::postgres::{
    PostgresAcceptedSubmissionFastPathStore, local_accepted_submission_fast_path_pool,
};
use learning_data_access::{
    AcceptedSubmissionCommand, AcceptedSubmissionExecutionDisposition,
    AcceptedSubmissionExecutionFastPathClaimStore, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionExecutionTarget, AcceptedSubmissionGrade,
    AssignmentRecord, AssignmentScoringCommitOutcome, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, AutomatedGradingStore,
    CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand, DraftRecord,
    FlatGradingCapability, IssueQuestionAttemptCommand, IssuedQuestionFamilyWitnessV1,
    IssuedQuestionSnapshotV1, JobClaimFilter, JobId, JobKind, JobLeaseDuration, JobPayload,
    JobStore, NativeExecutionEnvelopeCapability, PresentationCapability, PublishDraftCommand,
    QtiGradingCapability, Store, StudentWorkRoutingBinding, SubmissionIdempotencyKey,
    TenantContext, UpsertCourseMember, WebworkGradingCapability, WorkerId,
    canonical_attempt_result_json,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::{ContentBlock, QuestionEnvelope};
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::presentation::{
    NonceSourceV1, PresentationBuildError, PresentationDigestV1,
    build_presentation_v1_with_nonce_source,
};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, AttemptProvenance, AttemptResult, BackendCapabilities, Capability,
    CourseId, CourseMembershipId, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    GradingDefinition, ImplementationVersion, PointValue, PresentationBindingV1, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    RunId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

pub(super) struct InspectionFixture {
    pub(super) tenant: TenantId,
    pub(super) context: TenantContext,
    pub(super) course: CourseId,
    pub(super) instructor: UserId,
    pub(super) assignment: AssignmentId,
    pub(super) run: RunId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) membership: CourseMembershipId,
    pub(super) presentation_digest: PresentationDigestV1,
    pub(super) scoring_generation: question_model::ScoringGeneration,
}

struct FixtureNonce([u8; 16]);

impl NonceSourceV1 for FixtureNonce {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        Ok(self.0)
    }
}

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn envelope(reference: ProblemVersionRef, seed: u64) -> QuestionEnvelope {
    QuestionEnvelope {
        version: reference.version,
        seed: Seed::new(seed),
        title: "Student-work inspection numeric fixture".to_string(),
        prompt: vec![ContentBlock::Text {
            markdown: "Enter the expected numeric result.".to_string(),
        }],
        response: question_model::ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
            unit: None,
        },
    }
}

fn presentation(
    reference: ProblemVersionRef,
    seed: u64,
) -> (
    PresentationBindingV1,
    learning_data_access::ReceiptPresentationSnapshot,
) {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..].copy_from_slice(&seed.rotate_left(7).to_le_bytes());
    let mut nonce = FixtureNonce(bytes);
    let rendered =
        build_presentation_v1_with_nonce_source(&envelope(reference, seed), &[], &mut nonce)
            .expect("deterministic native fixture presentation");
    (
        PresentationBindingV1::new(rendered.envelope.presentation_nonce, rendered.digest),
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: rendered.envelope,
            asset_bindings: rendered.asset_bindings,
        },
    )
}

fn provenance() -> AttemptProvenance {
    AttemptProvenance {
        adapter: ImplementationVersion {
            id: "student-work-inspection-live".to_string(),
            version: "1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "student-work-inspection-live-grading".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: "student-work-inspection-live-render".to_string(),
    }
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Enter the expected numeric result.".to_string(),
            }],
            response: question_model::ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Student-work inspection numeric fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save native inspection fixture draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid fixture byline"),
                ])
                .expect("valid fixture byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish native inspection fixture");
    reference
}

async fn issued_snapshot(
    store: &PostgresStore,
    context: TenantContext,
    reference: ProblemVersionRef,
) -> IssuedQuestionSnapshotV1 {
    let question = store
        .get_catalog_problem(context, reference)
        .await
        .expect("read published inspection fixture")
        .expect("published inspection fixture exists")
        .question;
    IssuedQuestionSnapshotV1::new(
        question,
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct native issue snapshot")
}

pub(super) async fn create(store: &PostgresStore, fast_path_url: &str) -> InspectionFixture {
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Student-work inspection fixture course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: sysadmin_course_creation_authority(store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create inspection fixture course");
    let reference = publish_question(store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    create_published_assignment(
        store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "Student-work inspection fixture assignment".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(id()),
                reference,
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::StudentDisclosurePolicy::default(),
            policies: policies(),
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish one-item inspection fixture assignment");
    let membership = store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Inspection fixture Student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("activate fixture Student membership");
    let run = store
        .start_or_resume_run(
            context,
            student,
            StudentWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start ordinary Student run");
    let (presentation, presentation_snapshot) = presentation(reference, 18);
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(id()),
                run: run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot: issued_snapshot(store, context, reference).await,
                seed: 18,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(presentation),
                presentation_snapshot: Some(presentation_snapshot),
                grading_envelope: Some(envelope(reference, 18)),
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "student-work-inspection-parameters".to_string(),
                provenance: provenance(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue EnvelopeV1 native attempt with immutable presentation");
    let execution_job = JobId::from_uuid(id());
    let accepted = store
        .accept_automated_submission(
            context,
            AcceptedSubmissionCommand {
                actor: student,
                attempt: attempt.id,
                course,
                assignment,
                response: StudentResponse::Numeric { value: 18.0 },
                idempotency_key: SubmissionIdempotencyKey::parse("student-work-inspection")
                    .expect("valid fixture idempotency key"),
                execution_job,
            },
        )
        .await
        .expect("accept immutable numeric response");
    let fast_pool = local_accepted_submission_fast_path_pool(fast_path_url)
        .await
        .expect("attest disposable accepted-submission fast-path pool");
    let fast_path = PostgresAcceptedSubmissionFastPathStore::from_fast_path_pool(fast_pool);
    let claim = fast_path
        .claim_exact_accepted_submission_execution(
            AcceptedSubmissionExecutionTarget {
                tenant,
                attempt: attempt.id,
                submission: accepted.submission,
                job: execution_job,
            },
            WorkerId::from_uuid(id()),
            JobLeaseDuration::from_seconds(30).expect("bounded accepted-submission lease"),
        )
        .await
        .expect("claim exact accepted response")
        .expect("new accepted response is claimable");
    let grade = AcceptedSubmissionGrade {
        evidence: canonical_attempt_result_json(AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        })
        .expect("canonical deterministic result"),
        feedback: FeedbackContent::default(),
    };
    assert_eq!(
        fast_path
            .commit_or_fail_accepted_submission_execution(
                context,
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated { grade },
            )
            .await
            .expect("commit deterministic accepted response"),
        AcceptedSubmissionExecutionDisposition::Committed
    );
    let scoring_generation = publish_current_scoring_generation(store, tenant, assignment).await;
    InspectionFixture {
        tenant,
        context,
        course,
        instructor,
        assignment,
        run: run.id,
        attempt: attempt.id,
        membership: CourseMembershipId::from_uuid(membership.member.id.as_uuid()),
        presentation_digest: presentation.digest(),
        scoring_generation,
    }
}

async fn publish_current_scoring_generation(
    store: &PostgresStore,
    tenant: TenantId,
    expected_assignment: AssignmentId,
) -> question_model::ScoringGeneration {
    let filter =
        JobClaimFilter::new([JobKind::RecalculateAssignment]).expect("scoring worker filter");
    let lease = JobLeaseDuration::from_seconds(30).expect("bounded scoring worker lease");
    loop {
        let claimed = store
            .claim_next_job(&filter, lease)
            .await
            .expect("claim production scoring work")
            .expect("accepted completion enqueues scoring work");
        let JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } = claimed.payload
        else {
            unreachable!("scoring filter returned another job family")
        };
        let context = TenantContext::from_authenticated_session(claimed.tenant);
        let command = AssignmentScoringWorkerCommand {
            job: claimed.id,
            lease: claimed.lease_token,
            assignment,
            generation,
        };
        let preparation = store
            .prepare_assignment_scoring(context, command)
            .await
            .expect("prepare production scoring generation");
        let publication = store
            .commit_assignment_scoring(context, command)
            .await
            .expect("publish production scoring generation");
        if claimed.tenant == tenant && assignment == expected_assignment {
            assert_eq!(preparation, AssignmentScoringPreparationOutcome::Prepared);
            assert_eq!(publication, AssignmentScoringCommitOutcome::Committed);
            return generation;
        }
        assert!(matches!(
            (preparation, publication),
            (
                AssignmentScoringPreparationOutcome::Prepared,
                AssignmentScoringCommitOutcome::Committed
            ) | (
                AssignmentScoringPreparationOutcome::Superseded,
                AssignmentScoringCommitOutcome::Superseded
            )
        ));
    }
}
