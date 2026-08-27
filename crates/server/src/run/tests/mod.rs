mod archive_fence;
mod catalog_identity;
mod external_projection;
mod feedback;
mod imathas_launch;
mod imathas_submission;
mod imathas_support;
mod manual_grading_http;
mod pending_receipts;
mod prefetch;
mod start_binding;
mod submission;
mod support;
use imathas_support::*;
use support::*;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use grading::{AnswerKey, GradingError, grade};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, AssignmentUpdate, CatalogTransition, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, JobLeaseDuration, JobPayload, JobStore,
    MaterializeAssignmentEntitlementCommand, Page, PublishDraftCommand, PublishedProblemRecord,
    RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionRecord, SessionSubject,
    SessionTokenHash, TenantContext, UpsertCourseMember,
};
use question_model::answer::{NumericTolerance, SelectionCardinality};
use question_model::envelope::ContentBlock;
use question_model::generation::Seed;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::ResponseDefinition;
use question_model::response::{ChoiceId, ChoiceOption};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, BackendCapabilities, Capability, CatalogProblemSummary, CatalogSearchPage,
    CatalogSearchQuery, CourseId, DraftQuestionDefinition, DraftQuestionSource, EnrollmentId,
    EntitlementPurpose, GradingDefinition, ImplementationVersion, ObjectId, ProblemId,
    PublicationScope, QuestionMetadata, QuestionSource, TenantId, UserId, VersionId, WorkspaceId,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::imathas_backend::{ExternalToolSubmissionBackend, ImathasBackend};
use crate::native_backend::NativeBackend;

/// Test-only explicit injection mirrors production's separated grader
/// capability; tests never let the ordinary MemoryStore stand in for it.
fn sealed_memory(
    store: &Arc<MemoryStore>,
) -> Arc<dyn learning_data_access::SealedPrivateExecutionStore> {
    Arc::new(
        learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(Arc::clone(store)),
    )
}

fn learner_submission_status(
    store: &Arc<MemoryStore>,
) -> Arc<dyn learning_data_access::LearnerSubmissionStatusStore> {
    store.clone()
}

fn automated_grading(
    store: &Arc<MemoryStore>,
) -> Arc<dyn learning_data_access::AutomatedGradingStore> {
    store.clone()
}

#[derive(Debug, Default)]
struct NumericBackend {
    grade_calls: AtomicUsize,
    reproduce_calls: AtomicUsize,
    external_launch_calls: AtomicUsize,
    issued_seeds: std::sync::Mutex<Vec<u64>>,
    issued_response: std::sync::Mutex<Option<ResponseDefinition>>,
    graded_responses: std::sync::Mutex<Vec<StudentResponse>>,
    external_tool_launch_ready: bool,
    manual_grading_required: bool,
}

struct CountingNativeBackend {
    inner: NativeBackend<MemoryStore>,
    submissions: AtomicUsize,
}

struct OpaqueRenderedHashBackend {
    inner: Arc<CountingNativeBackend>,
}

/// Observes the otherwise opaque sealed boundary without reimplementing its
/// authority rules. This lets route tests prove the ordering invariant rather
/// than merely inferring it from backend counters.
struct CountingSealedExecution {
    inner: Arc<dyn learning_data_access::SealedPrivateExecutionStore>,
    calls: AtomicUsize,
    refuse: AtomicBool,
}

#[async_trait]
impl learning_data_access::SealedPrivateExecutionStore for CountingSealedExecution {
    async fn prepare_sealed_private_execution(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: learning_data_access::LearnerWorkRoutingBinding,
        intent: learning_data_access::AuthorizedSubmissionIntent,
        response: &StudentResponse,
        idempotency_key: &learning_data_access::SubmissionIdempotencyKey,
    ) -> Result<
        learning_data_access::SealedPrivateExecutionPreparation,
        learning_data_access::StoreError,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse.load(Ordering::SeqCst) {
            return Err(learning_data_access::StoreError::Forbidden);
        }
        self.inner
            .prepare_sealed_private_execution(
                context,
                actor,
                binding,
                intent,
                response,
                idempotency_key,
            )
            .await
    }
}

