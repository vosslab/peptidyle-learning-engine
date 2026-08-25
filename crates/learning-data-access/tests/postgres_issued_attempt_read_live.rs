#![cfg(feature = "postgres")]

//! PostgreSQL 17 oracle for broker-first, route-bound issued question reads.
//!
//! This deliberately uses the ordinary Store publication, roster, run, issue,
//! and submit workflow.  SQL below is reserved for observing the deployed
//! authority catalog and for corrupting a disposable immutable receipt after
//! it was created through the public workflow.

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;
#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;
#[path = "postgres_issued_attempt_read_live/authority.rs"]
mod authority;
#[path = "postgres_issued_attempt_read_live/fixture.rs"]
mod issued_fixture;
#[path = "postgres_issued_attempt_read_live/receipt_integrity.rs"]
mod receipt_integrity;
#[path = "postgres_issued_attempt_read_live/sealed_webwork.rs"]
mod sealed_webwork;
#[path = "postgres_issued_attempt_read_live/timing.rs"]
mod timing;
use authority::assert_application_authority_catalog;
use issued_fixture::{IssueFixture, issue_webwork};
use receipt_integrity::ReceiptIntegrityOracle;
use timing::OracleTimingWindow;

use learning_data_access::postgres::{
    PostgresGraderStore, PostgresStore, apply_migrations, lazy_pool, verify_application_schema,
};
use learning_data_access::{
    AssignmentRecord, AttemptSupportActionId, CatalogStore, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    IssuedAttemptRead, IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1,
    IssuedWebworkGradingContract, LearnerWorkRoutingBinding, NativeExecutionEnvelopeCapability,
    PresentationCapability, PublishDraftCommand, PublishedSourceArtifact,
    PutAssignmentTeachingSettingsCommand, QtiGradingCapability, RevokeCourseMember,
    SealedPrivateExecutionPreparation, SealedPrivateExecutionStore, SessionLifetime, SessionStore,
    SessionSubject, SessionTokenHash, Store, StoreError, SubmissionIdempotencyKey,
    SubmissionPreparation, SubmitQuestionAttemptCommand, TenantContext, UpsertCourseMember,
    WebworkGradingCapability, WebworkReplayControlV1, WebworkReplayMappingV1,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::{ContentBlock, QuestionEnvelope};
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::presentation::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_nonce_source,
};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeadlineBehavior, AssignmentDeliveryState,
    AssignmentId, AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, AssignmentTeachingSettings, AttemptProvenance, AttemptResult,
    BackendCapabilities, Capability, CourseId, DraftQuestionDefinition, DraftQuestionSource,
    FeedbackContent, GradingDefinition, ImplementationVersion, ObjectId, PointValue,
    PresentationBindingV1, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttemptId,
    QuestionBackend, QuestionMetadata, QuestionSource, RenderedItemIdV1, RunId, SourceArtifact,
    StudentResponse, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, ConnectOptions, PgPool, Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

struct DisposableDatabase {
    admin: PgPool,
    database: String,
    pool: PgPool,
}

impl DisposableDatabase {
    async fn provision(url: &str) -> Self {
        let admin = lazy_pool(url).expect("valid PostgreSQL administration URL");
        let database = format!("ple_t4_issued_read_{:x}", id().as_u128());
        assert!(
            database.len() < 64,
            "child database identifier fits PostgreSQL"
        );
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
            .execute(&admin)
            .await
            .expect("create isolated issued-read PostgreSQL database");
        let options = PgConnectOptions::from_str(url)
            .expect("PostgreSQL URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await
            .expect("connect isolated issued-read PostgreSQL database");
        apply_migrations(&pool)
            .await
            .expect("apply full migration epoch to isolated database");
        Self {
            admin,
            database,
            pool,
        }
    }

    async fn cleanup(self) {
        self.pool.close().await;
        sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
            .bind(&self.database)
            .execute(&self.admin)
            .await
            .expect("disconnect issued-read child database");
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {}",
            self.database
        )))
        .execute(&self.admin)
        .await
        .expect("drop isolated issued-read child database");
    }
}

struct FixedNonce([u8; 16]);

impl NonceSourceV1 for FixedNonce {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        Ok(self.0)
    }
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
        title: "Issued-read oracle question".to_string(),
        prompt: vec![ContentBlock::Text {
            markdown: "How many atoms are in one water molecule?".to_string(),
        }],
        response: question_model::ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
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
    bytes[8..].copy_from_slice(&seed.rotate_left(11).to_le_bytes());
    let mut nonce = FixedNonce(bytes);
    let rendered =
        build_presentation_v1_with_nonce_source(&envelope(reference, seed), &[], &mut nonce)
            .expect("deterministic native presentation");
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
            id: "issued-read-live".to_string(),
            version: "1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "issued-read-grader".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: "issued-read-live-render".to_string(),
    }
}

