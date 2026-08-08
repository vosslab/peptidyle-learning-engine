//! Reusable Store conformance suite, first run against memory in WP-C4.

use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun,
    AttemptProvenance, AttemptResult, AttemptTimerRecord, BackendCapabilities, Capability,
    CatalogLifecycle, CompletionRequirement, ContinuedPractice, CourseId, CourseMembership,
    CourseMembershipRole, CourseRole, DraftQuestionDefinition, DraftQuestionSource, EnrollmentId,
    FeedbackContent, GeneratorReference, GradePolicy, GradingDefinition, ImplementationVersion,
    ObjectId, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionBackend, QuestionMetadata, QuestionSource, ResponseDefinition, RunId, RunMode,
    RunPolicies, SourceArtifact, StudentId, TenantId, UserId, UserRole, VariationPolicy, VersionId,
    WorkspaceId, WorkspaceImportId,
};
use store::memory::MemoryStore;
use store::{
    ActivityTransition, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AssignmentRecord, AssignmentUpdate, CatalogSourceStore, CatalogStore, CatalogTransition,
    CourseListScope, CourseRecord, Cursor, DraftRecord, IssueQuestionAttemptCommand, PageRequest,
    PageSize, PrefetchedQuestion, PublishDraftCommand, PublishedSourceArtifact,
    PublishedVersionRef, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    SubmissionIdempotencyKey, SubmitQuestionAttemptCommand, TenantContext,
};
use store::{
    BeginExternalToolGradeCommand, CommitVerifiedExternalToolSubmissionCommand,
    CreateExternalToolLaunchSessionCommand, ExternalToolBegin, ExternalToolBrokerStore,
    ExternalToolLaunchProof, ExternalToolLaunchSessionStore, PersistedCorrelation,
    StageExternalToolVerificationCommand,
};
use store::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportRef, QtiImportRegistry, QtiImportStore, QtiPublicationPromotion,
    QtiUnsupportedFeature,
};
use store::{
    CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportJobCommit, ExportJobStore, JobFailureDisposition,
    JobFailureKind, JobLeaseDuration, JobLeaseToken, JobPayload, JobState, JobStore,
};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
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

fn qti_import_command(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> CreateQtiImportCommand {
    let source_key = ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object: ObjectId::from_uuid(uuid(9_011)),
    };
    let source = ObjectRecord {
        id: source_key.object_id(),
        bucket: source_key.bucket(),
        key: source_key,
        sha256: Sha256Digest::compute(b"qti zip fixture"),
        size_bytes: 15,
        media_type: "application/zip".to_string(),
        category: objects::ObjectCategory::Source,
        version: None,
        license: "CC BY-SA 4.0".to_string(),
        provenance: "QTI import conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let logical_asset = AssetId::from_uuid(uuid(9_012));
    let asset_key = ObjectKey::WorkspaceAsset {
        tenant,
        workspace,
        import,
        asset: logical_asset,
        object: ObjectId::from_uuid(uuid(9_013)),
    };
    let asset = object_record(asset_key, b"<svg/>", 1);
    let item = QtiImportItem {
        item_id: "item-1".to_string(),
        model_sha256: Sha256Digest::compute(b"canonical item model"),
        assets: vec![logical_asset],
    };
    CreateQtiImportCommand {
        registry: QtiImportRegistry {
            reference: QtiImportRef {
                tenant,
                workspace,
                import,
            },
            source,
            parse_schema: "imsqti_v2_1".to_string(),
            adapter_version: "1".to_string(),
            items: vec![item.clone()],
            assets: vec![asset],
            unsupported_features: vec![QtiUnsupportedFeature {
                code: "choiceInteraction.shuffle".to_string(),
                location: "item-1".to_string(),
            }],
        },
        item_bindings: vec![QtiImportItemRegistration {
            item,
            grading: QtiImportGradingPayload::new(b"correct-choice=2".to_vec())
                .expect("bounded test grading binding"),
        }],
    }
}

async fn exercise_qti_import_store<S, G>(store: &S, grader: &G)
where
    S: QtiImportStore + JobStore,
    G: QtiGradingStore,
{
    let tenant = TenantId::from_uuid(uuid(9_001));
    let foreign = TenantId::from_uuid(uuid(9_002));
    let workspace = WorkspaceId::from_uuid(uuid(9_003));
    let other_workspace = WorkspaceId::from_uuid(uuid(9_004));
    let import = WorkspaceImportId::from_uuid(uuid(9_005));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    let command = qti_import_command(tenant, workspace, import);
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("private registry prepares");
    let direct_job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object: command.registry.source.id,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("QTI job enqueue");
    let direct_claim = store
        .claim_next_job(JobLeaseDuration::from_seconds(60).expect("lease"))
        .await
        .expect("QTI job claim")
        .expect("QTI job ready");
    assert_eq!(direct_claim.id, direct_job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job: direct_job,
                    lease: direct_claim.lease_token,
                    reference: command.registry.reference,
                    source_object: command.registry.source.id,
                },
            )
            .await
            .expect("exact initial QTI commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let registry = store
        .get_qti_import(context, workspace, import)
        .await
        .expect("owner lookup")
        .expect("registry exists");
    assert_eq!(registry, command.registry);
    assert_eq!(
        store
            .get_qti_import(foreign_context, workspace, import)
            .await
            .expect("foreign lookup"),
        None
    );
    assert_eq!(
        store
            .get_qti_import(context, other_workspace, import)
            .await
            .expect("foreign workspace lookup"),
        None
    );
    let grading = grader
        .qti_import_grading(context, workspace, import, "item-1")
        .await
        .expect("grader lookup")
        .expect("only injected grader handle reads the private binding");
    assert_eq!(grading.sha256(), Sha256Digest::compute(b"correct-choice=2"));
    assert_eq!(
        grader
            .qti_import_grading(foreign_context, workspace, import, "item-1")
            .await
            .expect("foreign grading lookup"),
        None
    );
    assert!(matches!(
        store.prepare_qti_import(context, command).await,
        Err(StoreError::Conflict)
    ));

    let bad_import = WorkspaceImportId::from_uuid(uuid(9_006));
    let mut invalid = qti_import_command(tenant, workspace, bad_import);
    invalid.registry.assets.clear();
    assert!(matches!(
        store.prepare_qti_import(context, invalid).await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_qti_import(context, workspace, bad_import)
            .await
            .expect("failed creation lookup"),
        None
    );

    // Preparation is deliberately invisible, including to the dedicated
    // grader, until the exact durable QTI claim is committed.
    let staged_import = WorkspaceImportId::from_uuid(uuid(9_007));
    let staged = qti_import_command(tenant, workspace, staged_import);
    let source_object = staged.registry.source.id;
    store
        .prepare_qti_import(context, staged.clone())
        .await
        .expect("hidden QTI registry prepares atomically");
    store
        .prepare_qti_import(context, staged.clone())
        .await
        .expect("replayed QTI preparation is idempotent by import");
    let mut divergent_retry = staged.clone();
    divergent_retry.item_bindings[0].grading =
        QtiImportGradingPayload::new(b"a different server-only correct choice".to_vec())
            .expect("bounded divergent grading fixture");
    assert!(matches!(
        store.prepare_qti_import(context, divergent_retry).await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store
            .get_qti_import(context, workspace, staged_import)
            .await
            .expect("prepared registry lookup"),
        None
    );
    assert_eq!(
        grader
            .qti_import_grading(context, workspace, staged_import, "item-1")
            .await
            .expect("prepared grading lookup"),
        None
    );
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import: staged_import,
                    source_object,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("QTI job enqueue");
    let claim = store
        .claim_next_job(JobLeaseDuration::from_seconds(60).expect("lease"))
        .await
        .expect("QTI job claim")
        .expect("QTI job is ready");
    assert_eq!(claim.id, job);
    let wrong_source = ObjectId::from_uuid(uuid(9_099));
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: staged.registry.reference,
                    source_object: wrong_source,
                },
            )
            .await
            .expect("wrong QTI source must be safely refused"),
        CommitPreparedQtiImportOutcome::ClaimNoLongerActive
    );
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: staged.registry.reference,
                    source_object,
                },
            )
            .await
            .expect("exact claim atomically exposes prepared QTI import"),
        CommitPreparedQtiImportOutcome::Committed
    );
    assert_eq!(
        store
            .get_qti_import(context, workspace, staged_import)
            .await
            .expect("committed registry lookup"),
        Some(staged.registry)
    );
    assert!(
        grader
            .qti_import_grading(context, workspace, staged_import, "item-1")
            .await
            .expect("committed grading lookup")
            .is_some()
    );
}