/// Fails the one successor-issuance operation after a first answer has been
/// durably graded. This isolates the receipt-delivery recovery contract from
/// grading itself.
struct UnavailableSuccessorBackend {
    inner: Arc<CountingNativeBackend>,
    fail_next_issue: AtomicBool,
}

#[async_trait]
impl RunBackend for CountingNativeBackend {
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        self.inner.issue(context, reference, question, seed).await
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        self.inner
            .reproduce(context, reference, question, attempt)
            .await
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        self.inner
            .grade(context, reference, question, attempt, response)
            .await
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.submissions.fetch_add(1, Ordering::SeqCst);
        self.inner.submit(submission).await
    }
}

#[async_trait]
impl RunBackend for OpaqueRenderedHashBackend {
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        let mut issued = self.inner.issue(context, reference, question, seed).await?;
        issued.provenance.rendered_question_sha256 = format!("backend-owned-render-{seed:016x}");
        Ok(issued)
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        let issued = self
            .issue(context, reference, question, attempt.seed)
            .await?;
        if issued.parameter_hash != attempt.parameter_hash
            || issued.provenance != attempt.provenance
        {
            return Err(RunBackendError::Invalid(
                "opaque rendered artifact did not reproduce".to_string(),
            ));
        }
        Ok(issued.envelope)
    }

    async fn grade(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
        _response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        Err(RunBackendError::Unsupported(
            "opaque-render test backend does not grade".to_string(),
        ))
    }
}

#[async_trait]
impl RunBackend for UnavailableSuccessorBackend {
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        if self.fail_next_issue.swap(false, Ordering::SeqCst) {
            return Err(RunBackendError::Unavailable(
                "test successor issuance outage".to_string(),
            ));
        }
        self.inner.issue(context, reference, question, seed).await
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        self.inner
            .reproduce(context, reference, question, attempt)
            .await
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        self.inner
            .grade(context, reference, question, attempt, response)
            .await
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        self.inner.submit(submission).await
    }
}

#[async_trait]
impl RunBackend for NumericBackend {
    async fn issue(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        self.issued_seeds.lock().expect("seed record").push(seed);
        let response = self
            .issued_response
            .lock()
            .expect("issued response")
            .clone()
            .unwrap_or_else(|| _question.response.clone());
        Ok(IssuedAttemptMetadata {
            envelope: QuestionEnvelope {
                version: _question.version,
                seed: Seed::new(seed),
                title: _question.metadata.title.clone(),
                prompt: _question.prompt.clone(),
                response,
            },
            parameter_hash: format!("parameter-{seed:016x}"),
            provenance: AttemptProvenance {
                adapter: implementation("test-native"),
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: implementation("numeric"),
                rendered_question_sha256: format!("rendered-{seed:016x}"),
            },
            webwork_replay: None,
            flat_grading: None,
            flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
            webwork_grading: None,
            webwork_grading_capability:
                learning_data_access::WebworkGradingCapability::NotApplicable,
            qti_grading: None,
            qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
        })
    }

    async fn reproduce(
        &self,
        _context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        self.reproduce_calls.fetch_add(1, Ordering::SeqCst);
        if attempt.problem != reference.problem
            || attempt.question_version != reference.version
            || question.version != reference.version
            || question.problem != reference.problem
        {
            return Err(RunBackendError::Invalid(
                "attempt does not match its published question".to_string(),
            ));
        }
        let response = self
            .issued_response
            .lock()
            .expect("issued response")
            .clone()
            .unwrap_or_else(|| question.response.clone());
        Ok(QuestionEnvelope {
            version: question.version,
            seed: Seed::new(attempt.seed),
            title: question.metadata.title.clone(),
            prompt: question.prompt.clone(),
            response,
        })
    }

    async fn prepare_external_tool_launch(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        self.external_launch_calls.fetch_add(1, Ordering::SeqCst);
        if self.external_tool_launch_ready {
            Ok(())
        } else {
            Err(RunBackendError::Unsupported(
                "test backend has no external-tool broker".to_string(),
            ))
        }
    }

