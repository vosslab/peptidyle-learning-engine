use super::*;

#[path = "grading_operation_store_tests.rs"]
mod grading_operation_store_tests;
use crate::{AcceptedSubmission, AcceptedSubmissionId};
use uuid::Uuid;

#[test]
fn accepted_pending_submission_has_no_fabricated_completed_receipt() {
    let tenant = question_model::TenantId::from_uuid(Uuid::from_u128(301));
    let submission = AcceptedSubmissionId::from_uuid(Uuid::from_u128(302));
    let stored = StoredSubmission {
        key: crate::SubmissionIdempotencyKey::parse("accepted-pending").expect("key"),
        state: StoredSubmissionState::AcceptedPending(AcceptedSubmission {
            tenant,
            course: CourseId::from_uuid(Uuid::from_u128(303)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(304)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(305)),
            submission,
            actor: UserId::from_uuid(Uuid::from_u128(306)),
            idempotency_key: crate::SubmissionIdempotencyKey::parse("accepted-pending")
                .expect("key"),
            request_sha256: objects::Sha256Digest::compute(b"accepted"),
            accepted_at: ActivityTimestamp::from_unix_millis(0),
        }),
    };
    assert!(stored.completed_record_opt().is_none());
    assert_eq!(
        stored
            .accepted_pending()
            .map(|accepted| accepted.submission),
        Some(submission)
    );
    let debug = format!("{stored:?}");
    assert!(!debug.contains("accepted-pending"));
    assert!(!debug.contains("response"));
}

#[test]
fn private_response_identity_requires_canonical_text_digest_and_typed_value() {
    let tenant = question_model::TenantId::from_uuid(Uuid::from_u128(401));
    let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(402));
    let response = question_model::StudentResponse::Numeric { value: 88.0 };
    let private =
        StoredPrivateSubmissionResponse::from_response(response.clone()).expect("response");
    let mut state = State::default();
    state
        .private_submission_responses
        .insert((tenant, attempt), private.clone());
    assert!(
        stored_submission_matches_response(&state, tenant, attempt, &response).expect("stored")
    );
    assert!(
        !stored_submission_matches_response(
            &state,
            tenant,
            attempt,
            &question_model::StudentResponse::Numeric { value: 89.0 },
        )
        .expect("stored")
    );
    let debug = format!("{private:?}");
    assert!(!debug.contains("88"));
    assert!(!debug.contains(&private.canonical_text));
    assert!(debug.contains("[SERVER-ONLY]"));
}
