# Manual grading test-gate audit

Status: DONE_WITH_CONCERNS -- audit only; no production or test implementation changed.

This is the first remaining package in the recorded database-evolution order:
`partial_commit_status.md` names "manual grading and mixed automatic/manual assignment behavior"
as the next task (lines 100-103). The authoritative database plan requires a current
`submission_evaluation` manual-grading status, current-score recalculation, and a verification of
manual plus mixed assignments (lines 229-256 and 483-490).

## Current evidence and gap

The schema already reserves the necessary persistence vocabulary:

- `submission_evaluation.grading_status` admits `graded`, `needs_manual_grading`, and `exempt`
  (`2026080804_activity_feedback.sql:166-181`).
- `question_attempt` admits `needs_manual_grading` and requires a submission timestamp for that
  state (`2026080804_activity_feedback.sql:199-217`).
- PostgreSQL scoring deliberately excludes an ungraded evaluation from score staging and refuses
  to select a completed run that still has a manual-grading evaluation
  (`postgres.rs:814-909`).

The baseline has behavior evidence for adjacent foundations, not manual-grade completion:

- Scoring is staged, then atomically replaced only for the current generation in both backends
  (`memory.rs:663-816`,
  `postgres.rs:1024-1200`).
- The shared Store conformance flow covers Delete and Regrade plus force-submit/clear retry and
  cross-tenant non-enumeration (`conformance.rs:3980-4132`,
  `conformance.rs:4194-4350`).
- Force-submit reaches `needs_manual_grading` only by closing an active attempt without a response
  or result (`lib.rs:3336-3355`,
  `memory.rs:7193-7249`). It is support
  behavior, not an instructor manual-grade command.
- No `ManualGrade` command, Store method, server route, or browser grade-entry flow exists in the
  repository (`rg` of those terms on 2026-08-08). Therefore no current test proves that a manual
  grade can be recorded, corrected, cleared, or used in a mixed assignment.

The new command needs its own stable, tenant-scoped idempotency identity and revision/concurrency
contract. Reusing the support action ID would conflate "close without a grade" with "set current
evaluation." The plan already requires explicit, tenant-scoped, revision-checked instructor
commands (`database_schema_evolution_plan.md:439-462`).

## Permanent behavior matrix

Use fixed in-memory facts and controlled backend clocks. Tests should assert visible state and
current projections, not table counts, fixed JSON key lists, private staging representation, or a
specific worker implementation. Keep the primary shared scenarios in the existing Store
conformance harness so MemoryStore and PostgresStore cannot diverge.

| Behavior to prove | Permanent owner and test shape | Existing evidence / required addition |
| --- | --- | --- |
| Auto-only submitted assignment derives the same normal score after the package | `crates/learning-data-access/tests/conformance.rs`: one submitted automatic attempt, score worker prepare/commit, current run summary | Existing scoring and generation fixture; retain as regression. Add only if refactoring otherwise removes the observable automatic path. |
| Manual-only submitted assignment is visibly incomplete until an authorized instructor records a finite normalized manual grade | Store conformance: one manual evaluation; before grade, summary has no selected current grade and gradebook marks grading incomplete; after grade plus worker commit, the selected run receives the manual credit | **Required.** The existing code has only a status enum/schema, not the mutation. |
| Mixed automatic/manual assignment does not publish a partial aggregate while one submitted item is still manually pending; final aggregate includes both current credits exactly once | Store conformance: two delivered items in one completed run, one automatic and one manual; assert no selected completed grade before manual completion, then exact numerator/denominator after commit | **Required.** This is the package acceptance behavior, not a manual test of a helper. |
| A manual grade correction replaces the current evaluation and queues one newer scoring generation; a correction cannot leave the former score current | Store conformance with fixed grade-action identity/revision: grade, commit, correct, prepare/commit new generation; assert changed current result and no stale current projection | **Required.** Assert replacement semantics, not audit-row count or SQL update sequence. |
| Clearing a manually graded attempt retains protected evidence, removes it from current scoring, and retries remain harmless | Store conformance extends existing clear scenario: manually grade, clear under a distinct action ID, process recalculation; exact clear retry returns its first action outcome | **Required.** Existing clear coverage only proves an ungraded force-submit transition. |
| Regrade/clear races do not leak a partial score or overwrite a newer generation | Store conformance with deterministically interleaved prepare/commit calls: stage old grade, apply newer grade or clear, then commit old command; old result is `Superseded`/not current and newest full projection wins | **Required.** This is a concurrency invariant, not timing/sleep test. |
| Manual-grade command retries are idempotent; action identity reuse with a different target/value conflicts | Store conformance: submit the same command twice, then reuse its identity with different grade/attempt | **Required.** Match the existing support-action retry contract without mocking internal receipts. |
| Instructor from the wrong tenant, a student, and a non-instructor course member cannot grade or enumerate an attempt | Store conformance and server route test: expect non-enumerating `NotFound`/HTTP 404 where the established boundary uses it | **Required.** Existing force-submit coverage is adjacent evidence only (`conformance.rs:4194-4224`). |
| Recalculation treats manually graded and automatic evaluations alike once both are final, while stale generation results never publish | Store conformance; reuse `AssignmentScoringWorkerStore` through the server handler only for handler dispatch mapping | **Required.** Existing worker source establishes the handler/committer seam (`scoring_worker.rs:16-137`); test its behavior, not struct fields. |
| Retention deletes manual evaluation/evidence through the same tenant-owned record lifecycle | Extend the already-existing retention Store/worker behavior scenario with a manually evaluated submitted attempt and prove the student record disappears after the final deletion stage | **Required integration extension.** Retention policies already include `submission_evaluation` and current scores (`2026080806_retention.sql:2836-2842`); do not make it an isolated schema assertion. |
| Browser contracts and Wasm never receive answer keys, grading logic, manual-grading rubric/key, or an instructor-only grade mutation usable by learners | Keep Python crate-boundary and Node/Wasm allowlist checks; add generated API/decoder contract tests only if a new instructor manual-grade DTO is browser-visible | Existing: WASM has no grading dependency (`lib.rs:1-12`), the browser decoder rejects grading-bearing preview fields (`index.ts:169-204`), and the closure gate enforces the boundary (`test_crate_boundaries.py:148-185`). |
| Instructor manual-grade UI declares pending/manual state, submits only through authenticated server API, preserves entered feedback on transient failure, and refreshes current grade after server confirmation | Playwright only after a real instructor route and Solid surface exist; mock external grading backend only at its documented server boundary, not the Store command | **Required only when this package exposes a UI.** No current manual-grade UI exists, so do not add a speculative browser test. |