fn render_job(tenant: TenantId, value: u128, max_attempts: u16) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Render {
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(uuid(value)),
                version: VersionId::from_uuid(uuid(value + 1)),
            },
            seed: u64::try_from(value).expect("fixture seed fits"),
        },
        max_attempts,
    }
}

async fn exercise_job_store_claim_boundary<S>(store: &S)
where
    S: JobStore,
{
    let tenant = TenantId::from_uuid(uuid(9_100));
    let foreign = TenantId::from_uuid(uuid(9_101));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    let lease = JobLeaseDuration::from_seconds(30).expect("bounded lease");
    let first = store
        .enqueue_job(context, render_job(tenant, 9_110, 1))
        .await
        .expect("tenant enqueue");
    store
        .enqueue_job(context, render_job(tenant, 9_120, 1))
        .await
        .expect("second tenant enqueue");
    assert_eq!(
        store
            .get_job(foreign_context, first)
            .await
            .expect("foreign lookup"),
        None
    );
    let (left, right) = tokio::join!(store.claim_next_job(lease), store.claim_next_job(lease));
    let left = left.expect("left claim").expect("left job");
    let right = right.expect("right claim").expect("right job");
    assert_ne!(
        left.id, right.id,
        "two workers must not claim one row twice"
    );
    assert_eq!(left.tenant, tenant, "broker returns the stored tenant");
    assert_eq!(right.tenant, tenant, "broker returns the stored tenant");
    assert!(matches!(
        store
            .complete_job(left.id, JobLeaseToken::generate().expect("test token"))
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .complete_job(left.id, left.lease_token)
        .await
        .expect("current token completes left job");
    assert_eq!(
        store
            .fail_job(right.id, right.lease_token, JobFailureKind::Permanent)
            .await
            .expect("current token can dead-letter right job"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store
            .get_job(context, right.id)
            .await
            .expect("owner lookup")
            .expect("dead row remains inspectable")
            .state,
        JobState::Dead
    );
    assert_eq!(
        store
            .ready_queue_depth()
            .await
            .expect("depth after broker finalization")
            .ready,
        0
    );
}

fn source_artifact(
    reference: ProblemVersionRef,
    backend: QuestionBackend,
    object: ObjectId,
) -> PublishedSourceArtifact {
    PublishedSourceArtifact {
        reference,
        backend,
        object: object_record(
            ObjectKey::ProblemSource {
                problem: reference.problem,
                version: reference.version,
                object,
            },
            b"immutable source fixture",
            1_000,
        ),
    }
}

async fn exercise_asset_store<S>(store: &S)
where
    S: Store + CatalogStore + AssetStore,
{
    let tenant = TenantId::from_uuid(uuid(401));
    let foreign_tenant = TenantId::from_uuid(uuid(402));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(403));
    let student = UserId::from_uuid(uuid(404));
    let stranger = UserId::from_uuid(uuid(405));
    let course = CourseId::from_uuid(uuid(405_001));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Asset delivery course".to_string(),
                members: vec![
                    CourseMembership {
                        user: publisher,
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
        .expect("asset delivery course");
    let public_problem = ProblemId::from_uuid(uuid(406));
    let public_version = VersionId::from_uuid(uuid(407));
    let institution_problem = ProblemId::from_uuid(uuid(408));
    let institution_version = VersionId::from_uuid(uuid(409));

    for (problem, version, workspace, scope) in [
        (
            public_problem,
            public_version,
            WorkspaceId::from_uuid(uuid(410)),
            PublicationScope::Public,
        ),
        (
            institution_problem,
            institution_version,
            WorkspaceId::from_uuid(uuid(411)),
            PublicationScope::Institution,
        ),
    ] {
        let draft = DraftRecord {
            tenant,
            question: draft_question(workspace),
            revises: None,
            derived_from: None,
        };
        let saved_draft = store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("asset fixture draft should save");
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
                    publisher,
                    scope,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("asset fixture should publish");
    }

    let public_asset = AssetId::from_uuid(uuid(412));
    let public_object = ObjectId::from_uuid(uuid(413));
    let public_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(public_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: public_problem,
                version: public_version,
                asset: public_asset,
                object: public_object,
            },
            b"public",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
    };
    let second_public_asset = AssetId::from_uuid(uuid(419));
    let second_public_object = ObjectId::from_uuid(uuid(420));
    let second_public_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(second_public_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: public_problem,
                version: public_version,
                asset: second_public_asset,
                object: second_public_object,
            },
            b"second public asset",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: second_public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
    };
    let institution_asset = AssetId::from_uuid(uuid(414));
    let institution_object = ObjectId::from_uuid(uuid(415));
    let institution_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(institution_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: institution_problem,
                version: institution_version,
                asset: institution_asset,
                object: institution_object,
            },
            b"institution",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: institution_asset,
            reference: ProblemVersionRef {
                problem: institution_problem,
                version: institution_version,
            },
        },
    };
    let student_object = ObjectId::from_uuid(uuid(416));
    let student_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(student_object),
        object: object_record(
            ObjectKey::StudentRecord {
                tenant,
                object: student_object,
            },
            b"student export",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            course,
            authorized_users: vec![student],
        },
    };

    for record in [
        public_delivery.clone(),
        second_public_delivery,
        institution_delivery.clone(),
        student_delivery.clone(),
    ] {
        store
            .register_asset_delivery(context, record)
            .await
            .expect("valid asset delivery should register");
    }
    assert_eq!(
        store
            .register_asset_delivery(context, public_delivery.clone())
            .await,
        Err(StoreError::AlreadyExists),
        "delivery records are immutable"
    );

    assert_eq!(
        store
            .get_public_asset_delivery(public_delivery.id)
            .await
            .expect("public lookup should run"),
        Some(public_delivery.clone())
    );
    assert_eq!(
        store
            .get_public_asset_delivery(institution_delivery.id)
            .await
            .expect("institution lookup should run"),
        None
    );
    assert_eq!(
        store
            .get_public_asset_delivery(student_delivery.id)
            .await
            .expect("student-record lookup should run"),
        None
    );

    let public_reference = ProblemVersionRef {
        problem: public_problem,
        version: public_version,
    };
    let institution_reference = ProblemVersionRef {
        problem: institution_problem,
        version: institution_version,
    };
    let public_bindings = store
        .catalog_asset_bindings(context, public_reference)
        .await
        .expect("catalog asset bindings should resolve");
    assert_eq!(
        public_bindings,
        vec![
            store::CatalogAssetBinding {
                asset: public_asset,
                object: public_object,
            },
            store::CatalogAssetBinding {
                asset: second_public_asset,
                object: second_public_object,
            },
        ],
        "the resolver must select only the exact published version"
    );
    assert_eq!(
        store
            .catalog_asset_bindings(context, public_reference)
            .await
            .expect("repeat catalog asset resolution should run"),
        public_bindings,
        "catalog asset bindings must be deterministic"
    );
    assert_eq!(
        store
            .catalog_asset_bindings(context, institution_reference)
            .await
            .expect("institution catalog asset resolution should run"),
        vec![store::CatalogAssetBinding {
            asset: institution_asset,
            object: institution_object,
        }],
        "student records and another catalog version must not leak into the result"
    );
    assert!(
        store
            .catalog_asset_bindings(foreign_context, institution_reference)
            .await
            .expect("foreign catalog asset resolution should run")
            .is_empty(),
        "a foreign tenant must not learn institution catalog asset bindings"
    );
    assert!(
        store
            .catalog_asset_bindings(
                context,
                ProblemVersionRef {
                    problem: public_problem,
                    version: VersionId::from_uuid(uuid(418)),
                },
            )
            .await
            .expect("unknown exact version lookup should run")
            .is_empty(),
        "an absent version may resolve to an empty visible result"
    );

    let institution_authorized = store
        .authorize_asset_delivery(context, student, institution_delivery.id)
        .await
        .expect("institution asset should be visible in its tenant");
    assert_eq!(institution_authorized.record, institution_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, institution_delivery.id)
            .await,
        Err(StoreError::NotFound),
        "institution assets must not cross tenant grants"
    );
    let student_authorized = store
        .authorize_asset_delivery(context, student, student_delivery.id)
        .await
        .expect("named student should receive their record");
    assert_eq!(student_authorized.record, student_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(context, stranger, student_authorized.record.id)
            .await,
        Err(StoreError::NotFound),
        "unauthorized identities must not learn that a student record exists"
    );
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, student_authorized.record.id,)
            .await,
        Err(StoreError::NotFound),
        "RLS tenant context must protect student records"
    );

    let temporary = ObjectId::from_uuid(uuid(417));
    let invalid = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(temporary),
        object: object_record(
            ObjectKey::Temporary { object: temporary },
            b"temporary",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            course,
            authorized_users: vec![student],
        },
    };
    assert!(matches!(
        store.register_asset_delivery(context, invalid).await,
        Err(StoreError::InvalidRecord(_))
    ));
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
        problems: vec![PublishedVersionRef { problem, version }],
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
        problems: vec![published, deprecated],
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
        problems: vec![deprecated, published],
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
    assert_eq!(updated.record.problems, update.problems);
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
                    problems: vec![archived],
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
                    problems: vec![hidden],
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
                problems: vec![published, published],
                policies: policies(),
            },
        )
        .await
        .expect("one immutable version may occupy distinct ordered positions");
    assert_eq!(repeated.record.problems, vec![published, published]);
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
                    problems: vec![published],
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

