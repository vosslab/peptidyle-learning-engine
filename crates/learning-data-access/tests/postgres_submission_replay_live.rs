#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for immutable idempotent submission receipts.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, DraftRecord, IssueQuestionAttemptCommand,
    PublishDraftCommand, Store, StoreError, SubmissionIdempotencyKey, SubmitQuestionAttemptCommand,
    TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, BackendCapabilities, Capability, CourseId, CourseMembership,
    CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource, FeedbackContent,
    GradingDefinition, ImplementationVersion, PointValue, PresentationBindingV1,
    PresentationDigestV1, PresentationNonceV1, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttemptId, QuestionMetadata, QuestionSource, ResponseDefinition, RunId,
    StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

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

fn assignment_item(reference: ProblemVersionRef) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(id()),
        reference,
        position: 0,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

fn presentation_binding() -> PresentationBindingV1 {
    PresentationBindingV1::new(
        PresentationNonceV1::from_bytes([7; 16]),
        PresentationDigestV1::compute(&[7]),
    )
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
        .expect("create receipt snapshot course");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Receipt snapshot practice".to_string(),
                items: vec![assignment_item(reference)],
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("create receipt snapshot assignment");
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
                presentation: Some(presentation_binding()),
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
    assert_eq!(
        store
            .replay_submission(context, student, attempt.id, &response, &key)
            .await,
        Ok(Some(submitted)),
        "an intact receipt replays exactly"
    );

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
        store
            .replay_submission(context, student, attempt.id, &response, &key)
            .await,
        Err(StoreError::Unavailable(message)) if message.contains("receipt snapshot is missing")
    ));
}
