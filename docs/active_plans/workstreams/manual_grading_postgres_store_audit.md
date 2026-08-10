# Manual grading PostgreSQL Store audit

> **Historical audit.** This dated audit is retained as evidence, not current task direction.
> Current authority is the [release completion plan](../active/release_completion_plan.md) and
> [implementation status](../implementation_status.md).

## Status

**Ready for the shared Store-contract package; no PostgresStore production change should land before
that contract and the matching `MemoryStore` behavior exist.** The schema has the useful
`needs_manual_grading` states, but it cannot represent a genuinely ungraded evaluation because
`credit_fraction` and `correct` are required. The current submission path therefore always writes
an automatic grade and an `attempt_score_current` row. This workstream is the smallest correct
PostgreSQL implementation slice after the shared contract is frozen. It deliberately stops before
course-local item analysis.

The relevant plan boundary is manual and mixed grading, then item analysis
([database evolution plan](../decisions/database_schema_evolution_plan.md):229-252, 304-319;
[partial status](../partial_commit_status.md):100-105). It preserves server-only grading and
tenant-owned education records; a browser never supplies a tenant, role, scoring generation, or
SQL-visible grade authority.

## Evidence and gap inventory

| Finding                                                                                                    | Evidence                                                                                            | Consequence                                                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The relational model recognizes `needs_manual_grading`.                                                    | [activity migration](../../../schemas/migrations/2026080804_activity_feedback.sql):166-197, 199-215 | The base names are available, but the evaluation shape cannot record a pending review honestly because both score fields are `NOT NULL`.                                                           |
| The Store has only automatic submission plus force-submit/clear support.                                   | [Store contract](../../../crates/learning-data-access/src/lib.rs):3314-3355                         | A new instructor-authorized manual-evaluation capability and a submission disposition that means "response accepted, review pending" are required.                                                 |
| Automatic submission writes a response, `graded` evaluation, and current score in one transaction.         | [postgres.rs](../../../crates/learning-data-access/src/postgres.rs):8607-8663, 8776-8867            | Branch before `AttemptResult` validation and score insertion; do not represent a pending review as zero credit or a false `correct` flag.                                                          |
| The scoring worker already excludes incomplete manual evaluations and publishes only a current generation. | [postgres.rs](../../../crates/learning-data-access/src/postgres.rs):815-901, 1039-1219              | Retain this worker boundary. Manual evaluation changes must increment the generation and enqueue its existing idempotent recalculation job.                                                        |
| A unique expression index permits exactly one recalculation job per tenant/assignment/generation.          | [operations migration](../../../schemas/migrations/2026080805_operations_analytics.sql):1022-1045   | A retry must reuse the same generation/job semantics, never create duplicate jobs.                                                                                                                 |
| The current support mutation establishes the existing instructor and audit-idempotency conventions.        | [postgres.rs](../../../crates/learning-data-access/src/postgres.rs):7825-7851, 7892-7967            | Manual grading should authenticate the direct instructor from persisted membership and use an action identity/digest, rather than accepting a client role or silently replacing a different grade. |
| Tenant context is transaction-local and RLS-backed.                                                        | [postgres.rs](../../../crates/learning-data-access/src/postgres.rs):341-351                         | Every manual read/write starts through `begin_tenant`; all predicates retain `tenant_id = $1` even with forced RLS.                                                                                |
| Assignment advisory locks have an established order for assignment-policy writers.                         | [postgres.rs](../../../crates/learning-data-access/src/postgres.rs):6860-6877                       | Manual grading must take this lock before its assignment/attempt/evaluation row locks, and sort IDs if a future command grades more than one assignment.                                           |

## Required shared contract before PostgresStore work

Freeze this as a question-agnostic capability, implemented and behavior-tested first by both
`MemoryStore` and `PostgresStore`.

1. Submission grading has three explicit server-only outcomes: automatic `Graded(AttemptResult)`,
   `NeedsManualGrading`, and intentionally `Ungraded` practice. `NeedsManualGrading` is a normal,
   successful response submission, not a grading error. It is distinct from the existing
   `GradingError::ManualReviewRequired` refusal
   ([checker](../../../crates/grading/src/checker.rs):18-41, 171-177).
2. Add a narrowly-scoped instructor capability, for example `ManualGradingStore`, with a command
   containing: `actor`, `attempt`, an opaque/replay-safe `action_id`, normalized manual
   `AttemptResult`, and feedback. The command must not contain tenant, course, assignment,
   generation, role, or points-possible authority. The Store resolves all of those from the
   attempt's issued run and current assignment item.
3. The manual result is validated using the same finite/normalized-credit rules as automatic
   results. Assignment points remain current assignment state: manual grading sets normalized
   correctness/credit; `current_attempt_points` still applies normal/full/extra/excluded behavior.
   A manual grade must never embed a historical assignment point value.
