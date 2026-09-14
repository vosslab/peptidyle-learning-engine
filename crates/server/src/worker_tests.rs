use std::{collections::VecDeque, sync::Mutex};

use super::*;
use async_trait::async_trait;
use learning_data_access::{
    StudentAssignmentAttemptFinalizationKind, StudentAssignmentAttemptFinalizationPreparation,
};

#[derive(Default)]
struct FakeExpiryStore {
    commit_results: Mutex<VecDeque<Result<(), StoreError>>>,
    committed_attempts: Mutex<Vec<uuid::Uuid>>,
}

#[async_trait]
impl AssignmentAttemptExpirySweepStore for FakeExpiryStore {
    async fn prepare_expired_assignment_attempt_finalizations(
        &self,
        _limit: u32,
    ) -> Result<Vec<ExpiredAssignmentAttemptFinalizationPreparation>, StoreError> {
        Ok(Vec::new())
    }

    async fn commit_expired_assignment_attempt_finalization(
        &self,
        assignment_attempt_id: uuid::Uuid,
        _preparation: StudentAssignmentAttemptFinalizationPreparation,
        _evaluations: Vec<StudentAssignmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError> {
        self.committed_attempts
            .lock()
            .expect("committed attempts")
            .push(assignment_attempt_id);
        self.commit_results
            .lock()
            .expect("commit results")
            .pop_front()
            .unwrap_or(Ok(()))
    }
}

fn prepared_attempt(value: u128) -> ExpiredAssignmentAttemptFinalizationPreparation {
    ExpiredAssignmentAttemptFinalizationPreparation {
        assignment_attempt_id: uuid::Uuid::from_u128(value),
        preparation: StudentAssignmentAttemptFinalizationPreparation {
            kind: StudentAssignmentAttemptFinalizationKind::Deadline,
            saved_responses: Vec::new(),
        },
    }
}

#[tokio::test]
async fn failed_expiry_commit_does_not_block_later_prepared_attempts() {
    let store = FakeExpiryStore::default();
    store
        .commit_results
        .lock()
        .expect("commit results")
        .extend([
            Err(StoreError::Unavailable(
                "database transiently unavailable".to_string(),
            )),
            Ok(()),
        ]);
    let first = prepared_attempt(1);
    let second = prepared_attempt(2);

    finalize_prepared_expired_attempt(&store, first.clone(), Ok(Vec::new())).await;
    finalize_prepared_expired_attempt(&store, second.clone(), Ok(Vec::new())).await;

    assert_eq!(
        *store.committed_attempts.lock().expect("committed attempts"),
        vec![first.assignment_attempt_id, second.assignment_attempt_id]
    );
}

#[tokio::test]
async fn failed_expiry_evaluation_does_not_attempt_that_commit() {
    let store = FakeExpiryStore::default();

    finalize_prepared_expired_attempt(
        &store,
        prepared_attempt(3),
        Err(StoreError::Unavailable("backend unavailable".to_string())),
    )
    .await;

    assert!(
        store
            .committed_attempts
            .lock()
            .expect("committed attempts")
            .is_empty()
    );
}
