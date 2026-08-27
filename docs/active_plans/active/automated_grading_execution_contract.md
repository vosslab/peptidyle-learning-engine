# Automated grading execution contract

## Purpose and authority

This binding contract makes G1-W4's accepted-submission execution seam
implementable by coordinated lanes. The parent
[automated_grading_operations_plan.md](automated_grading_operations_plan.md)
owns package scope and dependency order. The
[implementation_status.md](../implementation_status.md) ledger owns migration
allocation and current-package status.

This contract applies after G1-W3 has passed its stabilization and review
gates. W2 supplies the accepted foundation for immutable accepted input,
answer-free metadata, private response storage, and the ready execution job.
W4 supplies the exact active worker claim, private load, one atomic completion
transition, route-bound learner status read, and common execution handler. The
W4 implementation gate is a narrow stabilization sequence: it proves the
fresh ordered install before adapter, route, or handler implementation resumes.
The approved exact-claim model remains binding.

## Execution model

G1-W4 has one private accepted-submission execution boundary with two caller
roles. The closed `GradeAcceptedSubmission` job family reaches that boundary
directly. The generic `JobStore` families retain their established queue
paths. An exact
fast-path target carries the accepted metadata tuple `(tenant, attempt,
submission, job)` and never carries a response, result, feedback, score, or
reason. Both generic next-ready and exact-target entry points use one shared
claim-state machine. They return the same opaque tenant-bound claim tuple
`(tenant, job, lease_token, submission, execution_generation, worker)` before
tenant-bound material is loaded. Every answer-bearing load and outcome uses
that complete claim tuple.

An inactive claim has the typed `ClaimNoLongerActive` disposition. It preserves
execution, evaluation, receipt, queue, operation, and score state. A known
commit/fail error propagates its `StoreError`. Only a lost final PostgreSQL
transaction acknowledgement becomes handler-local `OutcomeUnknown`; the common
handler later observes durable state and receipts instead of grading again.

The synchronous fast path and background recovery use the same common leased
handler. `AcceptedSubmissionExecutionWorker::execute_accepted_submission` is
the fast-path adapter: it makes the exact claim, returns
`ClaimNoLongerActive` for `None`, and otherwise delegates directly to the
existing `execute_claim`. `drain_one` remains the generic-claim background
adapter and delegates to that same handler. No route calls `RunBackend` or
writes an outcome directly. API and background processes each receive a
distinct `WorkerId` generated at process start and stable for that process;
the identity denotes a lease holder, never a user or learner. Both construct
their handler from the same validated positive, lease-shorter `WorkerSettings`
value. The production dispatcher alternates the general worker and sealed
accepted-submission worker under one bounded claim budget; together they serve
the six established families and the sealed seventh family. The authenticated
one-use iMathAS broker retains its atomic external-tool path.

## Current correction architecture

`ple_accepted_submission_execution` is the existing generic recovery caller.
Migration 1851 adds
`ple_accepted_submission_execution_fast_path` as the exact-target caller. Both
caller roles receive the common load, completion-lock, commit, and fail
functions directly. The recovery caller executes only the generic claim
wrapper; the fast-path caller executes only the exact claim wrapper. Neither
caller executes its sibling wrapper or the owner-only internal claim
transition. `ple_accepted_submission_execution_worker` remains the
membership-free NOLOGIN definer owner; it is not a process capability.

W4 adds a recovery login/pool with only
`ple_accepted_submission_execution` and a fast-path login/pool with only
`ple_accepted_submission_execution_fast_path`. Each membership is `SET TRUE`,
`INHERIT FALSE`, and `ADMIN FALSE`. The API and general worker retain their
ordinary pools, while the fast-path adapter and recovery worker receive their
respective private pool through type-distinct composition. Both adapters claim
through their own role and delegate a won claim to the same common handler,
which alone loads, locks, commits, or fails the claim tuple.

## Existing authority

| Artifact | W4 use |
| --- | --- |
| Migration `2026081849` | Creates immutable accepted input, `grading_execution`, a ready job, and `grading_execution_receipt`. |
| Migration `2026081850` | Owns the private child, atomic acceptance/replay, RLS, retention, caller capability, and W2 ready-state loader v1. |
| Migrations `2026081851`-`2026081860` | Own the ordered W4 schema/roles, integrity, public-function authority, table authority, claim, verified read, load, completion-lock, commit-v2, and fail capabilities. |
| Migration `2026081830` | Receives the successful execution's conditional recalculation enqueue. |
| Migration `2026081831` | Remains the sole publisher of current scores, summaries, and totals. |

The schema/roles layer adds `grading_execution.active_worker_id uuid NULL`. This field
fences the worker that owns an active lease. It adds the nullable pair
`submission_evaluation.automated_result_canonical_json text` and
`automated_result_sha256 character(64)`. Pair, size, digest, and focused update
guards apply. A populated pair persists until the existing retention capability
deletes governed records. Existing `payload` and `payload_sha256` retain their
queryable-projection semantics.

The integrity layer establishes one receipt invariant across native, manual,
external-tool, and accepted-submission receipt writers: every
`submission_receipt_snapshot` carries `receipt_attempt_payload` and
`receipt_attempt_payload_sha256`, an answer-free attempt snapshot in the
legitimate terminal state owned by that writer. A normal or automated
completed receipt uses `Submitted`; a pending-manual receipt retains
`NeedsManualGrading`. The normalized snapshot remains the immutable source for
receipt replay and status reads; it never consults mutable catalog sources to
reconstruct learner work. The accepted-submission completed branch additionally
requires the full `Submitted` plus `graded | exempt` completion aggregate.

