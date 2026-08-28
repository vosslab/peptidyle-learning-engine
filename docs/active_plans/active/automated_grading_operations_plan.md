# Plan: automated grading operations

## Status

Implementation state: accepted on 2026-08-28 after independent architecture, security/privacy,
HCI, and repository-rules review plus final material-tree Validation. The shared
[allocation ledger](../implementation_status.md) assigns
`2026081849_automated_grading_operations.sql` and
`2026081850_accepted_submission_execution_load.sql` to this package. The ledger
also records the approved forward allocations for W4 and W5:
`2026081851_accepted_submission_execution_schema.sql`,
`2026081852_accepted_submission_execution_integrity.sql`,
`2026081853_public_function_authority.sql`,
`2026081854_accepted_submission_execution_authority.sql`,
`2026081855_accepted_submission_execution_claim.sql`,
`2026081856_accepted_submission_execution_read.sql`,
`2026081857_accepted_submission_execution_load.sql`,
`2026081858_accepted_submission_execution_completion_lock.sql`,
`2026081859_accepted_submission_execution_commit.sql`,
`2026081860_accepted_submission_execution_fail.sql`,
`2026081861_instructor_grading_operation_capabilities.sql`,
`2026081862_grading_operation_lifecycle_projection.sql`,
`2026081863_scoring_invalidation_origin.sql`, and
`2026081864_scoring_invalidation_capability.sql`.

W2 through W7 are accepted. The 99-migration live evidence includes readable migrations 1851
through 1869, second-pass no-op, compatibility, executable role/RLS proof, visible browser recovery,
WebWork grading, replica restart, and exact cleanup. Final `source source_me.sh && ./all_test.sh`
Validation passed with Rust/Wasm, 369 Node tests, 7,978 pytest checks, every production-browser
scenario, the connected service oracles, and exact cleanup. The W4 execution contract at
`docs/active_plans/active/automated_grading_execution_contract.md` freezes the
sealed worker capability, versioned canonical immutable evidence protocol, state transitions, lane
ownership, and focused evidence boundary for contract-paired dispatch.

Rust, frontend, and pytest checks remain permanent regression gates. The production-browser and
connected-service lanes, final aggregate, screenshots, and independent review are disposable
acceptance evidence for the accepted G1 boundary.

This plan extends the accepted T6 assignment workspace with assignment-local automated-grading
operations. It consumes the existing learner delivery, deterministic grading, immutable evidence,
and generation-fenced scoring foundations. It provides the contract G2, G3, and G5 will consume.

Parallel-plan ready: yes. The accepted-submission and operation contract is the serial foundation.
After that contract freezes, the persistence owner may implement Memory contracts and PostgreSQL
authority as two coordinated lanes. Exception routing, worker execution, HTTP, browser, and
connected proof then follow in dependency order.

## G1-W7 reconciliation addendum

The 2026-08-28 architect decision approves one semantic closeout transition split across four
atomic forward migrations. This addendum preserves the accepted G1 package scope and contract,
keeps the existing receipt history canonical, and records the green final material-tree Validation.

### Ordered migration work

Allocate the four identities before source edits, restore the accepted migration contents, then
implement and apply these bounded responsibilities in order:

| Migration | Bounded outcome | Depends on |
| --- | --- | --- |
| `2026081866_g1_receipt_provenance_schema.sql` | Clean-volume preflight plus closed provenance/category schema and constraints for both receipt tables. | Restored accepted migrations and empty receipt history. |
| `2026081867_g1_execution_receipt_writers.sql` | Acceptance, claim, and failure writers supply the closed category and exclusive actor-or-worker identity. | 1866 valid receipt schema. |
| `2026081868_g1_completion_receipt_writer.sql` | The frozen 36-input commit-v2 writer records the `graded` completion category under worker authority. | 1867 writer contract. |
| `2026081869_g1_instructor_receipt_writers.sql` | Final recalculation writer plus five-input private retry V2, public routing, and V1 retirement. | 1868 completion writer and final 1865 body. |

Migration 1866 first asserts that both `grading_execution_receipt` and
`grading_operation_receipt` are empty. A nonempty table raises an actionable failure before any
schema change. The migration then adds the required non-null fields and closed constraints while
preserving append-only rows, requiring explicit provenance, and retaining immutability. The
nonempty-history refusal is a one-time pre-production acceptance probe. Retained history
requires a separately approved immutable augmentation design.

Migration 1869 creates the five-input private `ple_prepare_accepted_submission_retry_v2`, routes
the unchanged public retry caller through its session-derived actor, revokes V1 execution, and
drops the four-input V1 function with `RESTRICT`. The connected denial oracle calls the actual
well-formed V2 capability as `ple_app` and requires SQLSTATE `42501`, establishing authorization
through the actual capability boundary.

### Reconciliation gates

- Before SQL edits, prove byte-for-byte restoration of the seven accepted migrations and record
  all four allocated identities in the shared ledger.
- On a clean disposable volume, apply the sequence once, apply it again as a no-op, pass
  compatibility, and verify checksum mutation detection. Separately record the explicit 1866
  refusal against a nonempty receipt fixture or database.
- Run the connected PostgreSQL/RLS and worker oracle for receipt category/state guards,
  actor-or-worker provenance, V2 owner/ACL/`SECURITY DEFINER`/search-path authority, absent V1
  capability and grants, forced RLS, direct table denial, lease and generation fences,
  retry/recalculation idempotency, and sole score publication.
- Rerun the production HTTPS browser path for answer-free learner and Instructor projections,
  network observations, and screenshots, then run `source source_me.sh && ./all_test.sh` on the
  exact final material tree.

The historical acceptance rule in [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) required
every gate named above. G1 passed those gates; the shared migration ledger remains the allocation
authority, and this section records the reconciliation delta and accepted boundary.

## Pre-G1 problem context

PLE already has a dependable scoring-recalculation pipeline. Migration `2026081830` advances an
assignment generation, marks the assignment recalculating, and creates bounded work. Migration
`2026081831` lets the sole scoring committer publish only privately staged work for an exact lease
and current generation. A newer generation supersedes old work without stale score publication.

