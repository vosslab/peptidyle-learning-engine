# Manual grading core review

## Status

**RE-REVIEW: PASS for the corrected shared Rust/MemoryStore slice.** The
initial blockers below are retained as audit history. They are resolved in the
new modular implementation. The intentionally pending server/project-tools consumer
lane still leaves the workspace check red until it handles the new
`GradeOutcome` variant; that is an integration dependency, not a new core
blocker.

## Review scope

Reviewed only the requested core diff and
`manual_grading_core_implementation.md`, against the current database plan,
status handoff, and
`manual_grading_postgres_book_review.md`. The native target is the workspace
default host target; this is Store/Memory ownership and error-contract work,
not a Wasm change.

## Findings

1. **P0 - The manual action receipt retains prohibited grade history, and an
   idempotent replay exposes it after a later correction.**
   `MemoryManualGradeReceipt` retains both `ManualCredit` and a complete old
   `ManualEvaluationRecord` at
   `crates/learning-data-access/src/in_memory.rs:370-377`, then `set_memory_manual_grade`
   returns that old record for an exact replay at `:6640-6647` and stores it
   at `:6775-6783`. A correction replaces the current evaluation at
   `:6771-6774`, so replaying the first action afterwards returns a historical
   numeric result rather than the sole current evaluation. This directly
   conflicts with the book-grounded contract: a receipt may retain action
   identity, request digest, revisions, and scoring generation, but neither a
   past grade nor a response/rubric. It also means the Store trait's current
   return type (`ManualEvaluationRecord` at
   `crates/learning-data-access/src/lib.rs:3189-3197`) forces a PostgreSQL implementation to
   either retain a past grade or falsify replay.

   **Fix:** replace the return with a receipt-safe outcome containing at most
   the resulting evaluation revision and scoring generation. Store a canonical
   request digest plus actor, attempt, expected/resulting revisions, and
   generation in the Memory receipt; compare the digest on replay. Retrieve
   the current evaluation through the separately authorized read method. Do
   not retain `ManualCredit` or `ManualEvaluationRecord` in the receipt.

2. **P1 - A pre-correction manual result is published to global statistics
   before the generation-fenced score projection can commit.**
   The mutation identifies a completed run and derives contributions at
   `crates/learning-data-access/src/in_memory.rs:6707-6738`, then writes aggregates and receipts
   immediately through `stage_statistics_contributions` at `:6741-6749` and
   `:6977-7035`. A later correction supersedes the prepared scoring worker,
   but it cannot retract or replace that first contribution because the receipt
   is already stored. Thus the test's first grade of `1.0`, followed by a
   correction to `0.5`, can leave catalog statistics describing `1.0` even
   while the current assignment projection publishes `0.5`. This violates the
   accepted current-evaluation model and breaks the intended generation fence.

   **Fix:** keep manual completion/statistics contribution data private in the
   generation-specific scoring stage and publish it only with the successful
   current-generation commit, or defer this aggregate update until its own
   correction-safe projection contract is designed. Add a behavior test that
   corrects a manual grade before commit and proves no obsolete contribution is
   visible or receipted.

3. **P1 - The core enum change breaks the workspace, contrary to the frozen
   cross-crate contract requirement.**
   `GradeOutcome::NeedsManualGrading` is added in
   `crates/grading/src/checker.rs:18-25`, but not every consumer was updated.
   `cargo check --workspace` fails at
   `crates/project-tools/src/fixtures.rs:454` with E0004 because that match has no
   `NeedsManualGrading` arm. The same review search found still-unupdated
   outcome matches in server paths, so this should be fixed atomically with the
   server integration rather than treated as a green shared-core slice.

   **Fix:** update every `GradeOutcome` match in the same consumer patch,
   explicitly routing manual-required outcomes only to the server-owned
   pending-submission path and rejecting them in fixture-only/native contexts
   with an actionable error. Make `cargo check --workspace` a package gate.

