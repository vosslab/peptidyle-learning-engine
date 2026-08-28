//! Focused Memory contract tests for the Instructor grading-operation store.

use super::*;
use crate::{
    AcceptedSubmissionId, GradingExecution, GradingExecutionState, GradingOperation,
    GradingOperationActionId, GradingOperationActionReceipt, GradingOperationGroupBy,
    GradingOperationRevision, GradingOperationStore, GradingOperationTarget,
    GradingOperationTrustGeneration, ListInstructorGradingOperationsCommand,
    MAX_INSTRUCTOR_GRADING_RETRY_COUNT, RecalculateAssignmentCommand, RetryGradingOperationCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
};
use question_model::{
    CourseMembershipId, CourseMembershipRole, EntitlementMaterialization, EntitlementPurpose,
    GradingOperationAction, GradingOperationReason, GradingOperationReference,
    GradingOperationState, MaterializationAuthority, MaterializationBasis, UserRole,
};
use uuid::Uuid;

const COURSE: u128 = 75_003;
const ASSIGNMENT: u128 = 75_004;
const LEARNER_MEMBERSHIP: u128 = 75_013;
const ENROLLMENT: u128 = 75_005;

struct Fixture {
    store: MemoryStore,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    learner_membership: CourseMembershipId,
    instructor: UserId,
    instructor_membership: CourseMembershipId,
}

fn fixture() -> Fixture {
    let store = MemoryStore::default();
    let (tenant, learner, attempt, submission) =
        super::super::grading_execution_worker::completion_tests::seed_complete_issued_execution(
            &store,
        );
    let course = CourseId::from_uuid(Uuid::from_u128(COURSE));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(ASSIGNMENT));
    let learner_membership = CourseMembershipId::from_uuid(Uuid::from_u128(LEARNER_MEMBERSHIP));
    let instructor = UserId::from_uuid(Uuid::from_u128(75_020));
    let instructor_membership = CourseMembershipId::from_uuid(Uuid::from_u128(75_021));
    let mut state = store.write_state().expect("fixture state");
    state.accounts.insert(
        learner,
        crate::AccountRecord {
            user: learner,
            email: crate::AuthenticationEmail::parse("learner@example.edu").expect("email"),
            display_name: "Learner Fixture".to_string(),
            platform_roles: Vec::new(),
            created_at: ActivityTimestamp::from_unix_millis(0),
            updated_at: ActivityTimestamp::from_unix_millis(0),
        },
    );
    state.accounts.insert(
        instructor,
        crate::AccountRecord {
            user: instructor,
            email: crate::AuthenticationEmail::parse("instructor@example.edu").expect("email"),
            display_name: "Instructor Fixture".to_string(),
            platform_roles: Vec::new(),
            created_at: ActivityTimestamp::from_unix_millis(0),
            updated_at: ActivityTimestamp::from_unix_millis(0),
        },
    );
    state.course_membership_references.insert(
        (tenant, learner_membership),
        question_model::CourseMembershipReference::new(75_013).expect("membership reference"),
    );
    state.entitlement_materializations.insert(
        (tenant, EnrollmentId::from_uuid(Uuid::from_u128(ENROLLMENT))),
        EntitlementMaterialization {
            enrollment: EnrollmentId::from_uuid(Uuid::from_u128(ENROLLMENT)),
            membership: learner_membership,
            user: learner,
            occurred_at: ActivityTimestamp::from_unix_millis(900),
            purpose: EntitlementPurpose::StartRun,
            authority: MaterializationAuthority::Actor(learner),
            basis: MaterializationBasis::CourseWide,
            evaluator_version: question_model::EvaluatorVersion::INITIAL,
        },
    );
    state.course_memberships.insert(
        (tenant, instructor_membership),
        CourseMembershipRecord {
            id: instructor_membership,
            tenant,
            course,
            user: instructor,
            student: None,
            role: CourseMembershipRole::Instructor,
            roster_id: None,
            status: CourseMemberStatus::Active,
            joined_at: ActivityTimestamp::from_unix_millis(900),
            revoked_at: None,
        },
    );
    state
        .active_course_membership_by_user
        .insert((tenant, course, instructor), instructor_membership);
    state.assignment_revisions.insert(
        (tenant, assignment),
        question_model::AssignmentRevision::INITIAL,
    );
    let issued_snapshot = crate::IssuedQuestionSnapshotV1::new(
        state
            .published
            .values()
            .next()
            .expect("published question")
            .question
            .clone(),
        crate::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("issued question snapshot");
    state
        .attempt_issued_question_snapshots
        .insert((tenant, attempt), issued_snapshot);
    state
        .automated_grading_executions
        .values_mut()
        .for_each(|execution| {
            execution.state = GradingExecutionState::Exception;
        });
    state.automated_grading_operations.insert(
        (
            tenant,
            GradingOperationReference::new(1).expect("operation reference"),
        ),
        GradingOperation {
            tenant,
            course,
            assignment,
            reference: GradingOperationReference::new(1).expect("operation reference"),
            target: GradingOperationTarget::SubmissionRecovery { submission },
            reason: GradingOperationReason::GraderExecutionFailure,
            state: GradingOperationState::Actionable,
            revision: GradingOperationRevision::INITIAL,
            next_action: Some(GradingOperationAction::Retry),
        },
    );
    drop(state);
    Fixture {
        store,
        tenant,
        course,
        assignment,
        learner_membership,
        instructor,
        instructor_membership,
    }
}