async fn exercise_session_replicas(issuer: &dyn SessionStore, next_replica: &dyn SessionStore) {
    let token_hash = SessionTokenHash::compute(b"opaque replica test credential");
    let wrong_token_hash = SessionTokenHash::compute(b"different credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(101)),
        UserId::from_uuid(uuid(102)),
        "Replica Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    let lifetime = SessionLifetime::from_seconds(60).expect("positive lifetime");

    let issued = issuer
        .create_session(token_hash, subject.clone(), lifetime)
        .await
        .expect("first replica should issue a session");
    let resumed = next_replica
        .resolve_session(token_hash)
        .await
        .expect("second replica should resolve a session");

    assert_eq!(resumed, Some(issued));
    assert_eq!(
        next_replica.resolve_session(wrong_token_hash).await,
        Ok(None),
        "a different cookie must not reveal any session"
    );

    next_replica
        .revoke_session(token_hash)
        .await
        .expect("second replica should revoke the session");
    assert_eq!(issuer.resolve_session(token_hash).await, Ok(None));
    next_replica
        .revoke_session(token_hash)
        .await
        .expect("repeat revocation should be idempotent");
}

async fn exercise_run_api_store<S>(store: &S, feedback_disclosure: FeedbackDisclosure)
where
    S: Store + CatalogStore,
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
    let initial_assignment = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Run API assignment".to_string(),
                problems: vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ],
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
                    points_earned: 2.0,
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
        Ok(store::SubmissionNextAttempt::Issued(second_attempt.id)),
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
        Ok(store::SubmissionNextAttempt::Issued(second_attempt.id)),
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

    // Scale behavior is deliberately exercised through the Store, not just
    // the cursor helper: a later practice run may contain far more outcomes
    // than an ordinary small assignment. `apply_activity_transition` supplies
    // persisted, server-owned attempt records without invoking a grader.
    let scale_run_id = RunId::from_uuid(uuid(90_000 + fixture_offset));
    let scale_problems = vec![ProblemVersionRef { problem, version }; 51];
    store
        .replace_assignment(
            context,
            course,
            assignment,
            initial_assignment.revision,
            AssignmentUpdate {
                title: "Run summary scale fixture".to_string(),
                problems: scale_problems,
                policies: policies(),
            },
        )
        .await
        .expect("scale assignment update");
    let scale_run = store
        .start_or_resume_run(context, student_user, assignment, scale_run_id)
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

