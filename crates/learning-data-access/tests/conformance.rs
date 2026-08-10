//! Reusable Store conformance suite, first run against memory in WP-C4.

#[path = "conformance/assets.rs"]
mod assets;
#[path = "conformance/catalog.rs"]
mod catalog;
#[path = "conformance/course_appearance.rs"]
mod course_appearance;
#[path = "conformance/external_tool.rs"]
mod external_tool;
#[path = "conformance/flat_import_provenance.rs"]
mod flat_import_provenance;
#[path = "conformance/flat_question.rs"]
mod flat_question;
#[path = "conformance/item_analysis.rs"]
mod item_analysis;
#[path = "conformance/jobs.rs"]
mod jobs;
#[path = "conformance/manual_grading.rs"]
mod manual_grading;
#[path = "conformance/qti.rs"]
mod qti;
#[path = "conformance/qti_ingress.rs"]
mod qti_ingress;
#[path = "conformance/sessions.rs"]
mod sessions;

use assets::source_artifact;

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    ActivityTransition, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, AssignmentRecord, AssignmentScoringCommitOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, AssignmentUpdate,
    AttemptAutoSubmitCommitOutcome, AttemptAutoSubmitWorkerCommand, AttemptAutoSubmitWorkerStore,
    AttemptSupportAction, AttemptSupportActionId, CatalogSourceStore, CatalogStore,
    CatalogTransition, ClearAttemptCommand, CourseGroupRecord, CourseListScope, CourseRecord,
    Cursor, DeleteAndRegradeAssignmentItemCommand, DeleteAssignmentPolicyExceptionCommand,
    DraftRecord, EvaluationRevision, ForceSubmitAttemptCommand, IssueQuestionAttemptCommand,
    ManualCredit, ManualGradeActionId, ManualGradingStore, PageRequest, PageSize,
    PrefetchedQuestion, PublishDraftCommand, PublishedSourceArtifact, PublishedVersionRef,
    PutCourseGroupCommand, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    SetAssignmentPolicyExceptionCommand, SetManualGradeCommand, Store, StoreError,
    SubmissionIdempotencyKey, SubmitPendingManualQuestionAttemptCommand,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand,
};
use learning_data_access::{
    BeginExternalToolGradeCommand, CommitVerifiedExternalToolSubmissionCommand,
    CreateExternalToolLaunchSessionCommand, ExternalToolBegin, ExternalToolBrokerStore,
    ExternalToolLaunchProof, ExternalToolLaunchSessionStore, PersistedCorrelation,
    StageExternalToolVerificationCommand,
};
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportItemResult, QtiImportItemStatus, QtiImportRef, QtiImportRegistry, QtiImportStore,
    QtiPublicationPromotion, QtiUnsupportedFeature,
};
use learning_data_access::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore,
};
use learning_data_access::{
    CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportJobCommit, ExportJobStore, JobClaimFilter,
    JobFailureDisposition, JobFailureKind, JobKind, JobLeaseDuration, JobLeaseToken, JobPayload,
    JobState, JobStore,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssetId, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId,
    AssignmentItem, AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun,
    AssignmentScoringMode, AssignmentTimingPolicy, AttemptProvenance, AttemptResult, AttemptStatus,
    AttemptTimerRecord, BackendCapabilities, Capability, CatalogLifecycle, CompletionRequirement,
    ContinuedPractice, CourseGroupId, CourseId, CourseMembership, CourseMembershipRole, CourseRole,
    DraftQuestionDefinition, DraftQuestionSource, EnrollmentId, FeedbackContent,
    GeneratorReference, GradePolicy, GradingDefinition, ImplementationVersion,
    LateSubmissionPolicy, ObjectId, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttempt, QuestionAttemptId, QuestionBackend, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, RunMode, RunPolicies, SourceArtifact, StudentId, TenantId, UserId,
    UserRole, VariationPolicy, VersionId, WorkspaceId, WorkspaceImportId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn fixed_items(references: Vec<ProblemVersionRef>) -> Vec<AssignmentItem> {
    static NEXT_ITEM_ID: AtomicU64 = AtomicU64::new(900_000);
    references
        .into_iter()
        .enumerate()
        .map(|(position, reference)| AssignmentItem {
            id: AssignmentItemId::from_uuid(uuid(u128::from(
                NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
            ))),
            reference,
            position: u32::try_from(position).expect("fixture position fits"),
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        })
        .collect()
}

fn draft_question(workspace: WorkspaceId) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "molar_mass".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "What is the molar mass?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateFull,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Molar mass".to_string(),
            tags: vec![Tag::new("biochemistry")],
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        },
    }
}