## Canonical immutable evidence protocol

W4 uses `ple-canonical-json-v1` for every immutable evidence value: automated
result, receipt attempt, run, summary, optional presentation, and private
feedback. Rust creates the exact compact UTF-8 JSON source text once. Its
lowercase SHA-256 hex digest attests to those exact UTF-8 bytes. PostgreSQL
accepts that source text, parses it to validate the closed value and to store a
queryable `jsonb` projection, and proves that the projection is structurally
equal to the parsed source. Readers verify the stored source-text digest before
typed decoding and then verify the stored projection against that parsed text.

Every W4 immutable source-text/projection row carries
`canonical_json_version = 1`. The crate-private
`MAX_CANONICAL_JSON_V1_BYTES = 512 * 1024` bound is the existing broker JSON
ceiling made explicit at this protocol boundary. Feedback retains its
established stricter semantic budget. This makes the current typed `serde`
encoding an explicit protocol version and provides a clean additive path when
a future cross-language standard needs a new encoder. The hash is always over
the stored source text; the system never derives an evidence hash from
`jsonb::text`, a reconstructed JSON value, or a second JSON formatter.

`submission_receipt_snapshot` stores the following non-null-together source
text and projection pairs. Their existing digest columns attest to the source
text after W4:

| Evidence | Canonical source text | Query projection | Digest |
| --- | --- | --- | --- |
| Receipt attempt | `receipt_attempt_canonical_json` | `receipt_attempt_payload` | `receipt_attempt_payload_sha256` |
| Run | `run_canonical_json` | `run_payload` | `run_payload_sha256` |
| Summary | `summary_canonical_json` | `summary_payload` | `summary_payload_sha256` |
| Presentation when present | `presentation_canonical_json` | `presentation_payload` | `presentation_payload_sha256` |

`attempt_feedback.content_canonical_json` is the source text for the existing
private three-element feedback tuple `[hint, correct_response, rationale]`;
`content_sha256` attests to that text. Its parsed projection must equal
`jsonb_build_array(hint, correct_response, rationale)`. Existing content-block
shape checks continue to apply. Receipt source text, projections, digests, and
version are immutable after insertion alongside the established answer-free
terminal and tenant/identity guards.

Current lifecycle projections remain distinct. `question_attempt.payload`
retains the immutable issuance record; its relational status and submission
timestamp are the current lifecycle authority. `assignment_run.payload` retains
its mutable projection/digest contract. `student_assignment_summary` is the
typed scalar current per-enrollment completion projection: it has no payload or
digest contract. Its immutable
canonical summary source text, query projection, and digest belong exclusively
to `submission_receipt_snapshot`. This gives learners and instructors a durable
receipt without making browser contracts answer-bearing.

## Rust sealed contract

Lane A defines these types in
`crates/learning-data-access/src/contracts/grading_operations.rs` and
re-exports them through `grading_operations.rs` and `lib.rs`.

```rust
pub struct AcceptedSubmissionExecutionTarget {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub submission: AcceptedSubmissionId,
    pub job: JobId,
}

pub struct AcceptedSubmissionExecutionClaim {
    pub tenant: TenantId,
    pub job: JobId,
    pub lease_token: JobLeaseToken,
    pub submission: AcceptedSubmissionId,
    pub execution_generation: GradingExecutionGeneration,
    pub worker: WorkerId,
}

pub struct AcceptedSubmissionGrade {
    pub evidence: CanonicalAttemptResult,
    pub feedback: FeedbackContent,
}

pub struct CanonicalAttemptResult {
    pub result: AttemptResult,
    pub canonical_json: String,
    pub sha256: Sha256Digest,
}

pub fn canonical_attempt_result_json(
    result: AttemptResult,
) -> Result<CanonicalAttemptResult, StoreError>;

pub enum AcceptedSubmissionExecutionOutcome {
    Evaluated { grade: AcceptedSubmissionGrade },
    DeterministicFailure { reason: GradingOperationReason },
    TransientFailure,
    TimedOut,
    TerminalFailure,
}

pub struct CompletedSubmissionReceipt {
    pub attempt: QuestionAttempt,
    pub feedback: AttemptFeedbackRecord,
    pub run: AssignmentRun,
    pub summary: StudentAssignmentSummary,
    pub presentation: Option<ReceiptPresentationSnapshot>,
}

impl CompletedSubmissionReceipt {
    pub fn into_submission_record(
        self,
        disclosure: LearnerDisclosureInput,
    ) -> SubmissionRecord;
}

pub enum AcceptedSubmissionExecutionDisposition {
    Committed,
    Rescheduled,
    Terminal,
    ClaimNoLongerActive,
}

pub enum AcceptedSubmissionCommitError {
    Known(StoreError),
    OutcomeUnknown,
}

#[async_trait]
pub trait AcceptedSubmissionExecutionStore: Send + Sync {
    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, StoreError>;

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError>;
}

#[async_trait]
pub trait AcceptedSubmissionExecutionRecoveryStore:
    AcceptedSubmissionExecutionStore
{
    async fn claim_next_accepted_submission_execution(
        &self,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>;
}

#[async_trait]
pub trait AcceptedSubmissionExecutionFastPathStore:
    AcceptedSubmissionExecutionStore
{
    async fn claim_exact_accepted_submission_execution(
        &self,
        target: AcceptedSubmissionExecutionTarget,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError>;

}
```

