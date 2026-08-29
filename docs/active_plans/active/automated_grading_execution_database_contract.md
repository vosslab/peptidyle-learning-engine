# Automated grading execution database contract

## Purpose and authority

This binding database contract is the PostgreSQL companion to the
[automated grading execution contract](automated_grading_execution_contract.md).
The paired contract owns the immutable execution, evidence, state-transition,
handler, route, and learner-status semantics. This document owns their
PostgreSQL realization: migration order, roles, functions, RLS, grants,
transaction boundaries, recovery capability, and connected database proof.

Both documents belong to `WP-INST-G1 / G1-W4`. The
[automated grading operations plan](automated_grading_operations_plan.md) owns
the package scope and dependency order. The
[implementation status ledger](../implementation_status.md) is the sole
mutable migration-allocation authority. This split changes document ownership
only; it does not change the deterministic server-owned grading model or any
Human Guidance decision.

## Graphify and source evidence

One-time navigation on 2026-08-29 used Graphify at commit
`dc227871d18d`, followed by direct inspection of current source. The focused
traversal reached the W4 migration nodes for `2026081851` and `2026081860`,
the `AcceptedSubmissionExecutionStore` implementations, PostgreSQL migration
registration, worker tests, and the live database baseline. Current source
verification narrowed the database boundary to:

- `schemas/migrations/2026081851_accepted_submission_execution_schema.sql`
  through `2026081860_accepted_submission_execution_fail.sql`.
- `crates/learning-data-access/src/postgres/grading_operations.rs`,
  `postgres/submission.rs`, `postgres/submission_receipts.rs`, and the
  connection/pool composition modules.
- `crates/learning-data-access/tests/postgres_automated_grading_operations_live.rs`
  and its focused broker, receipt, recovery, retry, and completion modules.
- `tests/e2e/e2e_database_baseline.sh` and the migration registry in
  `docs/active_plans/implementation_status.md`.

Graphify and inventory output are one-time navigation evidence. Current
migration files, Rust callers, and executable database tests remain the
authority. The map does not become a permanent source-inventory test.

## Database scope

W2 migrations `2026081849` and `2026081850` provide immutable accepted input,
the private response child, retention and RLS foundations, the ready job, and
the initial sealed loader. W4 owns the ordered forward allocations
`2026081851` through `2026081860`. W5 begins at `2026081861` only after this
database boundary is stable. W4 preserves explicit identity, capability,
transition, and immutable-evidence boundaries; broader relational
normalization belongs to its owning package.

The database never grades a browser answer. Rust creates the deterministic
plan and canonical evidence; PostgreSQL verifies the closed inputs, holds the
required locks, and applies the one atomic transition. PostgreSQL does not
reconstruct immutable evidence from mutable catalog rows or reproduce the
scoring algorithm in PL/pgSQL.

## Migration decomposition

The ten W4 migrations are independently reviewable and independently atomic.
The installer applies them in order and does not launch an API or worker until
the migration tail is compatible. W5 receives `2026081861` in the ledger.

| Migration | Database capability | Focused proof |
| --- | --- | --- |
| `2026081851_accepted_submission_execution_schema.sql` | Roles/schema, fast-path caller, `active_worker_id`, and canonical-evidence columns. | Exact role shape, zero memberships, no direct data authority. |
| `2026081852_accepted_submission_execution_integrity.sql` | Immutable evidence guards and triggers. | Invalid and repeated immutable writes fail. |
| `2026081853_public_function_authority.sql` | PUBLIC/default EXECUTE revocation and legacy loader retirement. | Effective catalog has no unintended executable path. |
| `2026081854_accepted_submission_execution_authority.sql` | Witness, forced RLS/policies, table/sequence authority, and receipt version access. | Definer read succeeds; caller/app direct private reads fail. |
| `2026081855_accepted_submission_execution_claim.sql` | Owner-only transition, generic/exact claims, and ready/max convergence. | One winner; sibling wrappers and internal transition are denied. |
| `2026081856_accepted_submission_execution_read.sql` | Exact ownership and immutable-evidence reader. | Entitled route succeeds; changed keys and foreign targets fail. |
| `2026081857_accepted_submission_execution_load.sql` | Exact private execution load wrapper. | Matching claim loads once; mismatch loads nothing. |
| `2026081858_accepted_submission_execution_completion_lock.sql` | Exact completion-lock wrapper. | Stale or duplicate claim cannot acquire completion rows. |
| `2026081859_accepted_submission_execution_commit.sql` | Ordered commit-v2 wrapper. | One graded completion writes the immutable aggregate. |
| `2026081860_accepted_submission_execution_fail.sql` | Closed NULL-aware failure wrapper. | Invalid NULL vocabulary raises `22023` without state changes. |

