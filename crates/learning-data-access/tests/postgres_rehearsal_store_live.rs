#![cfg(feature = "postgres")]

//! Disposable Store-level conformance oracle for the rehearsal aggregate.
//!
//! Run this ignored target only against a fresh database named by
//! `PLE_TEST_DATABASE_URL`.  Its fixture uses ordinary Store course,
//! membership, publication, and assignment operations; direct SQL is limited
//! to migration verification, database-owned timestamps, and non-rehearsal
//! side-effect counts.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::postgres::{PostgresStore, apply_migrations, lazy_pool};
use learning_data_access::{
    AppendRehearsalFrozenItemCommand, AssignmentRecord, CatalogStore,
    ClaimRehearsalSubmissionCommand, CompleteRehearsalSubmissionCommand, CourseRecord,
    CourseRosterStore, CreateAssignmentCommand, CreateCourseCommand, DraftRecord,
    MarkRehearsalSubmissionDispatchedCommand, NavigationReferenceStore,
    PutAssignmentTeachingSettingsCommand, RehearsalLocator, RehearsalStore,
    RehearsalSubmissionClaimResult, RehearsalSubmissionIdempotencyKey, StartRehearsalCommand,
    Store, StoreError, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId,
    AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseId, CourseLocalDateTime,
    CourseTerm, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, IanaTimeZone,
    LateSubmissionPolicy, PointValue, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    ProblemId, ProblemVersionRef, PublicationScope, QuestionMetadata, QuestionSource,
    RehearsalAttemptId, RehearsalEvidenceDigest, RehearsalFrozenItemEvidence, RehearsalLifecycle,
    RehearsalPrivateGradingResult, RehearsalSubjectStart, RehearsalSyntheticSubjectRequest,
    ResponseDefinition, StudentResponse, SyntheticPreviewModifiers, TeachingAttemptLimitFieldPatch,
    TeachingLimitFieldPatch, TeachingOperationRevision, TeachingTimeFieldPatch, TenantId, UserId,
    VersionId, WorkspaceId,
};
use sqlx::PgPool;
use std::num::NonZeroU32;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn deterministic_grade() -> RehearsalPrivateGradingResult {
    RehearsalPrivateGradingResult::Graded {
        result: question_model::AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        },
        feedback: question_model::DisclosedFeedback::empty(),
        backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
            "native:postgres-test".into(),
        )
        .expect("valid deterministic rehearsal receipt"),
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