4. **P1 - The purported exact decimal boundary is lost inside the Store
   contract, not only at a browser projection.**
   `ManualCredit` is correctly parsed as `Decimal` at
   `crates/learning-data-access/src/lib.rs:1377-1429`, but `set_memory_manual_grade` converts
   it to `f64` at `crates/learning-data-access/src/in_memory.rs:6669-6675` and preserves only the
   `AttemptResult` in `ManualEvaluationRecord` (`lib.rs:1447-1456`). The trait
   therefore has no exact current `credit_fraction` to carry into PostgreSQL
   scoring or to return from an authorized evaluation read. This is not merely
   a legacy browser-safe projection: MemoryStore's current state has already
   discarded the exact decimal.

   **Fix:** put `Option<ManualCredit>` (or a dedicated exact normalized-credit
   value) in the current manual evaluation record and use `AttemptResult` only
   for the explicitly browser-safe/public projection. PostgreSQL should bind
   that exact value as `rust_decimal::Decimal` to `NUMERIC`, using SQLx's
   `rust_decimal` feature as planned. Add a 12-fractional-digit round-trip
   behavior test that demonstrates the current record retains the exact value,
   not just a nearby binary float.

5. **P2 - The new manual-grading ownership should be modular before the
   PostgreSQL/server lanes expand it.**
   The shared types and trait add 184 lines in an already broad
   `crates/learning-data-access/src/lib.rs` (`:1302-1477`, `:3162-3198`); Memory behavior adds
   286 lines to `crates/learning-data-access/src/in_memory.rs` (`:6501-6786`); and the behavior
   test adds 250 lines to the 7,000-plus-line conformance file. This will make
   the PostgreSQL and server integration harder to audit and conflicts with
   the requested oversized-file breakup.

   **Fix:** move the shared types/trait into
   `crates/learning-data-access/src/manual_grading.rs`, Memory implementation into
   `crates/learning-data-access/src/in_memory/manual_grading.rs`, and the forthcoming PostgreSQL
   implementation into `crates/learning-data-access/src/postgres/manual_grading.rs`. Export
   the small public Store surface from `lib.rs`. Move the manual behavior test
   into a dedicated Store integration-test module where existing fixtures can
   be reused. Current visibility is sufficient for a focused extraction:
   `memory.rs` can expose only the necessary `pub(super)` state/access helpers;
   it does not require a broad unrelated refactor.

## Checks and positive evidence

- `cargo test -p grading` - passed (6 tests).
- `cargo test -p learning-data-access --test conformance
  memory_manual_grading_is_response_bearing_revisioned_and_generation_fenced
  --no-default-features -- --exact` - passed (1 test).
- `cargo test -p learning-data-access --no-default-features` - passed (38 unit, 14
  conformance tests).
- `cargo clippy -p learning-data-access --no-default-features --all-targets -- -D warnings`
  - passed.
- `cargo fmt --check` - passed.
- `git diff --check` - passed.
- `cargo check --workspace` - **failed**, E0004 non-exhaustive
  `GradeOutcome` match in `crates/project-tools/src/fixtures.rs:454`.

The existing focused test does establish useful guardrails: a pending response
does not write a run/summary score, a learner/foreign tenant cannot enumerate
the edit view, a response-less force-submit conflicts, and a stale scoring
worker is superseded. It does **not** cover a genuinely mixed automatic/manual
run, exact-decimal current-state retention, old-action replay after correction,
or correction before initial scoring commit with statistics visibility.

## Recommended next action

Repair the shared contract and split the manual modules before asking the
PostgreSQL and server lanes to implement it. Then add the two missing
deterministic behaviors (mixed run and pre-commit correction) and re-run the
workspace check before database integration.

## Re-review of the corrected modular slice

### Resolved findings

- **No receipt grade history:**
  `crates/learning-data-access/src/in_memory/manual_grading.rs:18-25` stores only actor,
  attempt, expected/resulting revisions, scoring generation, request digest,
  and time. The public receipt at `:29-37` contains only action, attempt,
  resulting revision, generation, and time. The exact-replay path compares
  identity plus digest at `:229-239` and returns that minimal receipt; the
  test replays the first action after a correction at
  `crates/learning-data-access/tests/conformance/manual_grading.rs:179-182`.
- **Exact current decimal retained:** `ManualEvaluationRecord` now holds
  `Option<ManualCredit>` at `crates/learning-data-access/src/manual_grading.rs:145-153`, and
  MemoryStore writes the exact value only to that sole current record at
  `crates/learning-data-access/src/in_memory/manual_grading.rs:271-278`. `AttemptResult` is a
  separate legacy projection constructed at `:261-281`. The twelve-decimal
  current-value assertion is at
  `crates/learning-data-access/tests/conformance/manual_grading.rs:166-178`.