fn item(reference: ProblemVersionRef, position: u32) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(id()),
        reference,
        position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    owner: UserId,
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
                markdown: "Issued-read oracle question".to_string(),
            }],
            response: question_model::ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Issued-read oracle question".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, owner, None, draft.clone())
        .await
        .expect("save oracle draft");
    store
        .publish_draft(
            context,
            owner,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: owner,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("Oracle fixture".to_string())
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
        .expect("publish oracle question");
    reference
}

async fn publish_webwork(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    owner: UserId,
) -> (
    ProblemVersionRef,
    IssuedWebworkGradingContract,
    AttemptProvenance,
) {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let question = DraftQuestionDefinition {
        workspace: WorkspaceId::from_uuid(id()),
        source: DraftQuestionSource::Webwork {
            pg_path: "Library/PLE/replay-contract.pg".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "WebWork replay oracle".to_string(),
        }],
        response: question_model::ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: None,
        },
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "WebWork replay oracle".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    };
    let draft = DraftRecord {
        tenant,
        question: question.clone(),
        derived_from: None,
    };
    let object = ObjectId::from_uuid(id());
    let key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object,
    };
    let artifact = PublishedSourceArtifact {
        reference,
        backend: QuestionBackend::Webwork,
        object: ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(b"webwork replay source"),
            size_bytes: 21,
            media_type: "text/x-webwork".to_string(),
            category: ObjectCategory::Source,
            version: Some(reference.version),
            license: "CC-BY-4.0".to_string(),
            provenance: "issued-attempt PostgreSQL oracle".to_string(),
            created_at: question_model::ActivityTimestamp::from_unix_millis(1),
        },
    };
    let saved = store
        .upsert_draft(context, owner, None, draft.clone())
        .await
        .expect("save WebWork draft");
    store
        .publish_draft(
            context,
            owner,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Webwork {
                    pg_path: "Library/PLE/replay-contract.pg".to_string(),
                },
                publisher: owner,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("Oracle fixture".to_string())
                        .expect("byline"),
                ])
                .expect("byline"),
                source_artifact: Some(artifact.clone()),
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("publish WebWork question");
    let contract =
        IssuedWebworkGradingContract::new(question_model::QuestionDefinition::from_draft(
            question,
            reference.problem,
            reference.version,
            QuestionSource::Webwork {
                pg_path: "Library/PLE/replay-contract.pg".to_string(),
            },
        ))
        .expect("WebWork contract");
    let mut source = provenance();
    source.source_artifact = Some(SourceArtifact {
        object: artifact.object.id,
        sha256: artifact.object.sha256.to_string(),
    });
    source.renderer = Some(ImplementationVersion {
        id: "webwork-renderer".to_string(),
        version: "1".to_string(),
    });
    (reference, contract, source)
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn postgres_issued_attempt_read_is_broker_first_route_bound_and_lifecycle_aware() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let grader_url = runtime.grader_url().expose();
    let database = DisposableDatabase::provision(database_url).await;
    let pool = database.pool.clone();
    verify_application_schema(&pool)
        .await
        .expect("migrated application schema");
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int4")
        .fetch_one(&pool)
        .await
        .expect("PostgreSQL version");
    assert!(
        (170_000..180_000).contains(&version),
        "oracle requires PostgreSQL 17"
    );
    let timing = OracleTimingWindow::from_database(&pool).await;
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
    let grader_options = PgConnectOptions::from_str(grader_url)
        .expect("valid disposable grader PostgreSQL URL")
        .database(&database.database);
    let child_grader_url = grader_options.to_url_lossy().to_string();
    let grader = PostgresGraderStore::connect_local_development(&child_grader_url)
        .await
        .expect("connect the child database through the dedicated grader principal");
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let foreign_actor = UserId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Issued-read oracle course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        timing.start_date(),
                        timing.end_date(),
                        "America/Chicago",
                    )
                    .expect("course term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create ordinary course");
    let instructor_session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "Issued-read oracle instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("bounded fixture session"),
        )
        .await
        .expect("persist Instructor session for roster revocation");
    let active_reference = publish(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "Issued-read oracle assignment".to_string(),
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![item(active_reference, 0)],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: policies(),
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish ordinary assignment");
    let learner_membership = store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Oracle student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("upsert active Student membership");
    let run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start ordinary learner run");
    let issues = IssueFixture {
        context,
        student,
        course,
        assignment,
        run: run.id,
        reference: active_reference,
    };
    let active = issues.issue(&store, 0, 11).await;
    let binding = LearnerWorkRoutingBinding::new(course, assignment);
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(context, student, binding, active)
                .await,
            Ok(IssuedAttemptRead::Active(_))
        ),
        "active issuance returns the active-only capability"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student,
                LearnerWorkRoutingBinding::new(course, AssignmentId::from_uuid(id())),
                active
            )
            .await,
        Err(StoreError::NotFound),
        "wrong assignment is concealed"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student,
                LearnerWorkRoutingBinding::new(CourseId::from_uuid(id()), assignment),
                active
            )
            .await,
        Err(StoreError::NotFound),
        "wrong course is concealed"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(context, foreign_actor, binding, active)
            .await,
        Err(StoreError::NotFound),
        "foreign actor is concealed"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                TenantContext::from_authenticated_session(foreign_tenant),
                student,
                binding,
                active
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant is concealed"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(
                context,
                student,
                binding,
                QuestionAttemptId::from_uuid(id())
            )
            .await,
        Err(StoreError::NotFound),
        "unknown attempt is concealed"
    );
    assert_application_authority_catalog(&pool, tenant, course, assignment, student, active).await;

    let receipt_integrity = ReceiptIntegrityOracle::new(&pool, &store, context, tenant, student);
    receipt_integrity
        .assert_active_issuance_fails_closed(binding, active)
        .await;

    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                binding,
                attempt: active,
                response: StudentResponse::Numeric { value: 3.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("issued-read-submitted")
                    .expect("key"),
            },
        )
        .await
        .expect("ordinary submission creates immutable receipt");
    let lifecycle: (String, String, bool) = sqlx::query_as(
        "SELECT attempt.payload->>'status', attempt.attempt_status, \
                EXISTS(SELECT 1 FROM public.submission_receipt_snapshot receipt \
                       WHERE receipt.tenant_id=attempt.tenant_id AND receipt.attempt_id=attempt.attempt_id) \
           FROM public.question_attempt attempt \
          WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(active.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read immutable issuance and current submitted lifecycle");
    assert_eq!(
        lifecycle.0, "in_progress",
        "payload remains immutable issuance evidence"
    );
    assert_eq!(
        lifecycle.1, "submitted",
        "relational lifecycle is current authority"
    );
    assert!(
        lifecycle.2,
        "submission retains one immutable receipt snapshot"
    );
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(context, student, binding, active)
                .await,
            Ok(IssuedAttemptRead::Submitted(_))
        ),
        "raw issuance remains in_progress while relational/witness submitted selects submitted receipt"
    );

    receipt_integrity
        .assert_submitted_receipt_fails_closed(binding, active)
        .await;
    sqlx::query("ALTER TABLE public.submission_receipt_snapshot DISABLE TRIGGER ALL")
        .execute(&pool)
        .await
        .expect("open isolated missing-receipt probe");
    sqlx::query(
        "DELETE FROM public.submission_receipt_snapshot WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(active.as_uuid())
    .execute(&pool)
    .await
    .expect("delete isolated immutable receipt");
    sqlx::query("ALTER TABLE public.submission_receipt_snapshot ENABLE TRIGGER ALL")
        .execute(&pool)
        .await
        .expect("restore isolated missing-receipt trigger");
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(context, student, binding, active)
                .await,
            Err(StoreError::Unavailable(_))
        ),
        "a submitted attempt without its immutable receipt fails closed"
    );

    let (webwork_reference, webwork_contract, webwork_provenance) =
        publish_webwork(&store, context, tenant, instructor).await;
    let webwork_assignment = AssignmentId::from_uuid(id());
    create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: webwork_assignment,
            tenant,
            course_id: course,
            title: "WebWork issued-read assignment".to_string(),
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![item(webwork_reference, 0)],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: policies(),
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish WebWork assignment");
    let webwork_run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, webwork_assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start WebWork run");
    let webwork_fixture = IssueFixture {
        context,
        student,
        course,
        assignment: webwork_assignment,
        run: webwork_run.id,
        reference: webwork_reference,
    };
    let webwork_attempt = issue_webwork(
        &webwork_fixture,
        &store,
        webwork_contract,
        webwork_provenance,
    )
    .await;
    let webwork_binding = LearnerWorkRoutingBinding::new(course, webwork_assignment);
    let active_webwork = store
        .read_issued_attempt_evidence(context, student, webwork_binding, webwork_attempt)
        .await
        .expect("active WebWork evidence");
    assert!(
        matches!(active_webwork, IssuedAttemptRead::Active(ref read) if read.presentation_snapshot().is_some()),
        "ordinary active WebWork evidence remains an answer-free presentation projection"
    );
    sealed_webwork::assert_sealed_webwork_execution(
        &pool,
        &store,
        &grader,
        sealed_webwork::SealedWebworkFixture {
            context,
            tenant,
            student,
            binding: webwork_binding,
            attempt: webwork_attempt,
            mismatched_attempt: active,
        },
    )
    .await;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                binding: webwork_binding,
                attempt: webwork_attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("issued-read-webwork-submitted")
                    .expect("key"),
            },
        )
        .await
        .expect("public Store submits WebWork attempt");
    let submitted_webwork = store
        .read_issued_attempt_evidence(context, student, webwork_binding, webwork_attempt)
        .await
        .expect("submitted WebWork receipt remains readable");
    assert!(
        matches!(submitted_webwork, IssuedAttemptRead::Submitted(ref read) if read.presentation().is_some()),
        "submitted WebWork reads only immutable receipt presentation"
    );
    let webwork_storage: (bool, bool) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM public.submission_receipt_snapshot WHERE tenant_id=$1 AND attempt_id=$2), EXISTS(SELECT 1 FROM public.webwork_grade_replay_state WHERE tenant_id=$1 AND attempt_id=$2)")
        .bind(tenant.as_uuid()).bind(webwork_attempt.as_uuid()).fetch_one(&pool).await.expect("read-only WebWork receipt/replay probe");
    assert_eq!(
        webwork_storage,
        (true, false),
        "submitted WebWork retains receipt while deleting active-only replay"
    );
    receipt_integrity
        .assert_cleared_receipt_fails_closed(
            webwork_binding,
            instructor,
            webwork_attempt,
            AttemptSupportActionId::from_uuid(id()),
        )
        .await;

    let terminal_reference = publish(&store, context, tenant, instructor).await;
    let terminal_assignment = AssignmentId::from_uuid(id());
    create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: terminal_assignment,
            tenant,
            course_id: course,
            title: "Automatic close oracle".to_string(),
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![item(terminal_reference, 0)],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: policies(),
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("publish automatic-close assignment");
    let terminal_run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, terminal_assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start automatic-close run");
    let terminal_fixture = IssueFixture {
        context,
        student,
        course,
        assignment: terminal_assignment,
        run: terminal_run.id,
        reference: terminal_reference,
    };
    let terminal_attempt = terminal_fixture.issue(&store, 0, 31).await;
    let policy = store
        .get_base_assignment_policy(context, terminal_assignment)
        .await
        .expect("read current base policy")
        .expect("stored base policy");
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: terminal_assignment,
                expected_revision: policy.revision,
                settings: AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions: AssignmentInstructions::default(),
                    base_policy: question_model::BaseAssignmentPolicy {
                        closes_at: Some(timing.closes_at()),
                        deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                        ..policy.policy
                    },
                },
            },
        )
        .await
        .expect("public teaching settings close active attempt");
    let terminal_read = store
        .read_issued_attempt_evidence(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, terminal_assignment),
            terminal_attempt,
        )
        .await
        .expect("closed attempt read");
    assert!(
        matches!(terminal_read, IssuedAttemptRead::TerminalWithoutReceipt(ref read) if read.status() == question_model::AttemptStatus::AutoSubmitted),
        "public AutoSubmit close yields terminal-without-receipt"
    );
    assert!(
        matches!(
            store
                .read_issued_attempt_evidence(context, student, webwork_binding, webwork_attempt)
                .await,
            Ok(IssuedAttemptRead::TerminalWithoutReceipt(ref read))
                if read.status() == question_model::AttemptStatus::Cleared
        ),
        "the same route-bound cleared attempt is available before roster revocation"
    );
    let revoked_revision = store
        .revoke_course_member(
            context,
            instructor_session,
            RevokeCourseMember {
                course,
                member: learner_membership.member.id,
                expected_revision: learner_membership.roster_revision,
            },
        )
        .await
        .expect("public session-bound CAS roster revocation");
    assert!(
        revoked_revision.value() > learner_membership.roster_revision.value(),
        "successful roster revocation advances the exact CAS revision"
    );
    assert_eq!(
        store
            .read_issued_attempt_evidence(context, student, webwork_binding, webwork_attempt)
            .await,
        Err(StoreError::NotFound),
        "the same route-bound read is concealed after current Student membership revocation"
    );
    database.cleanup().await;
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