The pipeline lacked an Instructor operation model. It had no typed, immutable record for a
deterministic grader exception; no answer-free assignment-local queue; and no replay-safe recovery
command. `worker_job` was mutable execution state. `audit_event` was generic history.
`manual_grade_receipt` belongs to the separately authorized human-grading capability.

Source inspection established an important foundation requirement. `submit_response` called the
grader before `submit_question_attempt`, while preparation hydrated and authorized without
persisting the browser response or idempotency key. A deterministic grader exception therefore
lost its retry input when the request ended. G1 fixed the state machine: an accepted, shape-valid
learner input is first persisted as the existing canonical immutable `submission`, then a
synchronous grader or a worker grades that exact server-private record. `submission_evaluation`
remains the grade outcome. The successful synchronous fast path remains valuable as an optimization
over this durable state machine, not its source of truth.

ADAPT confirms the value of question-first grading and learner alternates. PLE carries that
information scent forward with deterministic server-owned grading, immutable evidence, no-store
browser contracts, and an operation queue rather than mutable score editing or diagnostic exports.

## Objectives

1. Persist accepted learner input before grading so deterministic recovery has exact immutable
   server-private evidence.
2. Give a current Instructor one assignment-local automated-grading operation surface.
3. Route validated deterministic exceptions into typed state and immutable action receipts.
4. Provide bounded, idempotent, generation-fenced retry and recalculation actions.
5. Keep existing lease/private-stage/generation-fenced worker publication as the sole score path.
6. Keep browser contracts answer-free, metadata-only, strict, and no-store.
7. Make total trust, next action, and recovery explicit at the 1280 by 800 Instructor desktop.
8. Leave durable G2 inspection, G3 analysis, and G5 attention-queue handoff seams.

## Design philosophy

- **Accept before grade.** The canonical `submission` records exactly one immutable
  server-accepted learner input before any grader effect. Mutable execution, evaluation, and
  Instructor-operation projections each have one separate owner. Existing learner-record retention
  owns deletion of the immutable input at the approved horizon.
- **Operate on server-owned evidence.** Browsers select typed operations with concurrency and
  idempotency proof. Servers derive learner, assignment, tenant, grader, generation, and job facts.
- **Separate state from history.** Mutable operation state informs a next safe action; append-only
  receipts establish what was requested and accepted.
- **Keep one score publisher.** Existing private staging, lease checks, generation fences, and
  atomic publication remain authoritative.
- **Make the next action obvious.** Question-first rows, named controls, safe status language, and
  focus recovery let an Instructor resolve work without opening learner responses.
- **Prove the actual boundary.** Offline tests protect stable behavior. The disposable production
  HTTPS stack proves real UI, PostgreSQL/RLS, worker recovery, and current-total visibility.

## Scope

- The accepted G1 `submission` and `submission_idempotency` parents become immutable,
  answer-free accepted-input metadata before grading. One composite-FK
  `accepted_submission_private_response` child owns canonical UTF-8 `StudentResponse`, exact
  replay, forced RLS, and retention/purge ownership.
- Forward migration `2026081849_automated_grading_operations.sql` owns the operation/evaluation/
  execution schema prerequisite, receipts, and closed worker payload. Forward migration
  `2026081850_accepted_submission_execution_load.sql` owns the complete private accepted-input and
  active-lease execution boundary: child storage, atomic acceptance/exact replay, retention and
  append-only guards, forced RLS, caller role, and sealed execution loader. W4 owns ordered
  forward allocations `2026081851_accepted_submission_execution_schema.sql` through
  `2026081860_accepted_submission_execution_fail.sql`. They each own one independently verifiable
  capability: schema/roles, integrity, public-function authority, table authority, claim, verified
  read, load, completion lock, commit-v2, then fail. W5 owns `2026081861_instructor_grading_operation_capabilities.sql` for the Instructor
  broker and `2026081862_grading_operation_lifecycle_projection.sql` for worker-authoritative
  operation transitions. W5 also owns `2026081863_scoring_invalidation_origin.sql` for immutable
  source evidence, `2026081864_scoring_invalidation_capability.sql` for the one atomic generation,
  job, operation, and supersession boundary, and
  `2026081865_scoring_invalidation_source_bindings.sql` for source-specific witness adapters.
  Accepted migrations remain immutable.
- Assignment-local route `/courses/:courseRef/assignments/:assignmentRef/grading-operations`,
  question-first grouping, learner alternate, cursor pagination, and explicit recovery controls.
- Deterministic exception routing, bounded retry, and generation-fenced recalculation.
- Reuse of existing `2026081830` enqueue behind W5's canonical invalidation capability and
  `2026081831` scoring publication as the only recalculation and visible-score path. Every
  recalculation origin receives one immutable origin record and one generation-bound operation;
  a newer generation supersedes its older in-progress thread.
- Strict course/assignment authorization, transaction-time Instructor recheck, tenant RLS,
  non-enumerating failures, bounded inputs, redacted diagnostics, and no-store responses.
- Focused permanent behavior tests, PostgreSQL/RLS and worker connected proof, canonical real-stack
  browser journey, and semantic 1280 by 800 Instructor review.

## Non-goals

- G2 audited learner-work inspection or browser delivery of raw responses.
- G3 item/course analysis and replacement-impact evidence.
- G5 cross-course attention projection.
- A second scheduler, direct score-row mutation, browser-to-worker commands, or browser fields for
  grade values, correctness, partial credit, answer keys, rubrics, or raw learner responses.
- Expanding the existing separately authorized human-grading capability into automated operations.
- Evidence uses behavior, authorization, semantic accessibility, and durable state transitions.
  Graph inventories, exact pixels, incidental counts, and wall-clock performance observations
  remain implementation evidence when they inform a decision.

## Current-state evidence

Focused Graphify mapping identifies `ScoringGeneration`, `Worker`,
`enqueue_assignment_recalculation`, the assignment workspace, and strict decoder/route composition
as the relevant connected communities. Direct source remains authoritative.

