# Manual grading core implementation handoff

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Scope delivered

Implemented the native shared contract and `MemoryStore` behavior for one
response-bearing, current-only manual evaluation. This is intentionally not a
PostgreSQL, server-route, TypeScript, Wasm, retention, item-analysis, or
assignment-override implementation.

## Changed files

- `Cargo.toml`, `crates/learning-data-access/Cargo.toml`, and `Cargo.lock`: add
  `rust_decimal` and enable SQLx's matching `rust_decimal` feature.
- `crates/grading/src/checker.rs`: a valid file upload now returns
  `GradeOutcome::NeedsManualGrading` rather than a grading error.
- `crates/learning-data-access/src/lib.rs`: adds the server-only `ManualGradingStore`, typed
  pending-submission/manual-grade commands, action identity, positive
  evaluation revision, current-evaluation record/status, and exact
  `ManualCredit`.
- `crates/learning-data-access/src/manual_grading.rs`: owns the public manual-grading types,
  exact-decimal validation, minimal replay receipt, and Store trait.
- `crates/learning-data-access/src/in_memory/manual_grading.rs`: implements response-backed pending submission,
  direct-instructor evaluation read/write, exact action replay, revision CAS,
  current-only result replacement, and generation-fenced recalculation jobs.
- `crates/learning-data-access/tests/conformance/manual_grading.rs`: contains deterministic
  MemoryStore coverage for pending/manual/correction/replay/authorization/
  force-submit/stale-worker behavior.

## Decimal boundary

`ManualCredit` is an exact `rust_decimal::Decimal`, accepts at most 12
fractional digits, rejects values outside `[-1000, 1000]`, normalizes signed
zero and trailing zeroes, and retains a canonical decimal string. It converts
to `f64` only for the existing browser-safe `AttemptResult` projection, with a
checked finite conversion. This matches SQLx's PostgreSQL type mapping for
`rust_decimal::Decimal` and `NUMERIC`: <https://docs.rs/sqlx/latest/sqlx/postgres/types/index.html>.

## Behavior proven

- A valid manual-required response persists as `NeedsManualGrading` with no
  result, run score, or current summary score.
- A persisted direct instructor can read and grade only the response-bearing
  pending evaluation. A learner/foreign tenant receives the ordinary
  non-enumerating `NotFound`; response-less force-submit is a conflict.
- Exact response/key and action retries return their first receipt; changed
  action payload and stale revision conflict.
- A replay receipt contains only action/attempt/resulting revision/scoring
  generation/time. Memory's private receipt retains an action request digest,
  not a credit, result, response, rubric, or prior evaluation; replay after a
  later correction still returns the first minimal receipt.
- The authorized current evaluation retains exact `ManualCredit`, including a
  tested twelve-fractional-digit decimal; the browser-safe attempt projection
  is the only `f64` conversion boundary.
- A correction replaces only the current manual result, advances the scoring
  generation, and supersedes already staged scoring work. The final manual
  result recomputes the current run score while retaining its original
  completion time, but enrollment and learner-summary projection remain
  pending until the scoring worker commits that generation.
- Existing clear logic observes the now-projected manual result and therefore
  retains its existing recalculation behavior.
- Manual/mixed anonymous statistics contribution is deliberately deferred.
  Publishing it in the mutation would make a later correction leave an
  obsolete aggregate, and it is not generation-fenced. The next analytics/item
  analysis contract must consume only the committed current generation.

## Commands run

```text
cargo test -p grading
# 6 passed

cargo test -p learning-data-access --no-default-features
# 38 unit tests and 14 conformance tests passed

cargo test -p learning-data-access --test conformance \
  memory_manual_grading_is_response_bearing_revisioned_and_generation_fenced \
  --no-default-features
# 1 passed

cargo check -p learning-data-access --features postgres
# passed

cargo clippy -p learning-data-access --no-default-features --all-targets -- -D warnings
# passed

cargo fmt --check
git diff --check
# passed
```

## PostgreSQL and server consumer notes

- PostgreSQL must implement `ManualGradingStore` using one mutable
  `submission_evaluation` row plus the minimal manual action receipt described
  in `manual_grading_postgres_book_review.md`; no grade-history or manual
  assignment-override table is implied by this API.
- The pending submission stores real response/idempotency evidence but no
  result. PostgreSQL must model that as a nullable current result only for a
  response-bearing pending evaluation. Force-submit remains response-less and
  has no evaluation.
- The server should branch on `GradeOutcome::NeedsManualGrading`, call the new
  pending-submission method, and later send parsed `ManualCredit` only through
  an authenticated instructor route. It must not accept correctness, points,
  course, tenant, or assignment authority from the client.
- `docs/CONTRACTS.md` and `docs/CHANGELOG.md` remain for the package-level
  integration owner because the frozen consumer set is not yet atomically
  updated by the PostgreSQL/server lanes.
- The focused conformance fixture has a one-item externally graded run, so it
  cannot construct a genuine automatic-plus-manual run without a new
  two-item immutable-run setup. That mixed fixture remains required from the
  PostgreSQL/server integration lane; this slice does not weaken its current
  pending/worker-fence assertions to simulate one.

## Remaining risks

- The MemoryStore uses the same public `QuestionAttempt` projection as the
  rest of the application and keeps its manual current evaluation in a private
  map. PostgreSQL must preserve the same one-current-evaluation semantics and
  its RLS/retention/purge coverage.
- This focused slice proves a one-item manual completion and stale-generation
  fencing. The PostgreSQL/server integration must add the mixed automatic plus
  manual run fixture and real-RLS acceptance before package acceptance.