fn published_source() -> QuestionSource {
    QuestionSource::Native {
        family: "molar_mass".to_string(),
    }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn generator(id: &str) -> GeneratorReference {
    GeneratorReference {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn object_record(key: ObjectKey, bytes: &[u8], at: i64) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("fixture size should fit"),
        media_type: "image/svg+xml".to_string(),
        category: key.category(),
        version: key.version_id(),
        license: "CC BY-SA 4.0".to_string(),
        provenance: "asset delivery conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(at),
        key,
    }
}

async fn exercise_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(1));
    let foreign_tenant = TenantId::from_uuid(uuid(2));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let workspace = WorkspaceId::from_uuid(uuid(3));
    let problem = ProblemId::from_uuid(uuid(4));
    let version = VersionId::from_uuid(uuid(5));
    let second_problem = ProblemId::from_uuid(uuid(6));
    let second_version = VersionId::from_uuid(uuid(7));
    let assignment_id = AssignmentId::from_uuid(uuid(8));
    let course_id = CourseId::from_uuid(uuid(17));
    let course_user = UserId::from_uuid(uuid(18));
    let enrollment_id = EnrollmentId::from_uuid(uuid(9));
    let run_id = RunId::from_uuid(uuid(10));
    let practice_run_id = RunId::from_uuid(uuid(14));
    let draft = DraftRecord {
        tenant,
        question: draft_question(workspace),
        revises: None,
        derived_from: None,
    };
    let publisher = UserId::from_uuid(uuid(16));
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id,
        title: "Molar mass mastery".to_string(),
        items: fixed_items(vec![PublishedVersionRef { problem, version }]),
        selection_groups: Vec::new(),
        policies: policies(),
    };
    let stored_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("conforming draft write should succeed");

    let mut invalid_draft = draft.clone();
    invalid_draft.question.attempt_policy.max_attempts = Some(0);
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, invalid_draft)
            .await,
        Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string()
        ))
    );

    let mut blank_title = draft.clone();
    blank_title.question.metadata.title = " \t\n ".to_string();
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, blank_title)
            .await,
        Err(StoreError::InvalidRecord(
            "question title must not be blank".to_string()
        ))
    );

    let mut invalid_publish = draft.clone();
    invalid_publish.question.metadata.title = "\u{2003}".to_string();
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: invalid_publish,
                    expected_revision: stored_draft.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(
            "question title must not be blank".to_string()
        ))
    );
    assert!(
        store
            .get_published_problem(problem, version)
            .await
            .expect("invalid publication lookup should run")
            .is_none(),
        "invalid publication must not mint or persist a record"
    );

    let mut oversized_title = draft.clone();
    oversized_title.question.metadata.title = "\u{1F9EC}".repeat(513);
    assert_eq!(
        store
            .upsert_draft(context, publisher, None, oversized_title)
            .await,
        Err(StoreError::InvalidRecord(
            "question title must contain at most 512 Unicode scalar values".to_string()
        ))
    );

    let stored_draft_json = serde_json::to_value(&stored_draft.record)
        .expect("stored draft should remain serializable");
    assert!(stored_draft_json["question"].get("problem").is_none());
    assert!(stored_draft_json["question"].get("version").is_none());
    let collaborator = UserId::from_uuid(uuid(19));
    store
        .grant_draft_collaborator(context, publisher, workspace, collaborator)
        .await
        .expect("owner should grant a workspace collaborator");
    assert_eq!(
        store
            .delete_draft(context, collaborator, workspace, stored_draft.revision)
            .await,
        Err(StoreError::Forbidden),
        "a collaborator must not delete an owner workspace"
    );
    assert_eq!(
        store.get_draft(context, collaborator, workspace).await,
        Ok(Some(stored_draft.clone())),
        "a refused deletion must preserve collaborator access"
    );

    let second_workspace = WorkspaceId::from_uuid(uuid(30));
    let paged_draft = DraftRecord {
        tenant,
        question: draft_question(second_workspace),
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, publisher, None, paged_draft)
        .await
        .expect("second private draft should save");
    let first_workspace_page = store
        .list_drafts(
            context,
            publisher,
            PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
        )
        .await
        .expect("tenant workspace list should succeed");
    assert_eq!(first_workspace_page.items.len(), 1);
    assert_eq!(first_workspace_page.items[0].workspace, workspace);
    assert_eq!(first_workspace_page.items[0].title, "Molar mass");
    assert_eq!(
        first_workspace_page.items[0].source_backend,
        QuestionBackend::Native
    );
    let summary_json = serde_json::to_value(&first_workspace_page.items[0])
        .expect("workspace summary should serialize");
    let summary_fields = summary_json
        .as_object()
        .expect("workspace summary should be an object");
    assert_eq!(summary_fields.len(), 3);
    for forbidden in [
        "problem", "version", "source", "grading", "object", "asset", "prompt", "response",
    ] {
        assert!(
            !summary_fields.contains_key(forbidden),
            "workspace summary must not expose {forbidden}"
        );
    }
    let workspace_cursor = first_workspace_page
        .next_cursor
        .clone()
        .expect("bounded first page should continue");
    assert!(
        !workspace_cursor.as_str().contains(&workspace.to_string()),
        "workspace cursor must be opaque rather than a UUID path fragment"
    );
    let second_workspace_page = store
        .list_drafts(
            context,
            publisher,
            PageRequest::after(
                workspace_cursor.clone(),
                PageSize::new(1).expect("one is a valid page size"),
            ),
        )
        .await
        .expect("tenant-bound continuation should resume");
    assert_eq!(second_workspace_page.items.len(), 1);
    assert_eq!(second_workspace_page.items[0].workspace, second_workspace);
    assert!(second_workspace_page.next_cursor.is_none());
    assert!(matches!(
        store
            .list_drafts(
                context,
                publisher,
                PageRequest::after(
                    Cursor::parse(format!("{}x", workspace_cursor.as_str()))
                        .expect("nonempty tampered cursor"),
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .list_drafts(
                foreign_context,
                publisher,
                PageRequest::after(
                    workspace_cursor,
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .list_drafts(
                foreign_context,
                publisher,
                PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
            )
            .await
            .expect("foreign workspace list should run")
            .items
            .is_empty()
    );
    assert_eq!(
        store
            .get_draft(foreign_context, publisher, workspace)
            .await
            .expect("foreign draft lookup should run"),
        None
    );
    assert!(
        !store
            .delete_draft(foreign_context, publisher, workspace, stored_draft.revision,)
            .await
            .expect("foreign deletion should not disclose existence")
    );
    assert!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .expect("foreign deletion must not affect local draft")
            .is_some()
    );
    let second_workspace_before_update = store
        .get_draft(context, publisher, second_workspace)
        .await
        .expect("second workspace lookup should run")
        .expect("second workspace should exist before an update");
    let second_workspace_after_update = store
        .upsert_draft(
            context,
            publisher,
            Some(second_workspace_before_update.revision),
            second_workspace_before_update.record.clone(),
        )
        .await
        .expect("second workspace update should advance its revision");
    assert_eq!(
        store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_before_update.revision,
            )
            .await,
        Err(StoreError::Conflict),
        "a stale delete must preserve the newer workspace and access binding"
    );
    assert_eq!(
        store.get_draft(context, publisher, second_workspace).await,
        Ok(Some(second_workspace_after_update.clone())),
        "a stale delete must not mutate the newer workspace"
    );
    assert!(
        store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_after_update.revision,
            )
            .await
            .expect("current owner revision should delete")
    );
    assert!(
        !store
            .delete_draft(
                context,
                publisher,
                second_workspace,
                second_workspace_after_update.revision,
            )
            .await
            .expect("repeat deletion should be an absence result")
    );
    assert_eq!(
        store
            .get_draft(context, publisher, second_workspace)
            .await
            .expect("deleted draft lookup should run"),
        None
    );

    let published = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: stored_draft.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("conforming publish should succeed");
    assert_eq!(published.problem, problem);
    assert_eq!(published.version, version);
    assert_eq!(published.question.problem, problem);
    assert_eq!(published.question.version, version);
    let deletable_workspace = WorkspaceId::from_uuid(uuid(31));
    let deletable_draft = store
        .upsert_draft(
            context,
            publisher,
            None,
            DraftRecord {
                tenant,
                question: draft_question(deletable_workspace),
                revises: None,
                derived_from: None,
            },
        )
        .await
        .expect("independent draft should save before deletion");
    assert!(
        store
            .delete_draft(
                context,
                publisher,
                deletable_workspace,
                deletable_draft.revision,
            )
            .await
            .expect("independent draft should delete")
    );
    assert!(
        store
            .get_published_problem(problem, version)
            .await
            .expect("published catalog lookup should run after draft deletion")
            .is_some(),
        "deleting a draft must not affect its already-published catalog version"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .expect("published draft lookup"),
        None
    );
    let second_draft = DraftRecord {
        tenant,
        question: draft_question(workspace),
        revises: None,
        derived_from: None,
    };
    let second_draft = store
        .upsert_draft(context, publisher, None, second_draft.clone())
        .await
        .expect("second draft write should succeed");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: second_draft.record,
                expected_revision: second_draft.revision,
                publication: ProblemVersionRef {
                    problem: second_problem,
                    version: second_version,
                },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("second publish should succeed");

    let first_page = store
        .list_published_problems(PageRequest::first(
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("first catalog page should load");
    let second_page = store
        .list_published_problems(PageRequest::after(
            first_page
                .next_cursor
                .clone()
                .expect("first page should carry a cursor"),
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("second catalog page should load");

    store
        .upsert_course(
            context,
            CourseRecord {
                id: course_id,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: course_user,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: UserId::from_uuid(uuid(14)),
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("conforming course write should succeed");
    store
        .create_assignment(context, assignment.clone())
        .await
        .expect("conforming assignment write should succeed");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment_id,
                tenant,
                assignment: assignment_id,
                user: UserId::from_uuid(uuid(14)),
                student: StudentId::from_uuid(uuid(11)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("conforming enrollment creation should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 1,
                    started_at: ActivityTimestamp::from_unix_millis(100),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Assigned,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("conforming run start should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::RecordQuestionAttempt {
                attempt: Box::new(QuestionAttempt {
                    id: QuestionAttemptId::from_uuid(uuid(12)),
                    tenant,
                    run: run_id,
                    problem,
                    question_version: version,
                    assignment_position: 0,
                    seed: 42,
                    parameter_hash: "parameters-sha256".to_string(),
                    response: Some(StudentResponse::Numeric { value: 18.0 }),
                    status: question_model::AttemptStatus::Submitted,
                    result: Some(AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    }),
                    timer: AttemptTimerRecord {
                        issued_at: ActivityTimestamp::from_unix_millis(110),
                        deadline: None,
                        submitted_at: Some(ActivityTimestamp::from_unix_millis(120)),
                    },
                    provenance: AttemptProvenance {
                        adapter: implementation("native"),
                        renderer: None,
                        generator: Some(generator("molar-mass")),
                        source_artifact: None,
                        asset_objects: vec![ObjectId::from_uuid(uuid(13))],
                        grading: implementation("numeric"),
                        rendered_question_sha256: "render-sha256".to_string(),
                    },
                }),
            },
        )
        .await
        .expect("conforming attempt write should succeed");
    let summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: run_id,
                score: 1.0,
                at: ActivityTimestamp::from_unix_millis(130),
            },
        )
        .await
        .expect("conforming completion should succeed");
    let completed_run = store
        .get_run(context, run_id)
        .await
        .expect("run read should succeed")
        .expect("completed run should exist");
    let attempt = store
        .get_question_attempt(context, QuestionAttemptId::from_uuid(uuid(12)))
        .await
        .expect("attempt read should succeed")
        .expect("question attempt should exist");

    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: practice_run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 2,
                    started_at: ActivityTimestamp::from_unix_millis(140),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Practice,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("continued practice should remain available after completion");
    let practice_summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: practice_run_id,
                score: 0.8,
                at: ActivityTimestamp::from_unix_millis(150),
            },
        )
        .await
        .expect("continued-practice completion should succeed");
    let enrollment = store
        .get_enrollment(context, enrollment_id)
        .await
        .expect("enrollment read should succeed")
        .expect("enrollment should exist");
    let persisted_summary = store
        .get_summary(context, enrollment_id)
        .await
        .expect("summary read should succeed")
        .expect("summary should exist");

    let second_student = UserId::from_uuid(uuid(20));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course_id,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: course_user,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: UserId::from_uuid(uuid(14)),
                        role: CourseMembershipRole::Student,
                    },
                    CourseMembership {
                        user: second_student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("course may add another enrolled student");
    let second_enrollment = EnrollmentId::from_uuid(uuid(21));
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: second_enrollment,
                tenant,
                assignment: assignment_id,
                user: second_student,
                student: StudentId::from_uuid(uuid(22)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("second course enrollment should create an empty projection");
    let first_gradebook_page = store
        .list_gradebook_rows(
            context,
            course_id,
            PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
        )
        .await
        .expect("summary-only gradebook page should load");
    let second_gradebook_page = store
        .list_gradebook_rows(
            context,
            course_id,
            PageRequest::after(
                first_gradebook_page
                    .next_cursor
                    .clone()
                    .expect("first gradebook page should carry a cursor"),
                PageSize::new(1).expect("one is a valid page size"),
            ),
        )
        .await
        .expect("gradebook cursor should resume after assignment and enrollment");
    assert_eq!(first_gradebook_page.items.len(), 1);
    assert_eq!(second_gradebook_page.items.len(), 1);
    assert_ne!(
        first_gradebook_page.items[0].enrollment_id, second_gradebook_page.items[0].enrollment_id,
        "gradebook cursor must not duplicate an enrollment"
    );
    let first_gradebook_row = first_gradebook_page
        .items
        .iter()
        .chain(second_gradebook_page.items.iter())
        .find(|row| row.enrollment_id == enrollment_id)
        .expect("completed enrollment should appear in the gradebook");
    assert_eq!(first_gradebook_row.tenant, tenant);
    assert_eq!(first_gradebook_row.course_id, course_id);
    assert_eq!(first_gradebook_row.assignment_id, assignment_id);
    assert_eq!(first_gradebook_row.assignment_title, "Molar mass mastery");
    assert_eq!(first_gradebook_row.summary, persisted_summary);
    assert!(matches!(
        store
            .list_gradebook_rows(
                context,
                course_id,
                PageRequest::after(
                    Cursor::parse("not-a-gradebook-cursor".to_string())
                        .expect("nonempty malformed cursor"),
                    PageSize::new(1).expect("one is a valid page size"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(message)) if message == "invalid gradebook cursor"
    ));
    assert_eq!(
        store
            .list_gradebook_rows(
                foreign_context,
                course_id,
                PageRequest::first(PageSize::new(1).expect("one is a valid page size")),
            )
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot discover this course or its summary rows"
    );

    let tenant_mismatch = store
        .upsert_draft(
            foreign_context,
            publisher,
            None,
            DraftRecord {
                tenant,
                question: draft_question(workspace),
                revises: None,
                derived_from: None,
            },
        )
        .await;
    let tenant_assignments = store
        .list_assignments(
            context,
            course_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("assignment list should load");
    let member_courses = store
        .list_courses(
            context,
            CourseListScope::Member(course_user),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("member course list should load");
    let nonmember_courses = store
        .list_courses(
            context,
            CourseListScope::Member(UserId::from_uuid(uuid(19))),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("nonmember course list should load");
    let administrator_courses = store
        .list_courses(
            context,
            CourseListScope::TenantAdministrator,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("administrator course list should load");
    let run_page = store
        .list_runs(
            context,
            enrollment_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("run list should load");

    assert_eq!((first_page.items.len(), second_page.items.len()), (1, 1));
    assert_eq!(
        store.get_draft(context, publisher, workspace).await,
        Ok(None)
    );
    assert_eq!(
        store.get_published_problem(problem, version).await,
        Ok(Some(published))
    );
    assert_eq!(
        store.get_assignment(context, assignment_id).await,
        Ok(Some(assignment))
    );
    assert_eq!(tenant_assignments.items.len(), 1);
    assert_eq!(member_courses.items.len(), 1);
    assert_eq!(member_courses.items[0].role, CourseRole::Instructor);
    assert!(nonmember_courses.items.is_empty());
    assert_eq!(
        administrator_courses.items[0].role,
        CourseRole::Administrator
    );
    assert_eq!(store.get_course(foreign_context, course_id).await, Ok(None));
    assert_eq!(
        (
            summary.current_score,
            summary.completed_run_count,
            summary.total_question_attempts,
        ),
        (Some(1.0), 1, 1)
    );
    assert_eq!(practice_summary, persisted_summary);
    assert_eq!(
        (
            persisted_summary.current_score,
            persisted_summary.best_score,
            persisted_summary.latest_score,
            persisted_summary.completed_run_count,
        ),
        (Some(1.0), Some(1.0), Some(0.8), 2)
    );
    assert_eq!(
        (
            enrollment.first_completed_at,
            enrollment.current_grade_run,
            enrollment.best_grade_run,
        ),
        (
            Some(ActivityTimestamp::from_unix_millis(130)),
            Some(run_id),
            Some(run_id),
        )
    );
    assert_eq!(
        (
            completed_run.completed_at,
            attempt.problem,
            run_page.items.len()
        ),
        (Some(ActivityTimestamp::from_unix_millis(130)), problem, 2,)
    );
    assert_eq!(tenant_mismatch, Err(StoreError::TenantMismatch));
    assert_eq!(
        store.get_draft(foreign_context, publisher, workspace).await,
        Ok(None)
    );
    assert_eq!(
        store.get_assignment(foreign_context, assignment_id).await,
        Ok(None)
    );
    assert_eq!(
        store.get_enrollment(foreign_context, enrollment_id).await,
        Ok(None)
    );
    assert_eq!(store.get_run(foreign_context, run_id).await, Ok(None));
    assert_eq!(
        store
            .get_question_attempt(foreign_context, QuestionAttemptId::from_uuid(uuid(12)))
            .await,
        Ok(None)
    );
    assert_eq!(
        store.get_summary(foreign_context, enrollment_id).await,
        Ok(None)
    );
}

async fn publish_assignment_version<S>(
    store: &S,
    context: TenantContext,
    tenant: TenantId,
    author: UserId,
    seed: u128,
    scope: PublicationScope,
) -> ProblemVersionRef
where
    S: Store + CatalogStore,
{
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(seed)),
        version: VersionId::from_uuid(uuid(seed + 1)),
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question(WorkspaceId::from_uuid(uuid(seed + 2))),
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, author, None, draft.clone())
        .await
        .expect("assignment fixture draft");
    store
        .publish_draft(
            context,
            author,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: author,
                scope,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("assignment fixture publication");
    reference
}

/// Exercises the revisioned assignment edit contract independently of HTTP.
/// Every Store backend must retain exact ordering/policies, refuse stale or
/// cross-course writes without mutation, and apply catalog visibility/lifecycle
/// rules before accepting a new course artifact.
async fn exercise_assignment_cas<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(70_000));
    let foreign_tenant = TenantId::from_uuid(uuid(70_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(70_002));
    let course = CourseId::from_uuid(uuid(70_003));
    let wrong_course = CourseId::from_uuid(uuid(70_004));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Assignment CAS course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("assignment CAS course");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: wrong_course,
                tenant,
                title: "Other course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("wrong-course fixture");
    let foreign_course = CourseId::from_uuid(uuid(70_005));
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("foreign course fixture");

    let published = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_100,
        PublicationScope::Public,
    )
    .await;
    let deprecated = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_110,
        PublicationScope::Public,
    )
    .await;
    store
        .transition_catalog_problem(
            context,
            instructor,
            deprecated,
            CatalogTransition::Deprecate {
                reason: "Revised but usable".to_string(),
            },
        )
        .await
        .expect("deprecated fixture");
    let archived = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_120,
        PublicationScope::Public,
    )
    .await;
    store
        .transition_catalog_problem(
            context,
            instructor,
            archived,
            CatalogTransition::Deprecate {
                reason: "Archive fixture".to_string(),
            },
        )
        .await
        .expect("archive deprecation");
    store
        .transition_catalog_problem(context, instructor, archived, CatalogTransition::Archive)
        .await
        .expect("archive fixture");
    let hidden = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_130,
        PublicationScope::Institution,
    )
    .await;

    let assignment = AssignmentId::from_uuid(uuid(70_200));
    let initial = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "Ordered source selection".to_string(),
        items: fixed_items(vec![published, deprecated]),
        selection_groups: Vec::new(),
        policies: policies(),
    };
    let created = store
        .create_assignment(context, initial.clone())
        .await
        .expect("published and deprecated versions are assignable");
    assert_eq!(created.revision.value(), 1);
    assert_eq!(created.record, initial);

    let updated_policies = RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Latest,
        continued_practice: ContinuedPractice::Closed,
        variation: VariationPolicy::SelectedProblemVariants,
    };
    let update = AssignmentUpdate {
        title: "Reordered source selection".to_string(),
        items: fixed_items(vec![deprecated, published]),
        selection_groups: Vec::new(),
        policies: updated_policies,
    };
    let updated = store
        .replace_assignment(
            context,
            course,
            assignment,
            created.revision,
            update.clone(),
        )
        .await
        .expect("fresh assignment revision updates");
    assert_eq!(updated.revision.value(), 2);
    assert_eq!(updated.record.items, update.items);
    assert_eq!(updated.record.policies, update.policies);
    assert_eq!(updated.record.title, update.title);
    assert_eq!(
        store
            .replace_assignment(
                context,
                course,
                assignment,
                created.revision,
                update.clone()
            )
            .await,
        Err(StoreError::Conflict),
        "stale revision must not overwrite"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read updated assignment"),
        Some(updated.clone())
    );
    assert_eq!(
        store
            .replace_assignment(
                context,
                wrong_course,
                assignment,
                updated.revision,
                update.clone()
            )
            .await,
        Err(StoreError::NotFound),
        "a course path cannot move an assignment"
    );
    assert_eq!(
        store
            .replace_assignment(
                foreign_context,
                course,
                assignment,
                updated.revision,
                update.clone()
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant must not enumerate assignment identity"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("failed writes leave assignment unchanged"),
        Some(updated.clone())
    );

    assert!(matches!(
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_201)),
                    tenant,
                    course_id: course,
                    title: "archived reference".to_string(),
                    items: fixed_items(vec![archived]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .create_assignment(
                foreign_context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_202)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "hidden reference".to_string(),
                    items: fixed_items(vec![hidden]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let repeated = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(70_203)),
                tenant,
                course_id: course,
                title: "Repeated immutable version positions".to_string(),
                items: fixed_items(vec![published, published]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("one immutable version may occupy distinct ordered positions");
    assert_eq!(
        repeated.record.references().collect::<Vec<_>>(),
        vec![published, published]
    );
    let invalid_threshold = RunPolicies {
        completion: CompletionRequirement::ScoreAtLeast { fraction: 1.1 },
        ..policies()
    };
    assert!(matches!(
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_204)),
                    tenant,
                    course_id: course,
                    title: "Invalid completion threshold".to_string(),
                    items: fixed_items(vec![published]),
                    selection_groups: Vec::new(),
                    policies: invalid_threshold,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

/// The draft-to-publication boundary is deliberately exercised against every
/// Store implementation.  These are permanent behavior tests: a failed
/// publication must not consume tenant-owned authoring state, and only the
/// caller that owns a visible lineage may mint its next immutable version.
async fn exercise_publication_identity_boundary<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(600));
    let foreign_tenant = TenantId::from_uuid(uuid(601));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(602));
    let foreign_author = UserId::from_uuid(uuid(603));
    let capabilities = BackendCapabilities::from_iter([Capability::ServerGrading]);

    let stale_workspace = WorkspaceId::from_uuid(uuid(604));
    let stored_stale_draft = DraftRecord {
        tenant,
        question: draft_question(stale_workspace),
        revises: None,
        derived_from: None,
    };
    let stored_stale = store
        .upsert_draft(context, publisher, None, stored_stale_draft.clone())
        .await
        .expect("stale-publication fixture draft should save");
    let mut stale_expected_draft = stored_stale_draft.clone();
    stale_expected_draft.question.metadata.title = "Changed after validation".to_string();
    let stale_publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(605)),
        version: VersionId::from_uuid(uuid(606)),
    };
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: stale_expected_draft,
                    expected_revision: stored_stale.revision,
                    publication: stale_publication,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale expected draft must not publish"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, stale_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(stored_stale_draft)),
        "a stale publication failure must preserve the exact stored draft"
    );
    assert_eq!(
        store.get_catalog_problem(context, stale_publication).await,
        Ok(None),
        "a stale publication failure must not leave an immutable version"
    );

    let base_workspace = WorkspaceId::from_uuid(uuid(607));
    let base_draft = DraftRecord {
        tenant,
        question: draft_question(base_workspace),
        revises: None,
        derived_from: None,
    };
    let base = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(608)),
        version: VersionId::from_uuid(uuid(609)),
    };
    let saved_base_draft = store
        .upsert_draft(context, publisher, None, base_draft.clone())
        .await
        .expect("base draft should save");
    let base_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: base_draft,
                expected_revision: saved_base_draft.revision,
                publication: base,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: capabilities.clone(),
            },
        )
        .await
        .expect("new work should mint a fresh published problem and version");
    assert_eq!(
        (base_record.problem, base_record.version),
        (base.problem, base.version)
    );
    assert_eq!(base_record.previous_version, None);
    assert_eq!(base_record.derived_from, None);

    let fork_workspace = WorkspaceId::from_uuid(uuid(610));
    let fork_draft = DraftRecord {
        tenant,
        question: draft_question(fork_workspace),
        revises: None,
        derived_from: Some(base),
    };
    let fork = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(611)),
        version: VersionId::from_uuid(uuid(612)),
    };
    let saved_fork_draft = store
        .upsert_draft(context, publisher, None, fork_draft.clone())
        .await
        .expect("fork draft should save");
    let fork_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: fork_draft,
                expected_revision: saved_fork_draft.revision,
                publication: fork,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: capabilities.clone(),
            },
        )
        .await
        .expect("fork should mint a fresh problem and version");
    assert_ne!(fork_record.problem, base.problem);
    assert_ne!(fork_record.version, base.version);
    assert_eq!(fork_record.previous_version, None);
    assert_eq!(fork_record.derived_from, Some(base));

    let revision_workspace = WorkspaceId::from_uuid(uuid(613));
    let revision_draft = DraftRecord {
        tenant,
        question: draft_question(revision_workspace),
        revises: Some(base),
        derived_from: None,
    };
    let revision = ProblemVersionRef {
        problem: base.problem,
        version: VersionId::from_uuid(uuid(614)),
    };
    let saved_revision_draft = store
        .upsert_draft(context, publisher, None, revision_draft.clone())
        .await
        .expect("revision draft should save");
    let revision_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: revision_draft,
                expected_revision: saved_revision_draft.revision,
                publication: revision,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: capabilities.clone(),
            },
        )
        .await
        .expect("owned revision should preserve its problem and mint a version");
    assert_eq!(revision_record.problem, base.problem);
    assert_ne!(revision_record.version, base.version);
    assert_eq!(revision_record.previous_version, Some(base.version));
    assert_eq!(revision_record.public_id, base_record.public_id);
    assert_eq!(base_record.version_number.value(), 1);
    assert_eq!(revision_record.version_number.value(), 2);
    assert_ne!(fork_record.public_id, base_record.public_id);
    assert_eq!(fork_record.version_number.value(), 1);
    assert_eq!(
        store
            .resolve_catalog_problem(
                context,
                question_model::ProblemDisplayRef {
                    problem: base_record.public_id,
                    version: None,
                },
            )
            .await
            .expect("stable public ID lookup should succeed")
            .map(|record| record.version),
        Some(revision.version),
        "a stable public ID resolves the latest assignable version"
    );
    assert_eq!(
        store
            .resolve_catalog_problem(
                context,
                question_model::ProblemDisplayRef {
                    problem: base_record.public_id,
                    version: Some(base_record.version_number),
                },
            )
            .await
            .expect("exact public reference lookup should succeed")
            .map(|record| record.version),
        Some(base.version),
        "an exact public reference never silently upgrades"
    );

    let foreign_author_workspace = WorkspaceId::from_uuid(uuid(615));
    let foreign_author_draft = DraftRecord {
        tenant,
        question: draft_question(foreign_author_workspace),
        revises: Some(revision),
        derived_from: None,
    };
    let saved_foreign_author_draft = store
        .upsert_draft(context, foreign_author, None, foreign_author_draft.clone())
        .await
        .expect("foreign-author draft should save before refusal");
    assert_eq!(
        store
            .publish_draft(
                context,
                foreign_author,
                PublishDraftCommand {
                    expected_draft: foreign_author_draft.clone(),
                    expected_revision: saved_foreign_author_draft.revision,
                    publication: ProblemVersionRef {
                        problem: base.problem,
                        version: VersionId::from_uuid(uuid(616)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: foreign_author,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden),
        "a non-author must not extend an owned revision chain"
    );
    assert_eq!(
        store
            .get_draft(context, foreign_author, foreign_author_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(foreign_author_draft)),
        "a forbidden revision must retain its draft"
    );

    let mismatch_workspace = WorkspaceId::from_uuid(uuid(617));
    let mismatch_draft = DraftRecord {
        tenant,
        question: draft_question(mismatch_workspace),
        revises: Some(revision),
        derived_from: None,
    };
    let saved_mismatch_draft = store
        .upsert_draft(context, publisher, None, mismatch_draft.clone())
        .await
        .expect("reference-mismatch draft should save");
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: mismatch_draft.clone(),
                    expected_revision: saved_mismatch_draft.revision,
                    publication: ProblemVersionRef {
                        problem: ProblemId::from_uuid(uuid(618)),
                        version: VersionId::from_uuid(uuid(619)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, mismatch_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(mismatch_draft)),
        "a reference mismatch must not consume a draft"
    );

    let foreign_tenant_workspace = WorkspaceId::from_uuid(uuid(620));
    let foreign_tenant_draft = DraftRecord {
        tenant,
        question: draft_question(foreign_tenant_workspace),
        revises: None,
        derived_from: None,
    };
    let saved_foreign_tenant_draft = store
        .upsert_draft(context, publisher, None, foreign_tenant_draft.clone())
        .await
        .expect("tenant-mismatch draft should save");
    assert_eq!(
        store
            .publish_draft(
                foreign_context,
                publisher,
                PublishDraftCommand {
                    expected_draft: foreign_tenant_draft.clone(),
                    expected_revision: saved_foreign_tenant_draft.revision,
                    publication: ProblemVersionRef {
                        problem: ProblemId::from_uuid(uuid(621)),
                        version: VersionId::from_uuid(uuid(622)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::TenantMismatch),
        "a foreign tenant cannot publish another tenant's draft"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, foreign_tenant_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(foreign_tenant_draft)),
        "a tenant mismatch must retain the owner's draft"
    );

    let imathas_workspace = WorkspaceId::from_uuid(uuid(623));
    let imathas_draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            source: DraftQuestionSource::Imathas {
                provider: "myopenmath".to_string(),
                item_ref: "4711".to_string(),
            },
            ..draft_question(imathas_workspace)
        },
        revises: None,
        derived_from: None,
    };
    let imathas_publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(624)),
        version: VersionId::from_uuid(uuid(625)),
    };
    let saved_imathas_draft = store
        .upsert_draft(context, publisher, None, imathas_draft.clone())
        .await
        .expect("iMathAS draft should save in the sandbox");
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: imathas_draft.clone(),
                    expected_revision: saved_imathas_draft.revision,
                    publication: imathas_publication,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, imathas_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(imathas_draft.clone())),
        "an unprepared iMathAS source must not consume the sandbox draft"
    );
    let prepared_imathas_artifact = source_artifact(
        imathas_publication,
        QuestionBackend::Imathas,
        ObjectId::from_uuid(uuid(626)),
    );
    let prepared_imathas_source = QuestionSource::Imathas {
        provider: "myopenmath".to_string(),
        item_ref: "4711".to_string(),
        snapshot: ObjectId::from_uuid(uuid(626)),
        snapshot_sha256: prepared_imathas_artifact.object.sha256.to_string(),
        integration_profile: "lti-1.3".to_string(),
    };
    let imathas_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: imathas_draft,
                expected_revision: saved_imathas_draft.revision,
                publication: imathas_publication,
                published_source: prepared_imathas_source,
                source_artifact: Some(prepared_imathas_artifact),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities,
            },
        )
        .await
        .expect("a server-prepared iMathAS snapshot should persist");
    assert!(matches!(
        imathas_record.question.source,
        QuestionSource::Imathas { .. }
    ));
}