async fn instructor_session(fixture: &Fixture, material: &'static [u8]) -> SessionTokenHash {
    let session = SessionTokenHash::compute(material);
    fixture
        .store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.instructor,
                "Instructor Fixture",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(60).expect("session lifetime"),
        )
        .await
        .expect("session");
    session
}

fn retry_command(
    fixture: &Fixture,
    session: SessionTokenHash,
    action: u128,
) -> RetryGradingOperationCommand {
    RetryGradingOperationCommand {
        tenant: fixture.tenant,
        session,
        course: fixture.course,
        assignment: fixture.assignment,
        operation: GradingOperationReference::new(1).expect("operation reference"),
        action: GradingOperationActionId::from_uuid(Uuid::from_u128(action)),
        expected_revision: GradingOperationRevision::INITIAL,
    }
}

fn list_command(
    fixture: &Fixture,
    session: SessionTokenHash,
    group_by: GradingOperationGroupBy,
    page: PageRequest,
) -> ListInstructorGradingOperationsCommand {
    ListInstructorGradingOperationsCommand {
        tenant: fixture.tenant,
        session,
        course: fixture.course,
        assignment: fixture.assignment,
        group_by,
        page,
    }
}

#[tokio::test]
async fn exact_retry_replay_survives_same_actor_session_rotation() {
    let fixture = fixture();
    let first_session = instructor_session(&fixture, b"w5-first-session").await;
    let command = retry_command(&fixture, first_session, 75_100);
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let first = fixture
        .store
        .retry_instructor_grading_operation(context, command.clone())
        .await
        .expect("first retry");
    assert!(matches!(
        first,
        GradingOperationActionReceipt::Retry {
            safe_category: crate::GradingOperationReceiptSafeCategory::InstructorRetry,
            ..
        }
    ));
    let (jobs_after_first, receipts_after_first) = {
        let state_after_first = fixture.store.read_state().expect("state");
        assert_eq!(
            state_after_first
                .jobs
                .get(&crate::JobId::from_uuid(command.action.as_uuid()))
                .expect("retry job")
                .max_attempts,
            crate::ACCEPTED_SUBMISSION_JOB_MAX_ATTEMPTS
        );
        let receipts_after_first = state_after_first
            .automated_grading_execution_receipts
            .values()
            .next()
            .expect("execution receipts")
            .len();
        assert_eq!(
            state_after_first
                .automated_grading_execution_receipts
                .values()
                .next()
                .and_then(|receipts| receipts.last())
                .expect("retry receipt")
                .worker,
            None
        );
        let retry_receipt = state_after_first
            .automated_grading_execution_receipts
            .values()
            .next()
            .and_then(|receipts| receipts.last())
            .expect("retry receipt");
        assert_eq!(
            retry_receipt.safe_category,
            crate::GradingExecutionReceiptSafeCategory::InstructorRetry
        );
        assert!(retry_receipt.actor.is_some());
        (state_after_first.jobs.len(), receipts_after_first)
    };
    fixture
        .store
        .revoke_session(first_session)
        .await
        .expect("rotate session");
    let second_session = instructor_session(&fixture, b"w5-second-session").await;
    let replay = fixture
        .store
        .retry_instructor_grading_operation(
            context,
            RetryGradingOperationCommand {
                session: second_session,
                ..command
            },
        )
        .await
        .expect("same actor replay");
    assert_eq!(replay, first);
    assert_eq!(
        fixture.store.read_state().expect("state").jobs.len(),
        jobs_after_first
    );
    assert_eq!(
        fixture
            .store
            .read_state()
            .expect("state")
            .automated_grading_execution_receipts
            .values()
            .next()
            .expect("execution receipts")
            .len(),
        receipts_after_first
    );
}

