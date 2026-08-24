//! Focused Memory conformance for the dedicated WP-PROF-T4 rehearsal aggregate.

use super::*;
use domain::RehearsalPreDispatchAbandonReason;
#[cfg(feature = "test-support")]
use learning_data_access::PutAssignmentTeachingSettingsCommand;
#[cfg(feature = "test-support")]
use learning_data_access::in_memory::MemoryRehearsalIntegrityTestCorruption;
use learning_data_access::{
    AbandonRehearsalSubmissionBeforeDispatchCommand, AppendRehearsalFrozenItemCommand,
    ClaimRehearsalSubmissionCommand, CompleteRehearsalSubmissionCommand,
    MarkRehearsalSubmissionDispatchedCommand, RehearsalLocator,
    RehearsalPreDispatchCompensationStore, RehearsalStore, RehearsalSubmissionClaimResult,
    RehearsalSubmissionIdempotencyKey, StartRehearsalCommand,
};
#[cfg(feature = "test-support")]
use question_model::RehearsalEvidenceDigest;
use question_model::answer::NumericTolerance;
use question_model::{
    AttemptResult, CourseLocalDateTime, DisclosedFeedback, IanaTimeZone, PreviewSelectedMoment,
    PreviewSyntheticGroupReferences, RehearsalAttemptId, RehearsalFrozenItemEvidence,
    RehearsalLifecycle, RehearsalPrivateGradingResult, RehearsalSubjectStart,
    RehearsalSyntheticSubjectRequest, ResponseDefinition, StudentResponse,
    SyntheticPreviewModifiers, TeachingAttemptLimitFieldPatch, TeachingLimitFieldPatch,
    TeachingOperationRevision, TeachingTimeFieldPatch,
};

fn deterministic_grade() -> RehearsalPrivateGradingResult {
    RehearsalPrivateGradingResult::Graded {
        result: AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        },
        feedback: DisclosedFeedback::empty(),
        backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
            "native:memory-test".into(),
        )
        .expect("valid deterministic rehearsal receipt"),
    }
}

#[path = "rehearsal_memory/derived.rs"]
mod derived;
#[path = "rehearsal_memory/integrity.rs"]
mod integrity;
#[path = "rehearsal_memory/lifecycle.rs"]
mod lifecycle;
#[path = "rehearsal_memory/response_shapes.rs"]
mod response_shapes;
#[path = "rehearsal_memory/retention.rs"]
mod retention;

fn synthetic_start() -> RehearsalSubjectStart {
    RehearsalSubjectStart::Synthetic {
        request: RehearsalSyntheticSubjectRequest {
            selected_moment: PreviewSelectedMoment {
                value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
            },
            groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("groups"),
            modifiers: SyntheticPreviewModifiers {
                mode: question_model::PolicyModificationModeView::ExtendOnly,
                patch: question_model::PolicyPatchView {
                    available_at: TeachingTimeFieldPatch::Inherit,
                    due_at: TeachingTimeFieldPatch::Inherit,
                    closes_at: TeachingTimeFieldPatch::Inherit,
                    time_limit_seconds: TeachingLimitFieldPatch::Inherit,
                    attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
                },
            },
        },
    }
}

async fn start_and_freeze(
    store: &MemoryStore,
) -> (
    effective_policy::EffectivePolicyFixture,
    RehearsalLocator,
    RehearsalFrozenItemEvidence,
) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("lookup")
        .expect("assignment");
    let revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("record")
            .expect("assignment")
            .revision
            .value(),
    )
    .expect("revision");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject: synthetic_start(),
                start_new_after_completion: false,
            },
        )
        .await
        .expect("start");
    let locator = RehearsalLocator {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        revision,
        rehearsal: receipt.rehearsal,
    };
    let assignment_record = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("record")
        .expect("assignment");
    let frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(850_001)),
        problem: assignment_record.record.items[0].reference,
        response_definition: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        },
        canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes([8; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(500),
    };
    store
        .append_rehearsal_frozen_item(
            fixture.context,
            AppendRehearsalFrozenItemCommand {
                locator,
                frozen: frozen.clone(),
            },
        )
        .await
        .expect("freeze");
    (fixture, locator, frozen)
}

#[cfg(feature = "test-support")]
async fn complete_submission(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    locator: RehearsalLocator,
    frozen: &RehearsalFrozenItemEvidence,
    idempotency_key: &str,
) {
    let key = RehearsalSubmissionIdempotencyKey::new(idempotency_key.into()).expect("key");
    let claimed = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: key,
            },
        )
        .await
        .expect("claim");
    let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
        panic!("fixture submission must claim");
    };
    let dispatched = store
        .mark_rehearsal_submission_dispatched(
            fixture.context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator,
                handle: claimed.handle,
            },
        )
        .await
        .expect("dispatch");
    store
        .complete_rehearsal_submission(
            fixture.context,
            CompleteRehearsalSubmissionCommand {
                locator,
                handle: dispatched,
                grading: deterministic_grade(),
            },
        )
        .await
        .expect("complete");
}

