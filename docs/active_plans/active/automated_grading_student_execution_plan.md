# Plan: automated grading Student execution

## Status and authority

This focused plan owns WP-INST-G1 / G1-W4. It turns the accepted-input and pending-read foundations from G1-W2/W3 into one deterministic Student execution path: accept one exact Student response, grade through a leased server-owned worker, retain immutable evidence, recover deterministically, publish scores through the existing scorer, and let the Student observe safe status without resubmitting an answer.

The [automated grading operations plan](automated_grading_operations_plan.md) owns G1 architecture, objectives, cross-cutting authorization, migration allocation, W1-W3, and dependency order. The [automated grading execution contract](automated_grading_execution_contract.md) is binding for W4 capability, canonical evidence, state machine, lanes, and contract validation. The [automated grading operations delivery plan](automated_grading_operations_delivery_plan.md) owns W5-W7 and consumes only W4's answer-free handoff.

Historical G1 acceptance does not accept the WP-SD1-A5 correction. This document records the future W4 evidence necessary for that corrected contract and makes no runtime-acceptance claim.

## Outcome

For a shape-valid, entitled Student submission, the first durable server effect is one immutable accepted submission. The browser never carries a grade, feedback, private response replay, grader result, worker identity, or private evidence. A worker grades exactly the accepted server-private response and either commits one immutable completion or records a typed durable recovery state. A Student acknowledges pending work through a route-bound status read. Instructors recover only through W5's audited operation contract; they do not grade or alter scores directly.

## Preconditions and dependencies

- G1-W2 has supplied immutable accepted input, private response storage, exact replay, typed evaluation/execution state, append-only receipts, and the worker-only load boundary.
- G1-W3 has supplied the minimal answer-free pending/read projection and closed deterministic exception classification.
- Migrations 2026081851_accepted_submission_execution_schema.sql through 2026081860_accepted_submission_execution_fail.sql are allocated in the ledger before source edits.
- The execution contract's exact WorkerManifest, WorkerLease, canonical evidence, state transitions, and handler rules are accepted by the relevant owners.
- Existing migrations 1830 and 1831 remain the sole score-generation and current-score publication path.

## W4 contract in operational form

### Accepted input

The submission route validates bounded public shape, active Student entitlement, route witnesses, and idempotency before acceptance. Shape/timing failure is a pre-persistence 422. Once accepted, one transaction:

1. inserts the immutable submission keyed to exact course, Student, assignment, run, attempt, item, idempotency key, published question version, issued-evidence witness, and retention policy;
2. binds idempotency and request digest to that record;
3. stores browser response text only in the private composite-FK response child;
4. initializes evaluation as automated_pending and execution at generation 1;
5. appends an acceptance receipt; and
6. creates one closed accepted-submission execution job.

The public submission parent stores only a fixed answer-free marker. Its private child stores canonical UTF-8 response text and the SHA-256 over those exact bytes. Retention owns deletion. A replay with the same exact input resolves the same accepted work and current safe status; a changed actor, target, response digest, or idempotency identity conflicts without creating another job.

### Exact worker execution

The exact accepted-submission job contains references and digests, never a response or result. Generic next-ready and exact fast-path entry points share one claim state machine and return the same opaque tuple:

~~~
course, assignment, Student, run, attempt, submission, job,
lease token, execution generation, worker, manifest digest
~~~

The worker validates the complete lease/manifest/generation tuple before every private load, result commit, or failure transition. An inactive, expired, duplicate, superseded, or mismatched claim changes nothing. The synchronous fast path may acquire the exact lease as a latency optimization; it delegates to the same common handler as the background worker. A route never invokes the grader or writes a result directly after acceptance.

The common handler owns one validated execution deadline around its RunBackend.submit future. It translates presentation-bearing response data, validates the grading envelope, invokes deterministic grading, then requests exactly one commit-or-fail outcome. Timeout cancels the owned future before the single timed-out outcome. A known persistence error remains known. Only failure to acknowledge the final PostgreSQL transaction commit after a decoded successful result may surface as local OutcomeUnknown; later durable status and receipts remain authoritative.