#[tokio::test]
async fn instructor_retry_ceiling_rejects_new_work_without_mutation() {
    let fixture = fixture();
    let session = instructor_session(&fixture, b"w5-retry-ceiling").await;
    {
        let mut state = fixture.store.write_state().expect("state");
        state
            .automated_grading_executions
            .values_mut()
            .next()
            .expect("accepted execution")
            .retry_count = MAX_INSTRUCTOR_GRADING_RETRY_COUNT;
    }
    let jobs_before = fixture.store.read_state().expect("state").jobs.len();
    assert_eq!(
        fixture
            .store
            .retry_instructor_grading_operation(
                TenantContext::from_authenticated_session(fixture.tenant),
                retry_command(&fixture, session, 75_102),
            )
            .await,
        Err(StoreError::Conflict)
    );
    let state = fixture.store.read_state().expect("state");
    assert_eq!(state.jobs.len(), jobs_before);
    assert_eq!(
        state
            .automated_grading_operations
            .get(&(
                fixture.tenant,
                GradingOperationReference::new(1).expect("operation reference"),
            ))
            .expect("operation")
            .state,
        GradingOperationState::Actionable
    );
}

#[tokio::test]
async fn revoked_current_authority_conceals_a_prior_action_receipt() {
    let fixture = fixture();
    let session = instructor_session(&fixture, b"w5-revocation-session").await;
    let command = retry_command(&fixture, session, 75_101);
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    fixture
        .store
        .retry_instructor_grading_operation(context, command.clone())
        .await
        .expect("first retry");
    fixture
        .store
        .write_state()
        .expect("state")
        .course_memberships
        .get_mut(&(fixture.tenant, fixture.instructor_membership))
        .expect("instructor membership")
        .status = CourseMemberStatus::Revoked;
    assert_eq!(
        fixture
            .store
            .retry_instructor_grading_operation(context, command)
            .await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
async fn authorized_instructor_learner_group_preserves_materialized_membership_reference_after_revoke()
 {
    let fixture = fixture();
    let session = instructor_session(&fixture, b"w5-historical-learner").await;
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let before = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Learner,
                PageRequest::first(PageSize::new(10).expect("page")),
            ),
        )
        .await
        .expect("learner list before revoke");
    assert_eq!(before.items.len(), 1);
    fixture
        .store
        .write_state()
        .expect("state")
        .course_memberships
        .get_mut(&(fixture.tenant, fixture.learner_membership))
        .expect("learner membership")
        .status = CourseMemberStatus::Revoked;
    let after = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Learner,
                PageRequest::first(PageSize::new(10).expect("page")),
            ),
        )
        .await
        .expect("historical learner list after revoke");
    assert_eq!(after.items, before.items);
}