    async fn grade(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        self.grade_calls.fetch_add(1, Ordering::SeqCst);
        self.graded_responses
            .lock()
            .expect("graded response record")
            .push(response.clone());
        if self.manual_grading_required {
            return Ok(GradeOutcome::NeedsManualGrading);
        }
        if self
            .issued_response
            .lock()
            .expect("issued response")
            .is_some()
        {
            return Ok(GradeOutcome::Graded(AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            }));
        }
        grade(
            question,
            response,
            Some(&AnswerKey::Numeric { expected: 18.0 }),
        )
        .map_err(grading_error)
    }
}

fn grading_error(error: GradingError) -> RunBackendError {
    RunBackendError::Invalid(error.to_string())
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn assignment_items(references: Vec<ProblemVersionRef>) -> Vec<question_model::AssignmentItem> {
    static NEXT_ITEM_ID: AtomicUsize = AtomicUsize::new(1_000_000);
    references
        .into_iter()
        .enumerate()
        .map(|(position, reference)| question_model::AssignmentItem {
            id: question_model::AssignmentItemId::from_uuid(id(NEXT_ITEM_ID
                .fetch_add(1, Ordering::Relaxed)
                as u128)),
            reference,
            position: u32::try_from(position).expect("test assignment position fits u32"),
            points_possible: question_model::PointValue::from_whole(1),
            delivery_state: question_model::AssignmentDeliveryState::Active,
            scoring_mode: question_model::AssignmentScoringMode::Normal,
        })
        .collect()
}

#[test]
fn fresh_server_seeds_fit_the_exact_json_integer_range() {
    for _ in 0..128 {
        assert!(fresh_seed().expect("OS random seed") <= MAX_JSON_SAFE_INTEGER);
    }
}

async fn fixture() -> (
    Arc<MemoryStore>,
    Arc<NumericBackend>,
    Router,
    String,
    String,
    AssignmentId,
    EnrollmentId,
) {
    fixture_with_response(
        ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
            unit: Some("g/mol".to_string()),
        },
        false,
    )
    .await
}

async fn fixture_with_response(
    response: ResponseDefinition,
    external_tool_launch_ready: bool,
) -> (
    Arc<MemoryStore>,
    Arc<NumericBackend>,
    Router,
    String,
    String,
    AssignmentId,
    EnrollmentId,
) {
    fixture_with_attempt_policy(response, external_tool_launch_ready, None).await
}

async fn fixture_with_attempt_policy(
    response: ResponseDefinition,
    external_tool_launch_ready: bool,
    max_attempts: Option<u32>,
) -> (
    Arc<MemoryStore>,
    Arc<NumericBackend>,
    Router,
    String,
    String,
    AssignmentId,
    EnrollmentId,
) {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(2));
    let student = UserId::from_uuid(id(3));
    let outsider = UserId::from_uuid(id(4));
    let course = CourseId::from_uuid(id(5));
    let assignment = AssignmentId::from_uuid(id(6));
    let problem = ProblemId::from_uuid(id(8));
    let version = VersionId::from_uuid(id(9));
    let workspace = WorkspaceId::from_uuid(id(10));
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "test_numeric".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass of water?".to_string(),
            }],
            response,
            attempt_policy: AttemptPolicy { max_attempts },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Water molar mass".to_string(),
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
        .expect("draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: "test_numeric".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("publish");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
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
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Molar mass mastery".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: assignment_items(vec![ProblemVersionRef { problem, version }]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    store
        .put_assignment_teaching_settings(
            context,
            learning_data_access::PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: stored.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: question_model::BaseAssignmentPolicy::default(),
                },
            },
        )
        .await
        .expect("publish assignment");
    let enrollment = match store
        .issue_assignment_entitlement(
            context,
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment,
                instructor,
                EntitlementPurpose::InstructorIssue,
            )
            .expect("fixture instructor issue command"),
        )
        .await
        .expect("fixture entitlement materialization")
    {
        learning_data_access::AssignmentEntitlementMaterialization::Granted(receipt) => {
            receipt.enrollment.id
        }
        learning_data_access::AssignmentEntitlementMaterialization::Denied(_) => {
            panic!("fixture student must be entitled")
        }
    };
    let student_cookie = issued_cookie(store.as_ref(), student, "Student").await;
    let outsider_cookie = issued_cookie(store.as_ref(), outsider, "Outsider").await;
    let backend = Arc::new(NumericBackend {
        external_tool_launch_ready,
        ..NumericBackend::default()
    });
    let app = router(
        Arc::clone(&store),
        Arc::clone(&backend),
        sealed_memory(&store),
        learner_submission_status(&store),
        automated_grading(&store),
    );
    (
        store,
        backend,
        app,
        student_cookie,
        outsider_cookie,
        assignment,
        enrollment,
    )
}

