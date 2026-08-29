# Plan: automated grading operations

## Status

G1-W2 through G1-W7 have historical acceptance evidence from 2026-08-28. It includes an earlier live-stack journey, PostgreSQL/RLS proof, and final aggregate. It remains useful evidence for that package boundary, but it does not accept the subsequent WP-SD1-A5 single-installation `ActorContext`, exact course/Student ownership, immutable question reference, or typed worker-manifest correction. Those contracts require their own future evidence lanes.

The [implementation status ledger](../implementation_status.md) owns package allocation and current status. It assigns migrations `2026081849_automated_grading_operations.sql` through `2026081869_g1_instructor_receipt_writers.sql` to this package. Accepted migrations remain immutable. Rust, frontend, and pytest behavior checks are permanent regression gates; real-stack browser, connected PostgreSQL, screenshots, migration replay, and independent review are disposable acceptance evidence under [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

This plan is the compact G1 architecture and allocation record. It pairs with:

- [Automated grading Student execution plan](automated_grading_student_execution_plan.md), which owns G1-W4 accepted input, execution, Student status, worker recovery, score-publication handoff, implementation sequence, and narrow validation.
- [Automated grading execution contract](automated_grading_execution_contract.md), which owns W4's sealed capability, canonical immutable evidence protocol, state machine, lanes, and contract validation.
- [Automated grading operations delivery plan](automated_grading_operations_delivery_plan.md), which owns G1-W5 through G1-W7 Instructor delivery, connected proof, and closeout.

## Purpose and objectives

G1 makes deterministic server-owned grading an operational loop: the server accepts immutable Student input, grades only that exact evidence, records immutable receipts, routes deterministic faults into an answer-free recovery operation, and publishes current scores through the existing generation-fenced scorer.

1. Persist accepted Student input before every grader effect.
2. Preserve immutable evidence and exact-version identity through execution, recovery, and scoring.
3. Give Instructors bounded, idempotent, auditable recovery and recalculation actions.
4. Keep browser contracts answer-free and make the next safe action visible.
5. Preserve one worker path and one current-score publisher.
6. Provide durable handoffs to G2 inspection, G3 analysis, and G5 actionable Instructor work.

## SD1-A5 identity and authorization correction

PLE is one installation with global accounts and a shared published question corpus. An authenticated request starts only from server-derived `ActorContext { user_id, session_id }`. The context identifies an actor; it does not select a course or grant authority. It is never read from a route, header, JSON body, browser state, or queue payload.

Every protected Student or Instructor operation verifies its exact chain in one transaction: `CourseId`, `AssignmentId`, Student `UserId`, `RunId`, `QuestionAttemptId`, `AcceptedSubmissionId`, immutable `QuestionId`, exact `ProblemVersionRef`, issued/evaluation evidence references, and digests. A foreign or revoked actor, route, course, assignment, Student, run, attempt, submission, question, evidence reference, or stale request returns one concealed no-row/unauthorized result without mutation.

Workers carry a typed locked `WorkerManifest`, not an actor. It binds the exact course, Student, run, attempt, submission, question/evidence references, and digest. `WorkerLease` binds that manifest to `JobId`, `JobLeaseToken`, `WorkerId`, and execution or scoring generation. Claim, load, completion, failure, retry, and recalculation recheck the full tuple in one transaction; one winner is selected with `FOR UPDATE SKIP LOCKED`, while stale generations cannot publish. Manifests contain references and digests only, never answers, keys, feedback internals, grades, or provider diagnostics.

Questions are deterministic validated global-corpus content. Manual grading, manual score ownership, and exemption transitions are outside this model. No G1 route, DTO, worker, receipt, or operation provides such a transition.

### Instructor operation contract

The recovery broker starts from `ActorContext` and exact resource witnesses:

```rust
trait GradingOperationStore {
    fn list(&self, actor: ActorContext, course: CourseId, assignment: AssignmentId, page: PageRequest);
    fn retry(&self, actor: ActorContext, target: ExactSubmissionTarget,
             expected_revision: GradingOperationRevision, idempotency_key: ActionId);
    fn recalculate(&self, actor: ActorContext, course: CourseId,
                   assignment: AssignmentId, expected_revision: AssignmentRevision,
                   idempotency_key: ActionId);
}
```

`ExactSubmissionTarget` contains the full Student/run/attempt/submission witness plus immutable question and evidence references. PostgreSQL checks the actor against the session within the same transaction. A worker-only broker accepts only a locked lease and manifest. The Student status reader returns only an answer-free, no-store `accepted_pending`, `instructor_attention`, or `completed` projection for its exact route-bound target.

## Immutable question versions and correction impact

Every assignment item, issued attempt, accepted submission, worker manifest, completion receipt, and grading evidence pins public `QuestionId`, exact immutable `ProblemVersionRef`, and its evidence digest. Publishing a later question version never retargets issued or graded work. A replacement-impact record links old/new references, the improvement thread, affected pinned population, privacy-safe summary, and an explicit deterministic decision: preserve history, future runs only, or a separately identified generation-fenced recalculation. The sole score committer publishes derived results; original receipts remain immutable.

Version metrics count only accepted server-graded attempts for one exact version. They exclude previews, Instructor Student view, aborted work, and pending/ungraded work. Privacy-thresholded rollups disclose no Student identity and label formula, version, and evidence time.

### ForcedQuestionCorrection

A Sysadmin may approve a `ForcedQuestionCorrection` only for a security or critical-correctness defect, after a validated replacement version exists. The correction is an append-only, privacy-safe manifest binding defective and replacement references, reason, impact, deterministic remediation, and generation. It commits one authoritative active-reference mapping before fan-out; new selection and issuance resolve to the replacement while issued and graded work remains pinned.

Generation-fenced workers materialize compatible unissued updates and deterministic remediation. In-progress work is reissued or excused; completed work receives full-credit or exclude-and-rescale according to the manifest. No per-course approval follows the Sysadmin decision. Instructors see exact-course audited impact and actions; Sysadmins receive only privacy-safe aggregates. Original prompts, responses, evidence, scores, and receipts remain resolvable history.

## G1 architecture and responsibility split

G1 has one operational loop:

```text
accepted immutable input
  -> leased deterministic execution
  -> immutable execution/evaluation receipt
  -> generation-fenced score publication
  -> answer-free Student status and Instructor recovery
```

W2 owns accepted private input and initial execution state. W3 owns the minimal answer-free pending/read projection and closed outcome classification. W4 owns accepted-input execution, Student status, recovery, completion, and the score-publication handoff in the [Student execution plan](automated_grading_student_execution_plan.md). W5 owns strict course-scoped Instructor operations HTTP; W6 renders it; W7 proves the live journey and persistence boundary.

The [execution contract](automated_grading_execution_contract.md) is the binding W4 contract. It defines canonical source-text evidence, exact claim/load/lock/commit/fail capabilities, lane ownership, and contract-level state transitions. The Student execution plan gives the focused implementation and validation sequence. The [delivery plan](automated_grading_operations_delivery_plan.md) starts only from W4's answer-free handoff and does not duplicate W4 worker ownership.

### State and publication ownership

| Owner | Mutable state | Immutable history | Handoff |
| --- | --- | --- | --- |
| Acceptance | Initial evaluation/execution state | Accepted input and receipt | Exact ready job |
| W4 worker | Execution/evaluation completion | Execution and completed receipt | One 1830 request |
| Instructor broker | Recovery operation revision | Operation receipt | Retry/recalculation generation |
| Scoring committer | Current assignment/course totals | Score generation evidence | Current score projection |

Migration `2026081830` alone creates a new assignment scoring generation and bounded work. Migration `2026081831` remains the only publisher of assignment/course current score and total. An execution retry advances execution generation; a recalculation advances scoring generation. Neither modifies accepted input, issued question reference, prior receipt, or history.

## Scope

- Migrations 1849/1850 allocate immutable accepted input, private response storage, typed evaluation/execution/operation state, append-only receipts, and an exact worker payload.
- W4 owns migrations 1851-1860: schema/roles, integrity, public-function authority, table authority, claim, verified read, private load, completion lock, commit, and failure.
- W5 owns 1861-1865 for Instructor capabilities, lifecycle projection, scoring invalidation origin/capability, and source binding. The G1 reconciliation addendum allocates 1866-1869 for receipt provenance and final writer closeout.
- G1 exposes answer-free Student delivery and assignment-local Instructor recovery. It reuses existing worker and scoring pipelines rather than adding a scheduler or score-mutation path.
- G2 owns protected Student-work inspection; G3 owns item/course analysis; G5 owns cross-course attention. G1 supplies their immutable, privacy-safe seams.

## Non-goals

- Browser delivery of raw Student responses, answer keys, private feedback, grader diagnostics, score fields, or provider details.
- A direct score-row mutation path, browser-to-worker command, or second scheduler.
- Manual grading, manual score ownership, or exemption transitions.
- Treating exact pixels, incidental counts, source inventories, or arbitrary timing thresholds as permanent behavioral tests.

## Current-state basis

Migration 1830 advances scoring generation and creates bounded work. Migration 1831 publishes only the exact current generation after lease and generation checks. G1 preserves that division. Existing worker and queue contracts provide leases and durable failure handling; G1 extends them with an exact accepted-submission family. The assignment workspace and strict browser client provide the course-scoped Instructor shell. Direct current source is authoritative over this planning summary.

## G1-W1: bind accepted-input and operation contracts

- **Owner/package:** architect, `WP-INST-G1 / G1-W1`.
- **Outcome:** accepted-input, execution, evaluation, operation, retention/RLS, receipt, exact-job lease, safe public enum, and migration allocation contract.
- **Owned artifacts:** this plan and [implementation status ledger](../implementation_status.md).
- **Depends on:** accepted T6 workspace and existing 1830/1831 capabilities.
- **Work:** bind canonical immutable `submission`, separate state ownership, idempotent replay, `GO-<positive>` resolution, private execution, and allocation before source edits. Confirm no manual score capability exists.
- **Verification:** one-time Graphify/source lifecycle audit, migration/privilege review, and architect decision record. W1 adds no permanent test.
- **Handoff:** stable types, migration IDs, and boundaries to W2-W7.

## G1-W2: persist accepted submissions and operation evidence

- **Owner/package:** learning-data-access expert coder, `WP-INST-G1 / G1-W2`.
- **Outcome:** immutable answer-free accepted metadata plus private response, typed evaluation/execution/operation projections, append-only receipts, and a worker-only execution boundary.
- **Owned artifacts:** question-model/public-route and grading-operation contracts, relevant Memory/PostgreSQL submission/grading-operation owners, migrations 1849/1850, and crate-local tests.
- **Work:** serialize a typed `StudentResponse` once, persist canonical UTF-8 bytes only in a composite-FK private child, hash exact bytes, and use fixed answer-free parent markers. Enforce replay, append-only behavior, retention-owned deletion, forced RLS, and dedicated API/worker role composition. General `PostgresStore` cannot load the private child; the dedicated execution store can only load it with an exact worker capability. Reuse 1830 rather than duplicating score enqueue logic.
- **Permanent validation:** deterministic offline Memory/contract tests for public/private split, digest, replay/conflict, state/receipt immutability, exact ownership, immutable question/evidence, retention metadata, and narrow store composition. Use controlled values and no services.
- **One-time/connected validation:** clean migration replay is acceptance evidence; W7b owns executable catalog/RLS/private-read proof.
- **Handoff:** typed submission, operation, receipt, action, and worker-only load contract to W3-W5.

## G1-W3: stabilize pending reads and classify outcomes

- **Owner/package:** expert coder, `WP-INST-G1 / G1-W3`.
- **Outcome:** minimal no-store accepted-pending replay/read projection and closed deterministic exception classification without changing first-effect acceptance or worker ownership.
- **Owned artifacts:** focused submission-record matching, submission/external-tool read helpers, backend disposition mappings, and narrow server tests.
- **Work:** preserve pre-acceptance shape/timing 422 behavior; distinguish missing, accepted-pending, and completed work; classify deterministic exceptions through the closed taxonomy. Preserve the opaque iMathAS invalid/unavailable/unsupported path and its atomic external-tool replay.
- **Permanent validation:** deterministic pending/read, 202-helper, outcome-matrix, and external-tool-bypass checks with controlled values and no service dependencies.
- **Connected validation:** one disposable-stack check of pending/read, deterministic exception, and dependency-unavailable outcomes.
- **Handoff:** final answer-free read shape, status vocabulary, exception category, operation target/revision, and worker-load invariant to W4. W5 receives only safe Instructor action semantics.

## G1-W4: Student accepted-input execution

G1-W4 is allocated to `WP-INST-G1 / G1-W4`. Its implementation, worker and browser scope, ordered migrations 1851-1860, stabilization decision, narrow permanent tests, and connected validation are in the [Automated grading Student execution plan](automated_grading_student_execution_plan.md). Its non-negotiable capability and evidence rules remain in the [Automated grading execution contract](automated_grading_execution_contract.md).

W4 hands W5 completed answer-free Student status, closed retry/recalculate semantics, immutable receipt/evidence references, a canonical score-publication trigger, and no private-response access.

## Dependency graph

```text
T6 assignment workspace
  -> G1-W1 contract and allocation
  -> G1-W2 accepted-input persistence
  -> G1-W3 pending/status classification
  -> G1-W4 Student execution and score-publication handoff
  -> G1-W5 strict Instructor operations HTTP
  -> G1-W6 Instructor operations page
  -> G1-W7a visible live recovery journey

G1-W4 + G1-W5 -> G1-W7b PostgreSQL/worker oracle
G1-W7a + G1-W7b -> G1-W7 evidence closeout
```

## Ownership matrix

| Boundary | Owner | Contract consumer |
| --- | --- | --- |
| Architecture and allocation | W1 architect | W2-W7 |
| Accepted input and private evidence | W2 data-access owner | W3-W4 |
| Pending read and outcome vocabulary | W3 server owner | W4-W5 |
| Student acceptance, worker, status, score trigger | W4 execution owner | W5, W7a, W7b |
| Instructor operation HTTP | W5 server owner | W6, W7a, W7b |
| Instructor browser workflow | W6 SolidJS/TypeScript owner | W7a |
| Connected browser proof | W7a integration owner | W7 |
| Connected persistence proof | W7b data-access owner | W7 |
| Evidence closeout | W7 integrator | G2-G5 |

## Evidence boundary

Permanent tests prove deterministic behavior, exact ownership, typed contracts, state transitions, and answer-free serialization without network services, elapsed-time waits, or fixture bloat. PostgreSQL/RLS, migration replay, browser, screenshots, Graphify maps, SQL catalogs, and independent review are connected or one-time acceptance evidence. G1 is accepted only when the appropriate current contract lanes have recorded their evidence; historical aggregate results are never proof of the SD1-A5 correction.