struct ExternalToolFixture {
    context: TenantContext,
    foreign_context: TenantContext,
    actor: UserId,
    stranger: UserId,
    attempt: QuestionAttemptId,
    binding: store::ExternalToolBinding,
}

async fn external_tool_fixture<S>(store: &S) -> ExternalToolFixture
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(10_001));
    let foreign_tenant = TenantId::from_uuid(uuid(10_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let actor = UserId::from_uuid(uuid(10_003));
    let stranger = UserId::from_uuid(uuid(10_004));
    let instructor = UserId::from_uuid(uuid(10_015));
    let workspace = WorkspaceId::from_uuid(uuid(10_005));
    let problem = ProblemId::from_uuid(uuid(10_006));
    let version = VersionId::from_uuid(uuid(10_007));
    let course = CourseId::from_uuid(uuid(10_008));
    let assignment = AssignmentId::from_uuid(uuid(10_009));
    let enrollment = EnrollmentId::from_uuid(uuid(10_010));
    let run_id = RunId::from_uuid(uuid(10_011));
    let attempt = QuestionAttemptId::from_uuid(uuid(10_012));
    let source_object = ObjectId::from_uuid(uuid(10_014));
    let reference = ProblemVersionRef { problem, version };
    let prepared_artifact = source_artifact(reference, QuestionBackend::Imathas, source_object);
    let source_sha256 = prepared_artifact.object.sha256.to_string();
    let mut question = draft_question(workspace);
    question.response = ResponseDefinition::ExternalTool {};
    question.source = DraftQuestionSource::Imathas {
        provider: "institution-imathas".to_string(),
        item_ref: "external-tool-item".to_string(),
    };
    let draft = DraftRecord {
        tenant,
        question,
        revises: None,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("external-tool draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
                publication: reference,
                published_source: QuestionSource::Imathas {
                    provider: "institution-imathas".to_string(),
                    item_ref: "external-tool-item".to_string(),
                    snapshot: source_object,
                    snapshot_sha256: source_sha256.clone(),
                    integration_profile: "institution-default".to_string(),
                },
                source_artifact: Some(prepared_artifact),
                qti_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("external-tool publication");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "External tool course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: actor,
                        role: CourseMembershipRole::Student,
                    },
                    CourseMembership {
                        user: stranger,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("external-tool course");
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "External tool assignment".to_string(),
                problems: vec![ProblemVersionRef { problem, version }],
                policies: policies(),
            },
        )
        .await
        .expect("external-tool assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: actor,
                student: StudentId::from_uuid(uuid(10_013)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("external-tool enrollment");
    let run = store
        .start_or_resume_run(context, actor, assignment, run_id)
        .await
        .expect("external-tool run");
    let binding = store::ExternalToolBinding {
        provider: "institution-imathas".to_string(),
        problem,
        version,
        seed: 761,
        source_object,
        source_sha256: source_sha256.clone(),
        integration_profile: "institution-default".to_string(),
        response_sha256: Sha256Digest::compute(
            &serde_json::to_vec(&StudentResponse::ExternalTool {}).expect("marker encoding"),
        ),
    };
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor,
                attempt,
                run: run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: binding.seed,
                parameter_hash: "external-tool-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("imathas"),
                    renderer: None,
                    generator: None,
                    source_artifact: Some(SourceArtifact {
                        object: source_object,
                        sha256: source_sha256,
                    }),
                    asset_objects: Vec::new(),
                    grading: implementation("imathas"),
                    rendered_question_sha256: "external-tool-rendered".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("external-tool attempt");
    ExternalToolFixture {
        context,
        foreign_context,
        actor,
        stranger,
        attempt,
        binding,
    }
}

fn external_begin(fixture: &ExternalToolFixture, key: &str) -> BeginExternalToolGradeCommand {
    BeginExternalToolGradeCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: SubmissionIdempotencyKey::parse(key).expect("valid external key"),
        binding: fixture.binding.clone(),
        proposed_correlation: PersistedCorrelation::new(b"opaque-broker-correlation".to_vec())
            .expect("correlation"),
        lease_millis: 30_000,
    }
}

fn assert_external_debug_is_redacted(value: impl std::fmt::Debug, fixture: &ExternalToolFixture) {
    let rendered = format!("{value:?}");
    let source_object = fixture.binding.source_object.to_string();
    let response_digest = fixture.binding.response_sha256.to_string();
    for secret_or_provenance in [
        fixture.binding.provider.as_str(),
        fixture.binding.integration_profile.as_str(),
        fixture.binding.source_sha256.as_str(),
        source_object.as_str(),
        response_digest.as_str(),
        "opaque-broker-correlation",
        "points_earned",
        "points_possible",
    ] {
        assert!(
            !rendered.contains(secret_or_provenance),
            "external broker debug output must redact `{secret_or_provenance}`: {rendered}"
        );
    }
}

async fn exercise_external_tool_broker<S>(store: &S)
where
    S: Store + CatalogStore + ExternalToolBrokerStore + ExternalToolLaunchSessionStore,
{
    let fixture = external_tool_fixture(store).await;
    let mut provider_url = fixture.binding.clone();
    provider_url.provider = "https://provider.invalid/grade?token=secret".to_string();
    assert!(
        matches!(provider_url.validate(), Err(StoreError::InvalidRecord(_))),
        "provider configuration is an opaque identifier, never a URL or credential container"
    );
    let begin = external_begin(&fixture, "external-tool-submission");
    assert!(
        matches!(
            store
                .begin_or_resume_external_grade(fixture.foreign_context, begin.clone())
                .await,
            Err(StoreError::NotFound)
        ),
        "a foreign tenant cannot discover an exchange or its attempt"
    );
    let mut foreign_actor = begin.clone();
    foreign_actor.actor = fixture.stranger;
    assert!(
        matches!(
            store
                .begin_or_resume_external_grade(fixture.context, foreign_actor)
                .await,
            Err(StoreError::NotFound)
        ),
        "a different tenant member cannot claim another learner's exchange"
    );

    let first = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("first claim");
    let ExternalToolBegin::Lease(lease) = first else {
        panic!("first broker claim must lease");
    };
    let grade_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("grade launch session");
    let grade_launch_proof = ExternalToolLaunchProof {
        session_id: grade_launch.id,
        token: grade_launch.token.clone(),
    };
    let copied_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: None,
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("independent launch session for hostile proof checks");
    assert_external_debug_is_redacted(fixture.binding.clone(), &fixture);
    assert_external_debug_is_redacted(lease.clone(), &fixture);
    assert_external_debug_is_redacted(ExternalToolBegin::Lease(lease.clone()), &fixture);
    for mutated in [
        {
            let mut command = begin.clone();
            command.binding.provider = "other-provider".to_string();
            command
        },
        {
            let mut command = begin.clone();
            command.binding.problem = ProblemId::from_uuid(uuid(10_101));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.version = VersionId::from_uuid(uuid(10_102));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.seed += 1;
            command
        },
        {
            let mut command = begin.clone();
            command.binding.source_object = ObjectId::from_uuid(uuid(10_103));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.source_sha256 = "0".repeat(64);
            command
        },
        {
            let mut command = begin.clone();
            command.binding.integration_profile = "other-profile".to_string();
            command
        },
        {
            let mut command = begin.clone();
            command.binding.response_sha256 = Sha256Digest::compute(b"mutated");
            command
        },
        {
            let mut command = begin.clone();
            command.idempotency_key =
                SubmissionIdempotencyKey::parse("other-external-key").unwrap();
            command
        },
    ] {
        assert!(matches!(
            store
                .begin_or_resume_external_grade(fixture.context, mutated)
                .await,
            Err(StoreError::Conflict)
        ));
    }
    let in_progress = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("active external claim must be observable as in progress");
    assert_external_debug_is_redacted(&in_progress, &fixture);
    assert!(
        matches!(in_progress, ExternalToolBegin::InProgress),
        "failed claims must not alter the active exchange"
    );
    let result = AttemptResult {
        correct: true,
        points_earned: 1.0,
        points_possible: 1.0,
    };
    let recovery_before_stage = CommitVerifiedExternalToolSubmissionCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: begin.idempotency_key.clone(),
        binding: fixture.binding.clone(),
        correlation: lease.correlation.clone(),
        launch_proof: grade_launch_proof.clone(),
    };
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(
                fixture.context,
                recovery_before_stage.clone(),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        store
            .stage_external_tool_verification(
                fixture.context,
                StageExternalToolVerificationCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    response: StudentResponse::Numeric { value: 1.0 },
                    idempotency_key: begin.idempotency_key.clone(),
                    binding: fixture.binding.clone(),
                    correlation: lease.correlation.clone(),
                    lease_token: lease.token.clone(),
                    result,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                response: StudentResponse::ExternalTool {},
                idempotency_key: begin.idempotency_key.clone(),
                binding: fixture.binding.clone(),
                correlation: lease.correlation.clone(),
                lease_token: lease.token.clone(),
                result,
            },
        )
        .await
        .expect("current lease stages exactly one verified result");
    let verified = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("crash recovery reads the staged result without regrading");
    let ExternalToolBegin::VerifiedPending(verified) = verified else {
        panic!("verified work must resume without another provider verification");
    };
    assert_external_debug_is_redacted(&verified, &fixture);
    assert_eq!(verified.binding, fixture.binding);
    assert_eq!(verified.correlation, lease.correlation);
    let recovery = CommitVerifiedExternalToolSubmissionCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: begin.idempotency_key.clone(),
        binding: verified.binding,
        correlation: verified.correlation,
        launch_proof: grade_launch_proof.clone(),
    };
    let mut wrong_token_proof = recovery.clone();
    wrong_token_proof.launch_proof.token = copied_launch.token.clone();
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.context, wrong_token_proof)
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .revoke_external_tool_launch_session(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            copied_launch.id,
        )
        .await
        .expect("revoke hostile copied launch");
    let mut revoked_proof = recovery.clone();
    revoked_proof.launch_proof = ExternalToolLaunchProof {
        session_id: copied_launch.id,
        token: copied_launch.token.clone(),
    };
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.context, revoked_proof)
            .await,
        Err(StoreError::Conflict)
    ));
    for invalid in [
        {
            let mut command = recovery.clone();
            command.actor = fixture.stranger;
            command
        },
        {
            let mut command = recovery.clone();
            command.response = StudentResponse::Numeric { value: 1.0 };
            command
        },
        {
            let mut command = recovery.clone();
            command.idempotency_key =
                SubmissionIdempotencyKey::parse("other-recovery-key").unwrap();
            command
        },
        {
            let mut command = recovery.clone();
            command.binding.integration_profile = "other-profile".to_string();
            command
        },
        {
            let mut command = recovery.clone();
            command.correlation = PersistedCorrelation::new(b"other-correlation".to_vec()).unwrap();
            command
        },
    ] {
        assert!(matches!(
            store
                .commit_verified_external_tool_submission(fixture.context, invalid)
                .await,
            Err(StoreError::NotFound)
                | Err(StoreError::Conflict)
                | Err(StoreError::InvalidRecord(_))
        ));
    }
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.foreign_context, recovery.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    let (first_commit, replay_commit) = tokio::join!(
        store.commit_verified_external_tool_submission(fixture.context, recovery.clone()),
        store.commit_verified_external_tool_submission(fixture.context, recovery.clone()),
    );
    let committed = first_commit.expect("one recovery committer persists the staged result");
    assert_eq!(
        replay_commit.expect("concurrent exact recovery replays the first receipt"),
        committed
    );
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                grade_launch.id,
                &grade_launch.token,
            )
            .await
            .expect("consumed launch lookup")
            .is_none(),
        "the launch capability is consumed in the same commit as the receipt"
    );
    assert!(committed.attempt.timer.submitted_at.is_some());
    let committed_begin = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("same key and marker replay the immutable first receipt");
    assert_external_debug_is_redacted(&committed_begin, &fixture);
    assert!(
        matches!(committed_begin, ExternalToolBegin::Committed(_)),
        "same key and marker replay the immutable first receipt"
    );
    let mut changed_key = begin.clone();
    changed_key.idempotency_key =
        SubmissionIdempotencyKey::parse("external-tool-changed-key").unwrap();
    assert!(matches!(
        store
            .begin_or_resume_external_grade(fixture.context, changed_key)
            .await,
        Err(StoreError::Conflict)
    ));

    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("launch session");
    let second_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: None,
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("independent launch session");
    for lifetime_millis in [300_001, 900_000] {
        store
            .create_external_tool_launch_session(
                fixture.context,
                CreateExternalToolLaunchSessionCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    binding: fixture.binding.clone(),
                    encrypted_provider_state: None,
                    lifetime_millis,
                },
            )
            .await
            .expect("documented launch-session lifetime must work in every Store");
    }
    assert!(matches!(
        store
            .create_external_tool_launch_session(
                fixture.context,
                CreateExternalToolLaunchSessionCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    binding: fixture.binding.clone(),
                    encrypted_provider_state: None,
                    lifetime_millis: 900_001,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_ne!(
        launch.id, second_launch.id,
        "launch IDs must be operating-system-random and unique"
    );
    assert!(
        !format!("{launch:?}").contains("ExternalToolLaunchToken"),
        "launch diagnostics must never print cookie material"
    );
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await
            .expect("owner resolve")
            .is_some()
    );
    assert!(matches!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.stranger,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        matches!(
            store
                .resolve_external_tool_launch_session(
                    fixture.foreign_context,
                    fixture.actor,
                    fixture.attempt,
                    launch.id,
                    &launch.token
                )
                .await,
            Err(StoreError::NotFound),
        ),
        "a foreign tenant must not discover a launch session tied to another tenant"
    );
    store
        .revoke_external_tool_launch_session(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            launch.id,
        )
        .await
        .expect("owner revoke");
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await
            .expect("revoked lookup")
            .is_none()
    );
}