Migration 1851 gives the fast-path caller NOLOGIN, NOINHERIT, no memberships,
schema USAGE only, and no direct relation, sequence, or unrelated function
authority. Migration 1852 applies immutable guards. Migration 1853 revokes
global PUBLIC function EXECUTE and migration-owner default EXECUTE, proves the
effective ACL set, and retires `ple_load_accepted_submission_execution_v1`.
Migration 1854 establishes the witness and sealed table authority, grants the
definer only the receipt columns it reads, and keeps private canonical source,
digest, and version columns away from `ple_app`.

Migration 1851 adds `grading_execution.active_worker_id uuid NULL` to fence the
lease holder and adds the nullable automated-result source-text/digest pair to
`submission_evaluation`. Pair presence, size, digest, and focused-update
guards apply; the existing `payload` and `payload_sha256` remain the queryable
projection. Migration 1852 gives every deterministic receipt writer the same
immutable `submission_receipt_snapshot` source-text, projection, digest, and
`canonical_json_version` invariant. A normal or automated completed receipt
uses `Submitted`; the accepted-submission completion branch additionally
requires `Submitted` plus `graded`. The normalized snapshot is the source for
replay and status reads and never consults mutable catalog state.

## Roles and function authority

`ple_accepted_submission_execution_worker` remains the membership-free NOLOGIN
definer owner and is not a process capability. The
`ple_accepted_submission_execution` recovery login/pool may execute only the
generic claim wrapper. The
`ple_accepted_submission_execution_fast_path` login/pool may execute only the
exact claim wrapper. Each membership is `SET TRUE`, `INHERIT FALSE`, and
`ADMIN FALSE`; neither process receives a broad table role or account/session
table access. Both wrappers delegate a winning lease to the shared Rust
handler.

Migrations 1855 through 1860 use `SECURITY DEFINER` wrappers with
`SET search_path TO 'pg_catalog', 'public', pg_temp` and server-owned time.
The shared claim transition is owner-only `SECURITY INVOKER` and has no public
grant. Every capability assertion proves owner, security-definer flag, fixed
search path, signature, and the complete non-owner execute ACL via
`aclexplode`, including grant options. Trigger functions have an empty
external ACL; read is app-only; load, lock, commit, and fail are granted only
to the two execution caller roles.

The public function surface is deliberately narrow. The signatures below are
the registered SQL boundary; the exact return payloads are intentionally
private except for the safe evaluation projection:

```sql
public.ple_claim_accepted_submission_execution_v1(
    p_lease_token uuid, p_worker_id uuid, p_lease_seconds integer
) RETURNS TABLE (
    course_id uuid, assignment_id uuid, student_id uuid, run_id uuid, attempt_id uuid,
    worker_job_id uuid, worker_lease_token uuid, submission_id uuid,
    question_id text, question_version_id uuid, evidence_digest character(64),
    execution_generation bigint, worker_id uuid, manifest_digest character(64)
);

public.ple_claim_exact_accepted_submission_execution_v1(
    p_course_id uuid, p_assignment_id uuid, p_student_id uuid, p_run_id uuid, p_attempt_id uuid,
    p_submission_id uuid, p_question_id text, p_question_version_id uuid,
    p_evidence_digest character(64),
    p_worker_job_id uuid, p_lease_token uuid, p_worker_id uuid,
    p_lease_seconds integer
) RETURNS TABLE (
    course_id uuid, assignment_id uuid, student_id uuid, run_id uuid, attempt_id uuid,
    worker_job_id uuid, worker_lease_token uuid, submission_id uuid,
    question_id text, question_version_id uuid, evidence_digest character(64),
    execution_generation bigint, worker_id uuid, manifest_digest character(64)
);

public.ple_read_accepted_submission_evaluation_v1(
    p_actor_user_id uuid, p_session_id uuid, p_course_id uuid,
    p_assignment_id uuid, p_student_id uuid, p_run_id uuid,
    p_attempt_id uuid, p_submission_id uuid, p_question_id text,
    p_question_version_id uuid, p_evidence_digest character(64)
) RETURNS TABLE (evaluation_payload jsonb);

public.ple_load_accepted_submission_execution_v2(
    p_course_id uuid, p_assignment_id uuid, p_student_id uuid, p_run_id uuid, p_attempt_id uuid,
    p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_question_id text, p_question_version_id uuid,
    p_evidence_digest character(64), p_execution_generation bigint,
    p_worker_id uuid, p_manifest_digest character(64)
) RETURNS TABLE (
    worker_job_id uuid, worker_lease_token uuid, execution_generation bigint,
    worker_id uuid, execution_state text,
    accepted_course_id uuid, accepted_student_id uuid, accepted_run_id uuid,
    accepted_assignment_id uuid, accepted_attempt_id uuid,
    accepted_submission_id uuid, accepted_actor_id uuid,
    accepted_question_id text, accepted_question_version_id uuid,
    accepted_evidence_digest character(64), accepted_manifest_digest character(64),
    accepted_idempotency_key text, accepted_request_sha256 character(64),
    accepted_millis bigint, response_canonical_json text,
    attempt_payload jsonb, attempt_payload_sha256 character(64),
    presentation_descriptor_version smallint, presentation_nonce bytea,
    presentation_digest bytea, presentation_capability text,
    presentation_payload jsonb, presentation_payload_sha256 character(64),
    grading_envelope_payload jsonb, grading_envelope_payload_sha256 character(64),
    issued_question_snapshot_payload jsonb,
    issued_question_snapshot_payload_sha256 character(64),
    flat_required boolean, flat_payload jsonb, flat_payload_sha256 character(64),
    webwork_required boolean, webwork_payload jsonb,
    webwork_payload_sha256 character(64), webwork_replay_payload jsonb,
    webwork_replay_payload_sha256 character(64), qti_required boolean,
    qti_payload bytea, qti_payload_sha256 character(64)
);

public.ple_lock_accepted_submission_completion_v1(
    p_course_id uuid, p_assignment_id uuid, p_student_id uuid, p_run_id uuid, p_attempt_id uuid,
    p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_question_id text, p_question_version_id uuid,
    p_evidence_digest character(64), p_execution_generation bigint,
    p_worker_id uuid, p_manifest_digest character(64)
) RETURNS TABLE (
    /* exact private completion input, including the private accepted response
       and named scalar summary fields */
);

public.ple_commit_accepted_submission_completion_v2(
    p_course_id uuid, p_assignment_id uuid, p_student_id uuid, p_run_id uuid, p_attempt_id uuid,
    p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_question_id text, p_question_version_id uuid,
    p_evidence_digest character(64), p_execution_generation bigint,
    p_worker_id uuid, p_manifest_digest character(64),
    p_canonical_json_version smallint, p_evaluation_status text,
    p_evaluation_canonical_json text, p_evaluation_sha256 character(64),
    p_attempt_canonical_json text, p_attempt_payload jsonb,
    p_attempt_payload_sha256 character(64), p_feedback_canonical_json text,
    p_feedback_content_sha256 character(64), p_run_canonical_json text,
    p_run_payload jsonb, p_run_payload_sha256 character(64),
    p_run_current_canonical_json text,
    p_run_current_payload_sha256 character(64),
    p_run_completed_at_millis bigint,
    p_enrollment_first_completed_at_millis bigint,
    p_enrollment_current_grade_run_id uuid, p_enrollment_best_grade_run_id uuid,
    p_summary_canonical_json text, p_summary_payload jsonb,
    p_summary_payload_sha256 character(64), p_presentation_canonical_json text,
    p_presentation_payload jsonb, p_presentation_payload_sha256 character(64),
    p_presentation_required boolean, p_assignment_item_id uuid,
    p_statistics jsonb, p_expected_scoring_generation bigint,
    p_recalculation_job_id uuid, p_recalculation_max_attempts integer
) RETURNS TABLE (
    disposition text, resulting_execution_state text,
    resulting_evaluation_status text
);

public.ple_fail_accepted_submission_execution_v1(
    p_course_id uuid, p_assignment_id uuid, p_student_id uuid, p_run_id uuid, p_attempt_id uuid,
    p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_question_id text, p_question_version_id uuid,
    p_evidence_digest character(64), p_execution_generation bigint,
    p_worker_id uuid, p_manifest_digest character(64),
    p_failure_kind text, p_operation_reason text
) RETURNS TABLE (
    disposition text, resulting_execution_state text,
    resulting_evaluation_status text
);
```

The exact SQL signatures are registered by migrations 1855 through 1860; the
metadata-only Rust traits and result semantics remain in the paired execution
contract. No route calls a private function directly.

## RLS and ownership checks