The target type and split traits are W4 implementation work; current Rust has
only the generic worker trait and adapter. W4 introduces type-distinct
PostgreSQL recovery and fast-path wrappers over a private shared execution core
and implements both claim traits over one locked Memory state machine.
`AcceptedSubmissionExecutionTarget` is metadata-only and is constructed from
the accepted submission. `Some(claim)` means the caller won the exact lease.
`None` means the target is no longer eligible: another executor owns it, it is
terminal, it is no longer pending, it is retention-inactive, or its exact
job-payload witness no longer matches. `None` has no reason payload and maps
to `ClaimNoLongerActive`; it never exposes private material. The exact method
requires all four target identifiers to match the persisted
`GradeAcceptedSubmission` payload. It returns the existing six-column claim
tuple, and load, completion lock, commit, and fail continue to fence every
field of that tuple.

The generic and exact methods are alternate entry points to one state machine,
not separate queues: they share candidate eligibility, controlled expiry and
retry-exhaustion convergence, `FOR UPDATE SKIP LOCKED`, lease and
`active_worker_id` writes, generation fencing, and one running receipt. At
most one caller obtains a claim. Memory applies that same eligibility rule
under its state write lock; PostgreSQL applies it in one sealed transaction and
commits the lease before grader I/O. A fast-path API process receives only an
opaque `AcceptedSubmissionFastPath` capability backed by its dedicated
least-privilege execution pool; its broad API store never receives the private
execution trait. The background worker has its distinct restricted execution
pool. Neither process may read account/session tables or use a broad table role.

`CanonicalAttemptResult` is constructed only by
`canonical_attempt_result_json(AttemptResult)`. The helper follows the
repository `encode_payload` convention: `serde_json::to_value` followed by
`serde_json::to_string`. Its exact UTF-8 bytes are hashed and carried with the
text. It stays private at the persistence boundary. Completed policy may later
project the established receipt; pending and attention DTOs remain answer-,
feedback-, result-, and score-free.

`CanonicalAttemptResult` uses a redacted diagnostic representation. Debug
output may identify the digest while omitting the typed result and canonical
JSON, so an outcome log cannot disclose learner score evidence.

`AcceptedSubmissionGrade` and `CompletedSubmissionReceipt` are server-only and
have redacted diagnostic representations. `CompletedSubmissionReceipt` is the
immutable, answer-free completion aggregate. Its `QuestionAttempt` preserves
issued provenance and timing, carries the trusted result and submitted time,
and always has `response = None`. `SubmissionRecord` remains the authorized
read-time projection: it combines this aggregate with current
`LearnerDisclosureInput`. The accepted response remains exclusively in
`accepted_submission_private_response` / `private_submission_responses` and is
available only through the lease-fenced execution capability.

Automated execution writes only `graded`. `SubmissionEvaluationStatus::Exempt`
remains a general read-model state and maps to learner-safe completed/graded
visibility. An authorized Instructor/policy capability owns any exemption
transition; the worker outcome, `RunBackend`, `GradeReceipt`, and completion
function do not carry that authority.
`GradingOperationReason` is the only deterministic or integrity datum crossing
the handler/store boundary. W3 maps `DeterministicGraderFailure` to this closed
type. `MemoryStore` and `PostgresAcceptedSubmissionExecutionStore` implement
the private execution traits. `Store`, `AutomatedGradingStore`,
`AssignmentScoringWorkerStore`, generic `JobStore`, and `PostgresStore` retain
their established distinct capabilities.

## Canonical result evidence

The result object contains exactly `correct`, `pointsEarned`, and
`pointsPossible`. The storage boundary validates typed scalars and positive,
finite possible points, then derives `credit_fraction` once. It stores the
`ple-canonical-json-v1` source text and digest in the 1852 integrity
pair, with parsed JSON retained only as its query projection. This preserves
the byte identity Rust attests while keeping a queryable JSON projection.

## Completion planning and persistence contract

Lane A adds crate-private
`crates/learning-data-access/src/submission_completion.rs` and re-exports it
as crate-private from `lib.rs`. It extracts the existing completion derivation
from `in_memory/runs.rs::submit_question_attempt_locked` and
`postgres/submission.rs::submit_question_attempt` into one pure planner:

```rust
pub(crate) struct AcceptedSubmissionCompletionInput { /* private loaded facts */ }
pub(crate) struct AcceptedSubmissionCompletionPlan {
    pub receipt: CompletedSubmissionReceipt,
    pub enrollment: AssignmentEnrollment,
    pub receipt_payloads: CompletedReceiptPayloads,
    pub statistics: Option<Vec<StatisticsContribution>>,
    pub recalculation: AssignmentRecalculationRequest,
}

pub(crate) fn plan_accepted_submission_completion(
    input: AcceptedSubmissionCompletionInput,
) -> Result<AcceptedSubmissionCompletionPlan, StoreError>;
```