- **Accepted input and evaluation:** `submission` and `submission_evaluation` already represent input
  and outcome, but both currently follow a successful grade. G1 makes `submission` immutable input
  and `submission_evaluation` the current outcome projection.
- **Assignment score trust:** positive `ScoringGeneration` and current/recalculating/failed status
  already exist. G1 links operations and receipts to that exact trust state.
- **Recalculation enqueue:** migration 1830 already advances generation, marks recalculating, and
  creates bounded work atomically. G1 reuses it as the only assignment-scoring enqueue capability.
- **Worker execution:** `scoring_worker.rs`, `worker.rs`, and queue contracts already provide exact
  leases, private preparation, and bounded durable failure handling. G1 extends this worker rather
  than creating another scheduler.
- **Score publication:** migration 1831 publishes only an exact current scoring generation and
  hands current data to G3 analysis. G1 retains it as the sole derived-score committer.
- **Instructor shell:** the T6 workspace, navigation, and strict client already provide exact
  assignment authority. G1 adds one local page and focused client seam.
- **Human grading:** the existing manual-grade route and receipt retain their separate authority.
  G1 proves automated commands cannot reach that mutation.

## W4 migration stabilization gate

This is a narrow pre-production stabilization gate, not a redesign of the
approved exact-claim algorithm. W4 source work resumes only after the sequence
below records `Keep` for every experiment. A failure stops at its first
actionable result, retains its disposable receipt, and returns only the named
migration layer to its owner.

| Experiment | Hypothesis | Single change | Success metric | Keep/Revert |
| --- | --- | --- | --- | --- |
| S1: schema through authority | The first four layers establish caller roles, one definer owner, closed public function authority, and exact data authority. | Apply 1851 schema/roles, 1852 integrity, 1853 public-function authority, then 1854 table authority. | 1851 proves the fast-path role has zero memberships; 1853 proves effective catalog closure after PUBLIC/default EXECUTE revocation and legacy v1 denial; 1854 proves receipt-version read while callers/app lack private canonical columns. | Keep each layer on its focused proof; repair its owning layer when the proof fails. |
| S2: claim through load | Separate callers compete through one transition while the read remains structurally route-bound. | Apply 1855 claim, 1856 four-key read, then 1857 load. | Generic/exact races have one winner; each caller is denied its sibling wrapper/internal transition; ready/expired maximum jobs converge; the reader follows same-transaction Rust entitlement. | Keep each layer on its focused proof; repair its owning layer when the proof fails. |
| S3: lock through fail and integration | Completion preserves one reusable handler and rejects invalid durable operations before mutation. | Apply 1858 lock, 1859 commit-v2, then 1860 fail; reapply the full migration command unchanged and run compatibility plus W7b. | Ordered apply reaches a compatible tail; second pass is a no-op; effective function ACLs are exhaustive; commit binds canonical version at position 7; NULL failure kind/reason returns `22023` without changes. | Keep the ordered stack on integration pass; repair the named failing layer when a focused or integration proof fails. |

If the focused experiments show the shared state machine cannot preserve
one-winner, tuple-fenced behavior, obtain an architect-approved replacement
before implementation continues.

## Approved architecture and contracts

The approved binding architecture freezes this section. W1 records the plan and allocation; W2
owns the production types and schema named here.

### Accepted learner input and grading lifecycle

The first server step validates bounded browser shape, session, assignment entitlement, active
attempt, and idempotency. Invalid shape remains a pre-persistence 422. For accepted input, the
server writes one immutable `submission` tied to tenant, learner/attempt, assignment item,
idempotency key, issued question/grader identity, and retention policy. Its private response child
is never a browser read DTO.

The accepted parent rows never change after insertion, apart from deletion by the existing retention
capability. For G1 contract-version-2 input, their generic `payload` fields contain only the fixed
answer-free marker `{"kind":"acceptedPrivateResponseV1"}`; their existing digest metadata remains
metadata. The composite-FK `accepted_submission_private_response` child is the sole response
authority. It contains canonical UTF-8 response text and its SHA-256 and is append-only except for
retention-owned deletion. `grading_execution` is the attempt-unique mutable execution owner: it carries
execution generation, state, retry bound, and current job/lease association.
`submission_evaluation` is the mutable current evaluation projection. Its canonical
`SubmissionEvaluationStatus` owns `automated_pending`, `automated_exception`,
`needs_manual_grading`, `graded`, and `exempt`; `ManualEvaluationRecord` may remain a focused view
for the separately authorized human-grading capability. `grading_operation` is the mutable,
Instructor-facing recovery-thread projection. Execution and operation receipts are append-only.

One acceptance transaction inserts the immutable `submission`, binds `submission_idempotency` to
that record and request digest, writes `submission_evaluation=automated_pending`, creates
`grading_execution` generation 1, appends its acceptance receipt, and enqueues one closed
accepted-submission execution job. Exact learner replay resolves the same immutable submission and
returns its current policy-projected grading status; it creates no second execution. A changed key,
response digest, attempt, or actor conflicts.

The request digest has one cross-store representation. A named Rust helper serializes the already
typed, closed `StudentResponse` exactly once with `serde_json::to_string`. Its UTF-8 bytes are the
canonical response bytes. Memory mirrors the public-metadata/private-response split, hashes those
bytes, and separately checks typed response equality. PostgreSQL bounds and parses the server-owned
text only to validate its object form, hashes the exact received UTF-8 bytes, and persists that text
only in the private child. The broker never hashes `jsonb::text`, reconstructs through
`serde_json::Value`, or accepts browser-owned raw JSON text.

State ownership and transitions are explicit:

- `grading_execution`: `ready -> running -> completed | exception | retry_wait | superseded`, with
  `retry_wait -> ready` under bounded broker backoff. An Instructor retry of `exception` advances
  execution generation and creates a new `ready` job; it does not resurrect the old job.
- `submission_evaluation`: `automated_pending -> graded | automated_exception`.
  An accepted Instructor retry may project `automated_exception -> automated_pending` for the new
  execution generation. Manual and exempt transitions remain owned by their existing capabilities.