The read broker follows the canonical actor-entitlement check in the same Rust
transaction; it is an integrity-and-route verifier, not a second actor
authorization API. It structurally verifies the exact
attempt -> run -> enrollment -> assignment -> course chain, the immutable
question/evidence references, and the completed receipt before returning the
app-readable evaluation projection. A wrong actor, course, assignment, run,
attempt, submission, question, evidence digest, route, or revoked membership
returns the same concealed no-row/unavailable result.

The load, completion-lock, commit, and fail wrappers recheck the complete
course/assignment/Student/run/attempt/submission/job/lease/worker/generation/
manifest tuple. Direct reads of private canonical source, response, key,
feedback, and diagnostic columns remain denied. Forced RLS, explicit policies,
column grants, and function ACL assertions are all part of the database
boundary; an application role never bypasses them through a direct table path.

## Transaction and recovery obligations

The generic and exact claim wrappers are alternate entry points to one state
machine. Candidate selection and guarded leasing use `FOR UPDATE SKIP LOCKED`;
at most one caller wins. An expired leased job at maximum attempts and a ready
or retry-wait job at maximum attempts converge to one terminal exception before
leasing. A valid lease is committed before grader I/O. Generation and
`active_worker_id` fencing make stale workers harmless.

The private load capability checks running state, active worker, unexpired
lease, accepted-response digest, issued witness, exact immutable references,
course scope, and retention fences. The completion lock repeats those checks
and locks the completion rows with `FOR UPDATE`. The PostgreSQL store holds
that transaction open while it decodes the private result, invokes the shared
pure Rust planner, and calls commit-v2. Locks therefore span private source
load through the complete write.

Commit-v2 accepts only the server-derived `graded` plan plus canonical
evidence. For every evidence value it verifies the
`ple-canonical-json-v1` source-text digest, parses the bounded text, and
requires structural equality with its `jsonb` projection. It validates exact
identity, lifecycle, ownership, and the eight canonical summary fields before
writing evaluation, feedback, answer-free attempt state, normalized receipt
snapshot, run/enrollment/summary state, statistics inputs, execution/job
receipts, and one `1830` recalculation enqueue. The `1831` capability remains
the sole assignment/course current-score publisher.

The fail wrapper accepts only non-NULL `deterministic`, `transient`,
`timed_out`, or `terminal` failure kinds. A deterministic failure requires a
closed reason; all other kinds require no reason. Invalid NULL or vocabulary
inputs raise `22023` before locks and preserve every job, execution,
evaluation, receipt, and operation row. SQL derives `retry_exhausted` from
persisted attempt state. A valid inactive claim returns
`claim_no_longer_active` and changes no row.

## Database validation

The connected database proof is owned by W7b and remains distinct from the
offline Rust/Memory tests and the production browser journey. The exact
owners are:

- `crates/learning-data-access/tests/postgres_automated_grading_operations_live.rs`
  and its focused modules for broker, ACL, RLS, claim, load, completion,
  receipt, recovery, retry, and replay behavior.
- `tests/e2e/e2e_database_baseline.sh` for fresh installation, migration
  ordering, second-pass no-op, compatibility, forced-RLS inventory, and exact
  disposable cleanup.

The oracle uses a fresh database migrated through 1851 through 1860, then a
second no-op pass and compatibility check. It proves private-read denial,
exact login/pool membership, generic/exact one-winner competition, claim/load/
completion fencing, append-only receipts, retention closure, retry/reclaim/
exhaustion including ready-at-max convergence, exhaustive non-owner function
ACLs, app verified-read success after actor entitlement, and direct-column
denial. It also proves canonical source/digest and projection equality,
transaction-scoped lock/commit fencing, replay/status convergence, known
function/statement failures, and acknowledgement uncertainty only after a
decoded completion result. Controlled stored lease timestamps replace elapsed
waiting.

The connected proof compares ordinary synchronous completion and accepted-
worker completion for equal run, enrollment, and scalar-summary results. It
also proves the `1830` enqueue followed by the `1831`-only assignment/course
publication path. Database success does not claim browser acceptance; the
paired execution contract and [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md)
retain that evidence boundary.

## Lane ownership and handoff

Database lane B owns the ten migrations and their SQL authority assertions.
Lane A1 owns connection/login composition and bounded private pools. Lane A2
owns the PostgreSQL execution and receipt implementation over this SQL seam.
The paired execution contract owns lanes A, C, and D: Rust/Memory semantics,
the common handler/dispatch, acceptance, route status, and learner client.

W4 succeeds only when this database contract's stabilization and connected
oracle are green along with the paired contract's focused deterministic gates.
The final package still requires the complete material-tree Validation in
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md); this database document
does not convert one-time Graphify or disposable service evidence into a
permanent source-inventory test.
