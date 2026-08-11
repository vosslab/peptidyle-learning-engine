# WP-W2 native retry lifecycle review

## Verdict

ACCEPTED.

The source fix is correct: `ensure_active_questions` now passes the committed
predecessor into the retry issuance request. That lets either store write the
immutable predecessor-to-successor receipt in the same issuance transaction.
It addresses the observed `AllCorrect` retry break without changing grading,
assignment policy, or the learner-visible public payload.

The strengthened flat native route regression proves the grading order, issues
an independently listed retry, and verifies that the returned receipt names
that exact retry.

## Lifecycle review

| Path | Result | Evidence |
| --- | --- | --- |
| Initial issue | OK | Initial runs pass `None`; no predecessor receipt is created. |
| Wrong-answer retry | OK | `ensure_active_questions` passes `Some(predecessor)` to retry issuance, and the Memory and PostgreSQL issuers require a submitted same-run predecessor before recording its immutable successor. |
| Prefetch promotion | OK | Promotion already passes the prefetch predecessor, validates the durable reservation, and consumes it atomically. |
| Submission replay | OK | Replay re-enters `finish_submission`; a pending receipt heals using the original submitted attempt, and a finalized receipt returns its stored successor. |
| Concurrent healers | OK | Memory accepts only the same existing successor. PostgreSQL uses the primary key plus `ON CONFLICT DO NOTHING`, then verifies the stored ID matches. |

The Memory implementation rejects a different active successor or an explicit
terminal receipt. The PostgreSQL implementation locks the run before issuance,
verifies the predecessor belongs to that run and is submitted, and treats a
losing concurrent insert as valid only when its stored successor is identical.
This preserves replay/idempotency semantics and prevents a later active attempt
from being attached to the wrong receipt.

## Regression adequacy

`flat_run_route_retries_wrong_first_source_choice_then_completes_correct_second_choice`
now independently lists `second_attempt` and asserts the receipt identity:

```rust
assert_eq!(
    wrong_receipt.pointer("/nextIssued/id"),
    Some(&serde_json::json!(second_attempt)),
    "wrong attempt receives a successor under unlimited AllCorrect policy"
);
```

This is the regression-specific proof that the repaired predecessor link is
returned to the learner. The existing generic prefetch/replay tests cover
reservation reuse, concurrent prefetch, replay immutability, and crash
recovery.

## Privacy review

The route test uses only fixture choice IDs and asserts boolean correctness.
It does not log a private grading key or inspect a private payload. The generic
prefetch test additionally rejects answer, key, provider, and provenance terms
from its public projection. The requested receipt-ID comparison is public
attempt metadata and does not expand disclosure.

## Validation

- PASS: `cargo fmt --check`
- PASS: `cargo check -p server_core`
- PASS: `cargo clippy -p server_core --tests -- -D warnings`
- PASS: `cargo test -p server_core native_backend::tests::flat_run_route_retries_wrong_first_source_choice_then_completes_correct_second_choice -- --exact`
- PASS: `cargo test -p server_core run::tests::prefetch::prefetch_is_body_free_idempotent_and_binds_the_submission_replay -- --exact`
- PASS: `cargo test -p server_core run::tests::prefetch::resumed_run_never_issues_an_unlinked_successor_before_submission_replay_heals -- --exact`
- PASS: `git diff --check`

No production or test source was changed by this review. PostgreSQL behavior
was reviewed from the transactional issuer and the shared receipt conformance
contract; this review did not run a local PostgreSQL service.
