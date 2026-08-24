use super::*;

/// Proves that successor-receipt projections and mutations remain bound to
/// their explicit course/assignment route and conceal mismatches without
/// changing the immutable receipt.
pub(super) struct SuccessorReceiptRouteFixture<'a> {
    pub(super) context: TenantContext,
    pub(super) student_user: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) first_attempt: &'a QuestionAttempt,
    pub(super) terminal_attempt: &'a QuestionAttempt,
    pub(super) run: RunId,
}

pub(super) async fn assert_successor_receipt_route_binding<S>(
    store: &S,
    fixture: SuccessorReceiptRouteFixture<'_>,
) where
    S: Store,
{
    let SuccessorReceiptRouteFixture {
        context,
        student_user,
        course,
        assignment,
        first_attempt,
        terminal_attempt,
        run,
    } = fixture;
    let wrong_bindings = [
        LearnerWorkRoutingBinding::new(course, AssignmentId::from_uuid(uuid(88_001))),
        LearnerWorkRoutingBinding::new(CourseId::from_uuid(uuid(88_002)), assignment),
    ];
    for wrong_binding in wrong_bindings {
        assert_eq!(
            store
                .submission_next_attempt(context, student_user, wrong_binding, first_attempt.id,)
                .await,
            Err(StoreError::NotFound),
            "a mismatched route cannot project a successor receipt",
        );
        assert_eq!(
            store
                .finalize_submission_next_attempt(
                    context,
                    student_user,
                    wrong_binding,
                    terminal_attempt.id,
                    None,
                )
                .await,
            Err(StoreError::NotFound),
            "a mismatched route cannot finalize a successor receipt",
        );
    }
    assert_eq!(
        store
            .submission_next_attempt(
                context,
                student_user,
                LearnerWorkRoutingBinding::new(course, assignment),
                first_attempt.id,
            )
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            receipt_next_attempt(terminal_attempt)
        )),
        "mismatched projections do not alter the issued successor receipt",
    );
    assert_eq!(
        store
            .pending_submission_for_run(context, student_user, run)
            .await,
        Ok(Some(terminal_attempt.id)),
        "mismatched finalization does not consume the pending terminal receipt",
    );
}