### Completion and immutable evidence

Under one Memory state lock or PostgreSQL transaction-scoped completion lock, a successful execution:

- advances execution/evaluation state to completed/graded;
- creates the immutable answer-free completed receipt and normalized receipt snapshot;
- stores trusted result and feedback evidence;
- advances the run/enrollment lifecycle and typed scalar Student assignment summary;
- writes first-statistics input once; and
- invokes migration 1830 once to create an assignment scoring generation.

Every immutable result, receipt attempt, run, summary, optional presentation, and feedback source uses ple-canonical-json-v1: one compact UTF-8 source text, its SHA-256 over exact source bytes, canonical_json_version = 1, and a structurally equal queryable JSONB projection. The source-text byte bound is MAX_CANONICAL_JSON_V1_BYTES = 512 * 1024; feedback retains its smaller semantic bound. The implementation never hashes jsonb::text or reconstructed JSON.

A deterministic or integrity failure moves execution/evaluation to the closed exception state, appends an execution receipt, and creates or updates one recovery thread. A transient failure returns the job to ready with bounded backoff. Retry exhaustion becomes a typed exception. No completion path writes a current score directly.

### Recovery and score publication

Execution generation and assignment scoring generation are separate. An Instructor retry, through W5's exact actor-authorized broker, advances execution generation and creates a fresh accepted-submission job for the original immutable submission. The worker rechecks exact job, lease, manifest, evidence witnesses, and generation before it changes evaluation.

Successful finalization calls 1830 exactly once. The scoring worker separately locks its exact job, lease, assignment generation, and scoring status before migration 1831 publishes assignment/course scores and totals. A newer generation supersedes earlier work. Original accepted input, completed receipts, scoring evidence, and operation receipts remain immutable.

### Student status and browser behavior

W4 exposes a flattened, answer-free tagged union using the repository browser naming policy:

| kind | Safe fields | HTTP result |
| --- | --- | --- |
| completed | Established immutable completed receipt projection | 200 |
| accepted_pending | accepted: true, route-bound attempt_id, automated_grading_status, next_action: check_status | 202 |
| instructor_attention | Same closed pending fields | 202 |

The authoritative route is:

~~~
GET /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submission-status
~~~

It resolves the session-derived actor, exact Student course/assignment/run/attempt/submission witnesses, immutable question/evidence references, and receipt consistency in one authoritative read. It returns the same union as accepted POST replay. Contradictory partial state is unavailable, never reconstructed from mutable catalog content.

After a 202 acknowledgement, the Student client clears the response buffer and idempotency key, enters the visible acceptedPending state, and offers keyboard-accessible Check grading status. That action uses the status GET and never sends a second answer POST. Before acknowledgement, transport recovery may replay the buffered response because acceptance is not yet known.

## State and receipt rules

| Event | Execution/evaluation | Job | Receipt and publication |
| --- | --- | --- | --- |
| Accept | ready / automated_pending | One ready job | Acceptance receipt |
| Claim | running / automated_pending | One active lease | Running receipt |
| Grade | completed / graded | Completed | Completed receipt, then one 1830 request |
| Deterministic failure | exception / automated_exception | Terminal | Exception receipt and recovery thread |
| Transient failure | retry_wait / automated_pending | Ready at bounded retry time | Retry-wait receipt |
| Retry exhausted | exception / automated_exception | Dead | Exception receipt and safe reason |
| Instructor retry | new ready generation / automated_pending | Fresh exact job | Operation receipt |
| Stale claim | unchanged | unchanged | No receipt or score publication |

Append-only execution receipts record worker transition provenance. Append-only operation receipts record Instructor actions. Both record safe scope, actor-or-worker identity, request digest when present, expected/resulting revision or generation, category, and time. They exclude responses, answer keys, raw diagnostics, feedback internals, and score values.

One recovery thread is unique for the exact course, assignment, Student, run, attempt, and submission. An exact idempotent action replay returns the original receipt; a changed action identity, revision, actor, target, or command conflicts without mutation.

## Implementation lanes

