use super::*;

pub(super) async fn exercise_run_api_store<S>(store: &S, feedback_disclosure: FeedbackDisclosure)
where
    S: Store + CatalogStore + CourseRosterStore + JobStore + AssignmentScoringWorkerStore,
{
    let fixture = exercise_run_api_receipts(store, feedback_disclosure).await;
    exercise_run_rescoring(store, &fixture).await;
    exercise_delete_and_regrade(store, &fixture).await;
    exercise_attempt_support(store, &fixture).await;
    exercise_run_summary_scale(store, &fixture).await;
}