- `grading_operation`: `actionable -> action_in_progress -> completed | repair_required | failed |
superseded`. Each accepted command advances its revision and appends a receipt.

Only the acceptance transaction creates the immutable input and initial projections. The exact-job
execution committer owns execution/evaluation changes and W4's typed scalar per-enrollment
completion projection. Instructor commands own operation revision and new execution/recalculation
requests. The assignment scoring committer owns derived attempt scores and the assignment/course
current-score and total trust state.

The successful execution transaction changes the current execution/evaluation projections, updates
the typed scalar `student_assignment_summary` per-enrollment completion projection, preserves the
immutable receipt summary source/projection/digest in `submission_receipt_snapshot`, appends an
execution receipt, and invokes `2026081830` to create the new assignment scoring generation.
Migrations 1830/1831 remain the generation-fenced assignment/course current-score publication path;
the assignment scoring committer publishes assignment and course scores and totals.
A deterministic failure changes execution/evaluation state, appends its receipt, and creates or
updates the recovery thread. A worker retry grades the exact persisted input, never ephemeral HTTP
data. Migration 1849 owns the operation/evaluation/execution schema prerequisite. Migration 1850
owns the coherent private accepted-response boundary: child, atomic acceptance/exact replay,
append-only/retention rules, forced RLS, broker/execution roles, and one read-only `SECURITY DEFINER`
loader. Given an exact active worker lease, accepted-submission identity, and execution generation,
the loader rechecks tenant, course, assignment, attempt, submission, job, and immutable
issued-evidence witnesses before returning one sealed server-private descriptor. It changes no
state, receipt, score, or assignment total.

`ple_app` may execute the acceptance broker but cannot read the child, `SET ROLE`
`ple_accepted_submission_execution`, or call the loader. Only `ple_worker_login` has SET-only
membership in the NOLOGIN/NOINHERIT `ple_accepted_submission_execution` capability; it has loader
execution and no child table privileges. The separate NOLOGIN reader owns only the loader's required
private reads. `PostgresAcceptedSubmissionExecutionStore` is the sole Rust implementation of
`AcceptedSubmissionExecutionStore`; general `PostgresStore` never implements that trait. This applies
ASVS 1.2.4, 1.5.2-1.5.3, 2.2, 2.3, 8.1-8.4, 11.4, 14.1-14.2, 15.3-15.4, and 16.2-16.5.

### Automated operation and receipt model

The domain owns closed values for operation target, deterministic exception class, state, action,
and outcome. Operation targets include assignment, public question reference, learner/attempt
reference, source submission, and source grading generation. The taxonomy distinguishes a safe
deterministic grader exception from invalid learner input, dependency outage, integrity failure, and
stale/superseded work. `RunBackendError` gains the approved closed deterministic-execution subtype,
separate from invalid issued authority and unavailable dependency failure.

Attempt-unique mutable `grading_execution` owns current execution generation, retry bound, and
state. Mutable `grading_operation` owns the public `GO-<positive>` reference, revision, current
state, and one explicitly derived safe next action.
Append-only `grading_execution_receipt` records execution transitions; append-only
`grading_operation_receipt` records Instructor actions. Both carry identity, scope,
actor-or-worker identity, request digest where applicable, expected/resulting generation or
revision, safe category, and time. They exclude the response, answer, key, private source, secret
diagnostic, feedback internals, and score values.

One submission recovery thread is unique for `(tenant, assignment, attempt, submission)` and its
first deterministic exception. Retries append receipts to that thread. A recalculation operation is
unique for `(tenant, assignment, requested_scoring_generation)` after the broker assigns the
generation. Question and learner grouping present these threads; grouping never creates them.
Every action has its own idempotency identity and expected operation revision. An exact action
replay returns the original receipt. The same key with a changed target, revision, actor, or command
conflicts without mutation. `worker_job`, `audit_event`, and `manual_grade_receipt` remain distinct
concepts.

The public learner field `automatedGradingStatus` has the closed symbolic values `pending`,
`graded`, and `instructor_attention`. The Instructor field `operationReason` has the closed safe
values `grader_contract_failure`, `grader_execution_failure`, `issued_evidence_integrity`,
`retry_exhausted`, and `scoring_recalculation_failed`. A server-owned mapping provides visible
messages. Backend text, tracing/provider data, answer material, private source, and feedback
internals never enter a public DTO or receipt.

### Recovery and score publication

An accepted action transaction rechecks current Instructor authority and the exact tenant/course/
assignment chain, appends its receipt, advances current operation state, and uses the established
enqueue capability where needed. It serializes assignment transition decisions so generations remain
monotonic.

Retry regrades the accepted immutable submission through one added closed accepted-submission
execution job payload in the existing worker family. The acceptance transaction creates that ready
job. The queue broker gains an exact-job lease command with the same tenant, worker, lease,
cancellation, completion, and failure rules as `claim_next_job`. The synchronous request may acquire
that exact lease and run the same private handler as a latency optimization. A concurrent request
and worker receive at most one lease; a request that does not acquire it returns the durable
`202 Accepted` projection. Transient dependency failure returns the job to ready with bounded
backoff, while deterministic and integrity failures commit their typed states and receipts.

Accepted-submission execution generation and assignment scoring generation remain distinct.
Instructor retry advances execution generation and creates a fresh accepted-submission job. Its
execution committer rechecks exact job, lease, submission identity, execution generation, and
issued-contract witnesses before changing the evaluation and calling `2026081830`.

Successful grade finalization invokes `2026081830`, which alone advances assignment scoring
generation and creates its recalculation job. The assignment scoring committer separately rechecks
its exact job, lease, assignment generation, and scoring status before `2026081831` publication.
Stale work at either generation is superseded and cannot publish. Original learner submission and
grade receipts remain immutable; operation receipts record recovery.

### HTTP and browser contract

The learner submission route remains the exact existing plural path:

```text
POST /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submissions
```