| Lane | Owner and bounded responsibility | Completion handoff |
| --- | --- | --- |
| A | Rust contract and Memory: canonical evidence, exact claim/load/commit/fail traits, pure completion planner, one in-memory state machine | Type-safe execution and receipt behavior |
| A1 | Deployment/login: distinct recovery and fast-path login profiles, bounded private pools | Private process identity composition |
| A2 | PostgreSQL: typed wrappers over one private execution core, exact actor/witness read, held transaction through commit-v2 | Lease- and RLS-protected durable execution |
| B | Migrations 1851-1860: ordered schema/roles, integrity, authority, claim, read/load, lock, commit, fail layers | Closed SQL capability stack |
| C | Worker and dispatch: common leased handler, deadline, deterministic outcome mapping, fast/recovery adapters | One execution behavior for both callers |
| D | Server and Student client: first acceptance effect, route-bound status, strict decoder, visible pending recovery | Answer-free Student journey |

Lane A and B freeze a single evidence/authority foundation. A1 and A2 establish private process and database seams. C uses the resulting execution capability. D uses the completed receipt handoff and one shared handler. W5 begins after D supplies the stable answer-free operation handoff.

## W4 migration stabilization

Apply the ten migrations in three bounded experiments before broad source work resumes:

| Experiment | Ordered migration layers | Focused proof |
| --- | --- | --- |
| S1 | 1851 schema/roles, 1852 integrity, 1853 public authority, 1854 table authority | Closed memberships/function ACLs and no private canonical-column exposure |
| S2 | 1855 claim, 1856 verified read, 1857 load | One claim winner, sibling denial, exact actor/witness read |
| S3 | 1858 completion lock, 1859 commit-v2, 1860 fail | Compatible ordered tail, no-op second pass, tuple fencing, invalid failure inputs rejected before mutation |

Each experiment has one owner and its focused disposable proof. A failure returns the named migration layer to that owner. The clean-volume and second-pass checks are acceptance evidence, not permanent fixtures.

## Validation

### Permanent deterministic regression checks

Use controlled identities, lease tokens, generations, and injected authoritative time. Keep these checks offline and behavioral:

- accept-before-grader, exact replay, changed replay conflict, and one accepted job;
- generic/exact claim competition, tuple mismatch, expiry/reclaim, and stale-worker fencing;
- graded, deterministic exception, transient, timed-out, exhausted, and terminal transitions;
- canonical source-text hashing, typed decoding, and source/projection mismatch refusal;
- immutable completed receipts, lifecycle planning, and no score effect after rejected commit;
- route-bound pending/attention/completed convergence and client buffer clearing with no second answer POST;
- closed decoder refusal of answer-, feedback-, result-, or score-bearing pending bodies;
- one timeout-owned cancellation and a single durable outcome request; and
- the same validated worker setting for fast and recovery handlers.

These checks protect stable observable behavior without services, sleeps, network connections, source snapshots, pixel comparisons, or incidental timing targets.

### Connected and one-time acceptance evidence

W7b owns the fresh-database oracle for migrations 1851-1860, effective RLS/ACLs, private-read denial, role/profile composition, generic-versus-exact claim competition, lease/retry/reclaim/exhaustion, canonical evidence, receipt immutability, and the 1830/1831 publication boundary.

W7a owns the canonical HTTPS journey: a Student submits real ordinary course work, sees answer-free pending/attention/completed status, and an Instructor completes visible recovery through the W5/W6 surface. The built application, real worker, score publication, screenshot provenance, and visible 1280x800 Instructor review are connected acceptance evidence. It does not use mock-backed browser success as final proof.

## W5 handoff

W4 provides W5 only:

- answer-free operation reason/status vocabulary and exact course/assignment scope;
- closed retry/recalculate disposition, revision, and idempotency semantics;
- immutable question/evidence and receipt references for safe Instructor projection;
- an execution-generation retry trigger and 1830 score-publication trigger; and
- documented absence of private response, key, raw feedback, worker identity, and direct-score capability.

W5 owns Instructor authorization, operation list/action routes, UI DTOs, and visible recovery actions. W4 does not expand into Instructor route or browser ownership.

