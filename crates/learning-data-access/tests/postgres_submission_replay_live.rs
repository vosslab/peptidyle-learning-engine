#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for immutable idempotent submission receipts.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, DraftRecord,
    FlatGradingCapability, IssueQuestionAttemptCommand, PresentationCapability,
    PublishDraftCommand, Store, StoreError, SubmissionIdempotencyKey, SubmitQuestionAttemptCommand,
    TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::envelope::QuestionEnvelope;
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::presentation::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_nonce_source,
};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, BackendCapabilities, Capability, CourseId, CourseMembership,
    CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    GradingDefinition, ImplementationVersion, PointValue, PresentationBindingV1, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

struct ReceiptNonce([u8; 16]);

impl NonceSourceV1 for ReceiptNonce {
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

fn assignment_item_at(reference: ProblemVersionRef, position: u32) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(id()),
        reference,
        position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

fn receipt_presentation(
    reference: ProblemVersionRef,
    seed: u64,
) -> (
    PresentationBindingV1,
    learning_data_access::ReceiptPresentationSnapshot,
) {
    let seed_bytes = seed.to_le_bytes();
    let mut nonce_bytes = [0_u8; 16];
    nonce_bytes[..8].copy_from_slice(&seed_bytes);
    nonce_bytes[8..].copy_from_slice(&seed.rotate_left(17).to_le_bytes());
    let mut nonce = ReceiptNonce(nonce_bytes);
    let presentation = build_presentation_v1_with_nonce_source(
        &QuestionEnvelope {
            version: reference.version,
            seed: Seed::new(seed),
            title: "Live submission replay question".to_string(),
            prompt: vec![ContentBlock::Text {
                markdown: "Live submission replay question".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
        },
        &[],
        &mut nonce,
    )
    .expect("native live receipt presentation");
    (
        PresentationBindingV1::new(
            presentation.envelope.presentation_nonce,
            presentation.digest,
        ),
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
    )
}

fn grading_envelope(reference: ProblemVersionRef, seed: u64) -> QuestionEnvelope {
    QuestionEnvelope {
        version: reference.version,
        seed: Seed::new(seed),
        title: "Live submission replay question".to_string(),
        prompt: vec![ContentBlock::Text {
            markdown: "Live submission replay question".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
    }
}

fn provenance() -> AttemptProvenance {
    AttemptProvenance {
        adapter: ImplementationVersion {
            id: "postgres-receipt-live".to_string(),
            version: "1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "postgres-receipt-live-grading".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: "postgres-receipt-live-render".to_string(),
    }
}

async fn begin_disposable_corruption(
    pool: &sqlx::PgPool,
    tenant: TenantId,
) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut transaction = pool
        .begin()
        .await
        .expect("begin disposable receipt corruption transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope the privileged corruption probe to its disposable tenant");
    transaction
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
                markdown: "Live submission replay question".to_string(),
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
                title: "Live submission replay question".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save live receipt draft");
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
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish live receipt question");
    reference
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_submission_replay_requires_its_immutable_receipt_snapshot() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Receipt snapshot course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("create receipt snapshot course");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let (presentation_binding, presentation) = receipt_presentation(reference, 1);
    let assignment = AssignmentId::from_uuid(id());
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Receipt snapshot practice".to_string(),
                items: vec![
                    assignment_item_at(reference, 0),
                    assignment_item_at(reference, 1),
                ],
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("create receipt snapshot assignment");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Live receipt student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("canonical roster upsert derives the receipt assignment enrollment");
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(id()))
        .await
        .expect("start receipt snapshot run");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(id()),
                run: run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                seed: 1,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(presentation_binding),
                presentation_snapshot: Some(presentation),
                grading_envelope: Some(grading_envelope(reference, 1)),
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                parameter_hash: "receipt-snapshot-parameters".to_string(),
                provenance: provenance(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue receipt snapshot question");
    let response = StudentResponse::Numeric { value: 18.0 };
    let key = SubmissionIdempotencyKey::parse("receipt-snapshot-replay")
        .expect("valid receipt snapshot key");
    let submitted = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("commit immutable receipt snapshot");
    let replay_store =
        PostgresStore::new(lazy_pool(&database_url).expect("fresh replay PostgreSQL pool"));
    assert_eq!(
        replay_store
            .replay_submission(context, student, attempt.id, &response, &key)
            .await,
        Ok(Some(submitted)),
        "an intact receipt replays exactly"
    );

    let (successor_binding, successor_presentation) = receipt_presentation(reference, 2);
    let successor = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(id()),
                run: run.id,
                assignment_position: 1,
                problem: reference.problem,
                question_version: reference.version,
                seed: 2,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(successor_binding),
                presentation_snapshot: Some(successor_presentation),
                grading_envelope: Some(grading_envelope(reference, 2)),
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                parameter_hash: "receipt-successor-parameters".to_string(),
                provenance: provenance(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: Some(attempt.id),
            },
        )
        .await
        .expect("issue and link the immutable successor receipt");
    let mut successor_corruption = begin_disposable_corruption(&pool, tenant).await;
    let corrupted_successor = sqlx::query(
        "UPDATE submission_next_attempt \
         SET next_payload_sha256 = repeat('0', 64) \
         WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .execute(&mut *successor_corruption)
    .await
    .expect("corrupt only the disposable successor descriptor checksum");
    assert_eq!(corrupted_successor.rows_affected(), 1);
    successor_corruption
        .commit()
        .await
        .expect("commit disposable successor corruption");
    let successor_verification_store =
        PostgresStore::new(lazy_pool(&database_url).expect("fresh successor verification pool"));
    assert!(matches!(
        successor_verification_store
            .finalize_submission_next_attempt(context, student, attempt.id, Some(successor.id))
            .await,
        Err(StoreError::Unavailable(_)) | Err(StoreError::InvalidRecord(_))
    ));

    let mut receipt_corruption = begin_disposable_corruption(&pool, tenant).await;
    let corrupted = sqlx::query(
        "UPDATE submission_receipt_snapshot \
         SET presentation_payload_sha256 = repeat('0', 64) \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .execute(&mut *receipt_corruption)
    .await
    .expect("corrupt only the disposable receipt checksum");
    assert_eq!(corrupted.rows_affected(), 1);
    receipt_corruption
        .commit()
        .await
        .expect("commit disposable receipt corruption");
    let checksum_replay_store = PostgresStore::new(
        lazy_pool(&database_url).expect("fresh checksum replay PostgreSQL pool"),
    );
    assert!(matches!(
        checksum_replay_store
            .replay_submission(context, student, attempt.id, &response, &key)
            .await,
        Err(StoreError::Unavailable(message)) if message.contains("receipt presentation checksum mismatch")
    ));
    assert!(matches!(
        checksum_replay_store
            .submission_record(context, student, attempt.id)
            .await,
        Err(StoreError::Unavailable(message)) if message.contains("receipt presentation checksum mismatch")
    ));

    let deleted = sqlx::query(
        "DELETE FROM submission_receipt_snapshot WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .execute(&pool)
    .await
    .expect("corrupt the disposable receipt fixture");
    assert_eq!(deleted.rows_affected(), 1);
    assert!(matches!(
        PostgresStore::new(lazy_pool(&database_url).expect("fresh missing-receipt PostgreSQL pool"))
            .replay_submission(context, student, attempt.id, &response, &key)
            .await,
        Err(StoreError::Unavailable(message)) if message.contains("receipt snapshot is missing")
    ));
    assert!(matches!(
        PostgresStore::new(lazy_pool(&database_url).expect("fresh missing-receipt GET PostgreSQL pool"))
            .submission_record(context, student, attempt.id)
            .await,
        Err(StoreError::Unavailable(message)) if message.contains("receipt snapshot is missing")
    ));
}