4. Grade submission and manual evaluation need explicit browser-safe result projections: students
   can see a pending-review state, while instructor review routes are separately authorized. Do
   not add answer keys, rubrics, raw response bytes, grading implementation detail, or an
   instructor role claim to client DTOs.
5. A repeat of the same `action_id` and request digest returns the first outcome. The same action
   ID with another actor, attempt, or grade payload conflicts. A new action ID may replace the
   _current_ manual evaluation; it increments scoring generation and leaves no score history.

The schema slice must make `submission_evaluation.credit_fraction` and `.correct` nullable only
when `grading_status = 'needs_manual_grading'`, and non-null for `graded`/`exempt`; add a CHECK
expressing that conditional shape. Pending rows retain the response/submission link and a
checksum-bearing, bounded evaluation payload but contain no invented result. If the accepted
six-file epoch has become immutable, use a new forward SQLx migration; otherwise consolidate this
shape only with the baseline owner before any durable data is accepted. The current status records
that this distinction is material ([partial status](../partial_commit_status.md):85-87).

## Smallest PostgresStore implementation slice

### 1. Commit a pending manual response

Modify only the existing submission transaction at
[postgres.rs](../../../crates/learning-data-access/src/postgres.rs):8607-8895 after the shared submission outcome
exists.

- Start with `begin_tenant(context)`, lock the single attempt, prove the persisted enrollment owner,
  and preserve the existing response/idempotency replay check.
- Persist `submission_idempotency`, `submission`, protected feedback policy, and the attempt's
  authoritative submission timestamp exactly once. Set `question_attempt.attempt_status` to
  `needs_manual_grading` and insert/upsert a current `submission_evaluation` with
  `grading_status = 'needs_manual_grading'`, null score fields, and a non-secret pending marker.
- Do **not** call `current_attempt_score`, write `attempt_score_current`, complete the run with a
  computed score, stage public aggregate question statistics, or manufacture zero credit. A
  mixed assignment may have automatic scores for other attempts while this attempt keeps the run
  and assignment summary incomplete/pending.
- Keep the existing automatic branch byte-for-byte equivalent in behavior: it validates an
  `AttemptResult`, writes `graded`, calculates current attempt points, and may complete a run.
  Refactor shared response persistence only when the atomic/replay behavior remains identical.

### 2. Apply an instructor manual evaluation

Implement the new contract in `PostgresStore` beside `force_submit_attempt`/`clear_attempt`
([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):7820-8035), reusing their error vocabulary and
audit discipline.

Transaction sequence (one short transaction):

1. `begin_tenant(context)` sets `ple.tenant_id`; identify the attempt's assignment/course using
   tenant-filtered joins solely to choose the advisory key. Missing/foreign rows return the
   contract's non-oracular absence result.
2. Acquire `lock_postgres_assignment_policy(tenant, assignment)` **before** `FOR UPDATE` locks.
   Lock the assignment, attempt, run, enrollment, and current evaluation in one stable order;
   confirm the actor is a persisted direct course instructor. Recheck that the attempt is
   submitted/auto-submitted or `needs_manual_grading`, never cleared/exempt/in-progress.
3. Take the action-ID advisory lock before looking up its audit/receipt row. An exact prior receipt
   returns; mismatched request identity conflicts. Store only the action identity, target,
   actor, transition, and payload digest as minimal audit evidence-no old/new numeric score copy.
4. Upsert the sole `(tenant_id, attempt_id)` evaluation to `graded` with normalized credit,
   correctness, payload/checksum, and database `evaluated_at`; restore the attempt state to the
   appropriate terminal graded/submitted state defined by the frozen contract. This is a replace
   of current state, not a second evaluation or grade event.
5. Update the current assignment row under the same lock:
   `scoring_generation = scoring_generation + 1`, `scoring_status = 'recalculating'`; insert the
   unique `RecalculateAssignment` job for that exact generation; write the audit receipt; commit.
   A unique-index conflict can only be accepted after verifying the existing job payload is the same
   tenant/assignment/generation; otherwise return `Conflict`.

No item-analysis job is written here. The next explicitly ordered package attaches a distinct,
course-local analysis trigger after successful scoring publication, so analytics cannot delay or
partially alter grading.

### 3. Preserve current-only rescoring semantics

The current prepare query correctly excludes `needs_manual_grading`
([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):842-900); retain that predicate. After a manual
evaluation becomes `graded`, the existing stage/count/commit fence sees it and replaces the entire
assignment's current attempt rows and student summaries atomically
([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):1066-1184). The generation fence must remain
the authority: a newer edit/manual grade discards an older prepared worker result, and a new
automatic submission between prepare and commit forces restaging.

