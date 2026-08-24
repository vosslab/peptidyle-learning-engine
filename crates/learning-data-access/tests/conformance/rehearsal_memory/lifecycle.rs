//! Lifecycle and no-trace conformance for the isolated rehearsal aggregate.

use super::*;

fn synthetic_start_at(value: &str) -> RehearsalSubjectStart {
    RehearsalSubjectStart::Synthetic {
        request: RehearsalSyntheticSubjectRequest {
            selected_moment: PreviewSelectedMoment {
                value: CourseLocalDateTime::parse(value).expect("course-local moment"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("course zone"),
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

async fn start_fixture(
    store: &MemoryStore,
) -> (
    effective_policy::EffectivePolicyFixture,
    question_model::AssignmentReference,
    TeachingOperationRevision,
) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment lookup")
        .expect("assignment reference");
    let revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment read")
            .expect("assignment")
            .revision
            .value(),
    )
    .expect("teaching revision");
    (fixture, assignment, revision)
}

fn command(
    fixture: &effective_policy::EffectivePolicyFixture,
    assignment: question_model::AssignmentReference,
    revision: TeachingOperationRevision,
    subject: RehearsalSubjectStart,
    start_new_after_completion: bool,
) -> StartRehearsalCommand {
    StartRehearsalCommand {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        revision,
        subject,
        start_new_after_completion,
    }
}

fn locator(
    fixture: &effective_policy::EffectivePolicyFixture,
    assignment: question_model::AssignmentReference,
    revision: TeachingOperationRevision,
    receipt: question_model::RehearsalRunReceipt,
) -> RehearsalLocator {
    RehearsalLocator {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        revision,
        rehearsal: receipt.rehearsal,
    }
}

#[tokio::test]
async fn start_resume_subject_replacement_and_explicit_restart_preserve_ordinary_state() {
    let store = MemoryStore::default();
    let (fixture, assignment, revision) = start_fixture(&store).await;
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("baseline");
    let first = store
        .start_rehearsal(
            fixture.context,
            command(&fixture, assignment, revision, synthetic_start(), false),
        )
        .await
        .expect("start");
    let resumed = store
        .start_rehearsal(
            fixture.context,
            command(&fixture, assignment, revision, synthetic_start(), false),
        )
        .await
        .expect("resume");
    assert_eq!(resumed.rehearsal, first.rehearsal);
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("resumed fingerprint")
            .has_only_rehearsal_effects_from(&before)
    );

    let replacement = store
        .start_rehearsal(
            fixture.context,
            command(
                &fixture,
                assignment,
                revision,
                synthetic_start_at("2026-08-25T10:00:00.000"),
                false,
            ),
        )
        .await
        .expect("new subject replaces active rehearsal");
    assert_ne!(replacement.rehearsal, first.rehearsal);
    let first_locator = locator(&fixture, assignment, revision, first);
    assert_eq!(
        store
            .read_rehearsal(fixture.context, first_locator)
            .await
            .expect("discarded rehearsal remains privately readable")
            .lifecycle,
        RehearsalLifecycle::DiscardedByNewSubject
    );

    let replacement_locator = locator(&fixture, assignment, revision, replacement);
    store
        .complete_rehearsal(fixture.context, replacement_locator)
        .await
        .expect("complete active rehearsal");
    assert_eq!(
        store
            .start_rehearsal(
                fixture.context,
                command(&fixture, assignment, revision, synthetic_start(), false),
            )
            .await,
        Err(StoreError::Conflict)
    );
    let restarted = store
        .start_rehearsal(
            fixture.context,
            command(&fixture, assignment, revision, synthetic_start(), true),
        )
        .await
        .expect("explicit action starts after completion");
    assert_ne!(restarted.rehearsal, replacement_locator.rehearsal);
}

#[tokio::test]
async fn stale_revision_revokes_prepared_and_dispatched_claims_without_ordinary_rehearsal_trace() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let key = RehearsalSubmissionIdempotencyKey::new("stale-dispatch".into()).expect("key");
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
        .expect("prepared claim");
    let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
        panic!("new request is prepared");
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
    let assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment read")
        .expect("assignment");
    let changed = store
        .replace_assignment_fixed_item(
            fixture.context,
            learning_data_access::ReplaceAssignmentFixedItemCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                current_item: assignment.record.items[0].id,
                expected_revision: assignment.revision,
                replacement: assignment.record.items[0].reference,
            },
        )
        .await
        .expect("assignment revision atomically invalidates rehearsal");
    assert_ne!(changed.revision, assignment.revision);
    assert_eq!(
        store.read_rehearsal(fixture.context, locator).await,
        Err(StoreError::Conflict),
        "the stale route binding is refused before a prior-revision rehearsal can be read"
    );
    assert!(
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
            .is_err()
    );
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn stale_revision_records_the_closed_revocation_phase_for_prepared_and_dispatched_claims() {
    for dispatch in [false, true] {
        let store = MemoryStore::default();
        let (fixture, locator, frozen) = start_and_freeze(&store).await;
        let claimed = store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: RehearsalSubmissionIdempotencyKey::new(format!(
                        "stale-phase-{dispatch}"
                    ))
                    .expect("key"),
                },
            )
            .await
            .expect("prepared claim");
        let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
            panic!("new request is prepared");
        };
        if dispatch {
            store
                .mark_rehearsal_submission_dispatched(
                    fixture.context,
                    MarkRehearsalSubmissionDispatchedCommand {
                        locator,
                        handle: claimed.handle,
                    },
                )
                .await
                .expect("dispatch");
        }
        let assignment = store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment read")
            .expect("assignment");
        store
            .replace_assignment_fixed_item(
                fixture.context,
                learning_data_access::ReplaceAssignmentFixedItemCommand {
                    actor: fixture.instructor,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    current_item: assignment.record.items[0].id,
                    expected_revision: assignment.revision,
                    replacement: assignment.record.items[0].reference,
                },
            )
            .await
            .expect("revision invalidation");
        let snapshot = store
            .rehearsal_test_snapshot(fixture.context.tenant_id(), locator.rehearsal)
            .expect("read-only feature-gated snapshot");
        assert_eq!(
            snapshot.lifecycle,
            RehearsalLifecycle::DiscardedStaleRevision
        );
        assert!(snapshot.claims.iter().all(|claim| {
            claim.phase == domain::RehearsalSubmissionClaimPhase::RevokedStaleRevision
        }));
    }
}
