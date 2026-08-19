#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for immutable idempotent submission receipts.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentUpdate, CatalogStore, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    PrefetchedQuestion, PresentationCapability, PublishDraftCommand,
    ReservePrefetchedQuestionCommand, Store, StoreError, SubmissionIdempotencyKey,
    SubmitQuestionAttemptCommand, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::envelope::QuestionEnvelope;
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::presentation::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_nonce_source,
};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, LearnerDisclosurePolicy,
    LearnerDisclosureTiming, RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, FeedbackContent, GradingDefinition,
    ImplementationVersion, PointValue, PresentationBindingV1, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource, ResponseDefinition,
    RunId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use std::sync::Arc;
use tokio::sync::Barrier;
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
            attempt_policy: AttemptPolicy { max_attempts: None },
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
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
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
async fn postgres_submission_replay_preserves_its_immutable_receipt_during_concurrent_prefetch() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);
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
                    title: "Receipt snapshot course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
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
                audience: question_model::AssignmentAudience::CourseWide,
                items: vec![
                    assignment_item_at(reference, 0),
                    assignment_item_at(reference, 1),
                ],
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
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
    let (successor_binding, successor_presentation) = receipt_presentation(reference, 2);
    let prefetched_successor = PrefetchedQuestion {
        tenant,
        run: run.id,
        predecessor: attempt.id,
        assignment_position: 1,
        problem: reference.problem,
        question_version: reference.version,
        seed: 2,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: successor_binding,
        presentation_snapshot: successor_presentation.clone(),
        grading_envelope: grading_envelope(reference, 2),
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_replay: None,
        webwork_grading: None,
        webwork_grading_capability: learning_data_access::WebworkGradingCapability::NotApplicable,
        parameter_hash: "receipt-successor-parameters".to_string(),
        provenance: provenance(),
    };
    let reserved_prefetch = store
        .reserve_or_resume_prefetched_question(
            context,
            ReservePrefetchedQuestionCommand {
                actor: student,
                reservation: prefetched_successor.clone(),
            },
        )
        .await
        .expect("reserve the exact successor before submission");
    assert_eq!(
        reserved_prefetch, prefetched_successor,
        "the sequential reservation retains the descriptor that promotion must consume"
    );
    let response = StudentResponse::Numeric { value: 18.0 };
    let key = SubmissionIdempotencyKey::parse("receipt-snapshot-replay")
        .expect("valid receipt snapshot key");
    let submission = SubmitQuestionAttemptCommand {
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
    };
    // This coordinates concurrent Store invocations. It intentionally makes
    // no claim that PostgreSQL held both transactions at a particular lock
    // point: establishing that would require a test-only database hook or
    // timing control outside this public Store behavior oracle.
    let barrier = Arc::new(Barrier::new(2));
    let submit_store = store.clone();
    let submit_barrier = Arc::clone(&barrier);
    let submit = tokio::spawn(async move {
        submit_barrier.wait().await;
        submit_store
            .submit_question_attempt(context, submission)
            .await
    });
    let prefetch_store = store.clone();
    let prefetch_barrier = Arc::clone(&barrier);
    let reservation = prefetched_successor.clone();
    let prefetch = tokio::spawn(async move {
        prefetch_barrier.wait().await;
        prefetch_store
            .reserve_or_resume_prefetched_question(
                context,
                ReservePrefetchedQuestionCommand {
                    actor: student,
                    reservation,
                },
            )
            .await
    });
    let submitted = submit
        .await
        .expect("submission task completes")
        .expect("concurrent submission commits one immutable receipt");
    match prefetch.await.expect("prefetch task completes") {
        Ok(reservation) => {
            assert_eq!(
                reservation, prefetched_successor,
                "a concurrent resume preserves the exact reserved descriptor"
            );
        }
        Err(StoreError::Conflict) => {}
        Err(error) => panic!("concurrent prefetch has a supported outcome: {error:?}"),
    }
    let initial_disclosure = submitted.disclosure.decision();
    assert!(
        initial_disclosure.score
            && initial_disclosure.per_item_correctness
            && initial_disclosure.feedback_text
            && initial_disclosure.solution
            && !initial_disclosure.class_statistics,
        "the submitted receipt uses the assignment's initial, after-submit policy"
    );
    let current = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("read current assignment before the disclosure-only revision")
        .expect("receipt assignment exists");
    store
        .replace_assignment_preserving_timing(
            context,
            course,
            assignment,
            current.revision,
            AssignmentUpdate {
                title: current.record.title.clone(),
                audience: current.record.audience.clone(),
                items: current.record.items.clone(),
                selection_groups: current.record.selection_groups.clone(),
                disclosure_policy: LearnerDisclosurePolicy {
                    score: LearnerDisclosureTiming::Never,
                    per_item_correctness: LearnerDisclosureTiming::Never,
                    feedback_text: LearnerDisclosureTiming::Never,
                    solution: LearnerDisclosureTiming::Never,
                    class_statistics: LearnerDisclosureTiming::Never,
                },
                policies: current.record.policies,
            },
        )
        .await
        .expect("current policy revision leaves the retained receipt intact");
    let replay_store =
        PostgresStore::new(lazy_pool(&database_url).expect("fresh replay PostgreSQL pool"));
    let replayed = replay_store
        .replay_submission(context, student, attempt.id, &response, &key)
        .await
        .expect("replay is valid after a disclosure-only revision")
        .expect("an intact receipt replays");
    assert_eq!(
        replayed.attempt, submitted.attempt,
        "a replay retains the immutable submitted attempt"
    );
    assert_eq!(
        replayed.run, submitted.run,
        "a replay retains the receipt run"
    );
    assert_eq!(
        replayed.summary, submitted.summary,
        "a replay retains the receipt summary"
    );
    assert!(
        replayed.feedback == submitted.feedback,
        "a replay retains private immutable feedback without exposing it in test output"
    );
    assert_eq!(
        replayed.presentation, submitted.presentation,
        "a replay retains the answer-free presentation receipt"
    );
    let replay_disclosure = replayed.disclosure.decision();
    assert!(
        !replay_disclosure.score
            && !replay_disclosure.per_item_correctness
            && !replay_disclosure.feedback_text
            && !replay_disclosure.solution
            && !replay_disclosure.class_statistics,
        "a replay re-evaluates its browser projection from the current policy rather than retaining historical disclosure"
    );

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
                prefetched: Some(prefetched_successor),
                predecessor_submission: Some(attempt.id),
            },
        )
        .await
        .expect("issue and link the immutable successor receipt");
    assert_eq!(
        store
            .finalize_submission_next_attempt(context, student, attempt.id, Some(successor.id))
            .await,
        Ok(()),
        "promotion records the exact immutable successor receipt"
    );
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