## Layer assignment

1. **Rust unit tests** belong only to new pure domain validation: finite/manual credit range,
   permitted state transition, and deterministic replacement/selection rules. Put them beside the
   pure module. Do not add unit tests for a Store trait forwarding method or SQL text.
2. **Store conformance tests** are the primary permanent gate for auto-only, manual-only, mixed,
   correction, clear, retry identity, deterministic race, scoring generation, and tenant denial.
   Exercise both backends once a PostgresStore conformance runner is available; otherwise retain
   MemoryStore conformance plus the targeted real-PostgreSQL acceptance below.
3. **Server tests** cover HTTP authentication, role authorization/non-enumeration, request decoding,
   response cache headers, and server-owned call to the command. They must not assert that an
   internal Store mock method was called.
4. **Node contract tests** cover only a durable public TypeScript decoder/client contract introduced
   by the route. No Node test is warranted for a server-only manual-grade detail.
5. **Playwright** is a durable instructor-visible flow gate only if a manual-grade screen is shipped.
   It should enter one manual grade in a mixed run and observe pending-to-current UI behavior; it
   must not fabricate correctness in the browser.
6. **Wasm/native parity** remains a boundary gate, not a manual-grading oracle. Manual credit and
   answer-bearing logic stay server-only under human guidance
   (`HUMAN_GUIDANCE.md:48-61`).
7. **One-time PostgreSQL acceptance** covers real roles/RLS, current migration epoch, and the
   SQL-visible atomicity/retention behavior. The plan explicitly classifies real RLS and populated
   PostgreSQL checks as disposable evidence rather than committed fixtures
   (`database_schema_evolution_plan.md:424-435`).

## One-time PostgreSQL acceptance fixture

Use a freshly migrated disposable PostgreSQL 17 database with two tenant principals, one instructor,
one student, and a completed two-item run. The fixture has one automatic evaluation and one manual
evaluation. Record the exact database URL only in shell environment/configuration, not this file.

1. Apply and re-verify the SQLx baseline: `cargo tools database migrate`, then
   `cargo tools database verify`.
2. Under the normal application role, submit the automatic item and create the manually pending
   item through the public Store/server boundary; before manual grade, prove no partial selected
   assignment summary is visible.
3. Grade the pending item as the direct instructor, run the leased scoring prepare/commit path, and
   prove the one current attempt/assignment result uses both final evaluations.
4. Interleave an old worker generation with a manual correction (and separately clear): the stale
   commit must be superseded and no incomplete/old result becomes current.
5. Connect as the foreign application principal and worker principal with `row_security=on`; attempt
   direct reads/writes and the command path against the first tenant's evaluation/attempt. Both must
   deny access without revealing the record.
6. Execute the configured tenant retention deletion flow for the course and verify the manual
   evaluation, submission, current score, and summary are absent while identity-free analytics remain
   available only as permitted by the plan. Remove the disposable database afterward.

Capture command output plus result queries; use `EXPLAIN (ANALYZE, BUFFERS)` only if this package
adds or changes an index/query plan. PostgreSQL guidance requires representative fixtures and a
before/after plan for plan-affecting changes, not a ritual plan capture for a pure behavior change.

## Exact gates for the implementation package

Run narrow gates while implementing (replace a filter only after the named test exists):

```bash
cargo test -p learning-data-access --features postgres --test conformance manual
cargo test -p server manual
node --import tsx --test 'tests/test_*.mjs'
python3 -m pytest -q tests/test_crate_boundaries.py
```

Run the full permanent gate after narrow checks pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./check_codebase.sh
source source_me.sh && python3 -m pytest -q tests/
bash run_playwright_tests.sh
```

Run browser/Node and Playwright gates only when the public TypeScript/UI surface changes. Run the
manual PostgreSQL fixture separately from the permanent suite using a disposable database and the
documented `DATABASE_URL` commands:

```bash
cargo tools database status
cargo tools database migrate
cargo tools database verify
```

## Rejected tests

- Do not assert the exact number of SQL rows, exact migration/table/enum names, staging-table
  contents, or a required list of generated DTO fields.
- Do not use sleeps, wall-clock races, network calls, external grading services, or a real browser
  to test a pure Store outcome.
- Do not test only that `NeedsManualGrading` serializes or that a method invokes a mock; neither
  proves an instructor can finish a mixed assignment safely.
- Do not move PostgreSQL role/RLS/retention fixtures into fast Rust or pytest lanes. They are
  production-shaped disposable acceptance evidence.

These choices follow the repository's fragile-pytest rules: durable tests assert behavior, run
deterministically, avoid date/count/default/internal-wiring assertions, and keep slow real-system
work outside the fast pytest lane (`PYTEST_STYLE.md:1-78`).