async fn exercise_source_artifact_binding<S>(store: &S)
where
    S: Store + CatalogStore + CatalogSourceStore,
{
    let tenant = TenantId::from_uuid(uuid(6_500));
    let foreign_tenant = TenantId::from_uuid(uuid(6_501));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(6_502));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(6_503)),
        version: VersionId::from_uuid(uuid(6_504)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            source: DraftQuestionSource::Qti {
                item_id: "item-1".to_string(),
                import_id: WorkspaceImportId::from_uuid(uuid(6_506)),
            },
            ..draft_question(WorkspaceId::from_uuid(uuid(6_505)))
        },
        revises: None,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("source-backed draft should save");
    let artifact = source_artifact(
        reference,
        QuestionBackend::Qti,
        ObjectId::from_uuid(uuid(6_507)),
    );
    let source = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: artifact.object.id,
        package_sha256: artifact.object.sha256.to_string(),
    };
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: source.clone(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, draft.question.workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(draft.clone()))
    );
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None)
    );

    let mismatched_item = QuestionSource::Qti {
        item_id: "other-item".to_string(),
        package_object: artifact.object.id,
        package_sha256: artifact.object.sha256.to_string(),
    };
    let mismatched_object = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: ObjectId::from_uuid(uuid(6_508)),
        package_sha256: artifact.object.sha256.to_string(),
    };
    let mismatched_checksum = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: artifact.object.id,
        package_sha256: "a".repeat(64),
    };
    for invalid_source in [mismatched_item, mismatched_object, mismatched_checksum] {
        assert!(matches!(
            store
                .publish_draft(
                    context,
                    publisher,
                    PublishDraftCommand {
                        expected_draft: draft.clone(),
                        expected_revision: saved_draft.revision,
                        publication: reference,
                        published_source: invalid_source,
                        source_artifact: Some(artifact.clone()),
                        qti_promotion: None,
                        flat_question_promotion: None,
                        publisher,
                        scope: PublicationScope::Institution,
                        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    let mut wrong_backend = artifact.clone();
    wrong_backend.backend = QuestionBackend::Webwork;
    let mut wrong_reference = artifact.clone();
    wrong_reference.reference.version = VersionId::from_uuid(uuid(6_509));
    let mut wrong_category = artifact.clone();
    wrong_category.object.key = ObjectKey::ProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset: AssetId::from_uuid(uuid(6_510)),
        object: wrong_category.object.id,
    };
    wrong_category.object.category = objects::ObjectCategory::Asset;
    for invalid in [wrong_backend, wrong_reference, wrong_category] {
        assert!(matches!(
            store
                .publish_draft(
                    context,
                    publisher,
                    PublishDraftCommand {
                        expected_draft: draft.clone(),
                        expected_revision: saved_draft.revision,
                        publication: reference,
                        published_source: source.clone(),
                        source_artifact: Some(invalid),
                        qti_promotion: None,
                        flat_question_promotion: None,
                        publisher,
                        scope: PublicationScope::Institution,
                        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert_eq!(
        store
            .get_draft(context, publisher, draft.question.workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(draft.clone()))
    );
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None)
    );
    assert_eq!(
        store.get_catalog_problem(context, reference).await,
        Ok(None),
        "a rejected source binding must not create a visible immutable version"
    );
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: source,
                    source_artifact: Some(artifact.clone()),
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None),
        "generic publication must not expose a QTI source binding"
    );
    assert_eq!(
        store
            .catalog_source_artifact(foreign_context, reference)
            .await,
        Ok(None),
        "foreign tenant must not learn a private source exists"
    );
}

async fn exercise_run_api_store<S>(store: &S, feedback_disclosure: FeedbackDisclosure)
where
    S: Store + CatalogStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture_offset = if feedback_disclosure == FeedbackDisclosure::OnRelease {
        10_000
    } else {
        0
    };
    let tenant = TenantId::from_uuid(uuid(401 + fixture_offset));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(402));
    let student_user = UserId::from_uuid(uuid(403));
    let second_instructor = UserId::from_uuid(uuid(10_403 + fixture_offset));
    let workspace = WorkspaceId::from_uuid(uuid(404));
    let problem = ProblemId::from_uuid(uuid(405 + fixture_offset));
    let version = VersionId::from_uuid(uuid(406 + fixture_offset));
    let course = CourseId::from_uuid(uuid(407));
    let assignment = AssignmentId::from_uuid(uuid(408));
    let enrollment = EnrollmentId::from_uuid(uuid(409));
    let first_run = RunId::from_uuid(uuid(410));
    let ignored_resume_id = RunId::from_uuid(uuid(411));
    let attempt_id = QuestionAttemptId::from_uuid(uuid(412));

    let mut run_question = draft_question(workspace);
    // This fixture specifically proves receipt-time replay behavior: a later
    // completion must not unlock deferred feedback on the earlier receipt.
    run_question.attempt_policy.feedback = feedback_disclosure;
    let draft = DraftRecord {
        tenant,
        question: run_question,
        revises: None,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("run fixture draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("run fixture publication");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Run API biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: publisher,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: second_instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student_user,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("run fixture course");
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Run API assignment".to_string(),
                items: fixed_items(vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("run fixture assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(413)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("run fixture enrollment");

    let run = store
        .start_or_resume_run(context, student_user, assignment, first_run)
        .await
        .expect("first run should start");
    let resumed = store
        .start_or_resume_run(context, student_user, assignment, ignored_resume_id)
        .await
        .expect("active run should resume");
    assert_eq!(resumed, run);

    let issue = IssueQuestionAttemptCommand {
        actor: student_user,
        attempt: attempt_id,
        run: run.id,
        assignment_position: 0,
        problem,
        question_version: version,
        seed: 991,
        parameter_hash: "parameter-hash".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "rendered-hash".to_string(),
        },
        prefetched: None,
        predecessor_submission: None,
    };
    let attempt = store
        .issue_or_resume_question_attempt(context, issue.clone())
        .await
        .expect("question should issue");
    let resumed_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                attempt: QuestionAttemptId::from_uuid(uuid(414)),
                seed: 992,
                ..issue
            },
        )
        .await
        .expect("unanswered question should resume");
    assert_eq!(resumed_attempt, attempt);

    let blocked_second_position = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 993,
                parameter_hash: "second-parameter-hash".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "second-rendered-hash".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await;
    assert!(matches!(
        blocked_second_position,
        Err(StoreError::InvalidRecord(message))
            if message == "another question attempt is already active in this run"
    ));

    let reservation = PrefetchedQuestion {
        tenant,
        run: run.id,
        predecessor: attempt.id,
        assignment_position: 1,
        problem,
        question_version: version,
        seed: 993,
        parameter_hash: "prefetched-parameter-hash".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "prefetched-rendered-hash".to_string(),
        },
    };
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Ok(reservation.clone()),
        "prefetch reserves immutable next-question inputs only",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Ok(reservation.clone()),
        "an identical prefetch retry is idempotent",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: PrefetchedQuestion {
                        seed: reservation.seed + 1,
                        ..reservation.clone()
                    },
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a conflicting prefetch retry cannot rewrite its immutable variation",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: second_instructor,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden),
        "another course member cannot reserve a student's next question",
    );
    assert_eq!(
        store
            .list_question_attempts(
                context,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid page size")),
            )
            .await
            .expect("reservation leaves the attempt list readable")
            .items,
        vec![attempt.clone()],
        "reservation neither creates an attempt nor starts a timer",
    );

    let response = StudentResponse::Numeric { value: 18.0 };
    let key = SubmissionIdempotencyKey::parse("submission-401").expect("valid key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(None)
    );
    let invalid_result = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: false,
                    points_earned: 1_001.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: key.clone(),
            },
        )
        .await;
    assert!(matches!(invalid_result, Err(StoreError::InvalidRecord(_))));
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(None),
        "a rejected backend result must leave the attempt unsubmitted"
    );
    let hostile_feedback = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent {
                    hint: Some(vec![ContentBlock::Table {
                        headers: vec!["residue".to_string(), "charge".to_string()],
                        rows: vec![vec!["Lys".to_string()]],
                        description: "malformed structural feedback fixture".to_string(),
                    }]),
                    correct_response: None,
                    rationale: None,
                },
                idempotency_key: key.clone(),
            },
        )
        .await;
    assert!(matches!(
        hostile_feedback,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(None),
        "rejected feedback must not leave a submission, feedback, or summary partial write"
    );
    let submitted = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent {
                    hint: Some(vec![ContentBlock::Text {
                        markdown: "Check the units.".to_string(),
                    }]),
                    correct_response: None,
                    rationale: Some(vec![ContentBlock::Text {
                        markdown: "The recorded calculation is dimensionally consistent."
                            .to_string(),
                    }]),
                },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("first response should commit");
    let replay = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("replay lookup")
        .expect("first receipt should replay");
    assert_eq!(replay.attempt, submitted.attempt);
    assert!(replay.feedback == submitted.feedback);
    assert_eq!(
        replay.feedback.content().hint,
        Some(vec![ContentBlock::Text {
            markdown: "Check the units.".to_string(),
        }]),
        "an exact replay returns the stored private feedback rather than regrading"
    );
    let before_completion = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("summary before completion");
    assert_eq!(before_completion.run.completed_at, None);
    assert_eq!(before_completion.outcomes.items.len(), 1);
    assert_eq!(
        before_completion.outcomes.items[0].feedback_policy, feedback_disclosure,
        "every policy must survive in the private redactor input"
    );
    assert!(before_completion.outcomes.items[0].feedback.is_some());
    assert_eq!(before_completion.outcomes.items[0].release, None);
    if feedback_disclosure == FeedbackDisclosure::OnRelease {
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(None),
            "a student may observe only their exact unreleased attempt state"
        );
        assert_eq!(
            store
                .get_run_summary_page(
                    context,
                    student_user,
                    run.id,
                    PageRequest::first(PageSize::new(10).expect("valid bounded page")),
                )
                .await
                .expect("unreleased summary")
                .outcomes
                .items[0]
                .release,
            None,
            "summary redaction input reflects current unreleased state"
        );
        assert_eq!(
            store
                .release_attempt_feedback(
                    TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(9_401))),
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::NotFound),
            "a foreign tenant must not enumerate a release target"
        );
        assert_eq!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: student_user,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::NotFound),
            "an ordinary student cannot release feedback"
        );
        let released = store
            .release_attempt_feedback(
                context,
                ReleaseAttemptFeedbackCommand {
                    actor: publisher,
                    attempt: attempt.id,
                },
            )
            .await
            .expect("course instructor releases on-release feedback");
        assert_eq!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Ok(released.clone()),
            "same authorized actor release is idempotent"
        );
        assert_eq!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: second_instructor,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::Conflict),
            "a release remains immutable for a different authorized instructor"
        );
        assert_eq!(
            store
                .get_attempt_feedback_release(context, student_user, attempt.id)
                .await,
            Ok(Some(released)),
            "the owner can read current released state without listing feedback"
        );
        assert!(
            store
                .get_run_summary_page(
                    context,
                    student_user,
                    run.id,
                    PageRequest::first(PageSize::new(10).expect("valid bounded page")),
                )
                .await
                .expect("released summary")
                .outcomes
                .items[0]
                .release
                .is_some(),
            "summary redaction input reads current release state, not receipt state"
        );
    } else {
        assert!(matches!(
            store
                .release_attempt_feedback(
                    context,
                    ReleaseAttemptFeedbackCommand {
                        actor: publisher,
                        attempt: attempt.id,
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    attempt: attempt.id,
                    response: response.clone(),
                    result: AttemptResult {
                        correct: false,
                        points_earned: 0.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent {
                        hint: Some(vec![ContentBlock::Text {
                            markdown: "a changed retry cannot replace this".to_string(),
                        }]),
                        correct_response: None,
                        rationale: None,
                    },
                    idempotency_key: key.clone(),
                },
            )
            .await
            .expect("exact replay should ignore the changed proposed grade")
            .feedback
            == submitted.feedback
    );
    assert_eq!(
        store
            .replay_submission(
                context,
                student_user,
                attempt.id,
                &StudentResponse::Numeric { value: 19.0 },
                &key,
            )
            .await,
        Err(StoreError::Conflict)
    );
    let changed_key =
        SubmissionIdempotencyKey::parse("submission-401-new").expect("valid changed key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &changed_key)
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(submitted.run.completed_at, None);
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(Some(attempt.id)),
        "one committed predecessor without a receipt successor is recoverable",
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, second_instructor, run.id)
            .await,
        Err(StoreError::Forbidden),
        "another course member cannot discover a student's pending submission",
    );

    let second_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 0,
                parameter_hash: "ignored-by-prefetch".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: Some(reservation.clone()),
                predecessor_submission: Some(attempt.id),
            },
        )
        .await
        .expect("the next position should issue after the active response commits");
    assert_eq!(second_attempt.seed, reservation.seed);
    assert_eq!(second_attempt.parameter_hash, reservation.parameter_hash);
    assert_eq!(
        store
            .submission_next_attempt(context, student_user, attempt.id)
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            second_attempt.id
        )),
        "promotion atomically fixes the predecessor receipt successor",
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(None),
        "promotion consumes the only pending receipt rather than leaving recovery ambiguous",
    );
    assert_eq!(
        store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student_user,
                    reservation: reservation.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "an already-attempted target position cannot be reserved again",
    );
    assert_eq!(
        store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor: student_user,
                    attempt: QuestionAttemptId::from_uuid(uuid(416)),
                    run: run.id,
                    assignment_position: 1,
                    problem,
                    question_version: version,
                    seed: 0,
                    parameter_hash: "ignored-by-prefetch".to_string(),
                    provenance: reservation.provenance.clone(),
                    prefetched: Some(reservation.clone()),
                    predecessor_submission: None,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a reservation cannot be consumed or resumed under another receipt predecessor",
    );
    let completed = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: second_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-402")
                    .expect("valid second key"),
            },
        )
        .await
        .expect("second response should complete the run");
    assert_eq!(
        completed.run.completed_at,
        completed.attempt.timer.submitted_at
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run.id)
            .await,
        Ok(Some(second_attempt.id)),
        "a terminal committed submission is the sole recoverable receipt until finalized",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, second_instructor, second_attempt.id, None)
            .await,
        Err(StoreError::NotFound),
        "another course member cannot enumerate or finalize a student's pending receipt",
    );
    let cross_run = store
        .start_or_resume_run(
            context,
            student_user,
            assignment,
            RunId::from_uuid(uuid(417)),
        )
        .await
        .expect("a completed run permits a new run");
    let cross_run_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(418)),
                run: cross_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 994,
                parameter_hash: "cross-run-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("cross-run active attempt");
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                second_attempt.id,
                Some(cross_run_attempt.id),
            )
            .await,
        Err(StoreError::Conflict),
        "a receipt cannot link to an attempt from another run",
    );
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: cross_run_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-cross-run-1")
                    .expect("valid cross-run key"),
            },
        )
        .await
        .expect("first deliberately unfinalized recovery fixture submission");
    let cross_run_second = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(419)),
                run: cross_run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 995,
                parameter_hash: "cross-run-second-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a recovery fixture can reproduce a second issue after a lost finalization");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: cross_run_second.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-cross-run-2")
                    .expect("valid second cross-run key"),
            },
        )
        .await
        .expect("second deliberately unfinalized recovery fixture submission");
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, cross_run.id)
            .await,
        Err(StoreError::Conflict),
        "multiple unresolved receipt links are ambiguous and must never be guessed",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, student_user, second_attempt.id, None)
            .await,
        Ok(()),
        "a terminal submission records its explicit no-successor receipt state",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, student_user, second_attempt.id, None)
            .await,
        Ok(()),
        "the explicit no-successor receipt state is idempotent",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                second_attempt.id,
                Some(attempt.id),
            )
            .await,
        Err(StoreError::Conflict),
        "a finalized no-successor receipt cannot later point at an attempt",
    );
    assert_eq!(
        store
            .submission_next_attempt(context, student_user, attempt.id)
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            second_attempt.id
        )),
        "the first receipt keeps its original successor after that successor is submitted",
    );
    assert_eq!(
        (
            completed.summary.completed_run_count,
            completed.summary.total_question_attempts,
            completed.summary.current_score,
        ),
        (1, 2, Some(1.0))
    );
    let replay_after_completion = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("first submission replay after later completion")
        .expect("first submission receipt remains available");
    assert_eq!(replay_after_completion.attempt, submitted.attempt);
    assert_eq!(replay_after_completion.run, submitted.run);
    assert_eq!(replay_after_completion.summary, submitted.summary);
    assert!(replay_after_completion.feedback == submitted.feedback);
    let attempt_page = store
        .list_question_attempts(
            context,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("attempt page");
    assert_eq!(
        attempt_page.items,
        vec![submitted.attempt, completed.attempt]
    );
    let first_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary page");
    assert_eq!(first_summary_page.run, completed.run);
    // The receipt retains the summary observed when it committed. The
    // enrollment summary is live and has since observed the deliberately
    // completed independent recovery fixture run above.
    assert_eq!(first_summary_page.summary.completed_run_count, 2);
    assert_eq!(first_summary_page.summary.total_question_attempts, 4);
    assert!(first_summary_page.practice_allowed);
    assert_eq!(first_summary_page.outcomes.items.len(), 1);
    assert!(first_summary_page.outcomes.items[0].response.is_some());
    assert!(first_summary_page.outcomes.items[0].feedback.is_some());
    let continuation = first_summary_page
        .outcomes
        .next_cursor
        .expect("two outcomes require a cursor");
    let second_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::after(continuation, PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary continuation");
    assert_eq!(second_summary_page.outcomes.items.len(), 1);
    assert_ne!(
        first_summary_page.outcomes.items[0].attempt, second_summary_page.outcomes.items[0].attempt,
        "keyset pages must not duplicate outcomes"
    );
    assert_eq!(second_summary_page.outcomes.next_cursor, None);
    let instructor_summary = store
        .get_run_summary_page(
            context,
            publisher,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("direct course instructor summary");
    assert_eq!(instructor_summary.outcomes.items.len(), 2);
    let foreign_actor = UserId::from_uuid(uuid(99_999 + fixture_offset));
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                foreign_actor,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .get_run_summary_page(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    99_998 + fixture_offset,
                ))),
                student_user,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));

    let locked_assignment = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("locked assignment read")
        .expect("run assignment exists");
    let mut rescored_items = locked_assignment.record.items.clone();
    rescored_items[0].points_possible = PointValue::from_whole(2);
    let rescored = store
        .replace_assignment(
            context,
            course,
            assignment,
            locked_assignment.revision,
            AssignmentUpdate {
                title: locked_assignment.record.title.clone(),
                items: rescored_items.clone(),
                selection_groups: locked_assignment.record.selection_groups.clone(),
                policies: locked_assignment.record.policies,
            },
        )
        .await
        .expect("point edits remain valid after a run exists");
    assert_eq!(
        rescored.scoring_status,
        question_model::ScoringStatus::Recalculating
    );
    let scoring_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("scoring lease"),
        )
        .await
        .expect("claim scoring job")
        .expect("point edit queues scoring work");
    let (queued_assignment, generation) = match scoring_job.payload {
        JobPayload::RecalculateAssignment {
            assignment,
            generation,
        } => (assignment, generation),
        payload => panic!("expected scoring job, got {payload:?}"),
    };
    assert_eq!(
        (queued_assignment, generation),
        (assignment, rescored.scoring_generation)
    );
    let scoring_command = AssignmentScoringWorkerCommand {
        job: scoring_job.id,
        lease: scoring_job.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, scoring_command)
        .await
        .expect("scoring generation stages privately");
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("staged assignment read")
            .expect("staged assignment exists")
            .scoring_status,
        question_model::ScoringStatus::Recalculating,
        "private staging must not publish partial current scores"
    );
    let mut superseding_items = rescored.record.items.clone();
    superseding_items[0].points_possible = PointValue::from_whole(3);
    let superseding = store
        .replace_assignment(
            context,
            course,
            assignment,
            rescored.revision,
            AssignmentUpdate {
                title: rescored.record.title.clone(),
                items: superseding_items.clone(),
                selection_groups: rescored.record.selection_groups.clone(),
                policies: rescored.record.policies,
            },
        )
        .await
        .expect("a newer scoring definition supersedes staged work");
    assert!(superseding.scoring_generation > generation);
    assert_eq!(
        store
            .commit_assignment_scoring(context, scoring_command)
            .await,
        Ok(AssignmentScoringCommitOutcome::Superseded),
        "an old generation must never replace current scores"
    );
    let still_pending = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("superseded assignment read")
        .expect("superseded assignment exists");
    assert_eq!(
        (
            still_pending.scoring_generation,
            still_pending.scoring_status
        ),
        (
            superseding.scoring_generation,
            question_model::ScoringStatus::Recalculating
        ),
        "discarding old staging must leave the new generation pending"
    );
    let current_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("current scoring lease"),
        )
        .await
        .expect("claim current scoring job")
        .expect("superseding edit queues scoring work");
    let current_generation = match current_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, assignment);
            generation
        }
        payload => panic!("expected superseding scoring job, got {payload:?}"),
    };
    assert_eq!(current_generation, superseding.scoring_generation);
    let current_command = AssignmentScoringWorkerCommand {
        job: current_job.id,
        lease: current_job.lease_token,
        assignment,
        generation: current_generation,
    };
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .expect("current scoring generation stages privately");
    let concurrent_run = store
        .start_or_resume_run(
            context,
            student_user,
            assignment,
            RunId::from_uuid(uuid(89_970 + fixture_offset)),
        )
        .await
        .expect("student activity may continue during recalculation");
    let concurrent_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_971 + fixture_offset)),
                run: concurrent_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 996,
                parameter_hash: "concurrent-scoring-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("concurrent scoring attempt issues");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: concurrent_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-during-scoring")
                    .expect("valid concurrent scoring key"),
            },
        )
        .await
        .expect("submission may commit while recalculation is pending");
    assert_eq!(
        store
            .commit_assignment_scoring(context, current_command)
            .await,
        Err(StoreError::Conflict),
        "staging prepared before a new submission must not publish an incomplete generation"
    );
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .expect("same live claim restages after concurrent activity");
    assert_eq!(
        store
            .commit_assignment_scoring(context, current_command)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let current_assignment = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("rescored assignment read")
        .expect("rescored assignment exists");
    assert_eq!(
        (
            current_assignment.scoring_generation,
            current_assignment.scoring_status
        ),
        (current_generation, question_model::ScoringStatus::Current)
    );
    let mut added_items = superseding_items;
    let mut added = added_items[0].clone();
    added.id = AssignmentItemId::from_uuid(uuid(89_980 + fixture_offset));
    added.position = u32::try_from(added_items.len()).expect("fixture position fits");
    added_items.push(added);
    assert_eq!(
        store
            .replace_assignment(
                context,
                course,
                assignment,
                superseding.revision,
                AssignmentUpdate {
                    title: superseding.record.title.clone(),
                    items: added_items,
                    selection_groups: superseding.record.selection_groups.clone(),
                    policies: superseding.record.policies,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "new pinned content is locked after the first run"
    );

    let delete_assignment = AssignmentId::from_uuid(uuid(89_960 + fixture_offset));
    let delete_enrollment = EnrollmentId::from_uuid(uuid(89_961 + fixture_offset));
    let delete_run_id = RunId::from_uuid(uuid(89_962 + fixture_offset));
    let delete_items = fixed_items(vec![
        ProblemVersionRef { problem, version },
        ProblemVersionRef { problem, version },
    ]);
    let retired_item = delete_items[0].id;
    let retained_item = delete_items[1].id;
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: delete_assignment,
                tenant,
                course_id: course,
                title: "Delete and Regrade fixture".to_string(),
                items: delete_items,
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("Delete and Regrade assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: delete_enrollment,
                tenant,
                assignment: delete_assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(89_963 + fixture_offset)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("Delete and Regrade enrollment");
    let delete_run = store
        .start_or_resume_run(context, student_user, delete_assignment, delete_run_id)
        .await
        .expect("Delete and Regrade run");
    let affected_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_964 + fixture_offset)),
                run: delete_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 997,
                parameter_hash: "delete-and-regrade-active".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("affected active attempt");
    let before_delete = store
        .get_assignment_for_edit(context, delete_assignment)
        .await
        .expect("Delete and Regrade edit read")
        .expect("Delete and Regrade assignment exists");
    let delete_command = DeleteAndRegradeAssignmentItemCommand {
        course,
        assignment: delete_assignment,
        item: retired_item,
        expected_revision: before_delete.revision,
    };
    assert_eq!(
        store
            .delete_and_regrade_assignment_item(context, delete_command)
            .await,
        Err(StoreError::Conflict),
        "an affected in-progress attempt blocks retirement"
    );
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: affected_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "submission-delete-and-regrade-affected",
                )
                .expect("valid Delete and Regrade key"),
            },
        )
        .await
        .expect("submitted evidence permits retirement");
    let retired = store
        .delete_and_regrade_assignment_item(context, delete_command)
        .await
        .expect("Delete and Regrade after submission");
    let retired_record = retired
        .record
        .items
        .iter()
        .find(|item| item.id == retired_item)
        .expect("retired item remains a tombstone");
    assert_eq!(
        (
            retired_record.delivery_state,
            retired_record.scoring_mode,
            retired.scoring_status
        ),
        (
            AssignmentDeliveryState::Retired,
            AssignmentScoringMode::Excluded,
            question_model::ScoringStatus::Recalculating
        )
    );
    assert_eq!(
        store
            .delete_and_regrade_assignment_item(
                context,
                DeleteAndRegradeAssignmentItemCommand {
                    expected_revision: retired.revision,
                    ..delete_command
                },
            )
            .await,
        Ok(retired.clone()),
        "an exact retry does not create another revision or generation"
    );
    let delete_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("Delete and Regrade lease"),
        )
        .await
        .expect("claim Delete and Regrade scoring job")
        .expect("Delete and Regrade queues scoring work");
    let delete_generation = match delete_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, delete_assignment);
            generation
        }
        payload => panic!("expected Delete and Regrade scoring job, got {payload:?}"),
    };
    let delete_scoring = AssignmentScoringWorkerCommand {
        job: delete_job.id,
        lease: delete_job.lease_token,
        assignment: delete_assignment,
        generation: delete_generation,
    };
    store
        .prepare_assignment_scoring(context, delete_scoring)
        .await
        .expect("Delete and Regrade scoring stages");
    assert_eq!(
        store
            .commit_assignment_scoring(context, delete_scoring)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    assert!(
        store
            .get_run_summary_page(
                context,
                student_user,
                delete_run.id,
                PageRequest::first(PageSize::new(10).expect("Delete and Regrade page")),
            )
            .await
            .expect("student Delete and Regrade summary")
            .outcomes
            .items
            .is_empty(),
        "normal student feedback hides retired evidence"
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                delete_run.id,
                PageRequest::first(PageSize::new(10).expect("support evidence page")),
            )
            .await
            .expect("instructor retained-evidence summary")
            .outcomes
            .items
            .len(),
        1,
        "authorized instructors retain support access to protected evidence"
    );
    let unaffected_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_965 + fixture_offset)),
                run: delete_run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 998,
                parameter_hash: "delete-and-regrade-unaffected".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("unaffected immutable run item remains answerable");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: unaffected_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "submission-delete-and-regrade-unaffected",
                )
                .expect("valid unaffected key"),
            },
        )
        .await
        .expect("existing run completes with retired evidence excluded");
    let future_delete_run = store
        .start_or_resume_run(
            context,
            student_user,
            delete_assignment,
            RunId::from_uuid(uuid(89_966 + fixture_offset)),
        )
        .await
        .expect("future run after Delete and Regrade");
    assert_eq!(
        store
            .assignment_run_items(context, future_delete_run.id)
            .await
            .expect("future Delete and Regrade run items")
            .iter()
            .map(|item| item.assignment_item)
            .collect::<Vec<_>>(),
        vec![retained_item],
        "future runs omit the tombstone while old evidence remains"
    );

    let support_assignment = AssignmentId::from_uuid(uuid(89_972 + fixture_offset));
    let support_enrollment = EnrollmentId::from_uuid(uuid(89_973 + fixture_offset));
    let support_run_id = RunId::from_uuid(uuid(89_974 + fixture_offset));
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: support_assignment,
                tenant,
                course_id: course,
                title: "Attempt support fixture".to_string(),
                items: fixed_items(vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("attempt support assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: support_enrollment,
                tenant,
                assignment: support_assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(89_975 + fixture_offset)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("attempt support enrollment");
    let support_run = store
        .start_or_resume_run(context, student_user, support_assignment, support_run_id)
        .await
        .expect("attempt support run");
    let support_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_976 + fixture_offset)),
                run: support_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 999,
                parameter_hash: "force-submit-active".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("attempt support question");
    let force_action = AttemptSupportActionId::from_uuid(uuid(89_977 + fixture_offset));
    assert_eq!(
        store
            .force_submit_attempt(
                context,
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: student_user,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a student cannot force-submit an educational record"
    );
    assert_eq!(
        store
            .force_submit_attempt(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    89_978 + fixture_offset,
                ))),
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot enumerate a support target"
    );
    let forced = store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: publisher,
                attempt: support_attempt.id,
            },
        )
        .await
        .expect("direct course instructor force-submits");
    assert_eq!(
        (forced.kind, forced.previous_status, forced.resulting_status),
        (
            AttemptSupportAction::ForceSubmit,
            AttemptStatus::InProgress,
            AttemptStatus::NeedsManualGrading,
        )
    );
    assert_eq!(
        store
            .force_submit_attempt(
                context,
                ForceSubmitAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Ok(forced),
        "an exact support retry returns the original audit record"
    );
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: force_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "one stable action identity cannot be reused for a different mutation"
    );
    assert_eq!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    attempt: support_attempt.id,
                    response: response.clone(),
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse(
                        "submission-after-force-submit",
                    )
                    .expect("valid force-submit conflict key"),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "force-submit closes the ordinary student submission path"
    );
    let forced_current = store
        .get_question_attempt(context, support_attempt.id)
        .await
        .expect("force-submitted attempt read")
        .expect("force-submitted attempt exists");
    assert_eq!(forced_current.status, AttemptStatus::NeedsManualGrading);
    assert!(forced_current.response.is_none());
    assert!(forced_current.result.is_none());
    assert_eq!(forced_current.timer.submitted_at, Some(forced.occurred_at));

    let clear_forced_action = AttemptSupportActionId::from_uuid(uuid(89_979 + fixture_offset));
    let cleared_forced = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_forced_action,
                actor: publisher,
                attempt: support_attempt.id,
            },
        )
        .await
        .expect("instructor clears force-submitted attempt");
    assert_eq!(
        (
            cleared_forced.previous_status,
            cleared_forced.resulting_status
        ),
        (AttemptStatus::NeedsManualGrading, AttemptStatus::Cleared)
    );
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: clear_forced_action,
                    actor: publisher,
                    attempt: support_attempt.id,
                },
            )
            .await,
        Ok(cleared_forced),
        "an exact clear retry is harmless"
    );
    assert!(
        store
            .get_run_summary_page(
                context,
                student_user,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("support student page")),
            )
            .await
            .expect("student support summary")
            .outcomes
            .items
            .is_empty(),
        "cleared evidence is absent from the ordinary student summary"
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("support instructor page")),
            )
            .await
            .expect("instructor support summary")
            .outcomes
            .items
            .len(),
        1,
        "the instructor retains raw evidence access after clear"
    );

    let replacement_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_981 + fixture_offset)),
                run: support_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 1_000,
                parameter_hash: "replacement-after-clear".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a cleared position may issue a replacement");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: replacement_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("submission-support-replacement")
                    .expect("valid support replacement key"),
            },
        )
        .await
        .expect("replacement attempt submits");
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: AttemptSupportActionId::from_uuid(uuid(89_982 + fixture_offset,)),
                    actor: student_user,
                    attempt: replacement_attempt.id,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a student cannot clear a submitted evaluation"
    );
    let clear_scored_action = AttemptSupportActionId::from_uuid(uuid(89_983 + fixture_offset));
    let cleared_scored = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: publisher,
                attempt: replacement_attempt.id,
            },
        )
        .await
        .expect("instructor clears submitted evaluation");
    assert_eq!(cleared_scored.previous_status, AttemptStatus::Submitted);
    assert_eq!(cleared_scored.resulting_status, AttemptStatus::Cleared);
    assert_eq!(
        store
            .clear_attempt(
                context,
                ClearAttemptCommand {
                    action: clear_scored_action,
                    actor: publisher,
                    attempt: replacement_attempt.id,
                },
            )
            .await,
        Ok(cleared_scored),
        "a clear retry neither advances the generation nor queues duplicate work"
    );
    let post_clear_replacement = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(89_984 + fixture_offset)),
                run: support_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: 1_001,
                parameter_hash: "replacement-after-scored-clear".to_string(),
                provenance: reservation.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("a cleared correct response does not block a replacement");
    assert_eq!(post_clear_replacement.status, AttemptStatus::InProgress);
    let support_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("support scoring lease"),
        )
        .await
        .expect("claim support scoring job")
        .expect("clearing a scored attempt queues recalculation");
    let support_generation = match support_job.payload {
        JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation,
        } => {
            assert_eq!(queued_assignment, support_assignment);
            generation
        }
        payload => panic!("expected attempt-clear scoring job, got {payload:?}"),
    };
    let support_scoring = AssignmentScoringWorkerCommand {
        job: support_job.id,
        lease: support_job.lease_token,
        assignment: support_assignment,
        generation: support_generation,
    };
    store
        .prepare_assignment_scoring(context, support_scoring)
        .await
        .expect("attempt-clear scoring stages without the cleared result");
    assert_eq!(
        store
            .commit_assignment_scoring(context, support_scoring)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );
    let support_assignment_current = store
        .get_assignment_for_edit(context, support_assignment)
        .await
        .expect("support assignment state read")
        .expect("support assignment exists");
    assert_eq!(
        (
            support_assignment_current.scoring_generation,
            support_assignment_current.scoring_status,
        ),
        (support_generation, question_model::ScoringStatus::Current,)
    );
    assert_eq!(
        store
            .get_run_summary_page(
                context,
                publisher,
                support_run.id,
                PageRequest::first(PageSize::new(10).expect("retained support evidence page")),
            )
            .await
            .expect("retained support evidence summary")
            .outcomes
            .items
            .len(),
        3,
        "the instructor sees both cleared records and the active replacement"
    );

    // Scale behavior is deliberately exercised through the Store, not just
    // the cursor helper: a later practice run may contain far more outcomes
    // than an ordinary small assignment. `apply_activity_transition` supplies
    // persisted, server-owned attempt records without invoking a grader.
    let scale_run_id = RunId::from_uuid(uuid(90_000 + fixture_offset));
    let scale_problems = vec![ProblemVersionRef { problem, version }; 51];
    let scale_assignment = AssignmentId::from_uuid(uuid(89_990 + fixture_offset));
    let scale_enrollment = EnrollmentId::from_uuid(uuid(89_991 + fixture_offset));
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: scale_assignment,
                tenant,
                course_id: course,
                title: "Run summary scale fixture".to_string(),
                items: fixed_items(scale_problems),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("independent scale assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: scale_enrollment,
                tenant,
                assignment: scale_assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(89_992 + fixture_offset)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("independent scale enrollment");
    let scale_run = store
        .start_or_resume_run(context, student_user, scale_assignment, scale_run_id)
        .await
        .expect("post-completion scale practice run");
    for position in 0_u32..51 {
        store
            .apply_activity_transition(
                context,
                ActivityTransition::RecordQuestionAttempt {
                    attempt: Box::new(QuestionAttempt {
                        id: QuestionAttemptId::from_uuid(uuid(
                            90_100 + fixture_offset + u128::from(position),
                        )),
                        tenant,
                        run: scale_run.id,
                        problem,
                        question_version: version,
                        assignment_position: position,
                        seed: u64::from(position),
                        parameter_hash: format!("scale-parameter-{position}"),
                        response: None,
                        status: question_model::AttemptStatus::InProgress,
                        result: None,
                        timer: AttemptTimerRecord {
                            issued_at: ActivityTimestamp::from_unix_millis(i64::from(position)),
                            deadline: None,
                            submitted_at: None,
                        },
                        provenance: AttemptProvenance {
                            adapter: implementation("native"),
                            renderer: None,
                            generator: None,
                            source_artifact: None,
                            asset_objects: Vec::new(),
                            grading: implementation("numeric"),
                            rendered_question_sha256: format!("scale-rendered-{position}"),
                        },
                    }),
                },
            )
            .await
            .expect("persisted scale attempt");
    }
    let mut cursor = None;
    let mut positions = Vec::new();
    let mut first_scale_cursor = None;
    loop {
        let request = match cursor {
            Some(cursor) => PageRequest::after(cursor, PageSize::new(7).expect("bounded page")),
            None => PageRequest::first(PageSize::new(7).expect("bounded page")),
        };
        let page = store
            .get_run_summary_page(context, student_user, scale_run.id, request)
            .await
            .expect("scale summary page");
        assert!(page.outcomes.items.len() <= 7, "every page stays bounded");
        positions.extend(
            page.outcomes
                .items
                .iter()
                .map(|outcome| outcome.assignment_position),
        );
        if first_scale_cursor.is_none() {
            first_scale_cursor = page.outcomes.next_cursor.clone();
        }
        cursor = page.outcomes.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(positions, (0_u32..51).collect::<Vec<_>>());
    let scale_cursor = first_scale_cursor.expect("first scale page has continuation");
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                student_user,
                run.id,
                PageRequest::after(
                    scale_cursor.clone(),
                    PageSize::new(7).expect("bounded page")
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let mut tampered = scale_cursor.as_str().as_bytes().to_vec();
    tampered[10] = if tampered[10] == b'A' { b'B' } else { b'A' };
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                student_user,
                scale_run.id,
                PageRequest::after(
                    Cursor::parse(String::from_utf8(tampered).expect("ASCII cursor"))
                        .expect("nonempty cursor"),
                    PageSize::new(7).expect("bounded page"),
                ),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_store_conforms() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    exercise_assignment_cas(&store).await;
    exercise_publication_identity_boundary(&store).await;
    exercise_source_artifact_binding(&store).await;
}

#[tokio::test]
async fn memory_assignment_timing_edits_and_auto_submit_are_generation_fenced() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(95_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(95_001));
    let student = UserId::from_uuid(uuid(95_002));
    let course = CourseId::from_uuid(uuid(95_003));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Mutable timing course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("timing course");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        95_010,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(95_020));
    let initial = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Server-owned deadlines".to_string(),
                items: fixed_items(vec![reference, reference, reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("timing assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(95_021)),
                tenant,
                assignment,
                user: student,
                student: StudentId::from_uuid(uuid(95_022)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("timing enrollment");

    let ten_seconds = AssignmentTimingPolicy {
        time_limit_seconds: Some(10),
        ..AssignmentTimingPolicy::default()
    };
    let initial_command = UpdateAssignmentTimingCommand {
        actor: instructor,
        course,
        assignment,
        expected_revision: initial.revision,
        policy: ten_seconds,
    };
    assert_eq!(
        store
            .update_assignment_timing(
                context,
                UpdateAssignmentTimingCommand {
                    actor: student,
                    ..initial_command
                },
            )
            .await,
        Err(StoreError::NotFound),
        "students cannot mutate the server timing policy"
    );
    let timed = store
        .update_assignment_timing(context, initial_command)
        .await
        .expect("initial time limit");
    assert_eq!(
        store
            .update_assignment_timing(context, initial_command)
            .await,
        Ok(timed),
        "an exact retry neither increments the revision nor duplicates work"
    );
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(uuid(95_023)))
        .await
        .expect("timed run");
    let issue = |attempt, position, seed| IssueQuestionAttemptCommand {
        actor: student,
        attempt,
        run: run.id,
        assignment_position: position,
        problem: reference.problem,
        question_version: reference.version,
        seed,
        parameter_hash: format!("timing-parameters-{position}"),
        provenance: AttemptProvenance {
            adapter: implementation("timing-native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("timing-numeric"),
            rendered_question_sha256: format!("timing-render-{position}"),
        },
        prefetched: None,
        predecessor_submission: None,
    };
    let first = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_024)), 0, 1),
        )
        .await
        .expect("first timed question");
    assert_eq!(
        first.timer.deadline,
        Some(ActivityTimestamp::from_unix_millis(11_000))
    );
    assert!(
        store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("lease")
            )
            .await
            .expect("queue read")
            .is_none(),
        "a deadline job is not claimable early"
    );

    let twenty_seconds = AssignmentTimingPolicy {
        time_limit_seconds: Some(20),
        ..AssignmentTimingPolicy::default()
    };
    let extended = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: timed.revision,
                policy: twenty_seconds,
                ..initial_command
            },
        )
        .await
        .expect("active extension");
    assert_eq!(
        store
            .get_question_attempt(context, first.id)
            .await
            .expect("extended attempt read")
            .expect("extended attempt")
            .timer
            .deadline,
        Some(ActivityTimestamp::from_unix_millis(21_000))
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(15_000))
        .expect("advance past shortened limit");
    let shortened = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: extended.revision,
                policy: ten_seconds,
                ..initial_command
            },
        )
        .await
        .expect("shortening is an immediate transaction");
    let current = store
        .get_question_attempt(context, first.id)
        .await
        .expect("shortened attempt read")
        .expect("shortened attempt");
    assert_eq!(current.status, AttemptStatus::AutoSubmitted);
    assert!(current.response.is_none());
    assert!(current.result.is_none());
    assert_eq!(
        current.timer.submitted_at,
        Some(ActivityTimestamp::from_unix_millis(15_000))
    );

    let closes_at = |millis| AssignmentTimingPolicy {
        closes_at: Some(ActivityTimestamp::from_unix_millis(millis)),
        late_submission: LateSubmissionPolicy::Accept,
        ..AssignmentTimingPolicy::default()
    };
    let closes_sixteen = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: shortened.revision,
                policy: closes_at(16_000),
                ..initial_command
            },
        )
        .await
        .expect("move the next question to a close boundary");
    let second = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_025)), 1, 2),
        )
        .await
        .expect("second timed question");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(16_000))
        .expect("reach close boundary");
    let due = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("due queue read")
        .expect("deadline job is due");
    let timing_generation = match due.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, second.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: due.id,
                    lease: due.lease_token,
                    attempt: second.id,
                    timing_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    );
    assert_eq!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student,
                    attempt: second.id,
                    response: StudentResponse::Numeric { value: 18.0 },
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse("after-auto-submit")
                        .expect("submission key"),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );

    let closes_seventeen = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closes_sixteen.revision,
                policy: closes_at(17_000),
                ..initial_command
            },
        )
        .await
        .expect("open a third bounded question");
    let third = store
        .issue_or_resume_question_attempt(
            context,
            issue(QuestionAttemptId::from_uuid(uuid(95_026)), 2, 3),
        )
        .await
        .expect("third timed question");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(17_000))
        .expect("reach original third deadline");
    let stale = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("stale queue read")
        .expect("third deadline job");
    let stale_generation = match stale.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, third.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closes_seventeen.revision,
                policy: closes_at(20_000),
                ..initial_command
            },
        )
        .await
        .expect("extension races safely with a leased old generation");
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: stale.id,
                    lease: stale.lease_token,
                    attempt: third.id,
                    timing_generation: stale_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::Rescheduled)
    );
    assert_eq!(
        store
            .get_question_attempt(context, third.id)
            .await
            .expect("extended third read")
            .expect("extended third")
            .status,
        AttemptStatus::InProgress
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("reach extended deadline");
    let current = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("current queue read")
        .expect("rescheduled job is due");
    assert_eq!(current.id, stale.id, "the extension reuses the durable job");
    let current_generation = match current.payload {
        JobPayload::AutoSubmitAttempt {
            attempt,
            timing_generation,
        } => {
            assert_eq!(attempt, third.id);
            timing_generation
        }
        payload => panic!("expected attempt auto-submit, got {payload:?}"),
    };
    assert!(current_generation > stale_generation);
    assert_eq!(
        store
            .commit_attempt_auto_submit(
                context,
                AttemptAutoSubmitWorkerCommand {
                    job: current.id,
                    lease: current.lease_token,
                    attempt: third.id,
                    timing_generation: current_generation,
                },
            )
            .await,
        Ok(AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    );

    let limited_assignment = AssignmentId::from_uuid(uuid(95_030));
    let limited = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: limited_assignment,
                tenant,
                course_id: course,
                title: "One allowed run".to_string(),
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("attempt-limited assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(95_031)),
                tenant,
                assignment: limited_assignment,
                user: student,
                student: StudentId::from_uuid(uuid(95_032)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("attempt-limited enrollment");
    store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: instructor,
                course,
                assignment: limited_assignment,
                expected_revision: limited.revision,
                policy: AssignmentTimingPolicy {
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
            },
        )
        .await
        .expect("one-run limit");
    let limited_run = store
        .start_or_resume_run(
            context,
            student,
            limited_assignment,
            RunId::from_uuid(uuid(95_033)),
        )
        .await
        .expect("first allowed run");
    let limited_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(uuid(95_034)),
                run: limited_run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                seed: 4,
                parameter_hash: "attempt-limit-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("timing-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("timing-numeric"),
                    rendered_question_sha256: "attempt-limit-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("question in the first allowed run");
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: limited_attempt.id,
                response: StudentResponse::Numeric { value: 18.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("finish-limited-run")
                    .expect("submission key"),
            },
        )
        .await
        .expect("complete the only allowed run");
    assert!(matches!(
        store
            .start_or_resume_run(
                context,
                student,
                limited_assignment,
                RunId::from_uuid(uuid(95_035)),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_student_and_group_exceptions_are_most_permissive_and_immediate() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(96_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(96_001));
    let student = UserId::from_uuid(uuid(96_002));
    let course = CourseId::from_uuid(uuid(96_003));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Accommodation course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("course");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        96_010,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(96_020));
    let student_record = StudentId::from_uuid(uuid(96_021));
    let created = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Most permissive accommodations".to_string(),
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(96_022)),
                tenant,
                assignment,
                user: student,
                student: student_record,
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("enrollment");
    let base = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                policy: AssignmentTimingPolicy {
                    available_at: Some(ActivityTimestamp::from_unix_millis(30_000)),
                    closes_at: Some(ActivityTimestamp::from_unix_millis(60_000)),
                    time_limit_seconds: Some(10),
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
            },
        )
        .await
        .expect("base policy");
    let group_id = CourseGroupId::from_uuid(uuid(96_023));
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: group_id,
                    tenant,
                    course,
                    title: "Extended testing".to_string(),
                    members: vec![student],
                },
            },
        )
        .await
        .expect("course group");
    let group_exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(uuid(96_024)),
        target: AssignmentPolicyExceptionTarget::CourseGroup(group_id),
        available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
        closes_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(80_000),
        )),
        time_limit_seconds: Some(AssignmentExceptionLimit::Value(20)),
        attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
    };
    let group_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: base.revision,
                exception: group_exception,
            },
        )
        .await
        .expect("group exception");
    let student_exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(uuid(96_025)),
        target: AssignmentPolicyExceptionTarget::Student(student_record),
        available_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(25_000),
        )),
        closes_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(70_000),
        )),
        time_limit_seconds: Some(AssignmentExceptionLimit::Value(15)),
        attempt_limit: Some(AssignmentExceptionLimit::Value(3)),
    };
    let student_command = SetAssignmentPolicyExceptionCommand {
        actor: instructor,
        course,
        assignment,
        expected_revision: group_exception.assignment_revision,
        exception: student_exception.clone(),
    };
    assert_eq!(
        store
            .set_assignment_policy_exception(
                context,
                SetAssignmentPolicyExceptionCommand {
                    actor: student,
                    ..student_command.clone()
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    let stored_student = store
        .set_assignment_policy_exception(context, student_command.clone())
        .await
        .expect("student exception");
    assert_eq!(
        store
            .set_assignment_policy_exception(context, student_command)
            .await,
        Ok(stored_student.clone()),
        "an exact exception retry is revision-stable"
    );

    let resolved = store
        .resolve_assignment_timing(context, assignment, student_record)
        .await
        .expect("resolve policy")
        .expect("enrollment policy");
    assert_eq!(resolved.policy.available_at, None);
    assert_eq!(
        resolved.policy.closes_at,
        Some(ActivityTimestamp::from_unix_millis(80_000))
    );
    assert_eq!(resolved.policy.time_limit_seconds, Some(20));
    assert_eq!(resolved.policy.attempt_limit, Some(3));
    assert_eq!(
        resolved.contributors,
        vec![
            AssignmentPolicyExceptionTarget::Student(student_record),
            AssignmentPolicyExceptionTarget::CourseGroup(group_id),
        ]
    );
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(uuid(96_026)))
        .await
        .expect("exception opens assignment early");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(uuid(96_027)),
                run: run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                seed: 5,
                parameter_hash: "exception-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("timing-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("timing-numeric"),
                    rendered_question_sha256: "exception-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("exception-timed attempt");
    assert_eq!(
        attempt.timer.deadline,
        Some(ActivityTimestamp::from_unix_millis(40_000))
    );
    let recorded = store
        .get_attempt_resolved_timing(context, attempt.id)
        .await
        .expect("attempt policy")
        .expect("attempt resolution");
    assert_eq!(recorded.policy, resolved.policy);
    assert_eq!(recorded.contributors, resolved.contributors);

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(35_000))
        .expect("advance beyond direct timer");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Accommodation course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("remove student membership");
    let empty_group = store
        .get_course_group(context, group_id)
        .await
        .expect("group after course membership update")
        .expect("group remains");
    assert!(empty_group.record.members.is_empty());
    let empty_group_command = PutCourseGroupCommand {
        actor: instructor,
        expected_revision: Some(empty_group.revision),
        record: empty_group.record.clone(),
    };
    assert_eq!(
        store.put_course_group(context, empty_group_command).await,
        Ok(empty_group.clone())
    );
    assert_eq!(
        store
            .get_question_attempt(context, attempt.id)
            .await
            .expect("closed attempt read")
            .expect("attempt remains")
            .status,
        AttemptStatus::AutoSubmitted
    );
    let terminal_resolution = store
        .get_attempt_resolved_timing(context, attempt.id)
        .await
        .expect("terminal policy")
        .expect("terminal resolution remains");
    assert_eq!(terminal_resolution.policy.time_limit_seconds, Some(15));
    assert_eq!(
        terminal_resolution.contributors,
        vec![AssignmentPolicyExceptionTarget::Student(student_record)]
    );

    let other_course = CourseId::from_uuid(uuid(96_028));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: other_course,
                tenant,
                title: "Other accommodation course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("other course");
    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(empty_group.revision),
                    record: CourseGroupRecord {
                        course: other_course,
                        ..empty_group.record.clone()
                    },
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stable group identity cannot move between courses"
    );

    let after_student_delete = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: stored_student.assignment_revision,
                exception: student_exception.id,
            },
        )
        .await
        .expect("delete student exception");
    let after_group_delete = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_student_delete,
                exception: group_exception.exception.id,
            },
        )
        .await
        .expect("delete group exception");
    let base_again = store
        .resolve_assignment_timing(context, assignment, student_record)
        .await
        .expect("base resolution")
        .expect("enrollment remains");
    assert_eq!(base_again.revision, after_group_delete);
    assert!(base_again.contributors.is_empty());
    assert_eq!(base_again.policy.time_limit_seconds, Some(10));
}

