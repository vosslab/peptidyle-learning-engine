use question_model::{
    GradingOperationAction, InstructorGradingOperationActionRequest,
    InstructorGradingOperationReceipt, InstructorGradingOperationReference,
    InstructorGradingOperationReplayError, InstructorGradingOperationReplayRegistry,
    InstructorGradingOperationRequestChecksum, InstructorGradingOperationRetryToken, Timestamp,
};

fn request(
    operation: u64,
    action: GradingOperationAction,
    checksum_byte: u8,
) -> InstructorGradingOperationActionRequest {
    InstructorGradingOperationActionRequest::new(
        InstructorGradingOperationReference::new(operation).expect("operation"),
        action,
        InstructorGradingOperationRequestChecksum::from_bytes([checksum_byte; 32]),
        InstructorGradingOperationRetryToken::parse("00000000-0000-0000-0000-000000000001")
            .expect("retry token"),
    )
}

fn accept(
    replay_registry: &mut InstructorGradingOperationReplayRegistry,
    request: InstructorGradingOperationActionRequest,
) {
    replay_registry
        .accept_or_replay(request, |accepted_request| {
            InstructorGradingOperationReceipt::new(
                accepted_request,
                4,
                None,
                None,
                Timestamp::from_unix_millis(1),
            )
        })
        .expect("first acceptance");
}

#[test]
fn retry_token_refuses_a_different_operation() {
    let mut replay_registry = InstructorGradingOperationReplayRegistry::default();
    accept(
        &mut replay_registry,
        request(7, GradingOperationAction::Retry, 1),
    );

    let result = replay_registry
        .accept_or_replay(request(8, GradingOperationAction::Retry, 1), |_| {
            panic!("a changed operation must not run")
        });

    assert_eq!(
        result,
        Err(InstructorGradingOperationReplayError::BindingMismatch)
    );
}

#[test]
fn retry_token_refuses_a_different_request_checksum() {
    let mut replay_registry = InstructorGradingOperationReplayRegistry::default();
    accept(
        &mut replay_registry,
        request(7, GradingOperationAction::Retry, 1),
    );

    let result = replay_registry
        .accept_or_replay(request(7, GradingOperationAction::Retry, 2), |_| {
            panic!("a changed checksum must not run")
        });

    assert_eq!(
        result,
        Err(InstructorGradingOperationReplayError::BindingMismatch)
    );
}
