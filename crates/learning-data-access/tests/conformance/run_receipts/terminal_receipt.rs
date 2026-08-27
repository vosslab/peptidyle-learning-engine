use super::*;

use learning_data_access::SubmissionRecord;

/// Inputs for the terminal receipt and summary projection checks.
pub(super) struct TerminalReceiptFixture<'a> {
    pub(super) context: TenantContext,
    pub(super) student_user: UserId,
    pub(super) publisher: UserId,
    pub(super) binding: LearnerWorkRoutingBinding,
    pub(super) run: &'a AssignmentRun,
    pub(super) submitted: &'a SubmissionRecord,
    pub(super) completed: &'a SubmissionRecord,
    pub(super) response: &'a StudentResponse,
    pub(super) key: &'a SubmissionIdempotencyKey,
    pub(super) fixture_offset: u128,
    pub(super) second_attempt: &'a QuestionAttempt,
    pub(super) first_attempt: &'a QuestionAttempt,
}

pub(super) async fn assert_terminal_receipt_state<S>(store: &S, fixture: TerminalReceiptFixture<'_>)
where
    S: Store,
{
    let TerminalReceiptFixture {
        context,
        student_user,
        publisher,
        binding,
        run,
        submitted,
        completed,
        response,
        key,
        fixture_offset,
        second_attempt,
        first_attempt,
    } = fixture;

    assert_eq!(
        store
            .submission_next_attempt(context, student_user, binding, first_attempt.id)
            .await,
        Ok(learning_data_access::SubmissionNextAttempt::Issued(
            receipt_next_attempt(second_attempt)
        )),
        "the first receipt keeps its original successor after that successor is submitted",
    );
    assert_eq!(
        (
            completed.summary.completed_run_count,
            completed.summary.total_question_attempts,
            completed.summary.current_score,
        ),
        (1, 2, Some(1.0))
    );
    let replay_after_completion = store
        .replay_submission(context, student_user, first_attempt.id, response, key)
        .await
        .expect("first submission replay after later completion");
    let learning_data_access::SubmissionReceiptRead::Completed(replay_after_completion) =
        replay_after_completion
    else {
        panic!("first submission receipt remains available");
    };
    assert_eq!(replay_after_completion.attempt, submitted.attempt);
    assert_eq!(replay_after_completion.run, submitted.run);
    assert_eq!(replay_after_completion.summary, submitted.summary);
    assert!(replay_after_completion.feedback == submitted.feedback);
    let attempt_page = store
        .list_question_attempts(
            context,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("attempt page");
    assert_eq!(
        attempt_page.items,
        vec![submitted.attempt.clone(), completed.attempt.clone()]
    );
    let first_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::first(PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary page");
    assert_eq!(first_summary_page.run, completed.run);
    // The receipt retains the summary observed when it committed. The
    // enrollment summary is live and has since observed the deliberately
    // completed independent recovery fixture run above.
    assert_eq!(first_summary_page.summary.completed_run_count, 2);
    assert_eq!(first_summary_page.summary.total_question_attempts, 4);
    assert!(first_summary_page.practice_allowed);
    assert_eq!(first_summary_page.outcomes.items.len(), 1);
    assert!(first_summary_page.outcomes.items[0].response.is_some());
    assert!(first_summary_page.outcomes.items[0].feedback.is_some());
    let continuation = first_summary_page
        .outcomes
        .next_cursor
        .expect("two outcomes require a cursor");
    let second_summary_page = store
        .get_run_summary_page(
            context,
            student_user,
            run.id,
            PageRequest::after(continuation, PageSize::new(1).expect("valid bounded page")),
        )
        .await
        .expect("owner summary continuation");
    assert_eq!(second_summary_page.outcomes.items.len(), 1);
    assert_ne!(
        first_summary_page.outcomes.items[0].attempt, second_summary_page.outcomes.items[0].attempt,
        "keyset pages must not duplicate outcomes"
    );
    assert_eq!(second_summary_page.outcomes.next_cursor, None);
    let instructor_summary = store
        .get_run_summary_page(
            context,
            publisher,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("direct course instructor summary");
    assert_eq!(instructor_summary.outcomes.items.len(), 2);
    let foreign_actor = UserId::from_uuid(uuid(99_999 + fixture_offset));
    assert!(matches!(
        store
            .get_run_summary_page(
                context,
                foreign_actor,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .get_run_summary_page(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(
                    99_998 + fixture_offset,
                ))),
                student_user,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid bounded page")),
            )
            .await,
        Err(StoreError::NotFound)
    ));
}