It returns an answer-free no-store flattened tagged learner union. The browser JSON field/
discriminant is lowerCamelCase `kind`; its cross-runtime symbolic values are snake_case
`completed`, `accepted_pending`, and `instructor_attention`. `200 OK` returns `completed` after
the exact leased fast path commits. `202 Accepted` returns `accepted_pending` for durable pending
work or `instructor_attention` for a typed execution exception. Pending and attention bodies contain
only acceptance, the route-bound `attemptId`, closed automated-grading status, and closed
snake_case `nextAction` value `check_status`; they exclude response, feedback, result, successor,
execution identity, and score. The browser owns the visible copy for `check_status`.
`422 Unprocessable Entity` remains only for shape/timing validation before accepted-input
persistence. A dependency outage after acceptance remains durable pending work and uses worker
backoff. Exact replay returns the current union projection for the same immutable submission.

```text
GET /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submission-status
```

The no-store status GET rechecks learner entitlement and the complete route witness, then returns
the same union. It allows an acknowledged learner to use visible **Check grading status** without a
second answer POST. A `202 Accepted` clears the response buffer and idempotency key, enters the
explicit `acceptedPending` client state, and provides that action; it does not restore or replay an
accepted answer. Transport recovery retains the buffered exact replay only before acknowledgement.

```text
GET  /api/courses/{course}/assignments/{assignment}/grading-operations
     ?groupBy=question|learner&cursor=<opaque>&pageSize=<bounded>
POST /api/courses/{course}/assignments/{assignment}/grading-operations/{operation}/retry
POST /api/courses/{course}/assignments/{assignment}/grading-operations/recalculate
```

`{operation}` is `GradingOperationReference` in its registered `GO-<positive>` public form. The
reference belongs in `crates/question_model/src/public_route.rs` and its reserved-prefix registry.
The server resolves it only within trusted tenant/course/assignment Instructor authority. Internal
UUID, job, attempt, and submission identities remain Store/server details.

The server establishes session, tenant, current Instructor authority, and exact course/assignment
relationship before interpreting optional values or returning facts. Learners, other tenants, and
stale memberships receive the common concealed no-store result.

Question grouping is the default because the first teaching question is "which item needs attention
across learners?" Learner grouping answers impact questions. A group contains public Question ID and
title or learner display identity, safe exception/status summary, affected-learner count, trust
generation/state, action eligibility, and stable cursor key. G2 owns protected learner-work detail.

Retry and recalculation POSTs are body-free. Required `Idempotency-Key` and `If-Match` headers carry
the bounded action identity and current operation or assignment revision. Strict no-store
receipts/list DTOs contain metadata only. Browser decoders reject unknown, answer-bearing, and
score-bearing fields and accept only the closed public status/reason values.

### Visible workflow

T6 assignment navigation gains **Grading operations** between **Policies** and **Student view**.
The header identifies assignment and scoring generation/status. A status band names the safe next
action; recalculating and failed totals are never presented as current or zero.

Rows lead with question title and `AAA-BBBB` ID or learner identity, then safe status, impact,
generation, and a named action such as **Retry automated grading for [question]**. Switching group
mode resets opaque cursor/action context and announces the new scope. Pending action controls expose
busy state without disabling unrelated work. Receipt, conflict, transport, and load recovery retain
context and direct focus to the next control.

The product-wide Instructor and Sysadmin visual profile is 1280 by 800 desktop 16:10. G1's visible
operation evidence is Instructor-only at that profile. A G1 Sysadmin surface uses the same profile
when that surface exists; student responsive evidence continues through ordinary learner journeys.

## Milestones

| M   | Title                | Summary                                           | Goal                                              |
| --- | -------------------- | ------------------------------------------------- | ------------------------------------------------- |
| M1  | Bind durability      | Freeze state ownership and migrations 1849/1850.  | Give every lane one contract.                     |
| M2  | Persist evidence     | Store immutable input, projections, and receipts. | Make recovery replayable and tenant-safe.         |
| M3  | Execute recovery     | Route exceptions through one leased worker path.  | Recover without browser answers or human scoring. |
| M4  | Expose teaching work | Add the strict client and assignment-local UI.    | Make the next safe Instructor action obvious.     |
| M5  | Prove the live loop  | Move one real exception to a current total.       | Establish end-to-end trust in recovery.           |

Milestone exit evidence is concrete: M1 requires this approved plan and the ledger allocation; M2
requires Memory conformance and the PostgreSQL/RLS oracle; M3 requires server, worker, and connected
race evidence; M4 requires offline client/model tests and semantic review; M5 requires the HTTPS
journey, connected oracles, and final Validation.

## Work packages

### G1-W1: bind accepted-input and operation contracts

- **Owner:** architect.
- **Outcome:** approved accepted-input, execution, evaluation, and operation ownership;
  canonical `SubmissionEvaluationStatus`,
  `GradingOperationReference`, operation taxonomy, private execution contract, retention/RLS
  ownership, receipt fields, and forward migration allocation.
- **Owned files/modules:** this plan and
  [implementation_status.md](../implementation_status.md). This plan is the durable decision record.
- **Dependencies:** accepted T6; existing 1830/1831 capabilities.
- **Implementation steps:** bind canonical reuse of immutable `submission`; distinct execution,
  evaluation, and operation state; idempotency/replay; `GO-<positive>` resolution; exact-job lease;
  private retry execution; retention/purge; safe public enums; receipts; and migrations 1849/1850.
  Confirm
  automated traits are structurally independent of `ManualGradingStore` and score mutation.
- **Permanent tests:** none; this package establishes contracts later behavior tests protect.
- **One-time/connected evidence:** Graphify/source lifecycle audit, migration/privilege review, and
  architect decision record.
- **Success criteria:** every dependent owner has one stable contract and allocated forward schema
  work; `submission` is accepted before every grader call; no retry depends on transient HTTP data.
- **Handoff:** publish approved types, fields, migration IDs, and source-boundary decision to W2-W7.

### G1-W2: persist accepted submissions and operation evidence

- **Owner:** expert coder, learning-data-access boundary.
- **Outcome:** one coherent private accepted-response and worker-only execution capability: answer-free
  accepted `submission`/`submission_idempotency` metadata parents plus one composite-FK
  `accepted_submission_private_response` child; `SubmissionEvaluationStatus`,
  `grading_execution`, `grading_operation`, execution receipts, and operation receipts remain
  brokered, tenant-scoped, retention-aware, and immutable where required.