async fn prepare_archive_fence(store: &MemoryStore, tenant: TenantId, course: CourseId) {
    store
        .seed_retention_cleanup_for_test(
            tenant,
            course,
            (0..4)
                .map(|offset| ObjectId::from_uuid(id(900 + offset)))
                .collect(),
        )
        .expect("archive cleanup fixture");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease duration"),
        )
        .await
        .expect("archive claim")
        .expect("archive job");
    let (claimed_course, stage, generation) = match claim.payload {
        JobPayload::Retention {
            course,
            stage,
            generation,
        } => (course, stage, generation),
        _ => panic!("fixture must claim retention work"),
    };
    assert_eq!(claimed_course, course);
    store
        .prepare_retention_work(RetentionWorkerCommand {
            tenant,
            course,
            stage,
            generation,
            job: claim.id,
            lease: claim.lease_token,
        })
        .await
        .expect("archive prepare fence");
}

fn peptide_choice(id: &str, body: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: body.to_string(),
        }],
    }
}

async fn native_feedback_fixture() -> (
    Arc<MemoryStore>,
    Arc<CountingNativeBackend>,
    Router,
    String,
    String,
    AssignmentId,
) {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(201));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(202));
    let student = UserId::from_uuid(id(203));
    let outsider = UserId::from_uuid(id(204));
    let course = CourseId::from_uuid(id(205));
    let assignment = AssignmentId::from_uuid(id(206));
    let problem = ProblemId::from_uuid(id(207));
    let version = VersionId::from_uuid(id(208));
    let workspace = WorkspaceId::from_uuid(id(209));
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["alanine".to_string(), "glycine".to_string()],
        },
    );
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                    .to_string(),
            }],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![
                    peptide_choice("ester", "An ester linkage"),
                    peptide_choice("amide", "The peptide linkage"),
                    peptide_choice("ether", "An ether linkage"),
                ],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                    version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.to_string(),
                },
                parameters,
            },
            grading: GradingDefinition::AllOrNothing { points: 2.0 },
            metadata: QuestionMetadata {
                title: "Peptide-bond geometry".to_string(),
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
        .expect("draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ClientRendering,
                    Capability::ServerGrading,
                    Capability::Hints,
                ]),
            },
        )
        .await
        .expect("publish");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
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
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Peptide feedback".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: assignment_items(vec![
                        ProblemVersionRef { problem, version },
                        ProblemVersionRef { problem, version },
                    ]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AnswerAll,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    store
        .put_assignment_teaching_settings(
            context,
            learning_data_access::PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: stored.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: question_model::BaseAssignmentPolicy::default(),
                },
            },
        )
        .await
        .expect("publish assignment");
    let student_cookie = issued_cookie_for(store.as_ref(), tenant, student, "Student").await;
    let outsider_cookie = issued_cookie_for(store.as_ref(), tenant, outsider, "Outsider").await;
    let backend = Arc::new(CountingNativeBackend {
        inner: NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        ),
        submissions: AtomicUsize::new(0),
    });
    let app = router(
        Arc::clone(&store),
        Arc::clone(&backend),
        sealed_memory(&store),
        learner_submission_status(&store),
        automated_grading(&store),
    );
    (
        store,
        backend,
        app,
        student_cookie,
        outsider_cookie,
        assignment,
    )
}