#[cfg(feature = "test-support")]
async fn assert_semantic_corruption_refuses_all_mutations(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    locator: RehearsalLocator,
) {
    let before_assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment read")
        .expect("assignment");
    let before_policy = store
        .get_base_assignment_policy(fixture.context, fixture.assignment)
        .await
        .expect("policy read")
        .expect("policy");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    assert!(
        store
            .start_rehearsal(
                fixture.context,
                StartRehearsalCommand {
                    actor: locator.actor,
                    course: locator.course,
                    assignment: locator.assignment,
                    revision: locator.revision,
                    subject: synthetic_start(),
                    start_new_after_completion: false,
                },
            )
            .await
            .is_err()
    );
    assert!(
        store
            .discard_rehearsal(fixture.context, locator)
            .await
            .is_err()
    );
    assert!(
        store
            .complete_rehearsal(fixture.context, locator)
            .await
            .is_err()
    );
    assert!(
        store
            .replace_assignment_fixed_item(
                fixture.context,
                learning_data_access::ReplaceAssignmentFixedItemCommand {
                    actor: fixture.instructor,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    current_item: before_assignment.record.items[0].id,
                    expected_revision: before_assignment.revision,
                    replacement: before_assignment.record.items[0].reference,
                },
            )
            .await
            .is_err()
    );
    assert!(
        store
            .put_assignment_teaching_settings(
                fixture.context,
                PutAssignmentTeachingSettingsCommand {
                    actor: fixture.instructor,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    expected_revision: before_policy.revision,
                    settings: question_model::AssignmentTeachingSettings {
                        lifecycle: before_assignment.record.lifecycle,
                        instructions: before_assignment.record.instructions.clone(),
                        base_policy: before_policy.policy,
                    },
                },
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment read"),
        Some(before_assignment),
    );
    assert_eq!(
        store
            .get_base_assignment_policy(fixture.context, fixture.assignment)
            .await
            .expect("policy read"),
        Some(before_policy),
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .is_unchanged_from(&before)
    );
}

#[tokio::test]
async fn memory_rehearsal_claim_is_pre_grade_idempotent_and_isolated() {
    let store = MemoryStore::default();
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(&store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("lookup")
        .expect("assignment");
    let revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("record")
            .expect("assignment")
            .revision
            .value(),
    )
    .expect("revision");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject: synthetic_start(),
                start_new_after_completion: false,
            },
        )
        .await
        .expect("Store resolves T3 subject itself");
    assert_eq!(receipt.lifecycle, RehearsalLifecycle::Active);
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .has_only_rehearsal_effects_from(&before)
    );
    let locator = RehearsalLocator {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        revision,
        rehearsal: receipt.rehearsal,
    };
    let assignment_record = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("record")
        .expect("assignment");
    let frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(800_001)),
        problem: assignment_record.record.items[0].reference,
        response_definition: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        },
        canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes([7; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(500),
    };
    store
        .append_rehearsal_frozen_item(
            fixture.context,
            AppendRehearsalFrozenItemCommand {
                locator,
                frozen: frozen.clone(),
            },
        )
        .await
        .expect("freeze");
    let key = RehearsalSubmissionIdempotencyKey::new("first".into()).expect("key");
    let claimed = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("claim");
    let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
        panic!("new request must claim");
    };
    assert!(matches!(
        store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: key.clone()
                }
            )
            .await
            .expect("pending"),
        RehearsalSubmissionClaimResult::Pending
    ));
    assert!(matches!(
        store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 4.0 },
                    idempotency_key: key.clone()
                }
            )
            .await
            .expect("conflict"),
        RehearsalSubmissionClaimResult::Conflict
    ));
    store
        .abandon_rehearsal_submission_before_dispatch(
            fixture.context,
            AbandonRehearsalSubmissionBeforeDispatchCommand {
                locator,
                handle: claimed.handle,
                reason: RehearsalPreDispatchAbandonReason::LocalPreparationFailed,
            },
        )
        .await
        .expect("only a definite pre-dispatch failure can abandon");
    let reclaimed = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("abandoned pre-dispatch claim creates the next generation");
    let RehearsalSubmissionClaimResult::Claimed(claimed) = reclaimed else {
        panic!("reclaimed request must receive a fresh prepared handle");
    };
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("clock");
    let dispatched = store
        .mark_rehearsal_submission_dispatched(
            fixture.context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator,
                handle: claimed.handle,
            },
        )
        .await
        .expect("commit dispatch before grade");
    let accepted = store
        .complete_rehearsal_submission(
            fixture.context,
            CompleteRehearsalSubmissionCommand {
                locator,
                handle: dispatched,
                grading: deterministic_grade(),
            },
        )
        .await
        .expect("complete once");
    assert!(!accepted.replayed);
    let replay = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: key,
            },
        )
        .await
        .expect("replay");
    let RehearsalSubmissionClaimResult::Replay(replay) = replay else {
        panic!("completed request replays its one durable receipt");
    };
    assert_eq!(replay.outcome, accepted.outcome);
    assert_eq!(
        store
            .read_rehearsal(
                fixture.context,
                RehearsalLocator {
                    actor: UserId::from_uuid(uuid(800_002)),
                    ..locator
                }
            )
            .await,
        Err(StoreError::NotFound)
    );
}