The planner accepts only coherent, validated input from one transaction or
state lock: a submitted answer-free attempt built from the trusted grade,
already-created private feedback, issued presentation/witness, assignment/run/
enrollment/previous summary, immutable run items, logically substituted
same-run attempts, and acceptance-fixed server time. It reuses
`current_run_questions`, `completed_run_score`, `project_summary`,
`project_enrollment_completion`, and `derive_statistics_contributions` to
derive run/enrollment/summary completion and one-time statistics inputs. It
assembles the completed aggregate from the already-created feedback and
projected attempt/run/summary/presentation. It is deterministic and performs
no I/O.

The caller that receives `AcceptedSubmissionGrade` owns the trusted result
validation, submitted-attempt construction, private-feedback construction, and
receipt/private persistence. This narrow extraction shares only the duplicated
pure lifecycle composition; it retains backend-specific authorization,
timing/issuance reads, private response handling, checksums, and writes at the
appropriate authority boundary.

The plan deliberately computes no `attempt_score_current` write and performs
no current assignment or course score publication. W4 updates the typed scalar
per-enrollment completion projection in `student_assignment_summary` as part of
the accepted completion transition, while preserving the immutable receipt
summary in `submission_receipt_snapshot`. It then requests exactly one
generation-advancing 1830 recalculation. Migrations 1830 and 1831 remain the
generation-fenced assignment/course current-score publication path; 1831 is the
sole publisher of assignment and course scores and totals.

Memory invokes the planner under one `write_state()` lock. Its accepted
submission state changes from a stored `SubmissionRecord` to
`Completed(Box<CompletedSubmissionReceipt>)`; receipt replay and status reads
apply current disclosure only when projecting that aggregate. PostgreSQL owns
the same transaction-scoped workflow through the lock/commit capability below.

## W4 SQL migration decomposition

The sole mutable allocation is the [implementation status
ledger](../implementation_status.md). It assigns nine small, pre-production
W4 migrations in dependency order. Each owns one independently reviewable
capability and one focused PostgreSQL proof. W5 receives `2026081861` in the
ledger and begins schema work after its allocation and W4 stable handoff.
W4 keeps explicit identity, capability, transition, and immutable-evidence
boundaries that support the roadmap's later database normalization work; that
roadmap evolves the broader relational model in its own package.

| Migration | Owns | Focused PostgreSQL proof |
| --- | --- | --- |
| `2026081851_accepted_submission_execution_schema.sql` | Roles and schema, including the exact fast-path caller role and `active_worker_id`/canonical-evidence columns. | Exact role shape, zero memberships, and no direct data authority. |
| `2026081852_accepted_submission_execution_integrity.sql` | Immutable guards and triggers. | Invalid or repeated immutable writes fail. |
| `2026081853_public_function_authority.sql` | Global PUBLIC function-EXECUTE revocation, migration-current-role default-privilege revocation, effective privilege catalog proof, and legacy loader retirement. | PUBLIC and migration-owner defaults grant no executable path; legacy v1 load is absent or denied. |
| `2026081854_accepted_submission_execution_authority.sql` | Witness, RLS/policies, table/sequence authority, receipt `canonical_json_version` SELECT, and exact authority attestation. | Definer receipt version read succeeds; caller and app direct reads remain denied. |
| `2026081855_accepted_submission_execution_claim.sql` | One owner-only internal transition, generic and exact claim wrappers, and ready/max convergence. | Generic/exact race has one winner; each caller is denied its sibling wrapper and the internal transition. |
| `2026081856_accepted_submission_execution_read.sql` | Four-key structural integrity reader for the safe graded projection. | Authorized route read succeeds; wrong structural key, direct canonical data, and cross-tenant access fail. |
| `2026081857_accepted_submission_execution_load.sql` | Exact private execution load wrapper. | Exact claim loads once; a mismatched tuple loads nothing. |
| `2026081858_accepted_submission_execution_completion_lock.sql` | Exact completion-lock wrapper. | A stale or duplicate claim cannot acquire completion rows. |
| `2026081859_accepted_submission_execution_commit.sql` | Commit-v2 wrapper. | One graded completion writes the expected immutable aggregate through all 36 ordered inputs. |
| `2026081860_accepted_submission_execution_fail.sql` | Fail wrapper with closed NULL-aware failure validation. | Invalid NULL vocabulary raises `22023` and preserves all state. |

Migration 1851 creates the exact fast-path caller with NOLOGIN, NOINHERIT,
no memberships, schema USAGE only, and no direct relation, sequence, or
unrelated function authority. Migration 1852 applies immutable guards and
triggers. Migration 1853 removes global PUBLIC function EXECUTE, removes
function default EXECUTE for the role that runs the migration, and proves the
effective catalog privilege set is closed. It explicitly retires
`ple_load_accepted_submission_execution_v1`: the legacy function is absent or
has no executable path for every non-owner role. Migration 1854 establishes
the witness and sealed table authority; it grants the definer exactly the
receipt columns it reads, including `canonical_json_version`, while `ple_app`
receives no private canonical source, digest, or version column. Its authority
proof compares actual and expected non-owner function ACLs as complete
`aclexplode` sets, including grant options.

Migrations 1855 through 1860 each create their one public `SECURITY DEFINER`
wrapper or tightly paired claim wrappers with
`SET search_path TO 'pg_catalog', 'public', pg_temp` and server-owned time. The
shared claim transition is owner-only `SECURITY INVOKER` and receives no public
grant. Each capability's catalog assertion proves owner, security-definer flag,
fixed search path, signature, and exact complete non-owner execute ACL. The
1852 trigger functions attest an empty external ACL; 1855 attests the internal
transition as empty, the generic wrapper for recovery only, and the exact
wrapper for fast path only; 1856 attests `ple_app` only; and 1857-1860 attest
exactly both caller roles. Every `ALTER FUNCTION`, revoke, and grant statement
remains individually readable and executable.

