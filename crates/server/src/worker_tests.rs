use std::{collections::VecDeque, sync::Mutex};

use super::*;
use async_trait::async_trait;
use learning_data_access::{
    StudentAssessmentAttemptFinalizationKind, StudentAssessmentAttemptFinalizationPreparation,
};

#[derive(Default)]
struct FakeExpiryStore {
    commit_results: Mutex<VecDeque<Result<(), StoreError>>>,
    committed_attempts: Mutex<Vec<uuid::Uuid>>,
    watch_results: Mutex<VecDeque<Result<u32, StoreError>>>,
    watch_attempts: Mutex<u32>,
}

#[async_trait]
impl AssessmentAttemptExpirySweepStore for FakeExpiryStore {
    async fn prepare_expired_assessment_attempt_finalizations(
        &self,
        _limit: u32,
    ) -> Result<Vec<ExpiredAssessmentAttemptFinalizationPreparation>, StoreError> {
        Ok(Vec::new())
    }

    async fn commit_expired_assessment_attempt_finalization(
        &self,
        assessment_attempt_id: uuid::Uuid,
        _preparation: StudentAssessmentAttemptFinalizationPreparation,
        _evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError> {
        self.committed_attempts
            .lock()
            .expect("committed attempts")
            .push(assessment_attempt_id);
        self.commit_results
            .lock()
            .expect("commit results")
            .pop_front()
            .unwrap_or(Ok(()))
    }
}

#[async_trait]
impl LibraryWatchNotificationStore for FakeExpiryStore {
    async fn materialize_library_watch_notifications(
        &self,
        _limit: u16,
    ) -> Result<u32, StoreError> {
        *self.watch_attempts.lock().expect("watch attempts") += 1;
        self.watch_results
            .lock()
            .expect("watch results")
            .pop_front()
            .unwrap_or(Ok(0))
    }
}

fn prepared_attempt(value: u128) -> ExpiredAssessmentAttemptFinalizationPreparation {
    ExpiredAssessmentAttemptFinalizationPreparation {
        assessment_attempt_id: uuid::Uuid::from_u128(value),
        preparation: StudentAssessmentAttemptFinalizationPreparation {
            kind: StudentAssessmentAttemptFinalizationKind::Deadline,
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
        vec![first.assessment_attempt_id, second.assessment_attempt_id]
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

#[tokio::test]
async fn failed_watch_materialization_does_not_block_the_next_retry() {
    let store = FakeExpiryStore::default();
    store.watch_results.lock().expect("watch results").extend([
        Err(StoreError::Unavailable(
            "database transiently unavailable".to_owned(),
        )),
        Ok(1),
    ]);

    materialize_library_watch_notifications_iteration(&store).await;
    materialize_library_watch_notifications_iteration(&store).await;

    assert_eq!(*store.watch_attempts.lock().expect("watch attempts"), 2);
}
