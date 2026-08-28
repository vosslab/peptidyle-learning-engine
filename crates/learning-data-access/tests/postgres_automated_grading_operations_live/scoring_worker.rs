//! Production Store path for publishing one connected scoring generation.

use learning_data_access::{
    AssignmentScoringCommitOutcome, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, JobClaimFilter, JobKind,
    JobLeaseDuration, JobPayload, JobStore, TenantContext,
};
use question_model::{AssignmentId, ScoringGeneration};
use uuid::Uuid;

pub(super) async fn publish(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    assignment: AssignmentId,
    expected_job: Uuid,
    generation: i64,
) {
    let expected_generation = ScoringGeneration::new(
        u64::try_from(generation).expect("connected scoring generation is nonnegative"),
    )
    .expect("connected scoring generation is positive");
    let filter =
        JobClaimFilter::new([JobKind::RecalculateAssignment]).expect("scoring-only worker filter");
    let lease = JobLeaseDuration::from_seconds(300).expect("bounded scoring-worker lease");
    loop {
        let claimed = store
            .claim_next_job(&filter, lease)
            .await
            .expect("claim connected scoring through the production queue")
            .expect("expected connected scoring job remains ready");
        let JobPayload::RecalculateAssignment {
            assignment: queued_assignment,
            generation: queued_generation,
        } = claimed.payload
        else {
            unreachable!("the scoring-only queue filter returned another family")
        };
        let command = AssignmentScoringWorkerCommand {
            job: claimed.id,
            lease: claimed.lease_token,
            assignment: queued_assignment,
            generation: queued_generation,
        };
        let claimed_context = TenantContext::from_authenticated_session(claimed.tenant);
        let preparation = store
            .prepare_assignment_scoring(claimed_context, command)
            .await
            .expect("prepare connected scoring through the production Store path");
        let publication = store
            .commit_assignment_scoring(claimed_context, command)
            .await
            .expect("commit connected scoring through the production Store path");
        if claimed.id.as_uuid() == expected_job {
            assert_eq!(claimed_context, context);
            assert_eq!(queued_assignment, assignment);
            assert_eq!(queued_generation, expected_generation);
            assert_eq!(preparation, AssignmentScoringPreparationOutcome::Prepared);
            assert_eq!(publication, AssignmentScoringCommitOutcome::Committed);
            return;
        }
        assert!(matches!(
            (preparation, publication),
            (
                AssignmentScoringPreparationOutcome::Prepared,
                AssignmentScoringCommitOutcome::Committed
            ) | (
                AssignmentScoringPreparationOutcome::Superseded,
                AssignmentScoringCommitOutcome::Superseded
            )
        ));
    }
}