```sql
public.ple_claim_accepted_submission_execution_v1(
    p_lease_token uuid, p_worker_id uuid, p_lease_seconds integer
) RETURNS TABLE (
    tenant_id uuid, worker_job_id uuid, worker_lease_token uuid,
    submission_id uuid, execution_generation bigint, worker_id uuid
);

public.ple_claim_exact_accepted_submission_execution_v1(
    p_tenant_id uuid, p_attempt_id uuid, p_submission_id uuid,
    p_worker_job_id uuid, p_lease_token uuid, p_worker_id uuid,
    p_lease_seconds integer
) RETURNS TABLE (
    tenant_id uuid, worker_job_id uuid, worker_lease_token uuid,
    submission_id uuid, execution_generation bigint, worker_id uuid
);

public.ple_read_accepted_submission_evaluation_v1(
    p_tenant_id uuid, p_course_id uuid, p_assignment_id uuid,
    p_attempt_id uuid
) RETURNS TABLE (evaluation_payload jsonb);

public.ple_load_accepted_submission_execution_v2(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid,
    p_submission_id uuid, p_execution_generation bigint, p_worker_id uuid
) RETURNS TABLE (
    worker_job_id uuid, worker_lease_token uuid, execution_generation bigint,
    worker_id uuid, execution_state text,
    accepted_tenant_id uuid, accepted_course_id uuid,
    accepted_assignment_id uuid, accepted_attempt_id uuid,
    accepted_submission_id uuid, accepted_actor_id uuid,
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
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid,
    p_submission_id uuid, p_execution_generation bigint, p_worker_id uuid
) RETURNS TABLE (
    /* exact private completion input, including the private accepted response
       and named scalar summary fields */
);

public.ple_commit_accepted_submission_completion_v2(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid,
    p_submission_id uuid, p_execution_generation bigint, p_worker_id uuid,
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
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid,
    p_submission_id uuid, p_execution_generation bigint, p_worker_id uuid,
    p_failure_kind text, p_operation_reason text
) RETURNS TABLE (
    disposition text, resulting_execution_state text,
    resulting_evaluation_status text
);
```

The full commit signature has 36 inputs; canonical JSON version is explicitly
position 7, before evaluation status. The 1855 generic and exact wrappers call
the same internal transition. It
performs closed-payload and target-witness checks, controlled exhausted-work
convergence, candidate eligibility, `FOR UPDATE SKIP LOCKED`, lease and
`active_worker_id` updates, and one running receipt. The exact wrapper returns
zero rows for a noneligible or racing target. The reader accepts
tenant/course/assignment/attempt and structurally verifies the exact attempt ->
run -> enrollment -> assignment -> course chain in the tenant transaction. It
returns only the existing app-readable `submission_evaluation.payload` after
proving `completed`, `Submitted`, `graded`, receipt, canonical version, UTF-8
digest, payload digest, and canonical-text/projection equality. It is an
integrity-and-route verifier, not an independent actor-authorization API: the
module-private call follows the canonical `require_attempt_owner_for_read`
entitlement check in the same Rust transaction, and
`LearnerSubmissionStatusStore` is the sole public capability for this result.
Route values are assertions; no-row or contradiction maps to unavailable. It
contains no exempt branch; the existing authorized exempt receipt path remains
separate.

Migration 1855 converges both supported exhausted stored states before leasing:
an expired leased job at its maximum and a ready or retry-wait job at its
maximum. The candidate query and guarded lease update require
`attempt_count < max_attempts`; convergence writes the one terminal exception
aggregate and immutable receipt. A ready exhausted job retains its permitted
`last_error` or uses `permanent` when absent, so generic and exact wrappers
share one closed state transition.

The lock/load capability acquires the exact completion rows with `FOR UPDATE`
after rechecking the complete lease tuple, active retention, immutable accepted
response digest, issued witness, and pending evaluation. It returns the typed
summary's named scalar fields for `decode_summary_row_named`, not a fabricated
current JSON payload, and returns private response material only to the
execution capability. The Rust PostgreSQL store holds that transaction open,
validates and decodes the result, invokes the shared pure planner, and calls
commit v2 before committing. Thus locks persist from source load through the
complete write.

Migration 1859 installs commit v2 and migration 1860 installs fail after the
acquisition/read layers are available. Commit v2 accepts only server-derived
plan payloads plus canonical evidence.
For each immutable evidence value it receives source text and `jsonb`
projection, verifies the `ple-canonical-json-v1` text digest, parses the text,
requires parsed-text/projection structural equality, and applies the closed
W4 identity, lifecycle, and tenant invariants. For the receipt summary, it
accepts exactly the eight canonical `StudentAssignmentSummary` fields:
`tenant`, `enrollment`, `currentScore`, `bestScore`, `latestScore`,
`completedRunCount`, `totalQuestionAttempts`, and `lastActivityAt`. It validates
their identities and scalar bounds, stores the exact immutable receipt-summary
source/projection/digest triplet, and derives the typed scalar
`student_assignment_summary` update from that parsed value. The capability has
exactly 36 positional values. It rechecks the full claim and validates every
identity and checksum against locked source rows, writes the evaluation,
feedback, answer-free completed attempt, normalized receipt-attempt snapshot,
run/enrollment/summary transition, one-time statistics inputs, execution/job
receipts, and one 1830 enqueue. A valid inactive claim returns
`claim_no_longer_active` and changes no row. This keeps grading and lifecycle
semantics in the shared Rust planner rather than reproducing the scoring
algorithm in PL/pgSQL.