- **No correction-unsafe statistics write:** the manual module has no call to
  `derive_statistics_contributions` or `stage_statistics_contributions`.
  The prior premature aggregate write is absent; the updated handoff records
  that manual/mixed anonymous statistics are deferred until a
  committed-generation analytics contract exists.
- **No premature summary publication:** manual mutation moves the assignment
  to `Recalculating` and queues a generation-fenced job at
  `crates/learning-data-access/src/in_memory/manual_grading.rs:319-338`; it does not write an
  enrollment or summary projection. The conformance test requires no current
  score, completion count, first-completion time, or current-grade run before
  worker commit at `crates/learning-data-access/tests/conformance/manual_grading.rs:72-101`.
- **Force-submit, authorization, and generation guards remain intact:**
  direct-instructor authority is at
  `crates/learning-data-access/src/in_memory/manual_grading.rs:219-227`, response evidence is
  required at `:246-256`, and the stale prepared worker is required to be
  superseded at `crates/learning-data-access/tests/conformance/manual_grading.rs:184-189`.
  The response-less force-submit refusal remains covered at `:233-264`.
- **Modularization is clean:** `lib.rs` now contains only the private module
  declaration and small public re-export (`crates/learning-data-access/src/lib.rs:40,62-66`),
  `memory.rs` contains only state registration and its submodule declaration
  (`crates/learning-data-access/src/in_memory.rs:68,235-237`), and the dedicated test is loaded
  from `crates/learning-data-access/tests/conformance/manual_grading.rs`. No duplicate old
  manual implementation remains in parent modules. This leaves a focused
  `crates/learning-data-access/src/postgres/manual_grading.rs` home for the next lane.

### Remaining package work, not a shared-core blocker

- A genuine mixed automatic/manual immutable-run fixture and real PostgreSQL
  RLS/retention transaction oracle still belong to the PostgreSQL/server
  integration package.
- The server and `project-tools` consumers must exhaustively handle
  `GradeOutcome::NeedsManualGrading`; until that immediately following lane
  lands, `cargo check --workspace` remains expected to fail at the previously
  recorded consumer match.

### Re-review checks

- `cargo test -p grading` - passed (6 tests).
- `cargo test -p learning-data-access --test conformance
  manual_grading::memory_manual_grading_is_response_bearing_revisioned_and_generation_fenced
  --no-default-features -- --exact` - passed (1 test).
- `cargo test -p learning-data-access --no-default-features` - passed (38 unit, 14
  conformance tests).
- `cargo check -p learning-data-access --features postgres` - passed.
- `cargo clippy -p learning-data-access --no-default-features --all-targets -- -D warnings`
  - passed.
- `cargo fmt --check` and `git diff --check` - passed.

## 2026-08-08 stale-run-score correction addendum

**PASS.** The bounded follow-up correction repairs the stale durable
`AssignmentRun.score` concern without weakening receipt, authorization, or
projection boundaries.

- `crates/learning-data-access/src/in_memory/manual_grading.rs:295-318` now resolves every
  immutable run item against the updated current attempt and writes the
  resulting score on every successful manual grade. The value is derived from
  the current command's validated `ManualCredit` (retained exactly in the
  current evaluation at `:271-278`) through the established legacy
  `AttemptResult`/four-decimal assignment scoring projection. It no longer
  leaves an earlier manual score in the run after a correction.
- `:313-317` sets `completed_at` only when it was previously absent, so a
  correction changes the mutable current run score while retaining the first
  completion timestamp.
- It still only queues/replaces the generation-fenced worker state at
  `:319-338`; it does not write enrollment or learner-summary projections.
  The focused test proves the corrected run score is visible while summary
  current score remains absent at
  `crates/learning-data-access/tests/conformance/manual_grading.rs:179-200`.
- Exact replay remains the minimal prior receipt (`manual_grading.rs:229-239`
  and conformance `:201-205`), and the stale prepared worker remains
  superseded (`:206-211`). No statistics mutation was introduced.

The focused test, `cargo fmt --check`, and `git diff --check` all passed. The
test fixture currently keeps its authoritative timestamp constant, so the
first-completion-timestamp invariant is established directly by the guarded
write above rather than by two distinct clock values; a future mixed-run
integration fixture can make that temporal assertion explicit.