- **Owned files/modules:** `crates/question_model/src/public_route.rs`; proposed
  `crates/question_model/src/grading_operations.rs`; proposed
  `crates/learning-data-access/src/contracts/grading_operations.rs`; proposed
  `crates/learning-data-access/src/grading_operations.rs`; relevant submission contracts and
  Memory/PostgreSQL submission owners; proposed Memory/PostgreSQL grading-operation modules; parent
  exports; `schemas/migrations/2026081849_automated_grading_operations.sql`;
  `schemas/migrations/2026081850_accepted_submission_execution_load.sql`; crate-local tests.
- **Dependencies:** W1.
- **Implementation steps:** 1849 creates only operation/evaluation/execution prerequisites. In 1850,
  serialize each typed `StudentResponse` once with the shared Rust helper; make Memory and PostgreSQL
  mirror the public-metadata/private-response split and hash the exact UTF-8 bytes; write fixed
  answer-free parent markers plus the composite-FK canonical-text private child atomically; enforce
  exact replay, append-only/retention deletion, and forced RLS. Create broker, reader, and
  NOLOGIN/NOINHERIT execution caller roles so API may call acceptance only, while worker login alone
  can SET the execution caller and call the lease-bound loader. Make
  `PostgresAcceptedSubmissionExecutionStore` the only `AcceptedSubmissionExecutionStore`
  implementation; keep API/general `PostgresStore` unable to load the child. Create current
  execution/evaluation/operation projections and append-only receipts; extend the closed worker
  payload/exact-job lease; implement conflict/replay; and call 1830 rather than duplicating
  assignment-scoring enqueue SQL. W2 protects the G1 version-2 input now. WP-P2 Persistent bindings
  follows with the legacy-consumer transition: it uses
  `/private/tmp/ple_g1_plan/g1_p0_ple_app_read_inventory.md` to replace each consumer of
  `submission`, `submission_idempotency`, `question_attempt`, and `worker_job` with a narrow
  capability, then reduces the corresponding broad grants. WP-P2 begins that phase after W2 exposes
  the protected input boundary and records its migration-allocation review before changing schema or
  privileges.
- **Permanent tests:** deterministic offline Rust contracts with inline values and controlled clocks
  cover Memory's public-metadata/private-response split, canonical variants and digest, exact replay,
  state transitions, receipt immutability, changed replay, tenant/course scoping, retention metadata,
  typed login-profile/role/store composition, and the fact that only the dedicated store has the
  execution trait. They use no services, sleeps, fixture files, PostgreSQL catalogs, or executable
  privilege checks.
- **One-time evidence:** apply the amended active migrations twice to a fresh disposable database and
  run the migration verifier. This clean-schema proof is not a permanent fixture.
- **Connected evidence:** W7b alone owns the PostgreSQL catalog and executable authority oracle for
  this boundary: API cannot assume execution, execute the loader, or read child/sensitive fields; a
  worker exact claim alone receives matching response and issued evidence; invalid tenant/lease/
  submission/generation/retention facts return no descriptor. Migration `DO` catalog assertions are
  schema self-verification, not a mock or migration-text snapshot test.
- **Success criteria:** an accepted input survives grading failure safely; one accepted action yields
  one receipt and at most one matching enqueue; public operation projections contain no answer or
  score material.
- **Handoff:** W3-W5 receive typed submission, operation, receipt, action, and the worker-only
  execution-store capability; W4 composes that narrow handle only into the accepted-submission
  handler and does not alter grants, roles, or loader authority.

### G1-W3: stabilize typed pending reads and classify outcomes

- **Owner/package:** expert coder, `WP-INST-G1 / G1-W3`, learner
  submission/grader boundary.
- **Depends on:** G1-W2. W4 and W5 wait for the stabilization gate below.
- **Owned artifacts:** the two `submission_record` matches in
  `crates/server/src/run/queries.rs`; the established accepted-pending
  replay/read helper in `crates/server/src/run/submission.rs`; the matching
  typed read arm in `crates/server/src/run/external_tool.rs`; a minimal
  server-side answer-free `accepted_pending` 202 projection/helper with
  lowerCamelCase `kind`, `attemptId`, `automatedGradingStatus`, and `nextAction`
  fields for those established replay states; the closed `RunBackendError`/
  `SubmissionDisposition` mappings in native, WebWork, QTI, and composite
  backends; and an explicit preservation decision for the iMathAS broker's
  separate opaque `Invalid`/`Unavailable`/`Unsupported` mapping.
- **Required behavior:** preserve pre-acceptance 422 validation; map
  `Missing`, `AcceptedPending`, and `Completed` deliberately; return the final
  minimal no-store 202 projection for an already accepted pending replay; and
  classify validated deterministic grader failures through the closed operation
  taxonomy. Manual grading retains its path and dependency outages remain
  unavailable. The generic W4 accepted-submission worker does not replace the
  authenticated one-use iMathAS broker: its launch/session/binding refusals
  remain opaque, its provider and object-store outages remain unavailable, and
  its committed replay stays atomic on the external-tool path. W3 makes no
  first-effect acceptance call, exact claim, outcome commit, job mutation, or
  learner-client mutation.
- **Accepted stabilization evidence:** the named read/replay call sites, minimal
  accepted-pending helper, backend mappings, and canonical Memory learner read
  are green under the focused W3 gates and independent architecture and
  security/privacy approval. W4 consumes these contracts while retaining
  ownership of first-effect acceptance, claims, outcomes, and learner state.
- **Permanent offline gate:** `cargo fmt --check`, `cargo check -p server_core`,
  and focused deterministic pending/read, 202-helper, outcome-matrix, and
  external-tool provider-bypass tests. The pending read proof covers list and
  single-attempt projections without fabricating a completed receipt. Use
  controlled backend results and issued evidence without services, sleeps,
  current time, randomness, or answer-bearing responses.
