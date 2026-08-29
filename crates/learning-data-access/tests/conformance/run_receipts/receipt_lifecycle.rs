use super::*;

/// Receipt state transferred from setup to terminal lifecycle checks.
pub(super) struct ReceiptLifecycleFixture {
    pub(super) fixture_offset: u128,
    pub(super) tenant: TenantId,
    pub(super) context: TenantContext,
    pub(super) publisher: UserId,
    pub(super) student_user: UserId,
    pub(super) unrelated_user: UserId,
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) grade_policy: GradePolicy,
    pub(super) run: AssignmentRun,
    pub(super) attempt: QuestionAttempt,
    pub(super) reservation: PrefetchedQuestionDescriptorV1,
    pub(super) reservation_private_execution: PrefetchedPrivateExecutionV1,
    pub(super) response: StudentResponse,
    pub(super) key: SubmissionIdempotencyKey,
    pub(super) submitted: learning_data_access::SubmissionRecord,
}

/// Completes a committed receipt, checks successor semantics, and returns the fixture.
pub(super) async fn complete_receipt_lifecycle<S>(
    store: &S,
    fixture: ReceiptLifecycleFixture,
) -> RunApiFixture
where
    S: Store
        + CatalogStore
        + CourseRosterStore
        + JobStore
        + AssignmentScoringWorkerStore
        + SessionStore,
{
    let ReceiptLifecycleFixture {
        fixture_offset,
        tenant,
        context,
        publisher,
        student_user,
        unrelated_user,
        workspace,
        problem,
        version,
        course,
        assignment,
        grade_policy,
        run,
        attempt,
        reservation,
        reservation_private_execution,
        response,
        key,
        submitted,
    } = fixture;

    let second_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                issued_question_snapshot: reservation.issued_question_snapshot.clone(),
                seed: 0,
                presentation_capability: reservation.presentation_capability,
                presentation: Some(reservation.presentation),
                presentation_snapshot: Some(reservation.presentation_snapshot.clone()),
                grading_envelope: Some(reservation.grading_envelope.clone()),
                native_execution_envelope_capability: reservation
                    .native_execution_envelope_capability,
                flat_grading: None,
                flat_grading_capability: reservation.flat_grading_capability,
                webwork_grading: None,
                webwork_grading_capability: reservation.webwork_grading_capability,
                qti_grading: None,
                qti_grading_capability: reservation.qti_grading_capability,
                parameter_hash: "ignored-by-prefetch".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
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
            .submission_next_attempt(
                context,
                student_user,
                StudentWorkRoutingBinding::new(course, assignment),
                attempt.id,
            )
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            receipt_next_attempt(&second_attempt)
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
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    reservation: reservation.clone(),
                    private_execution: reservation_private_execution.clone(),
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
                    binding: StudentWorkRoutingBinding::new(course, assignment),
                    attempt: QuestionAttemptId::from_uuid(uuid(416)),
                    run: run.id,
                    assignment_position: 1,
                    problem,
                    question_version: version,
                    issued_question_snapshot: reservation.issued_question_snapshot.clone(),
                    seed: 0,
                    presentation_capability: reservation.presentation_capability,
                    presentation: Some(reservation.presentation),
                    presentation_snapshot: Some(reservation.presentation_snapshot.clone()),
                    grading_envelope: Some(reservation.grading_envelope.clone()),
                    native_execution_envelope_capability: reservation
                        .native_execution_envelope_capability,
                    flat_grading: None,
                    flat_grading_capability: reservation.flat_grading_capability,
                    webwork_grading: None,
                    webwork_grading_capability: reservation.webwork_grading_capability,
                    qti_grading: None,
                    qti_grading_capability: reservation.qti_grading_capability,
                    parameter_hash: "ignored-by-prefetch".to_string(),
                    provenance: reservation.provenance.clone(),
                    webwork_replay: None,
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
                binding: StudentWorkRoutingBinding::new(course, assignment),
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
            .finalize_submission_next_attempt(
                context,
                unrelated_user,
                StudentWorkRoutingBinding::new(course, assignment),
                second_attempt.id,
                None,
            )
            .await,
        Err(StoreError::NotFound),
        "another course member cannot enumerate or finalize a student's pending receipt",
    );
    routing_binding::assert_successor_receipt_route_binding(
        store,
        routing_binding::SuccessorReceiptRouteFixture {
            context,
            student_user,
            course,
            assignment,
            first_attempt: &attempt,
            terminal_attempt: &second_attempt,
            run: run.id,
        },
    )
    .await;
    cross_run_finalization::assert_cross_run_finalization_guards(
        store,
        context,
        cross_run_finalization::CrossRunFinalizationFixture {
            student_user,
            course,
            assignment,
            version,
            problem,
            second_attempt: &second_attempt,
            response: &response,
            reservation: &reservation,
            first_attempt: &attempt,
        },
    )
    .await;
    terminal_receipt::assert_terminal_receipt_state(
        store,
        TerminalReceiptFixture {
            context,
            student_user,
            publisher,
            binding: StudentWorkRoutingBinding::new(course, assignment),
            run: &run,
            submitted: &submitted,
            completed: &completed,
            response: &response,
            key: &key,
            fixture_offset,
            grade_policy,
            second_attempt: &second_attempt,
            first_attempt: &attempt,
        },
    )
    .await;

    RunApiFixture {
        fixture_offset,
        tenant,
        context,
        publisher,
        student_user,
        workspace,
        problem,
        version,
        course,
        assignment,
        reservation,
        response,
        run: completed.run,
    }
}