async fn exercise_catalog_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(301));
    let foreign_tenant = TenantId::from_uuid(uuid(302));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(303));
    let other_user = UserId::from_uuid(uuid(304));
    let tenant_course = CourseId::from_uuid(uuid(317));
    let foreign_course = CourseId::from_uuid(uuid(318));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: tenant_course,
                tenant,
                title: "Tenant biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: publisher,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("tenant course should save");
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: other_user,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("foreign course should save");
    let institution_workspace = WorkspaceId::from_uuid(uuid(305));
    let institution_problem = ProblemId::from_uuid(uuid(306));
    let institution_version = VersionId::from_uuid(uuid(307));
    let mut institution_question = draft_question(institution_workspace);
    institution_question.metadata.taxonomy = vec![
        TaxonomyTerm {
            scheme: "discipline/core".to_string(),
            code: "BIOC".to_string(),
            label: "Biochemistry".to_string(),
        },
        TaxonomyTerm {
            scheme: "discipline".to_string(),
            code: "core/BIOC".to_string(),
            label: "Biochemistry integration".to_string(),
        },
    ];
    let institution_draft = DraftRecord {
        tenant,
        question: institution_question,
        revises: None,
        derived_from: None,
    };
    let saved_institution_draft = store
        .upsert_draft(context, publisher, None, institution_draft.clone())
        .await
        .expect("institution draft should save");
    let institution_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: institution_draft.clone(),
                expected_revision: saved_institution_draft.revision,
                publication: ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
                published_source: published_source(),
                publisher,
                scope: PublicationScope::Institution,
                source_artifact: None,
                qti_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("institution publication should succeed");

    assert_eq!(institution_record.question.problem, institution_problem);
    assert_eq!(
        store
            .get_draft(context, publisher, institution_workspace)
            .await
            .expect("published draft lookup"),
        None
    );
    assert_eq!(
        store
            .get_catalog_problem(
                foreign_context,
                ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
            )
            .await,
        Ok(None),
        "institution publication must not cross its visibility grant"
    );
    assert_eq!(
        store
            .get_published_problem(institution_problem, institution_version)
            .await,
        Ok(None),
        "the context-free public-content contract must not expose institution content"
    );
    let tenant_taxonomy = store
        .list_catalog_taxonomy(
            context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("tenant taxonomy should list");
    let foreign_taxonomy = store
        .list_catalog_taxonomy(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign taxonomy should list");
    assert_eq!(
        tenant_taxonomy
            .items
            .iter()
            .map(|term| (term.scheme.as_str(), term.code.as_str()))
            .collect::<Vec<_>>(),
        vec![("discipline", "core/BIOC"), ("discipline/core", "BIOC"),],
        "taxonomy identity is the scheme/code pair, even when either contains a slash"
    );
    assert!(foreign_taxonomy.items.is_empty());
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(313)),
                tenant,
                course_id: tenant_course,
                title: "Institution content".to_string(),
                problems: vec![ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                }],
                policies: policies(),
            },
        )
        .await
        .expect("publishing tenant should assign institution content");
    assert!(matches!(
        store
            .create_assignment(
                foreign_context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(314)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "Hidden institution content".to_string(),
                    problems: vec![ProblemVersionRef {
                        problem: institution_problem,
                        version: institution_version,
                    }],
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    let public_workspace = WorkspaceId::from_uuid(uuid(308));
    let public_problem = ProblemId::from_uuid(uuid(309));
    let public_version = VersionId::from_uuid(uuid(310));
    let public_draft = DraftRecord {
        tenant,
        question: draft_question(public_workspace),
        revises: None,
        derived_from: None,
    };
    let saved_public_draft = store
        .upsert_draft(context, publisher, None, public_draft.clone())
        .await
        .expect("public draft should save");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: public_draft,
                expected_revision: saved_public_draft.revision,
                publication: ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                published_source: published_source(),
                publisher,
                scope: PublicationScope::Public,
                source_artifact: None,
                qti_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("public publication should succeed");
    let foreign_catalog = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign public catalog should list");
    assert_eq!(foreign_catalog.items.len(), 1);
    assert_eq!(foreign_catalog.items[0].problem, public_problem);

    let revision_version = VersionId::from_uuid(uuid(311));
    let revision_workspace = WorkspaceId::from_uuid(uuid(312));
    let revision_draft = DraftRecord {
        tenant,
        question: draft_question(revision_workspace),
        revises: Some(ProblemVersionRef {
            problem: public_problem,
            version: public_version,
        }),
        derived_from: None,
    };
    let saved_revision_draft = store
        .upsert_draft(context, publisher, None, revision_draft.clone())
        .await
        .expect("revision draft should save");
    let revision = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: revision_draft,
                expected_revision: saved_revision_draft.revision,
                publication: ProblemVersionRef {
                    problem: public_problem,
                    version: revision_version,
                },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("owned linear revision should publish");
    assert_eq!(revision.previous_version, Some(public_version));
    assert_eq!(revision.authors, vec![publisher]);

    assert_eq!(
        store
            .transition_catalog_problem(
                context,
                other_user,
                ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                CatalogTransition::Deprecate {
                    reason: "Correction available".to_string(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    let deprecated = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Deprecate {
                reason: " Correction available ".to_string(),
            },
        )
        .await
        .expect("author should deprecate");
    assert!(matches!(
        deprecated.lifecycle,
        CatalogLifecycle::Deprecated { ref reason } if reason == "Correction available"
    ));
    let exact_deprecated = store
        .get_catalog_problem(
            foreign_context,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        )
        .await
        .expect("exact deprecated lookup should run");
    assert!(
        exact_deprecated.is_some(),
        "existing references remain resolvable"
    );
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(315)),
                tenant,
                course_id: tenant_course,
                title: "Deprecated exact reference".to_string(),
                problems: vec![ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                }],
                policies: policies(),
            },
        )
        .await
        .expect("a deprecated version remains assignable by exact reference");
    let browse_after_deprecation = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("catalog should list");
    assert_eq!(browse_after_deprecation.items.len(), 1);
    assert_eq!(browse_after_deprecation.items[0].version, revision_version);

    let archived = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Archive,
        )
        .await
        .expect("deprecated version should archive");
    assert!(matches!(
        archived.lifecycle,
        CatalogLifecycle::Archived { .. }
    ));
    assert!(matches!(
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(316)),
                    tenant,
                    course_id: tenant_course,
                    title: "Archived exact reference".to_string(),
                    problems: vec![ProblemVersionRef {
                        problem: public_problem,
                        version: public_version,
                    }],
                    policies: policies(),
                },
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
    exercise_session_replicas(&store, &store.clone()).await;
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
        .claim_next_job(JobLeaseDuration::from_seconds(60).expect("bounded lease"))
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
        .claim_next_job(JobLeaseDuration::from_seconds(60).expect("bounded lease"))
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
        store::StudentExportState::Failed
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

#[tokio::test]
async fn memory_external_tool_broker_conforms() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("memory clock");
    exercise_external_tool_broker(&store).await;
}