The ten migrations are independently atomic: each starts and commits its own
transaction, and each either installs its single capability or rolls back
entirely. A fresh installer applies 1851 through 1860 in order and does not
launch an API or worker until the migration tail is compatible. A failure
leaves an incompatible disposable database for diagnosis, never a partially
installed capability. The pre-production baseline rejects nonempty legacy receipt
or feedback tables with a clear rebuild instruction and never fabricates source
text from historical `jsonb`. Fresh disposable apply of all nine layers, a
second no-op pass, and compatibility verification are required evidence for
the ordered stack.

The commit function accepts only `graded` as `p_evaluation_status`. Its
canonical-text parameter is bounded. The failure function accepts only
non-NULL `deterministic`, `transient`, `timed_out`, and `terminal` values as
`p_failure_kind`. A deterministic failure carries a non-NULL reason from the
existing closed reason set; every other failure kind carries no reason. Invalid
NULL or vocabulary inputs raise `22023` before a lock and preserve every job,
execution, evaluation, receipt, and operation row. SQL derives
`retry_exhausted` from persisted attempt state.

V2 load reads only after rechecking the full tuple, `running` state, active
worker, unexpired lease, response digest, issued witness, course, and active
retention fences. It returns zero rows when a predicate fails. The completion
lock and commit v2 repeat every predicate in their shared mutation transaction.
A predicate failure returns `claim_no_longer_active` and preserves state.

## State transitions

| Outcome | Execution and evaluation | Job and receipt | Other effect | Disposition |
| --- | --- | --- | --- | --- |
| Claim | `ready` or reclaimable `retry_wait` to `running`; record worker | Lease and append `running` | None | Claim or no row |
| `Evaluated { grade }` | `completed`; atomically store graded evidence, answer-free completed receipt, feedback, run/enrollment transition, typed scalar summary update, and statistics input | Complete, clear worker and lease, append completed | Invoke exactly one 1830 request; 1831 later publishes assignment/course current scores | `Committed` |
| Deterministic failure | `exception`; `automated_exception` | Terminal, clear worker and lease, append exception | Converge recovery using closed reason | `Terminal` |
| Transient or timed out with attempts | `retry_wait`; evaluation remains pending | Ready at server retry time, clear worker and lease, append retry-wait | None | `Rescheduled` |
| Retry exhausted | `exception`; `automated_exception` | Dead, clear worker and lease, append exception | Recovery reason derives as `retry_exhausted` | `Terminal` |
| Terminal failure | `exception`; `automated_exception` | Dead, clear worker and lease, append exception | Recovery receives execution failure | `Terminal` |
| Known commit failure | No new state claimed | The returned `StoreError` is diagnosable | Propagate `Known(error)` | Handler error |
| Final commit acknowledgement unknown | Handler waits for durable status | Durable rows remain the later authority | Local `OutcomeUnknown` | Handler-local |
| Stale or mismatched claim | Unchanged | Unchanged | None | `ClaimNoLongerActive` |

## Common handler rules

The common handler receives a valid private claim, loads the sealed execution,
translates presentation-bearing response data, validates the grading envelope,
and invokes the deterministic grader. A closed trusted `GradeReceipt` becomes
`AcceptedSubmissionGrade`, carrying both canonical result evidence and closed
feedback. It invokes exactly one commit-or-fail request for the current claim
and returns the durable disposition when the store responds. Browser requests
and job payloads never supply result JSON, feedback, run, summary, receipt, or
learner projection.

The handler owns one validated `execution_deadline` and wraps exactly its owned
`RunBackend::submit` future in `tokio::time::timeout`. It does not spawn that
future. On timeout, dropping the future cancels it before the handler constructs
one `TimedOut` outcome and makes its one commit-or-fail request. It does not
regrade, detach work, use a queue failure path, or create another outcome. C2
reads the existing positive, lease-shorter `WorkerSettings` value once through a
crate-private semantic accessor and passes that value to both fast and recovery
handler construction; it adds no timeout or outcome writer. `Known(error)`
propagates. Only `OutcomeUnknown` returns the local unknown result and uses
later durable status or recovery for observation. Presentation translation or
envelope validation failures after acceptance map to typed
`instructor_attention`. Learners receive answer-free status rather than a
resubmission request for accepted work.

## Fast-path route outcomes

The route validates and accepts once through W2 before it invokes the opaque
fast-path capability. It does not receive a private execution store. After
acceptance, it maps only durable or lease-safe results as follows:

| Fast-path result | Route result | Boundary |
| --- | --- | --- |
| `Committed` | Re-read the route-bound status capability and return its existing `completed` projection with `200` and `no-store`. | The immutable receipt remains the only completed browser representation. |
| `Rescheduled` | Return answer-free `accepted_pending` with `202` and `no-store`. | Retry is durable and worker-owned. |
| `Terminal` | Re-read status and return `instructor_attention` with `202` and `no-store`. | Durable operation state supplies the learner-safe message. |
| `ClaimNoLongerActive` from exact `None` | Return answer-free `accepted_pending` with `202` and `no-store`. | A competing executor is authoritative; polling converges. |
| `OutcomeUnknown` | Return answer-free `accepted_pending` with `202` and `no-store`. | Only the final acknowledgement is ambiguous; durable state remains authoritative. |
| `Known(StoreError)` after acceptance | Return answer-free `accepted_pending` with `202` and `no-store`, and emit structured server telemetry. | The acceptance boundary has already been crossed. |

Pre-acceptance validation and error mapping remain unchanged. No post-acceptance
path calls the grader a second time, restores an accepted answer, or creates a
route-local receipt. A committed status read that cannot prove a coherent
completed aggregate fails closed through the existing unavailable path.

## Learner status contract

W4 extends W3's pending response into one flattened tagged union. Browser JSON
uses lowerCamelCase fields and the `kind` discriminant. Portable symbolic values
use snake_case, in accordance with
[NAMING_CONVENTIONS.md](../../NAMING_CONVENTIONS.md).

Lane D makes `question_model::AutomatedGradingStatus` the canonical symbolic
source by serializing that enum with snake_case and regenerating
`generated/api/AutomatedGradingStatus.ts`. This model-level correction gives
Rust and TypeScript the same `instructor_attention` value without a route-local
duplicate vocabulary.

| Kind | Fields |
| --- | --- |
| `completed` | Established completed receipt projection |
| `accepted_pending` | `accepted: true`, route-bound `attemptId`, `automatedGradingStatus`, `nextAction: "check_status"` |
| `instructor_attention` | `accepted: true`, route-bound `attemptId`, `automatedGradingStatus`, `nextAction: "check_status"` |

The no-store route-bound read is:

```text
GET /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submission-status
```

Lane D receives one focused server-only `LearnerSubmissionStatusStore`; it is
injected beside the existing sealed execution capability rather than added to
the broad `Store` facade. It proves tenant, learner, course, assignment, run,
and attempt in one authoritative read before returning the union. Memory reads
the completed aggregate under one lock; PostgreSQL checks the exact route
binding and receipt in one tenant transaction. Both use the same closed table:

| Execution / evaluation / receipt | Read result |
| --- | --- |
| `ready`, `running`, or `retry_wait` / `automated_pending` / no completed receipt | `accepted_pending` |
| `exception` / `automated_exception` or `needs_manual_grading` / no completed receipt | `instructor_attention` |
| `completed` / `graded` or `exempt` / valid immutable completed receipt | `completed` |
| Any other combination | unavailable closed failure |

The completed branch verifies the receipt-attempt payload/checksum,
canonical evaluation text/digest and projection digest, feedback, and
run/summary/presentation snapshot before disclosure. It fails closed on a
partial aggregate and uses the snapshot rather than current catalog state.
POST replay, receipt GET, and status GET therefore converge on the same
completed aggregate. A `202 Accepted` clears the response buffer and
idempotency key, enters `acceptedPending`, presents an accessible Check grading
status action, and reads this route.

## Lane ownership

| Lane | Owned files | Deliverable |
| --- | --- | --- |
| A: Rust contract and Memory completion | `canonical_json.rs`, `contracts/grading_operations.rs`, `contracts/runs.rs`, `submission_completion.rs`, `grading_operations.rs`, `lib.rs`, `in_memory.rs`, `in_memory/grading_execution_worker.rs`, `in_memory/runs/issued_contracts.rs`, `in_memory/runs.rs` | Common execution, recovery-claim, and exact-claim traits; metadata-only target; type-distinct Memory behavior over one state machine; graded-only outcomes and convergent receipt state. |
| B: SQL authority | `schemas/migrations/2026081851_accepted_submission_execution_schema.sql` through `schemas/migrations/2026081860_accepted_submission_execution_fail.sql` | Ten readable W4 layers: schema/roles, integrity, public-function authority, table authority, claim, read, load, completion lock, commit, and fail. The ordered stack provides versioned evidence, closed executable authority, one-winner claims, verified reads, and W7b setup. |
| A1: Login and deployment | `postgres/connection.rs`, `postgres/connection_contract.rs`, `local_stack_control/process_logins.py`, pool composition/settings, and focused tests | Exact recovery and fast-path login profiles, one membership each, bounded private pools, and constructors that keep both pools out of general stores. |
| A2: PostgreSQL execution and receipt implementation | `postgres/grading_operations.rs`, `postgres/submission_receipts.rs`, `postgres/submission.rs`, `postgres/manual_grading.rs`, `postgres/external_tool.rs`, `postgres/feedback_data.rs`, `postgres/feedback.rs`, `postgres/row_decode.rs`, `postgres.rs`, focused PostgreSQL-gated unit tests | Type-distinct recovery/fast-path wrappers over a private shared execution core; same-transaction entitlement then four-key verified read; one held transaction from lock through Rust planning and commit v2. |
| C: common handler and dispatch | Focused `accepted_submission_worker.rs`, `worker.rs`, `scoring_worker.rs` registration seam, `composition/worker.rs`, `composition/settings.rs`, focused tests | One handler-owned validated deadline and cancellation-safe timeout-to-one-`TimedOut` outcome; C2 passes the same existing worker setting to fast and recovery callers; closed grade plus feedback mapping and durable uncertainty handling. Depends on A/A2/B. |
| D: acceptance, status, and learner client | `contracts/runs/issue_contracts.rs`, `contracts/store_capabilities.rs`, `in_memory/runs/learner_submission_status.rs`, `postgres/runs/learner_submission_status.rs`, `run/submission.rs`, `run/support.rs`, `run/queries.rs`, `run/routes.rs`, `composition/router.rs`, `composition/backend.rs`, `src/api/contracts.ts`, `src/api/decoders/run.ts`, `src/api/http_client/{request,response}.ts`, `src/api/client.ts`, `src/features/attempt/attempt_state.ts`, focused page/tests | First acceptance effect, route-bound convergent status capability, answer-free union, decoder/client state, and visible learner recovery. Depends on completed receipt delivery from A/A2/C. |