- **Connected one-time gate:** exercise typed pending/read mapping,
  deterministic exception, and dependency-unavailable outcomes on the
  disposable stack after the compile gate.
- **Handoff:** send W4 the final minimal pending projection, pending/completed
  read shape, exception category, operation target/revision, and private-loader
  invariant. Send W5 only answer-free Instructor action semantics and bounds.

### G1-W4: accept, execute, and recover learner grading

- **Owner/package:** expert coder, `WP-INST-G1 / G1-W4`, learner acceptance,
  worker/scoring, and persistence-capability boundary.
- **Depends on:** G1-W3 green stabilization gate, W2 accepted SQL/security
  review, and the W4 migration stabilization gate. Migrations
  `2026081851_accepted_submission_execution_schema.sql` through
  `2026081860_accepted_submission_execution_fail.sql` are allocated
  before source edits.
- **Binding contract:**
  `docs/active_plans/active/automated_grading_execution_contract.md` freezes
  W4's sealed claim/load/completion capability, immutable answer-free receipt,
  `ple-canonical-json-v1` evidence protocol, state transitions, lane ownership, and focused
  evidence boundary. Its explicit identity, capability, transition, and
  immutable-evidence boundaries prepare the later database-normalization
  roadmap package without expanding W4's focused operation boundary.
- **Owned artifacts:** the first-effect branch and helpers in
  `crates/server/src/run/submission.rs`; learner delivery serialization in
  `crates/server/src/run/support.rs`; the route-bound status GET in the run
  router/query owner; `CompletedSubmissionReceipt`, the crate-private
  `submission_completion.rs` lifecycle planner, and
  `AcceptedSubmissionExecutionWorkerStore`; the crate-private
  `crates/learning-data-access/src/canonical_json.rs` owner for versioned
  evidence source text; Memory/PostgreSQL
  claim/load/lock/commit-v2/fail parity; `jobs.rs`,
  `in_memory/queue.rs`, and `postgres/jobs.rs`; worker, scoring-worker, and
  worker-composition modules; `src/api/contracts.ts`, `src/api/decoders/run.ts`,
  `src/api/http_client/request.ts`, `src/features/attempt/attempt_state.ts`, and
  `src/api/client.ts`, `src/api/http_client/response.ts`, and focused learner
  page/presentation modules; focused tests; and migrations 1851 through 1860.
- **Required behavior:** validate public response shape and learner route
  witnesses without invoking a grader, then call `accept_automated_submission`
  once. W2 persists immutable input, replay evidence, pending projection,
  execution receipt, and the exact ready job. W4 implements the binding
  contract's split caller roles/private pools and one shared handler; a
  non-winner returns pending. The handler uses W2's worker-only loader, grades, and calls W4's atomic
  completion capability. Automated execution produces only graded
  results with trusted result evidence and feedback; an exemption remains an
  authorized Instructor/policy lifecycle transition and a general read state.
  Under one state lock or PostgreSQL
  transaction-scoped completion lock, the backend builds the immutable,
  answer-free `CompletedSubmissionReceipt` and persists the exact completed
  attempt, feedback, normalized receipt snapshot, run/enrollment transition,
  typed scalar per-enrollment summary update, and one-time statistics inputs.
  Every immutable result, receipt
  attempt, run, summary, optional presentation, and feedback value uses one
  `ple-canonical-json-v1` source text plus a structurally equal queryable JSONB
  projection; SHA-256 always attests to the source text,
  `canonical_json_version = 1` records the protocol, and
  `MAX_CANONICAL_JSON_V1_BYTES = 512 * 1024` applies the existing broker JSON
  ceiling while feedback retains its stricter semantic budget. The immutable
  receipt summary source/projection/digest belongs to
  `submission_receipt_snapshot`; `student_assignment_summary` remains the typed
  scalar current projection. Migrations 1856/1857 load and lock return named scalar
  summary fields, and commit-v2 validates exactly `tenant`, `enrollment`,
  `currentScore`, `bestScore`, `latestScore`, `completedRunCount`,
  `totalQuestionAttempts`, and `lastActivityAt` before writing those scalars.
  The capability has exactly 36 positional values, with canonical JSON version
  at position 7 before evaluation status. The attempt's immutable
  issuance payload remains unchanged while relational lifecycle fields advance;
  the pure planner composes the
  existing lifecycle helpers after coherent validated inputs; backend callers
  retain private-response, feedback, checksum, authorization, and persistence
  authority. It produces no `attempt_score_current` write. Exact claim, load,
  lock, commit-v2, and fail recheck tenant, job, token, unexpired lease,
  submission, worker, and execution generation. A lost, expired, duplicate, or
  superseded claim returns the typed lease-loss/conflict disposition and leaves
  durable state unchanged. Success invokes 1830 exactly once; migrations
  1830/1831 remain the generation-fenced assignment/course current-score
  publication path. The pre-production migration applies this protocol on a
  clean live-demo baseline, so no historical JSONB
  row needs a reconstructed source text. Presentation-bearing response
  translation and envelope validation run inside the common leased handler, so
  a post-acceptance integrity failure becomes typed `instructor_attention`.

  W4 extends W3's minimal helper into a flattened learner tagged union. Its
  browser JSON field/discriminant is lowerCamelCase `kind`; its cross-runtime
  symbolic values are snake_case `completed`, `accepted_pending`, and
  `instructor_attention`. `completed` contains the established receipt
  projection sourced from the immutable completed receipt. The two pending
  alternatives contain only `accepted: true`, the
  route-bound `attemptId`, `automatedGradingStatus`, and closed snake_case
  `nextAction: "check_status"`; the browser maps that action to visible copy.
  They contain no response, feedback, result, successor, execution identifier,
  or score. W4 adds the no-store route-bound
  `GET /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submission-status`;
  it rechecks learner entitlement and the full route witness in the same Rust
  transaction before calling the four-key verified integrity reader, which
  returns only the existing app-readable projection and does not authorize an
  actor independently. It returns the
  same union. Receipt replay, status GET, and accepted POST replay converge on
  the completed aggregate after evaluation; partial or contradictory durable
  state resolves as an unavailable closed failure rather than a reconstructed
  receipt.

  A `202 Accepted` clears the response buffer and idempotency key, enters
  `acceptedPending`, and presents visible keyboard-accessible **Check grading
  status**. That action invokes the status GET, retains pending with its visible
  acknowledged state, or consumes `completed`; it never replays an answer POST. Before
  acknowledgement, transport recovery retains the buffered exact replay because
  acceptance is unknown.
