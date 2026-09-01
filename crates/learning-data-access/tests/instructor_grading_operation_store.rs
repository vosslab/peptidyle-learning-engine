use std::sync::Mutex;

use async_trait::async_trait;
use learning_data_access::{InstructorGradingOperationStore, StoreError};
use question_model::{
    GradingOperationAction, InstructorGradingOperationActionRequest,
    InstructorGradingOperationReceipt, InstructorGradingOperationReference,
    InstructorGradingOperationReplay, InstructorGradingOperationReplayLedger,
    InstructorGradingOperationRequestChecksum, InstructorGradingOperationRetryToken, Timestamp,
};

#[derive(Default)]
struct DeterministicGradingOperationStore {
    ledger: Mutex<InstructorGradingOperationReplayLedger>,
}

#[async_trait]
impl InstructorGradingOperationStore for DeterministicGradingOperationStore {
    async fn accept_or_replay_instructor_grading_operation(
        &self,
        request: InstructorGradingOperationActionRequest,
    ) -> Result<InstructorGradingOperationReplay, StoreError> {
        self.ledger
            .lock()
            .expect("test store lock")
            .accept_or_replay(request, |accepted_request| {
                InstructorGradingOperationReceipt::new(
                    accepted_request,
                    4,
                    None,
                    None,
                    Timestamp::from_unix_millis(1),
                )
            })
            .map_err(|_| StoreError::Conflict)
    }
}

fn request() -> InstructorGradingOperationActionRequest {
    InstructorGradingOperationActionRequest::new(
        InstructorGradingOperationReference::new(7).expect("operation"),
        GradingOperationAction::Retry,
        InstructorGradingOperationRequestChecksum::from_bytes([1; 32]),
        InstructorGradingOperationRetryToken::parse("00000000-0000-0000-0000-000000000001")
            .expect("retry token"),
    )
}

#[tokio::test]
async fn store_trait_replays_the_same_accepted_receipt_for_an_exact_request() {
    let store = DeterministicGradingOperationStore::default();
    let accepted = store
        .accept_or_replay_instructor_grading_operation(request())
        .await
        .expect("accepted request");
    let replayed = store
        .accept_or_replay_instructor_grading_operation(request())
        .await
        .expect("exact replay");

    assert_eq!(
        replayed,
        match accepted {
            InstructorGradingOperationReplay::Accepted(receipt) => {
                InstructorGradingOperationReplay::Replayed(receipt)
            }
            InstructorGradingOperationReplay::Replayed(_) => {
                panic!("first request must be accepted")
            }
        }
    );
}
