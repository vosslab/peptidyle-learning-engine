use super::*;

/// Proves that a terminal receipt cannot be linked across runs and that its
/// explicit no-successor state remains immutable.
pub(super) struct CrossRunFinalizationFixture<'a> {
    pub(super) student_user: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) version: VersionId,
    pub(super) problem: ProblemId,
    pub(super) second_attempt: &'a QuestionAttempt,
    pub(super) response: &'a StudentResponse,
    pub(super) reservation: &'a PrefetchedQuestionDescriptorV1,
    pub(super) first_attempt: &'a QuestionAttempt,
}

pub(super) async fn assert_cross_run_finalization_guards<S>(
    store: &S,
    context: TenantContext,
    fixture: CrossRunFinalizationFixture<'_>,
) where
    S: Store,
{
    let CrossRunFinalizationFixture {
        student_user,
        course,
        assignment,
        version,
        problem,
        second_attempt,
        response,
        reservation,
        first_attempt,
    } = fixture;
    let binding = StudentWorkRoutingBinding::new(course, assignment);
    let cross_run = store
        .start_or_resume_run(
            context,
            student_user,
            StudentWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(uuid(417)),
        )
        .await
        .expect("a completed run permits a new run");
    let (cross_run_presentation_binding, cross_run_presentation) =
        receipt_presentation(version, 994, 10);
    let cross_run_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(418)),
                run: cross_run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                issued_question_snapshot: reservation.issued_question_snapshot.clone(),
                seed: 994,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(cross_run_presentation_binding),
                presentation_snapshot: Some(cross_run_presentation.clone()),
                grading_envelope: Some(grading_envelope(version, 994)),
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "cross-run-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
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
                binding,
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
                binding: StudentWorkRoutingBinding::new(course, assignment),
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
    let (cross_run_second_presentation_binding, cross_run_second_presentation) =
        receipt_presentation(version, 995, 11);
    let cross_run_second = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                binding: StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(419)),
                run: cross_run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                issued_question_snapshot: reservation.issued_question_snapshot.clone(),
                seed: 995,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(cross_run_second_presentation_binding),
                presentation_snapshot: Some(cross_run_second_presentation.clone()),
                grading_envelope: Some(grading_envelope(version, 995)),
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "cross-run-second-parameter-hash".to_string(),
                provenance: reservation.provenance.clone(),
                webwork_replay: None,
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
                binding: StudentWorkRoutingBinding::new(course, assignment),
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
            .finalize_submission_next_attempt(
                context,
                student_user,
                binding,
                second_attempt.id,
                None,
            )
            .await,
        Ok(()),
        "a terminal submission records its explicit no-successor receipt state",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                binding,
                second_attempt.id,
                None,
            )
            .await,
        Ok(()),
        "the explicit no-successor receipt state is idempotent",
    );
    assert_eq!(
        store
            .finalize_submission_next_attempt(
                context,
                student_user,
                binding,
                second_attempt.id,
                Some(first_attempt.id),
            )
            .await,
        Err(StoreError::Conflict),
        "a finalized no-successor receipt cannot later point at an attempt",
    );
}