Lanes A and B establish the canonical-encoding contract pair. A1 establishes
the private process identities. A2 consumes the stable pair and delivers the
PostgreSQL transaction seam. C consumes the completed execution
interfaces. D consumes the completed-receipt handoff and C's one internal
handler after W2's single acceptance effect. W5 begins from W4's stable worker
handoff. W7b adds connected authority evidence after W4/W5 integration.

## Validation and success criteria

Permanent deterministic tests use explicit identities, tokens, workers,
generations, and injected authoritative time. They remain offline and avoid
network services, database services, sleeps, arbitrary timing targets, and
pixel comparisons. They prove:

- One claim winner across competing generic and exact entry points, full
  target/claim-tuple mismatch rejection, expiry, reclaim, ready-at-max and
  expired-lease-at-max convergence, and sibling/internal claim-wrapper denial.
- Graded, deterministic, transient, timed-out, exhausted, and terminal outcome
  transitions; the worker cannot originate `exempt`, while general status reads
  preserve it as completed/graded visibility.
- `ple-canonical-json-v1` creates one source text and SHA-256 for each typed
  evidence value; altered text or digest and source/projection disagreement
  fail closed. The small result source-text example remains the public wire
  check; tests do not prescribe incidental formatting beyond the explicit
  versioned source-text protocol.
- Pure completion planning uses the established lifecycle helpers, preserves
  ordinary completion/first-statistics behavior, updates the typed scalar
  per-enrollment completion projection, and produces no
  `attempt_score_current` write on the accepted-worker path.
- Immutable completed receipts and convergent Memory replay/status reads;
  generic-queue separation; and no aggregate or score effect after a rejected
  commit.
- Known commit failure propagation; injected acknowledgement ambiguity alone
  returns local `OutcomeUnknown`; production Memory remains known-only.
- A paused-time handler test proves one submit, cancellation before one
  `TimedOut` commit, and the durable store disposition after the validated
  deadline. C2 tests prove the same existing worker setting reaches fast and
  recovery handler construction.
- Same-transaction actor-entitlement ordering before the four-key verified
  read; a same-tenant other actor or changed route remains unavailable, and
  only the existing safe evaluation projection can reach the caller.
- Exact route-bound status authorization, state-table projection, and
  answer-free 202/status recovery with no second answer POST. Fast `Committed`
  re-reads the same route-bound completed projection (`200`); `None`,
  rescheduled, known post-acceptance error, and acknowledgement uncertainty
  return pending (`202`); terminal work re-reads as instructor attention
  (`202`). No route synthesizes a receipt or replays an accepted answer.

W7b owns connected executable proof in
`crates/learning-data-access/tests/postgres_automated_grading_operations_live.rs`
and `tests/e2e/e2e_database_baseline.sh`. It runs against a fresh database
migrated through 1851 through 1860, followed by a second no-op pass and
compatibility check. It proves capability and private-read denial,
exact role/login/pool membership, generic-queue separation,
generic-versus-exact one-winner claim competition,
worker claim/load/one-time outcome, claim fences,
receipt append-only protection, retention and witness closure,
retry/reclaim/exhaustion including ready-at-max convergence, executable
role/RLS/catalog authority including exhaustive non-owner function ACLs, the
app's verified-read success after canonical actor entitlement, and direct-column
denial; it proves NULL failure kind and NULL deterministic reason yield `22023`
without durable changes. It also verifies canonical source-text/digest and JSONB-projection
evidence across normal, manual, and automated writers, normalized receipt
immutability, transaction-scoped lock/commit-v2 claim fencing, and completion
replay/status convergence. It proves known function/statement failure remains
`Known(StoreError)` and uses the reviewed final-acknowledgement fault seam to
prove `OutcomeUnknown` only after a decoded completion result. It compares ordinary synchronous completion and
accepted-worker completion for the same transition, requiring equal run,
enrollment, and scalar summary results. It also proves 1830 enqueue followed
by the 1831-only assignment/course current-score publication path. The proof
uses controlled stored lease timestamps rather than elapsed waiting.

W4 succeeds only after the fresh disposable stabilization sequence, all five
lanes' focused deterministic gates, and W7b's connected authority oracle are
green. The sealed claim tuple prevents stale mutation, Rust and PostgreSQL preserve one
versioned canonical evidence protocol and one immutable completed receipt,
synchronous and background paths share one handler, ordinary and
accepted-worker completion have equal run/enrollment/scalar-summary results,
replay and status preserve accepted learner work without exposing private
material, and 1831 remains the only assignment/course current-score publisher.
G1 acceptance continues to require the complete
final material-tree Validation specified by
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).