#[tokio::test]
async fn operation_pagination_rejects_cross_view_and_cross_scope_cursors() {
    let fixture = fixture();
    let session = instructor_session(&fixture, b"w5-pagination").await;
    {
        let mut state = fixture.store.write_state().expect("state");
        let base_attempt = state
            .automated_grading_executions
            .keys()
            .next()
            .expect("attempt")
            .1;
        for number in 2..=3 {
            add_distinct_submission_recovery_thread(&mut state, base_attempt, number);
            let reference = GradingOperationReference::new(number).expect("reference");
            let submission = state
                .automated_grading_executions
                .get(&(
                    fixture.tenant,
                    QuestionAttemptId::from_uuid(Uuid::from_u128(u128::from(76_000 + number))),
                ))
                .expect("distinct execution")
                .submission;
            state.automated_grading_operations.insert(
                (fixture.tenant, reference),
                GradingOperation {
                    tenant: fixture.tenant,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    reference,
                    target: GradingOperationTarget::SubmissionRecovery { submission },
                    reason: GradingOperationReason::GraderExecutionFailure,
                    state: GradingOperationState::Actionable,
                    revision: GradingOperationRevision::INITIAL,
                    next_action: Some(GradingOperationAction::Retry),
                },
            );
        }
    }
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let first = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Question,
                PageRequest::first(PageSize::new(2).expect("page")),
            ),
        )
        .await
        .expect("first page");
    assert_eq!(first.items.len(), 2);
    let cursor = first.next_cursor.expect("bounded continuation");
    let second = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Question,
                PageRequest::after(cursor.clone(), PageSize::new(2).expect("page")),
            ),
        )
        .await
        .expect("second page");
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    let maximum = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Question,
                PageRequest::first(PageSize::new(PageSize::MAX).expect("maximum page")),
            ),
        )
        .await
        .expect("maximum page size");
    assert_eq!(maximum.items.len(), 3);
    assert!(maximum.next_cursor.is_none());
    assert!(matches!(
        fixture
            .store
            .list_instructor_grading_operations(
                context,
                list_command(
                    &fixture,
                    session,
                    GradingOperationGroupBy::Learner,
                    PageRequest::after(cursor.clone(), PageSize::new(2).expect("page")),
                ),
            )
            .await,
        Err(StoreError::Conflict) | Err(StoreError::InvalidRecord(_))
    ));
    let wrong_course = CourseId::from_uuid(Uuid::from_u128(75_030));
    assert_eq!(
        crate::GradingOperationCursor::decode(
            &cursor,
            fixture.tenant,
            wrong_course,
            fixture.assignment,
            GradingOperationGroupBy::Question,
        ),
        Err(StoreError::Conflict)
    );
}