- **C1/C2 execution rule:** C1's common handler is the sole owner of one
  validated execution deadline. It times out only its owned `RunBackend::submit`
  future; elapsed timeout cancels that future before one `TimedOut` outcome and
  one commit-or-fail request. Its capability-specific
  `AcceptedSubmissionCommitError` maps `Known(StoreError)` to the original
  error and only `OutcomeUnknown` to the local unknown result. PostgreSQL may
  return the latter only for final transaction-commit acknowledgement after a
  decoded function result; begin, function/statement, decode, validation, and
  pre-commit failures are known. Production Memory is known-only outside a
  deterministic test seam. C2 reads the existing positive, lease-shorter
  `WorkerSettings` value once through a semantic validated accessor and passes
  it to both fast and recovery C1 callers. C2 adds no timeout or outcome writer.
- **Permanent offline gate:** deterministic Memory/planner/handler/client tests
  cover first acceptance before any grader call, accepted replay, competing
  claims, bad token, controlled expiry/reclaim, tuple mismatch, success,
  deterministic exception, retry/exhaustion, known commit propagation, injected
  acknowledgement ambiguity, immutable
  completed receipt preservation, and no aggregate or score effect after a
  rejected commit. The planner test uses structured lifecycle outputs to prove
  existing completion and first-statistics rules, rather than implementation
  shape or byte/pixel equivalence. Status tests cover exact route binding,
  pending/attention/completed convergence, and unavailable contradictory state.
  Client tests cover closed union decoding, rejection of mixed/
  answer-bearing/feedback-bearing/score-bearing pending bodies, and 202 buffer
  clearing into `acceptedPending` with status-read recovery and no answer POST
  replay. Canonical-evidence tests exercise source-text hashing, typed decoding,
  and structural JSONB-projection equality, including altered source/digest and
  mismatched projection failures. A paused-time handler test uses a pending fake
  backend with drop signalling to prove cancellation before one `TimedOut`
  commit, one submit, one commit request, and the durable disposition. C2 tests
  prove the same validated worker setting reaches both handler callers. Use
  explicit lease/generation values and no sleeps, wall-clock expiry, network,
  or live database.
- **Connected one-time gate:** W7b runs the fresh-database claim/load/
  transaction-lock/commit-v2/fail oracle with explicit IDs, lease tokens,
  workers, and generations. It demonstrates canonical source-text/digest
  verification and source-to-JSONB structural equality for normal, manual, and
  automated writers, immutable receipt normalization, private-response denial,
  replay/status convergence, and ordinary synchronous completion versus
  accepted-worker completion parity for equal run, enrollment, and scalar
  summary results. It also proves 1830 enqueue and the 1831-only
  assignment/course current-score publication path. W7a runs the visible learner
  pending-to-completed status journey on the built HTTPS stack.
- **Handoff:** send W5 closed retry/recalculate results and metadata-only
  operation projections. W6 receives no learner contract work and remains the
  Instructor operations page. Send W7a/W7b the status route, tagged union,
  claim/load/lock/commit-v2/fail names, fault hook, visible learner states,
  role assumptions, canonical evidence protocol, immutable-completion invariant, and 1830/1831 score
  receipt, including known function failure and the reviewed final-
  acknowledgement fault proof that only final commit acknowledgement becomes
  `OutcomeUnknown`.

## Delivery detail

The binding architecture, dependency order, and contract decisions remain in this plan. The
focused W5-W7 delivery, validation, risks, acceptance, and dispatch details live in the linked
[automated_grading_operations_delivery_plan.md](automated_grading_operations_delivery_plan.md).
The sibling is an execution
companion: it refines ownership and evidence without changing scope or contracts.

## Dependency graph

```text
T6 assignment workspace
       |
       v
G1-W1 contract + migration allocation
       |
       v
G1-W2 accepted submission + operation persistence
       |
       v
G1-W3 typed pending reads and classification
       |
       v
G1-W4 learner acceptance, execution, and status
       |
       v
G1-W5 strict HTTP boundary
                   |
                   v
        G1-W6 Instructor operations page
                   |
                   v
        G1-W7a visible recovery journey

G1-W4 worker execution + G1-W5 HTTP boundary
                   |
                   v
        G1-W7b PostgreSQL/worker oracle

        G1-W7a + G1-W7b
                   |
                   v
        G1-W7 closeout and handoff
```

## Ownership and file matrix

- **Architecture/allocation - architect:** W1 plan and ledger. Freeze the decision before source
  edits.
- **Accepted input/execution/receipts - expert coder:** W2 domain/LDA contracts, stores, and
  migration. Own schema, RLS, retention, and transactions.
- **Typed pending/classification - expert coder:** W3 submission read/replay and backend modules.
  Own the minimal accepted-pending helper and closed classification.
- **Learner acceptance/execution - expert coder:** W4 first-effect submission, worker/scoring/queue,
  route-bound status, learner delivery contracts, and learner client state. Retain sole
  derived-score authority.
- **HTTP authorization - expert coder:** W5 focused course route and policy. Own public
  representations and authorization.
- **Instructor browser workflow - TypeScript/Solid engineer:** W6 Instructor operations decoder,
  client, workspace page, and nav. Consume W5 DTOs and preserve no-store.
- **Visible connected proof - Playwright/integration engineer:** W7a scenario, fault orchestration,
  screenshots, and provenance. Reuse the existing fixture seam and production runner.
- **Persistence connected proof - expert coder:** W7b PostgreSQL oracle and baseline registration.
  Reuse the disposable database baseline.
- **Evidence/closeout - integrator:** W7 status, changelog, reviews, and final Validation record.
  Advance after both connected evidence handoffs.
