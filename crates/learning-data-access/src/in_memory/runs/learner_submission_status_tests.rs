//! Deterministic route-witness and durable-status proof for Memory.

use super::completion_tests::seed_complete_issued_execution;
use crate::{
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionRecoveryClaimStore, AcceptedSubmissionGrade, GradingExecutionState,
    JobLeaseDuration, LearnerSubmissionStatusRead, LearnerSubmissionStatusStore,
    LearnerWorkRoutingBinding, StoreError, WorkerId, canonical_attempt_result_json,
};
use question_model::{
    AssignmentId, AttemptResult, CourseId, FeedbackContent, SubmissionEvaluationStatus, UserId,
};
use uuid::Uuid;

use super::*;

fn binding() -> LearnerWorkRoutingBinding {
    LearnerWorkRoutingBinding::new(
        CourseId::from_uuid(Uuid::from_u128(75_003)),
        AssignmentId::from_uuid(Uuid::from_u128(75_004)),
    )
}

fn accepts_status_capability(_: &dyn LearnerSubmissionStatusStore) {}

#[test]
fn learner_submission_status_capability_is_object_safe() {
    accepts_status_capability(&MemoryStore::default());
}

#[tokio::test]
async fn learner_submission_status_binds_the_exact_route_and_actor() {
    let store = MemoryStore::default();
    let (tenant, actor, attempt, _) = seed_complete_issued_execution(&store);
    let context = TenantContext::from_authenticated_session(tenant);

    assert!(matches!(
        store
            .learner_submission_status(context, actor, binding(), attempt)
            .await,
        Ok(LearnerSubmissionStatusRead::AcceptedPending(_))
    ));
    for wrong_binding in [
        LearnerWorkRoutingBinding::new(
            binding().course,
            AssignmentId::from_uuid(Uuid::from_u128(76001)),
        ),
        LearnerWorkRoutingBinding::new(
            CourseId::from_uuid(Uuid::from_u128(76002)),
            binding().assignment,
        ),
    ] {
        assert_eq!(
            store
                .learner_submission_status(context, actor, wrong_binding, attempt)
                .await,
            Err(StoreError::NotFound),
        );
    }
    assert_eq!(
        store
            .learner_submission_status(
                context,
                UserId::from_uuid(Uuid::from_u128(76003)),
                binding(),
                attempt,
            )
            .await,
        Err(StoreError::NotFound),
    );
}

#[tokio::test]
async fn learner_submission_status_projects_completed_attention_and_closed_contradictions() {
    let store = MemoryStore::default();
    let (tenant, actor, attempt, _) = seed_complete_issued_execution(&store);
    let context = TenantContext::from_authenticated_session(tenant);
    {
        let mut state = store.write_state().expect("injected Memory state");
        state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .expect("execution")
            .state = GradingExecutionState::Exception;
        state.automated_grading_evaluations.insert(
            (tenant, attempt),
            SubmissionEvaluationStatus::AutomatedException,
        );
    }
    assert!(matches!(
        store
            .learner_submission_status(context, actor, binding(), attempt)
            .await,
        Ok(LearnerSubmissionStatusRead::InstructorAttention(_))
    ));
    {
        let mut state = store
            .write_state()
            .expect("injected contradictory Memory state");
        state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .expect("execution")
            .state = GradingExecutionState::Ready;
    }
    assert!(matches!(
        store
            .learner_submission_status(context, actor, binding(), attempt)
            .await,
        Err(StoreError::Unavailable(_))
    ));

    let completed_store = MemoryStore::default();
    let (tenant, actor, attempt, _) = seed_complete_issued_execution(&completed_store);
    {
        let mut state = completed_store
            .write_state()
            .expect("completed fixture policy state");
        state
            .assignments
            .get_mut(&(tenant, binding().assignment))
            .expect("fixture assignment")
            .policies
            .completion = question_model::CompletionRequirement::AllCorrect;
    }
    let claim = completed_store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(Uuid::from_u128(76004)),
            JobLeaseDuration::from_seconds(30).expect("valid deterministic lease"),
        )
        .await
        .expect("claim read")
        .expect("pending execution claim");
    assert_eq!(
        completed_store
            .commit_or_fail_accepted_submission_execution(
                TenantContext::from_authenticated_session(tenant),
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated {
                    grade: AcceptedSubmissionGrade {
                        evidence: canonical_attempt_result_json(AttemptResult {
                            correct: false,
                            points_earned: 0.0,
                            points_possible: 2.0,
                        })
                        .expect("canonical result"),
                        feedback: FeedbackContent::default(),
                    },
                },
            )
            .await
            .expect("completed execution"),
        AcceptedSubmissionExecutionDisposition::Committed,
    );
    let completed_status = completed_store
        .learner_submission_status(
            TenantContext::from_authenticated_session(tenant),
            actor,
            binding(),
            attempt,
        )
        .await;
    assert!(
        matches!(
            completed_status,
            Ok(LearnerSubmissionStatusRead::Completed {
                next_pending: true,
                ..
            })
        ),
        "completed status: {completed_status:?}"
    );
}