fn add_distinct_submission_recovery_thread(
    state: &mut State,
    base_attempt: QuestionAttemptId,
    number: u64,
) {
    let (tenant, base_execution) = state
        .automated_grading_executions
        .iter()
        .find(|((_, attempt), _)| *attempt == base_attempt)
        .map(|((tenant, _), execution)| (*tenant, *execution))
        .expect("base execution");
    let new_attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(u128::from(76_000 + number)));
    let new_submission =
        AcceptedSubmissionId::from_uuid(Uuid::from_u128(u128::from(77_000 + number)));
    let new_job = crate::JobId::from_uuid(Uuid::from_u128(u128::from(78_000 + number)));

    let mut attempt = state
        .attempts
        .get(&(tenant, base_attempt))
        .cloned()
        .expect("base attempt");
    attempt.id = new_attempt;
    state.attempts.insert((tenant, new_attempt), attempt);
    if let Some(snapshot) = state
        .attempt_issued_question_snapshots
        .get(&(tenant, base_attempt))
        .cloned()
    {
        state
            .attempt_issued_question_snapshots
            .insert((tenant, new_attempt), snapshot);
    }
    let mut submission = state
        .submissions
        .get(&(tenant, base_attempt))
        .cloned()
        .expect("base accepted submission");
    let idempotency_key =
        crate::SubmissionIdempotencyKey::parse(format!("pagination-thread-{number}"))
            .expect("distinct idempotency key");
    match &mut submission.state {
        StoredSubmissionState::AcceptedPending(accepted) => {
            accepted.attempt = new_attempt;
            accepted.submission = new_submission;
            accepted.idempotency_key = idempotency_key.clone();
        }
        StoredSubmissionState::Completed(_) => panic!("pagination fixture needs pending input"),
    }
    submission.key = idempotency_key;
    state.submissions.insert((tenant, new_attempt), submission);
    state.automated_grading_evaluations.insert(
        (tenant, new_attempt),
        question_model::SubmissionEvaluationStatus::AutomatedPending,
    );
    state.automated_grading_executions.insert(
        (tenant, new_attempt),
        GradingExecution {
            submission: new_submission,
            job: new_job,
            ..base_execution
        },
    );
    if let Some(mut job) = state.jobs.get(&base_execution.job).cloned() {
        job.payload = crate::JobPayload::GradeAcceptedSubmission {
            attempt: new_attempt,
            submission: new_submission,
            execution_generation: base_execution.generation,
        };
        state.jobs.insert(new_job, job);
    }
}

#[tokio::test]
async fn recalculation_projects_reason_count_generation_and_replay_without_extra_job() {
    let fixture = fixture();
    let session = instructor_session(&fixture, b"w5-recalculation").await;
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let command = RecalculateAssignmentCommand {
        tenant: fixture.tenant,
        session,
        course: fixture.course,
        assignment: fixture.assignment,
        action: GradingOperationActionId::from_uuid(Uuid::from_u128(75_200)),
        expected_assignment_revision: question_model::AssignmentRevision::INITIAL,
    };
    let first = fixture
        .store
        .recalculate_instructor_assignment(context, command.clone())
        .await
        .expect("recalculation");
    let jobs_after_first = fixture.store.read_state().expect("state").jobs.len();
    let row = fixture
        .store
        .list_instructor_grading_operations(
            context,
            list_command(
                &fixture,
                session,
                GradingOperationGroupBy::Question,
                PageRequest::first(PageSize::new(10).expect("page")),
            ),
        )
        .await
        .expect("recalculation row")
        .items
        .into_iter()
        .find(|row| {
            row.operation.reason == GradingOperationReason::InstructorRequestedRecalculation
        })
        .expect("row");
    assert_eq!(row.affected_learner_count, 1);
    assert_eq!(
        row.operation.reason,
        GradingOperationReason::InstructorRequestedRecalculation
    );
    assert_eq!(
        row.trust_generation,
        GradingOperationTrustGeneration::AssignmentScoring(
            ScoringGeneration::new(2).expect("generation"),
        )
    );
    let replay = fixture
        .store
        .recalculate_instructor_assignment(context, command.clone())
        .await
        .expect("exact replay");
    assert_eq!(replay, first);
    assert_eq!(
        fixture.store.read_state().expect("state").jobs.len(),
        jobs_after_first
    );
    let conflict = fixture
        .store
        .recalculate_instructor_assignment(
            context,
            RecalculateAssignmentCommand {
                tenant: fixture.tenant,
                session,
                course: fixture.course,
                assignment: fixture.assignment,
                action: GradingOperationActionId::from_uuid(Uuid::from_u128(75_201)),
                expected_assignment_revision: question_model::AssignmentRevision::INITIAL,
            },
        )
        .await;
    assert_eq!(conflict, Err(StoreError::Conflict));
    assert_eq!(
        fixture.store.read_state().expect("state").jobs.len(),
        jobs_after_first
    );
}
