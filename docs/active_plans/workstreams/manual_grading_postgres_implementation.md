# Manual grading PostgreSQL implementation handoff

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Scope delivered

This slice implements `ManualGradingStore` for `PostgresStore` in the new
`crates/learning-data-access/src/postgres/manual_grading.rs` module. It keeps the existing
shared/Memory contract intact and confines parent-module changes to the
module hook and current-evaluation attempt-projection overlay.

## Persisted behavior

- A learner's response requiring manual grading atomically writes immutable
  response/idempotency evidence, an empty private feedback record, a
  `needs_manual_grading` attempt, and one mutable `submission_evaluation`
  row with null `NUMERIC` credit/correctness and revision 1.
- A response-less force-submit has no submission/evaluation row and cannot be
  manually graded.
- Instructor reads and mutations derive course authority from persisted
  membership and return `NotFound` for a foreign/non-instructor actor.
- `rust_decimal::Decimal` binds directly to PostgreSQL `NUMERIC`; the sole
  browser-compatible `f64` conversion remains the existing `AttemptResult`
  projection boundary. SQLx's documented mapping supports this binding.
- Manual grade actions take the assignment advisory lock before stable attempt,
  run, enrollment, and evaluation row locks. A minimal action receipt is
  serialized by its own advisory lock and verifies actor, attempt, expected
  revision, and request digest before exact replay.
- A grade or correction replaces the one current evaluation, increments its
  revision, derives correctness from exact `Decimal::ONE`, recomputes the
  current run score while preserving first completion time in both the payload
  and `assignment_run.completed_at` column, increments the
  assignment generation, marks recalculating, and enqueues one fenced worker
  job. It does not publish learner summaries or global/item statistics.
- The evaluation table also holds automatic grades. A graded evaluation is a
  manual correction target only when a prior manual-grade receipt proves this
  was a manual workflow; automatic-only evaluations are neither editable nor
  returned by the manual edit read.
- Receipt and audit writes contain no credit, response, rubric, prior grade,
  or result payload.

## Projection boundary

`submission_idempotency.payload` remains immutable pending evidence. The
attempt list, single-attempt read, run summary, automatic-submit completion
query, and manual score recomputation now join the current evaluation and
overlay its verified result only when it is graded. This prevents a later
automatic submission in a mixed run from treating an already manually graded
attempt as permanently pending.

## Evidence-view follow-up

The shared `get_manual_evaluation_for_edit` return type deliberately contains
only current evaluation state, not the learner response. This implementation
keeps that reviewed contract rather than expanding a shared API mid-slice.
The server lane should either compose its already-authorized attempt read with
this call or introduce a reviewed single-call instructor evidence view; that
is an explicit API-design follow-up, not a hidden client-side authority check.

## Validation

```text
cargo fmt --check
# passed

cargo check -p learning-data-access --features postgres
# passed

cargo test -p learning-data-access --features postgres
# 42 unit and 14 conformance tests passed

cargo clippy -p learning-data-access --features postgres --all-targets -- -D warnings
# passed

git diff --check
# passed
```

No live PostgreSQL container was started in this slice. The current focused
fixture has only one externally graded item, so it cannot honestly prove a
mixed automatic/manual PostgreSQL run. The integration owner must add that
two-item fixture and run the documented real-RLS transaction acceptance gate.
