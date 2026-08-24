use super::claims_tests::{
    claim, claim_root, event, fingerprint, frozen, operation, proof, rehearsal_context, request,
};
use super::*;

#[test]
fn completed_claim_replay_precedes_revision_and_lifecycle_gates() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let generation = RehearsalClaimGeneration::first();
    let proof = proof(context, &root);
    let completed = hydrate_claim_history(
        &root,
        &[
            event(
                &root,
                1,
                operation(11),
                generation,
                RehearsalSubmissionClaimPhase::Prepared,
            ),
            event(
                &root,
                2,
                operation(11),
                generation,
                RehearsalSubmissionClaimPhase::GradingDispatched,
            ),
            root.restore_transition(
                3,
                operation(11),
                generation,
                RehearsalSubmissionClaimPhase::Completed,
                question_model::ActivityTimestamp::from_unix_millis(3),
                None,
                Some(proof.completion_material()),
            ),
        ],
        Some(proof),
    )
    .unwrap();
    for (lifecycle, current) in [
        (RehearsalLifecycle::Completed, true),
        (RehearsalLifecycle::Completed, false),
    ] {
        assert!(matches!(
            decide_submission_claim(
                lifecycle,
                current,
                Some(&completed),
                root.fingerprint(),
                &root,
                operation(12)
            ),
            RehearsalSubmissionClaimDecision::Replay { .. }
        ));
        assert!(matches!(
            decide_submission_claim(
                lifecycle,
                current,
                Some(&completed),
                fingerprint(99),
                &root,
                operation(12)
            ),
            RehearsalSubmissionClaimDecision::Conflict
        ));
    }
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Completed,
            true,
            None,
            root.fingerprint(),
            &root,
            operation(12)
        ),
        RehearsalSubmissionClaimDecision::TerminalLifecycle
    ));
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Active,
            false,
            None,
            root.fingerprint(),
            &root,
            operation(12)
        ),
        RehearsalSubmissionClaimDecision::StaleRevision
    ));
}
