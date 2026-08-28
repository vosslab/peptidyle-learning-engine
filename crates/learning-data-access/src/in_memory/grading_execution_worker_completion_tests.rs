//! Stable completion-contract proof for the Memory automated-grading worker.

use super::*;
use crate::{
    AcceptedSubmissionGrade, AcceptedSubmissionId, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, GradingExecution,
    GradingExecutionGeneration, GradingOperation, GradingOperationActionId,
    GradingOperationGroupBy, GradingOperationRevision, GradingOperationStore,
    GradingOperationTarget, JobClaimFilter, JobKind, JobStore,
    ListInstructorGradingOperationsCommand, RetryGradingOperationCommand, SessionLifetime,
    SessionStore, SessionSubject, SessionTokenHash, Store, SubmissionReceiptRead,
    canonical_attempt_result_json,
};
use question_model::envelope::ContentBlock;
use question_model::{
    AssignmentEnrollment, AssignmentItem, AssignmentItemId, AssignmentRun, AttemptResult,
    CourseMembershipId, CourseMembershipRole, CourseTerm, FeedbackContent, GradingOperationAction,
    GradingOperationReason, GradingOperationReference, GradingOperationState, RunMode, StudentId,
};

pub(crate) fn seed_complete_issued_execution(
    store: &MemoryStore,
) -> (TenantId, UserId, QuestionAttemptId, AcceptedSubmissionId) {
    let tenant = TenantId::from_uuid(Uuid::from_u128(75_001));
    let actor = UserId::from_uuid(Uuid::from_u128(75_002));
    let course = CourseId::from_uuid(Uuid::from_u128(75_003));
    let assignment_id = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let enrollment_id = EnrollmentId::from_uuid(Uuid::from_u128(75_005));
    let run_id = RunId::from_uuid(Uuid::from_u128(75_006));
    let attempt_id = QuestionAttemptId::from_uuid(Uuid::from_u128(75_007));
    let submission = AcceptedSubmissionId::from_uuid(Uuid::from_u128(75_008));
    let execution_job = crate::JobId::from_uuid(Uuid::from_u128(75_009));
    let published = super::super::catalog_search_tests::record(75_010);
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id: course,
        title: "Worker completion fixture".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: AssignmentItemId::from_uuid(Uuid::from_u128(75_011)),
            reference,
            position: 0,
            points_possible: question_model::PointValue::from_whole(2),
            delivery_state: question_model::AssignmentDeliveryState::Active,
            scoring_mode: question_model::AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
        policies: question_model::RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: question_model::GradePolicy::First,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let enrollment = AssignmentEnrollment {
        id: enrollment_id,
        tenant,
        assignment: assignment_id,
        user: actor,
        student: StudentId::from_uuid(Uuid::from_u128(75_012)),
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    let run = AssignmentRun {
        id: run_id,
        reference: question_model::RunReference::new(1).expect("valid run reference"),
        tenant,
        enrollment: enrollment_id,
        run_number: 1,
        started_at: ActivityTimestamp::from_unix_millis(900),
        completed_at: None,
        score: None,
        mode: RunMode::Assigned,
        variation: question_model::VariationPolicy::NewSeeds,
    };
    let attempt = super::super::catalog_search_tests::statistics_attempt(
        75_007, tenant, run_id, reference, 0, 900,
    );
    let accepted = AcceptedSubmission {
        tenant,
        course,
        assignment: assignment_id,
        attempt: attempt_id,
        submission,
        actor,
        idempotency_key: crate::SubmissionIdempotencyKey::parse("worker-completion")
            .expect("valid idempotency key"),
        request_sha256: objects::Sha256Digest::compute(b"accepted-response"),
        accepted_at: ActivityTimestamp::from_unix_millis(1_000),
    };
    let mut state = store.write_state().expect("fixture state");
    state.authoritative_time = accepted.accepted_at;
    state.courses.insert(
        (tenant, course),
        CourseRecord {
            id: course,
            tenant,
            title: "Worker completion course".to_string(),
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("valid course term"),
        },
    );
    let membership = CourseMembershipId::from_uuid(Uuid::from_u128(75_013));
    state.course_memberships.insert(
        (tenant, membership),
        CourseMembershipRecord {
            id: membership,
            tenant,
            course,
            user: actor,
            student: Some(enrollment.student),
            role: CourseMembershipRole::Student,
            roster_id: None,
            status: CourseMemberStatus::Active,
            joined_at: ActivityTimestamp::from_unix_millis(900),
            revoked_at: None,
        },
    );
    state
        .active_course_membership_by_user
        .insert((tenant, course, actor), membership);
    state
        .assignments
        .insert((tenant, assignment_id), assignment.clone());
    state
        .enrollments
        .insert((tenant, enrollment_id), enrollment);
    state.runs.insert((tenant, run_id), run.clone());
    state.run_items.insert(
        (tenant, run_id),
        select_assignment_run_items(&assignment, &run).expect("valid issued item"),
    );
    state.summaries.insert(
        (tenant, enrollment_id),
        StudentAssignmentSummary::empty(tenant, enrollment_id),
    );
    state.attempts.insert((tenant, attempt_id), attempt.clone());
    state
        .published
        .insert((published.problem, published.version), published);
    super::super::catalog_search_tests::insert_statistics_issued_authority(&mut state, &attempt);
    state.submissions.insert(
        (tenant, attempt_id),
        StoredSubmission {
            key: accepted.idempotency_key.clone(),
            state: StoredSubmissionState::AcceptedPending(accepted),
        },
    );
    state.automated_grading_executions.insert(
        (tenant, attempt_id),
        GradingExecution {
            submission,
            generation: GradingExecutionGeneration::INITIAL,
            state: crate::GradingExecutionState::Ready,
            job: execution_job,
            retry_count: 0,
        },
    );
    state.automated_grading_evaluations.insert(
        (tenant, attempt_id),
        SubmissionEvaluationStatus::AutomatedPending,
    );
    state.assignment_scoring.insert(
        (tenant, assignment_id),
        (ScoringGeneration::INITIAL, ScoringStatus::Current),
    );
    state.jobs.insert(
        execution_job,
        StoredJob {
            tenant,
            payload: JobPayload::GradeAcceptedSubmission {
                attempt: attempt_id,
                submission,
                execution_generation: GradingExecutionGeneration::INITIAL,
            },
            state: JobState::Ready,
            available_at: ActivityTimestamp::from_unix_millis(1_000),
            lease_token: None,
            lease_expires_at: None,
            attempt_count: 0,
            max_attempts: 3,
            failure: None,
        },
    );
    (tenant, actor, attempt_id, submission)
}

#[tokio::test]
async fn instructor_retry_is_replayed_exactly_and_revocation_conceals_operations() {
    let store = MemoryStore::default();
    let (tenant, actor, attempt, submission) = seed_complete_issued_execution(&store);
    let course = CourseId::from_uuid(Uuid::from_u128(75_003));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let operation = GradingOperationReference::new(1).expect("positive operation reference");
    let membership = CourseMembershipId::from_uuid(Uuid::from_u128(75_013));
    let session = SessionTokenHash::compute(b"instructor operation session");
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                actor,
                "Instructor",
                vec![question_model::UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(60).expect("session lifetime"),
        )
        .await
        .expect("session");
    {
        let mut state = store.write_state().expect("fixture state");
        state
            .course_memberships
            .get_mut(&(tenant, membership))
            .expect("membership")
            .role = CourseMembershipRole::Instructor;
        state.assignment_revisions.insert(
            (tenant, assignment),
            question_model::AssignmentRevision::INITIAL,
        );
        state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .expect("execution")
            .state = crate::GradingExecutionState::Exception;
        state.automated_grading_operations.insert(
            (tenant, operation),
            GradingOperation {
                tenant,
                course,
                assignment,
                reference: operation,
                target: GradingOperationTarget::SubmissionRecovery { submission },
                reason: GradingOperationReason::GraderExecutionFailure,
                state: GradingOperationState::Actionable,
                revision: GradingOperationRevision::INITIAL,
                next_action: Some(GradingOperationAction::Retry),
            },
        );
    }
    let command = RetryGradingOperationCommand {
        tenant,
        session,
        course,
        assignment,
        operation,
        action: GradingOperationActionId::from_uuid(Uuid::from_u128(75_100)),
        expected_revision: GradingOperationRevision::INITIAL,
    };
    let context = TenantContext::from_authenticated_session(tenant);
    let receipt = store
        .retry_instructor_grading_operation(context, command.clone())
        .await
        .expect("current Instructor retry");
    assert_eq!(
        store
            .retry_instructor_grading_operation(context, command.clone())
            .await
            .expect("exact replay"),
        receipt
    );
    {
        let state = store.read_state().expect("retried state");
        let execution = state.automated_grading_executions[&(tenant, attempt)];
        assert_eq!(execution.generation.as_u64(), 2);
        let expected_job = crate::JobId::from_uuid(command.action.as_uuid());
        assert_eq!(execution.job, expected_job);
        assert_eq!(state.jobs[&expected_job].state, JobState::Ready);
    }

    store
        .write_state()
        .expect("revoke state")
        .course_memberships
        .get_mut(&(tenant, membership))
        .expect("membership")
        .status = CourseMemberStatus::Revoked;
    assert_eq!(
        store
            .list_instructor_grading_operations(
                context,
                ListInstructorGradingOperationsCommand {
                    tenant,
                    session,
                    course,
                    assignment,
                    group_by: GradingOperationGroupBy::Question,
                    page: crate::PageRequest::first(
                        crate::PageSize::new(1).expect("bounded page"),
                    ),
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
async fn evaluated_worker_commit_seals_answer_free_receipt_and_queues_recalculation() {
    let store = MemoryStore::default();
    let (tenant, actor, attempt, submission) = seed_complete_issued_execution(&store);
    let issued_attempt =
        store.read_state().expect("issued state").attempts[&(tenant, attempt)].clone();
    let worker = WorkerId::from_uuid(Uuid::from_u128(75_014));
    let claim = store
        .claim_next_accepted_submission_execution(
            worker,
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("issued execution is claimable");
    let result = AttemptResult {
        correct: true,
        points_earned: 2.0,
        points_possible: 2.0,
    };
    let feedback = FeedbackContent {
        hint: Some(vec![ContentBlock::Text {
            markdown: "Inspect the carbonyl oxygen.".to_string(),
        }]),
        ..FeedbackContent::default()
    };
    let grade = AcceptedSubmissionGrade {
        evidence: canonical_attempt_result_json(result).expect("canonical result"),
        feedback: feedback.clone(),
    };

    // ASVS 2.3.1-4 and 15.4.1-3: the claim-bound terminal transition either
    // commits its complete aggregate once or leaves the fenced work active.
    assert_eq!(
        store
            .commit_or_fail_accepted_submission_execution(
                TenantContext::from_authenticated_session(tenant),
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated {
                    grade: grade.clone(),
                },
            )
            .await
            .expect("worker commit succeeds"),
        AcceptedSubmissionExecutionDisposition::Committed
    );

    let first_replay = store
        .submission_record(
            TenantContext::from_authenticated_session(tenant),
            actor,
            attempt,
        )
        .await
        .expect("completed receipt reads");
    let second_replay = store
        .submission_record(
            TenantContext::from_authenticated_session(tenant),
            actor,
            attempt,
        )
        .await
        .expect("receipt replay reads");
    assert_eq!(first_replay, second_replay);

    let state = store.read_state().expect("completed state");
    assert_eq!(
        state.attempts[&(tenant, attempt)],
        issued_attempt,
        "worker completion preserves the immutable issuance snapshot"
    );
    let receipt = state.submissions[&(tenant, attempt)]
        .completed_record_opt()
        .expect("worker wrote immutable receipt");
    // ASVS 14.2.6: the replayable learner receipt excludes accepted answers;
    // feedback remains in the server-owned receipt until disclosure permits it.
    assert!(
        receipt.attempt.response.is_none()
            && receipt.attempt.status == question_model::AttemptStatus::Submitted
            && receipt.attempt.result == Some(result)
            && receipt.feedback.content() == &feedback
    );
    assert_eq!(
        state
            .automated_grading_result_evidence
            .get(&(tenant, attempt)),
        Some(&grade.evidence)
    );
    assert_eq!(
        state.automated_grading_evaluations[&(tenant, attempt)],
        SubmissionEvaluationStatus::Graded,
        "the automated worker can persist only a graded successful evaluation"
    );
    assert!(
        state.runs[&(tenant, receipt.run.id)].completed_at.is_some()
            && state.summaries[&(tenant, receipt.summary.enrollment)].completed_run_count == 1
            && state.enrollments[&(tenant, receipt.run.enrollment)]
                .first_completed_at
                .is_some()
            && !state.attempt_scores.contains_key(&(tenant, attempt))
    );
    assert_eq!(
        state
            .assignment_scoring
            .get(&(tenant, AssignmentId::from_uuid(Uuid::from_u128(75_004)),)),
        Some(&(
            ScoringGeneration::new(2).expect("next generation"),
            ScoringStatus::Recalculating,
        ))
    );
    assert_eq!(
        state
            .jobs
            .values()
            .filter(|job| matches!(
                job.payload,
                JobPayload::RecalculateAssignment { assignment, .. }
                    if assignment == AssignmentId::from_uuid(Uuid::from_u128(75_004))
            ))
            .count(),
        1,
        "the terminal evaluation owns one recalculation request"
    );
    assert!(matches!(
        first_replay,
        SubmissionReceiptRead::Completed(record)
            if record.attempt.response.is_none() && record.attempt.result == Some(result)
    ));
    assert_eq!(
        state.automated_grading_executions[&(tenant, attempt)].submission,
        submission
    );
}

#[tokio::test]
async fn worker_outcomes_project_the_exact_instructor_operation_threads() {
    let store = MemoryStore::default();
    let (tenant, _actor, _attempt, submission) = seed_complete_issued_execution(&store);
    let course = CourseId::from_uuid(Uuid::from_u128(75_003));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let submission_operation = GradingOperationReference::new(1).expect("positive reference");
    {
        let mut state = store.write_state().expect("memory state");
        state.automated_grading_operations.insert(
            (tenant, submission_operation),
            GradingOperation {
                tenant,
                course,
                assignment,
                reference: submission_operation,
                target: GradingOperationTarget::SubmissionRecovery { submission },
                reason: GradingOperationReason::GraderExecutionFailure,
                state: GradingOperationState::ActionInProgress,
                revision: GradingOperationRevision::INITIAL,
                next_action: None,
            },
        );
    }
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(Uuid::from_u128(75_110)),
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("issued execution is claimable");
    let grade = AcceptedSubmissionGrade {
        evidence: canonical_attempt_result_json(AttemptResult {
            correct: true,
            points_earned: 2.0,
            points_possible: 2.0,
        })
        .expect("canonical result"),
        feedback: FeedbackContent::default(),
    };
    assert_eq!(
        store
            .commit_or_fail_accepted_submission_execution(
                TenantContext::from_authenticated_session(tenant),
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated { grade },
            )
            .await
            .expect("worker completion"),
        AcceptedSubmissionExecutionDisposition::Committed
    );

    let generation = ScoringGeneration::new(2).expect("next generation");
    let scoring_operation = GradingOperationReference::new(2).expect("positive reference");
    {
        let mut state = store.write_state().expect("completed state");
        state.automated_grading_operations.insert(
            (tenant, scoring_operation),
            GradingOperation {
                tenant,
                course,
                assignment,
                reference: scoring_operation,
                target: GradingOperationTarget::AssignmentScoringGeneration {
                    requested_generation: generation,
                },
                reason: GradingOperationReason::InstructorRequestedRecalculation,
                state: GradingOperationState::ActionInProgress,
                revision: GradingOperationRevision::INITIAL,
                next_action: None,
            },
        );
    }
    let scoring_claim = store
        .claim_next_job(
            &JobClaimFilter::new([JobKind::RecalculateAssignment]).expect("scoring filter"),
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("scoring claim")
        .expect("recalculation is queued");
    let command = AssignmentScoringWorkerCommand {
        job: scoring_claim.id,
        lease: scoring_claim.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(TenantContext::from_authenticated_session(tenant), command)
        .await
        .expect("scoring preparation");
    assert_eq!(
        store
            .commit_assignment_scoring(TenantContext::from_authenticated_session(tenant), command)
            .await
            .expect("scoring publication"),
        crate::AssignmentScoringCommitOutcome::Committed
    );

    let state = store.read_state().expect("projected state");
    assert_eq!(
        state.automated_grading_operations[&(tenant, submission_operation)].state,
        GradingOperationState::Completed
    );
    let scoring = state.automated_grading_operations[&(tenant, scoring_operation)];
    assert_eq!(scoring.state, GradingOperationState::Completed);
    assert_eq!(scoring.next_action, None);
}

#[tokio::test]
async fn terminal_worker_failure_reopens_the_existing_submission_thread() {
    let store = MemoryStore::default();
    let (tenant, _actor, _attempt, submission) = seed_complete_issued_execution(&store);
    let course = CourseId::from_uuid(Uuid::from_u128(75_003));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let reference = GradingOperationReference::new(1).expect("positive reference");
    {
        let mut state = store.write_state().expect("memory state");
        state.automated_grading_operations.insert(
            (tenant, reference),
            GradingOperation {
                tenant,
                course,
                assignment,
                reference,
                target: GradingOperationTarget::SubmissionRecovery { submission },
                reason: GradingOperationReason::GraderExecutionFailure,
                state: GradingOperationState::ActionInProgress,
                revision: GradingOperationRevision::INITIAL,
                next_action: None,
            },
        );
    }
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(Uuid::from_u128(75_111)),
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("issued execution is claimable");
    assert_eq!(
        store
            .commit_or_fail_accepted_submission_execution(
                TenantContext::from_authenticated_session(tenant),
                claim,
                AcceptedSubmissionExecutionOutcome::TerminalFailure,
            )
            .await
            .expect("terminal failure is committed"),
        AcceptedSubmissionExecutionDisposition::Terminal
    );

    let state = store.read_state().expect("failed state");
    let operation = state.automated_grading_operations[&(tenant, reference)];
    assert_eq!(operation.state, GradingOperationState::Actionable);
    assert_eq!(operation.next_action, Some(GradingOperationAction::Retry));
}

#[tokio::test]
async fn scoring_worker_retires_a_generation_superseded_before_preparation() {
    let store = MemoryStore::default();
    let (tenant, _actor, _attempt, _submission) = seed_complete_issued_execution(&store);
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let stale_generation = ScoringGeneration::INITIAL;
    let current_generation = ScoringGeneration::new(2).expect("next generation");
    let job = crate::JobId::from_uuid(Uuid::from_u128(75_111));
    {
        let mut state = store.write_state().expect("memory state");
        let now = state.authoritative_time;
        state.assignment_scoring.insert(
            (tenant, assignment),
            (current_generation, ScoringStatus::Recalculating),
        );
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: JobPayload::RecalculateAssignment {
                    assignment,
                    generation: stale_generation,
                },
                state: JobState::Ready,
                available_at: now,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 3,
                failure: None,
            },
        );
    }
    let claim = store
        .claim_exact_job(
            job,
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("stale generation remains claimable");
    let command = AssignmentScoringWorkerCommand {
        job,
        lease: claim.lease_token,
        assignment,
        generation: stale_generation,
    };

    assert_eq!(
        store
            .prepare_assignment_scoring(TenantContext::from_authenticated_session(tenant), command,)
            .await
            .expect("valid stale claim has a terminal preparation outcome"),
        AssignmentScoringPreparationOutcome::Superseded
    );
    assert_eq!(
        store
            .commit_assignment_scoring(TenantContext::from_authenticated_session(tenant), command,)
            .await
            .expect("superseded scoring job retires normally"),
        crate::AssignmentScoringCommitOutcome::Superseded
    );
    assert_eq!(
        store.read_state().expect("completed state").jobs[&job].state,
        JobState::Completed
    );
}

#[tokio::test]
async fn terminal_scoring_failure_reopens_the_exact_recalculation_thread() {
    let store = MemoryStore::default();
    let (tenant, _actor, _attempt, _submission) = seed_complete_issued_execution(&store);
    let course = CourseId::from_uuid(Uuid::from_u128(75_003));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(75_004));
    let generation = ScoringGeneration::new(2).expect("next generation");
    let operation = GradingOperationReference::new(1).expect("positive reference");
    let job = crate::JobId::from_uuid(Uuid::from_u128(75_112));
    {
        let mut state = store.write_state().expect("memory state");
        let now = state.authoritative_time;
        state.assignment_scoring.insert(
            (tenant, assignment),
            (generation, ScoringStatus::Recalculating),
        );
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: JobPayload::RecalculateAssignment {
                    assignment,
                    generation,
                },
                state: JobState::Ready,
                available_at: now,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 1,
                failure: None,
            },
        );
        state.automated_grading_operations.insert(
            (tenant, operation),
            GradingOperation {
                tenant,
                course,
                assignment,
                reference: operation,
                target: GradingOperationTarget::AssignmentScoringGeneration {
                    requested_generation: generation,
                },
                reason: GradingOperationReason::InstructorRequestedRecalculation,
                state: GradingOperationState::ActionInProgress,
                revision: GradingOperationRevision::INITIAL,
                next_action: None,
            },
        );
    }
    let claim = store
        .claim_exact_job(
            job,
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("recalculation is queued");
    assert_eq!(
        store
            .fail_job(job, claim.lease_token, crate::JobFailureKind::Permanent)
            .await
            .expect("terminal failure"),
        crate::JobFailureDisposition::Dead
    );

    let state = store.read_state().expect("failed state");
    assert_eq!(
        state.assignment_scoring[&(tenant, assignment)],
        (generation, ScoringStatus::Failed)
    );
    let operation = state.automated_grading_operations[&(tenant, operation)];
    assert_eq!(operation.state, GradingOperationState::Actionable);
    assert_eq!(
        operation.next_action,
        Some(GradingOperationAction::Recalculate)
    );
}

#[tokio::test]
async fn mixed_version_result_evidence_preserves_the_active_execution_aggregate() {
    let store = MemoryStore::default();
    let (tenant, _actor, attempt, _submission) = seed_complete_issued_execution(&store);
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(Uuid::from_u128(75_015)),
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("issued execution is claimable");
    let result = AttemptResult {
        correct: true,
        points_earned: 2.0,
        points_possible: 2.0,
    };
    let mut evidence = canonical_attempt_result_json(result).expect("canonical result");
    evidence.canonical_json_version = evidence
        .canonical_json_version
        .checked_add(1)
        .expect("test version remains representable");
    let grade = AcceptedSubmissionGrade {
        evidence,
        feedback: FeedbackContent::default(),
    };
    let before = {
        let state = store.read_state().expect("claimed state");
        (
            state.attempts[&(tenant, attempt)].clone(),
            state.runs[&(tenant, RunId::from_uuid(Uuid::from_u128(75_006)))].clone(),
            state.summaries[&(tenant, EnrollmentId::from_uuid(Uuid::from_u128(75_005)))].clone(),
            state.enrollments[&(tenant, EnrollmentId::from_uuid(Uuid::from_u128(75_005)))].clone(),
            state.assignment_scoring[&(tenant, AssignmentId::from_uuid(Uuid::from_u128(75_004)))],
            state.automated_grading_executions[&(tenant, attempt)],
            state.jobs[&claim.job].state,
            state.jobs[&claim.job].available_at,
            state.jobs[&claim.job].lease_token.is_some(),
            state.jobs[&claim.job].lease_expires_at,
            state.jobs[&claim.job].attempt_count,
            state
                .automated_grading_result_evidence
                .get(&(tenant, attempt))
                .cloned(),
        )
    };

    assert!(matches!(
        store
            .commit_or_fail_accepted_submission_execution(
                TenantContext::from_authenticated_session(tenant),
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated { grade },
            )
            .await,
        Err(AcceptedSubmissionCommitError::Known(
            StoreError::InvalidRecord(_)
        ))
    ));

    let state = store.read_state().expect("rejected state");
    let after = (
        state.attempts[&(tenant, attempt)].clone(),
        state.runs[&(tenant, RunId::from_uuid(Uuid::from_u128(75_006)))].clone(),
        state.summaries[&(tenant, EnrollmentId::from_uuid(Uuid::from_u128(75_005)))].clone(),
        state.enrollments[&(tenant, EnrollmentId::from_uuid(Uuid::from_u128(75_005)))].clone(),
        state.assignment_scoring[&(tenant, AssignmentId::from_uuid(Uuid::from_u128(75_004)))],
        state.automated_grading_executions[&(tenant, attempt)],
        state.jobs[&claim.job].state,
        state.jobs[&claim.job].available_at,
        state.jobs[&claim.job].lease_token.is_some(),
        state.jobs[&claim.job].lease_expires_at,
        state.jobs[&claim.job].attempt_count,
        state
            .automated_grading_result_evidence
            .get(&(tenant, attempt))
            .cloned(),
    );
    assert_eq!(after, before);
    assert!(
        state.submissions[&(tenant, attempt)]
            .completed_record_opt()
            .is_none()
    );
    assert_eq!(
        state
            .jobs
            .values()
            .filter(|job| matches!(job.payload, JobPayload::RecalculateAssignment { .. }))
            .count(),
        0,
        "rejected evidence does not schedule score recalculation"
    );
    assert!(!state.attempt_scores.contains_key(&(tenant, attempt)));
}