Avoid an incremental student-summary patch in the manual-grade request. It would bypass the
first/last/highest/lowest selection policy and could expose a partial mixed-assignment grade.

## Expected SQL and index use

| Operation                     | Predicate/order                                  | Expected existing access path                                                                         | Required review                                                                                       |
| ----------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Pending submission replay     | `(tenant_id, attempt_id)`                        | `submission_idempotency` primary key; `question_attempt_lookup_idx` plus one partition row            | Preserve partition-safe `ORDER BY occurred_at LIMIT 1 FOR UPDATE` used at postgres.rs:8613-8631.      |
| Manual authorization and lock | tenant + attempt, then run/enrollment/assignment | `submission_evaluation` PK; `question_attempt_lookup_idx`; primary keys for run/enrollment/assignment | Every join includes tenant; RLS remains the backstop, not an excuse to omit predicates.               |
| Current evaluation replace    | `(tenant_id, attempt_id)`                        | `submission_evaluation_pkey`                                                                          | One row only; no append-only manual evaluation history.                                               |
| Current-score publish         | tenant + assignment; staged tenant + job         | `attempt_score_current_assignment_idx`; staging primary keys and `*_assignment_idx`                   | The existing delete/insert/update remains one transaction, after generation/current-count validation. |
| Recalculation enqueue         | tenant + JSON assignment/generation and `kind`   | `worker_job_assignment_scoring_generation_idx`                                                        | Verify a collision is the exact same semantic job before treating it as idempotent.                   |

On a disposable PostgreSQL 17 database, capture `EXPLAIN (ANALYZE, BUFFERS)` for the manual
authorization/evaluation lookup and the rescoring staging query with representative mixed data.
Confirm the named indexes/partition pruning actually appear before adding an index. This is a
one-time plan-selection oracle, not a permanent unit-test assertion about planner text.

## Focused verification

Permanent behavior tests (run for MemoryStore and PostgreSQL conformance):

- A file/rubric/manual backend response records protected response evidence and exactly one pending
  evaluation, has no `attempt_score_current` row, and never fabricates correctness/zero credit.
- A mixed run with automatic and manual questions remains pending until manual evaluation; automatic
  results remain server-derived. Manual grading then updates all affected current grades according
  to normal/full/extra/excluded semantics and the assignment attempt-selection policy.
- Only a persisted direct course instructor can evaluate; wrong actor, foreign tenant, expired or
  wrong session, cleared/exempt attempt, and mismatched run identity are denied without exposing
  existence or response data.
- Same manual action ID and same digest is replay-safe; reuse with different grade/actor/attempt
  conflicts. Two concurrent manual grades or a manual grade plus assignment-policy edit serialize
  and leave one current evaluation, one current score per attempt, one current summary, and one
  recalculation job per generation.
- A newer generation supersedes a prepared scoring job; an automatic submission during staging
  requires restaging; pending manual rows remain excluded from completed grade selection.
- Student/instructor response payloads and API DTOs contain no answer key, rubric, grader secret,
  raw protected manual-feedback field, or tenant/role authority.

Focused commands once the contract lands:

```bash
cargo test -p learning-data-access --features postgres manual
cargo test -p learning-data-access --features postgres conformance -- --nocapture
cargo test -p server manual
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
./check_codebase.sh
```

Use the repository's maintained disposable PostgreSQL acceptance fixture to exercise the actual
RLS role/session path; run `cargo tools database verify` before and after it. Keep the following
as one-time operational checks, not brittle permanent tests: fresh apply/no-op migration/status
verification; `EXPLAIN (ANALYZE, BUFFERS)` comparison; concurrent-session lock observation; and a
manual `SELECT` confirming no pending manual attempt acquired a score. Do not assert fixed row
counts, internal SQL formatting, query-plan wording, timestamps, or migration-file counts in unit
tests; those are fragile pytest-style checks rather than durable behavior contracts.

## Assumptions and risks

- `NeedsManualGrading` applies to a submitted response requiring human judgment, while force-submit
  remains an instructor closure without a response/evaluation. The shared contract must preserve
  that distinction.
- A manual evaluator grades normalized credit; current assignment points and scoring modes remain
  authoritative. This makes recalculation fair after point/policy changes and keeps the feature
  question-agnostic.
- The existing `submission_evaluation` payload must be confirmed safe for a pending marker; manual
  rubric text and answer-bearing feedback remain server-only and should not be copied into generic
  evaluation JSON merely for convenience.
- This audit assumes accepted baseline migrations remain immutable once durable data exists. The
  implementation owner must choose baseline consolidation versus a forward migration using the
  actual epoch status, not edit an applied database in place.
- Item analysis is intentionally an unimplemented next package. Its incomplete-manual/recent-
  rescoring flags belong to a derived course-local projection, never to this operational transaction.