#[tokio::test]
async fn memory_export_commits_exact_four_private_artifacts_atomically() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let view = store
        .create_assignment_export(
            context,
            CreateAssignmentExport {
                assignment: AssignmentId::from_uuid(uuid(8)),
                requested_by: UserId::from_uuid(uuid(18)),
                max_attempts: 2,
            },
        )
        .await
        .expect("assignment export should freeze and queue");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("export job should claim")
        .expect("queued export job");
    let JobPayload::Export { delivery_object } = claim.payload else {
        panic!("assignment export must have the closed export payload");
    };
    let frozen = store
        .load_export_job(context, delivery_object)
        .await
        .expect("frozen export lookup")
        .expect("manifest resolves only its request");
    assert_eq!(frozen.expected_artifacts.len(), 4);
    let artifacts = frozen
        .expected_artifacts
        .iter()
        .map(|(kind, object)| {
            let (filename, media_type) = match kind {
                ExportArtifactKind::Docx => ("exam.docx", kind.media_type()),
                ExportArtifactKind::Pdf => ("exam.pdf", kind.media_type()),
                ExportArtifactKind::AccessibleDocx => ("exam-accessible.docx", kind.media_type()),
                ExportArtifactKind::AccessiblePdf => ("exam-accessible.pdf", kind.media_type()),
            };
            let key = ObjectKey::StudentRecord {
                tenant,
                object: *object,
            };
            ExportArtifactRecord {
                kind: *kind,
                filename: filename.to_string(),
                object: ObjectRecord {
                    id: *object,
                    bucket: key.bucket(),
                    key,
                    sha256: Sha256Digest::compute(filename.as_bytes()),
                    size_bytes: u64::try_from(filename.len()).expect("fixture length"),
                    media_type: media_type.to_string(),
                    category: ObjectCategory::Export,
                    version: None,
                    license: "educational-record".to_string(),
                    provenance: "export conformance fixture".to_string(),
                    created_at: ActivityTimestamp::from_unix_millis(1),
                },
            }
        })
        .collect::<Vec<_>>();
    let commit = ExportJobCommit {
        job: claim.id,
        lease: claim.lease_token,
        manifest: delivery_object,
        artifacts,
    };
    assert_eq!(
        store
            .commit_export_effect(context, commit.clone())
            .await
            .expect("all artifacts and completion commit together"),
        ExportCommitDisposition::Committed
    );
    assert_eq!(
        store
            .commit_export_effect(context, commit)
            .await
            .expect("same effect replay is safe"),
        ExportCommitDisposition::AlreadyCommitted
    );
    let ready = store
        .get_assignment_export_for_requester(context, view.id, UserId::from_uuid(uuid(18)))
        .await
        .expect("requester status lookup")
        .expect("requester sees export");
    assert_eq!(ready.artifacts.expect("ready has all deliveries").len(), 4);
    assert!(
        store
            .get_assignment_export_for_requester(context, view.id, UserId::from_uuid(uuid(19)))
            .await
            .expect("nonrequester lookup")
            .is_none()
    );

    let failed = store
        .create_assignment_export(
            context,
            CreateAssignmentExport {
                assignment: AssignmentId::from_uuid(uuid(8)),
                requested_by: UserId::from_uuid(uuid(18)),
                max_attempts: 1,
            },
        )
        .await
        .expect("second export queues independently");
    let failed_claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("second export claim")
        .expect("second export ready");
    assert_eq!(
        store
            .fail_job(
                failed_claim.id,
                failed_claim.lease_token,
                JobFailureKind::Permanent,
            )
            .await
            .expect("permanent refusal records terminal failure"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store
            .get_assignment_export_for_requester(context, failed.id, UserId::from_uuid(uuid(18)))
            .await
            .expect("failed requester status")
            .expect("failed request remains visible")
            .state,
        learning_data_access::StudentExportState::Failed
    );
}

#[tokio::test]
async fn memory_run_api_store_conforms() {
    for disclosure in [
        FeedbackDisclosure::ImmediateFull,
        FeedbackDisclosure::ImmediateCorrectness,
        FeedbackDisclosure::Deferred,
        FeedbackDisclosure::OnRelease,
    ] {
        let store = MemoryStore::default();
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
            .expect("memory clock");
        exercise_run_api_store(&store, disclosure).await;
    }
}