#[tokio::test]
async fn memory_catalog_store_conforms() {
    exercise_catalog_store(&MemoryStore::default()).await;
}

#[tokio::test]
async fn memory_job_store_claim_boundary_conforms() {
    exercise_job_store_claim_boundary(&MemoryStore::default()).await;
}

#[tokio::test]
async fn memory_job_store_enforces_atomic_leases_retries_depth_and_tenants() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(9_001));
    let foreign = TenantId::from_uuid(uuid(9_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    let lease = JobLeaseDuration::from_seconds(1).expect("bounded lease");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");

    let first = store
        .enqueue_job(context, render_job(tenant, 9_010, 2))
        .await
        .expect("tenant should enqueue its job");
    let _second = store
        .enqueue_job(context, render_job(tenant, 9_020, 1))
        .await
        .expect("tenant should enqueue another job");
    assert_eq!(store.ready_queue_depth().await.expect("depth").ready, 2);
    assert_eq!(
        store
            .get_job(foreign_context, first)
            .await
            .expect("foreign read is bounded"),
        None,
        "tenant inspection must not reveal another tenant's job"
    );

    let (claim_one, claim_two) =
        tokio::join!(store.claim_next_job(lease), store.claim_next_job(lease));
    let claim_one = claim_one.expect("first claim").expect("first queued job");
    let claim_two = claim_two.expect("second claim").expect("second queued job");
    assert_ne!(
        claim_one.id, claim_two.id,
        "two claims must never duplicate a job"
    );
    assert_eq!(claim_one.tenant, tenant);
    assert_eq!(claim_two.tenant, tenant);
    assert_eq!(store.ready_queue_depth().await.expect("depth").ready, 0);
    let (reclaimable_claim, exhausted_claim) = if claim_one.id == first {
        (claim_one, claim_two)
    } else {
        (claim_two, claim_one)
    };

    // Let the first lease expire. Its token can no longer complete after the
    // reclaimed lease is issued.
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("memory clock should advance");
    let reclaimed = store
        .claim_next_job(lease)
        .await
        .expect("reclaim should succeed")
        .expect("expired job should be reclaimable");
    assert_eq!(reclaimed.id, reclaimable_claim.id);
    assert_eq!(reclaimed.attempt_count, 2);
    assert!(matches!(
        store
            .complete_job(reclaimable_claim.id, reclaimable_claim.lease_token)
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .complete_job(reclaimed.id, reclaimed.lease_token)
        .await
        .expect("current lease token completes exactly once");
    assert_eq!(
        store
            .get_job(context, reclaimed.id)
            .await
            .expect("owner lookup")
            .expect("completed row retained")
            .state,
        JobState::Completed
    );

    // The one-attempt job was left leased by the parallel claim. Its expiry
    // becomes a dead row and never inflates ready depth.
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_002))
        .expect("memory clock should advance");
    assert_eq!(store.claim_next_job(lease).await.expect("claim"), None);
    assert_eq!(
        store
            .get_job(context, exhausted_claim.id)
            .await
            .expect("owner lookup")
            .expect("dead row retained")
            .state,
        JobState::Dead
    );

    let retry = store
        .enqueue_job(context, render_job(tenant, 9_030, 2))
        .await
        .expect("retry fixture enqueue");
    let retry_claim = store
        .claim_next_job(lease)
        .await
        .expect("claim retry fixture")
        .expect("retry fixture ready");
    assert_eq!(retry_claim.id, retry);
    assert_eq!(
        store
            .fail_job(retry, retry_claim.lease_token, JobFailureKind::Transient)
            .await
            .expect("first transient failure"),
        JobFailureDisposition::Retrying
    );
    assert_eq!(
        store
            .ready_queue_depth()
            .await
            .expect("delayed depth")
            .ready,
        0
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(4_003))
        .expect("memory clock should advance through backoff");
    let final_claim = store
        .claim_next_job(lease)
        .await
        .expect("second retry claim")
        .expect("retry becomes eligible");
    assert_eq!(final_claim.id, retry);
    assert_eq!(
        store
            .fail_job(retry, final_claim.lease_token, JobFailureKind::Transient)
            .await
            .expect("attempt exhaustion"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store.ready_queue_depth().await.expect("dead depth").ready,
        0
    );
}

#[tokio::test]
async fn memory_asset_store_conforms_and_records_protected_access() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(7_000))
        .expect("memory clock should be writable");
    exercise_asset_store(&store).await;
    let events = store
        .asset_access_events()
        .expect("memory audit events should be readable");
    assert_eq!(events.len(), 2, "only authorized protected requests log");
    assert!(
        events
            .iter()
            .all(|event| event.occurred_at == ActivityTimestamp::from_unix_millis(7_000))
    );
}