fn synthetic_subject(moment: &str) -> RehearsalSubjectStart {
    RehearsalSubjectStart::Synthetic {
        request: RehearsalSyntheticSubjectRequest {
            selected_moment: PreviewSelectedMoment {
                value: CourseLocalDateTime::parse(moment).expect("moment"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
            },
            groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("groups"),
            modifiers: SyntheticPreviewModifiers {
                mode: question_model::PolicyModificationModeView::ExtendOnly,
                patch: question_model::PolicyPatchView {
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

async fn publish(
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
                family: "rehearsal_store_live".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Store rehearsal fixture".into(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Store rehearsal fixture".into(),
                tags: vec![],
                taxonomy: vec![],
                license: question_model::taxonomy::License::CcBy,
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
            learning_data_access::PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "rehearsal_store_live".into(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".into()).expect("byline"),
                ])
                .expect("byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("published question");
    reference
}

async fn ordinary_counts(pool: &PgPool, tenant: TenantId) -> [i64; 7] {
    async fn count(pool: &PgPool, tenant: TenantId, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .bind(tenant.as_uuid())
            .fetch_one(pool)
            .await
            .expect("ordinary effect count")
    }
    [
        count(
            pool,
            tenant,
            "SELECT count(*) FROM enrollment WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM assignment_run WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM question_attempt WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM submission WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM course_grade_scheme WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM course_item_analysis_current WHERE tenant_id=$1",
        )
        .await,
        count(
            pool,
            tenant,
            "SELECT count(*) FROM worker_job WHERE tenant_id=$1",
        )
        .await,
    ]
}

async fn database_millis(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT (extract(epoch FROM date_trunc('milliseconds', transaction_timestamp())) * 1000)::bigint",
    )
    .fetch_one(pool)
    .await
    .expect("database timestamp")
}

/// The Store requires the browser-originated frozen receipt to carry the exact
/// database-owned millisecond that it will validate inside its transaction.
/// Sampling that instant from this independent test connection can straddle a
/// millisecond boundary, so retry only the optimistic timestamp precondition.
async fn append_frozen_at_store_instant(
    store: &PostgresStore,
    context: TenantContext,
    locator: RehearsalLocator,
    pool: &PgPool,
    mut frozen: RehearsalFrozenItemEvidence,
) -> RehearsalFrozenItemEvidence {
    for _ in 0..64 {
        frozen.frozen_at = ActivityTimestamp::from_unix_millis(database_millis(pool).await);
        match store
            .append_rehearsal_frozen_item(
                context,
                AppendRehearsalFrozenItemCommand {
                    locator,
                    frozen: frozen.clone(),
                },
            )
            .await
        {
            Ok(()) => return frozen,
            Err(StoreError::Conflict) => tokio::task::yield_now().await,
            Err(error) => panic!("append frozen evidence: {error:?}"),
        }
    }
    panic!("could not sample the Store-owned frozen-evidence millisecond")
}

async fn assert_application_cannot_update_rehearsal_rows(pool: &PgPool) {
    let can_update: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('ple_app', 'public.rehearsal_run', 'UPDATE')",
    )
    .fetch_one(pool)
    .await
    .expect("application rehearsal-row privilege inventory");
    assert!(
        !can_update,
        "the Store verifies broker-prelocked rehearsals without application UPDATE authority"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL for a disposable migrated PostgreSQL database"]
async fn postgres_rehearsal_store_live_conformance() {
    let url = std::env::var("PLE_TEST_DATABASE_URL").expect("disposable database URL");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool).await.expect("full migration epoch");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x54; 32]);
    assert_application_cannot_update_rehearsal_rows(&pool).await;
    let tenant = TenantId::from_uuid(id());
    let foreign = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "PostgreSQL rehearsal Store".into(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
                        .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
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
                user: learner,
                display_name: "Ordinary learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("ordinary learner");
    let publication = publish(&store, context, tenant, instructor).await;
    let assignment_id = AssignmentId::from_uuid(id());
    let policy = BaseAssignmentPolicy {
        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_580_000_000)),
        due_at: None,
        closes_at: None,
        time_limit_seconds: Some(NonZeroU32::new(300).expect("limit")),
        attempt_limit: Some(NonZeroU32::new(2).expect("limit")),
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    };
    let instructions =
        AssignmentInstructions::try_new("Work through the problem.".into()).expect("instructions");
    let created = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                base_policy: policy,
                assignment: AssignmentRecord {
                    id: assignment_id,
                    tenant,
                    course_id: course,
                    title: "Rehearsal Store assignment".into(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: instructions.clone(),
                    audience: AssignmentAudience::CourseWide,
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id()),
                        reference: publication,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: vec![],
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            },
        )
        .await
        .expect("draft assignment");
    let published = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: assignment_id,
                expected_revision: created.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions,
                    base_policy: policy,
                },
            },
        )
        .await
        .expect("published assignment");
    let assignment = store
        .assignment_reference(context, instructor, assignment_id)
        .await
        .expect("reference query")
        .expect("published reference");
    let revision = TeachingOperationRevision::new(published.revision.value()).expect("revision");
    let before = ordinary_counts(&pool, tenant).await;
    let start = StartRehearsalCommand {
        actor: instructor,
        course,
        assignment,
        revision,
        subject: synthetic_subject("2026-08-25T09:00:00.000"),
        start_new_after_completion: false,
    };
    let first = store
        .start_rehearsal(context, start.clone())
        .await
        .expect("create active rehearsal");
    assert_eq!(first.lifecycle, RehearsalLifecycle::Active);
    assert_eq!(
        store
            .start_rehearsal(context, start.clone())
            .await
            .expect("resume"),
        first,
        "same identity resumes the durable aggregate"
    );
    let initial_locator = RehearsalLocator {
        actor: instructor,
        course,
        assignment,
        revision,
        rehearsal: first.rehearsal,
    };
    assert_eq!(
        store
            .read_rehearsal(context, initial_locator)
            .await
            .expect("rehydrate"),
        first,
        "Store rehydrates the exact durable receipt"
    );
    assert_eq!(
        store
            .read_rehearsal(
                TenantContext::from_authenticated_session(foreign),
                initial_locator,
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant cannot distinguish rehearsal existence"
    );
    assert_eq!(
        store
            .read_rehearsal(
                context,
                RehearsalLocator {
                    actor: outsider,
                    ..initial_locator
                },
            )
            .await,
        Err(StoreError::NotFound),
        "foreign actor cannot inspect the aggregate"
    );
    let changed = store
        .start_rehearsal(
            context,
            StartRehearsalCommand {
                subject: synthetic_subject("2026-08-26T09:00:00.000"),
                ..start.clone()
            },
        )
        .await
        .expect("replace active rehearsal for changed subject");
    assert_ne!(changed.rehearsal, first.rehearsal);
    assert_eq!(
        store
            .read_rehearsal(context, initial_locator)
            .await
            .expect("discarded predecessor")
            .lifecycle,
        RehearsalLifecycle::DiscardedByNewSubject,
        "changed live subject persists an explicit predecessor disposition"
    );
    let locator = RehearsalLocator {
        rehearsal: changed.rehearsal,
        ..initial_locator
    };
    let frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(id()),
        problem: publication,
        response_definition: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        },
        canonical_content_digest: RehearsalEvidenceDigest::from_bytes([0x44; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(0),
    };
    let frozen = append_frozen_at_store_instant(&store, context, locator, &pool, frozen).await;
    assert_eq!(
        store
            .append_rehearsal_frozen_item(
                context,
                AppendRehearsalFrozenItemCommand {
                    locator,
                    frozen: frozen.clone()
                },
            )
            .await,
        Err(StoreError::Conflict),
        "frozen inventory append is not silently duplicated"
    );
    let claim = store
        .claim_rehearsal_submission(
            context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new("submission-1".into())
                    .expect("key"),
            },
        )
        .await
        .expect("claim");
    let RehearsalSubmissionClaimResult::Claimed(claim) = claim else {
        panic!("first submission must create a claim");
    };
    assert!(matches!(
        store
            .claim_rehearsal_submission(
                context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: RehearsalSubmissionIdempotencyKey::new("submission-1".into())
                        .expect("key"),
                },
            )
            .await
            .expect("pending"),
        RehearsalSubmissionClaimResult::Pending
    ));
    let dispatched = store
        .mark_rehearsal_submission_dispatched(
            context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator,
                handle: claim.handle,
            },
        )
        .await
        .expect("dispatch");
    let completion = store
        .complete_rehearsal_submission(
            context,
            CompleteRehearsalSubmissionCommand {
                locator,
                handle: dispatched,
                grading: deterministic_grade(),
            },
        )
        .await
        .expect("complete submission");
    assert!(
        !completion.replayed,
        "initial result is a fresh public receipt"
    );
    assert!(matches!(
        store
            .claim_rehearsal_submission(
                context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: RehearsalSubmissionIdempotencyKey::new("submission-1".into())
                        .expect("key"),
                },
            )
            .await
            .expect("replay"),
        RehearsalSubmissionClaimResult::Replay(_)
    ));
    let completed = store
        .complete_rehearsal(context, locator)
        .await
        .expect("complete run");
    assert_eq!(completed.lifecycle, RehearsalLifecycle::Completed);
    assert_eq!(
        store.start_rehearsal(context, start.clone()).await,
        Err(StoreError::Conflict),
        "completed rehearsal requires explicit restart intent"
    );
    let replacement = store
        .start_rehearsal(
            context,
            StartRehearsalCommand {
                start_new_after_completion: true,
                ..start
            },
        )
        .await
        .expect("explicit replacement");
    assert_ne!(replacement.rehearsal, first.rehearsal);
    assert_eq!(replacement.lifecycle, RehearsalLifecycle::Active);
    assert_eq!(
        ordinary_counts(&pool, tenant).await,
        before,
        "rehearsal has no learner effects"
    );
    let persisted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_run WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("rehearsal count");
    assert_eq!(
        persisted, 3,
        "changed-subject, completed, and replacement rehearsal rows are durable"
    );
}