#[tokio::test]
async fn memory_qti_import_registry_is_private_complete_and_secret_redacted() {
    let (store, grader) = MemoryStore::with_qti_grader();
    exercise_qti_import_store(&store, &grader).await;
    let redacted = format!(
        "{:?}",
        QtiImportGradingPayload::new(b"never-in-debug".to_vec()).expect("fixture payload")
    );
    assert!(!redacted.contains("never-in-debug"));
}

async fn exercise_qti_published_grading_visibility<S, G>(store: &S, grader: &G)
where
    S: Store + CatalogStore + AssetStore + QtiImportStore + JobStore,
    G: QtiGradingStore,
{
    let tenant = TenantId::from_uuid(uuid(9_100));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(9_101));
    let workspace = WorkspaceId::from_uuid(uuid(9_102));
    let import = WorkspaceImportId::from_uuid(uuid(9_103));
    let command = qti_import_command(tenant, workspace, import);
    store
        .prepare_qti_import(context, command.clone())
        .await
        .expect("private QTI staging prepares");
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object: command.registry.source.id,
                },
                max_attempts: 2,
            },
        )
        .await
        .expect("QTI promotion fixture job enqueues");
    let claim = store
        .claim_next_job(JobLeaseDuration::from_seconds(60).expect("bounded lease"))
        .await
        .expect("QTI promotion fixture job claims")
        .expect("queued fixture is available");
    assert_eq!(claim.id, job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: command.registry.reference,
                    source_object: command.registry.source.id,
                },
            )
            .await
            .expect("committed private staging"),
        CommitPreparedQtiImportOutcome::Committed
    );

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(9_104)),
        version: VersionId::from_uuid(uuid(9_105)),
    };
    let mut draft_question = draft_question(workspace);
    draft_question.source = DraftQuestionSource::Qti {
        item_id: "item-1".to_string(),
        import_id: import,
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question,
        revises: None,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("QTI draft saves before promotion");
    let mut artifact = source_artifact(
        reference,
        QuestionBackend::Qti,
        ObjectId::from_uuid(uuid(9_106)),
    );
    artifact.object.sha256 = command.registry.source.sha256;
    artifact.object.size_bytes = command.registry.source.size_bytes;
    artifact.object.media_type = command.registry.source.media_type.clone();
    let staged_asset = &command.registry.assets[0];
    let logical_asset = command.registry.items[0].assets[0];
    let object = ObjectId::from_uuid(uuid(9_107));
    let asset_key = ObjectKey::ProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset: logical_asset,
        object,
    };
    let delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(logical_asset),
        object: ObjectRecord {
            id: object,
            bucket: asset_key.bucket(),
            key: asset_key,
            sha256: staged_asset.sha256,
            size_bytes: staged_asset.size_bytes,
            media_type: staged_asset.media_type.clone(),
            category: objects::ObjectCategory::Asset,
            version: Some(reference.version),
            license: staged_asset.license.clone(),
            provenance: "published QTI asset fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(2),
        },
        scope: AssetDeliveryScope::Catalog {
            asset: logical_asset,
            reference,
        },
    };
    let collaborator = UserId::from_uuid(uuid(9_109));
    store
        .grant_draft_collaborator(context, publisher, workspace, collaborator)
        .await
        .expect("owner grants the QTI collaborator");
    let mut intervening_draft = draft.clone();
    intervening_draft.question.metadata.title = "Intervening workspace save".to_string();
    let intervening_save = store
        .upsert_draft(
            context,
            collaborator,
            Some(saved_draft.revision),
            intervening_draft,
        )
        .await
        .expect("collaborator save advances the workspace revision");
    let reverted_save = store
        .upsert_draft(
            context,
            publisher,
            Some(intervening_save.revision),
            draft.clone(),
        )
        .await
        .expect("owner may restore content while advancing the revision");
    let promotion = QtiPublicationPromotion {
        staging: command.registry.reference,
        assets: vec![delivery],
    };
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: QuestionSource::Qti {
                        item_id: "item-1".to_string(),
                        package_object: artifact.object.id,
                        package_sha256: artifact.object.sha256.to_string(),
                    },
                    source_artifact: Some(artifact.clone()),
                    qti_promotion: Some(promotion.clone()),
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a content-equivalent reverted draft cannot publish with its stale revision"
    );
    assert_eq!(
        store.get_catalog_problem(context, reference).await,
        Ok(None),
        "a stale revision cannot mint the catalog version"
    );
    assert!(
        grader
            .qti_published_grading(context, reference, "item-1")
            .await
            .expect("stale QTI grading lookup")
            .is_none(),
        "a stale publication must leave private QTI staging unconsumed"
    );
    let published = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: reverted_save.revision,
                publication: reference,
                published_source: QuestionSource::Qti {
                    item_id: "item-1".to_string(),
                    package_object: artifact.object.id,
                    package_sha256: artifact.object.sha256.to_string(),
                },
                source_artifact: Some(artifact),
                qti_promotion: Some(promotion),
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("exact committed QTI staging atomically publishes");
    assert_eq!(published.problem, reference.problem);
    assert!(
        grader
            .qti_published_grading(context, reference, "item-1")
            .await
            .expect("published grading read")
            .is_some(),
        "the grader receives the copied server-only binding"
    );
    assert!(
        grader
            .qti_published_grading(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(9_108))),
                reference,
                "item-1",
            )
            .await
            .expect("public published grading read")
            .is_some(),
        "public QTI content remains gradeable from another tenant"
    );
    assert_eq!(
        store.catalog_asset_bindings(context, reference).await,
        Ok(vec![store::CatalogAssetBinding {
            asset: logical_asset,
            object,
        }])
    );
}

#[tokio::test]
async fn memory_qti_publication_copies_only_committed_staging_grading() {
    let (store, grader) = MemoryStore::with_qti_grader();
    exercise_qti_published_grading_visibility(&store, &grader).await;
}

#[tokio::test]
async fn memory_sessions_use_the_backend_clock_for_expiry() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");
    let token_hash = SessionTokenHash::compute(b"expiring credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(201)),
        UserId::from_uuid(uuid(202)),
        "Expiring Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    store
        .create_session(
            token_hash,
            subject,
            SessionLifetime::from_seconds(1).expect("positive lifetime"),
        )
        .await
        .expect("session should be created");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("memory clock should advance");

    assert_eq!(store.resolve_session(token_hash).await, Ok(None));
}
