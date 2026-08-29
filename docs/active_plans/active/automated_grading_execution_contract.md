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
The approved exact-claim model remains binding. PostgreSQL realization and its
connected proof are owned by
[automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md);
this document remains the semantic execution/evidence/state/handler authority.

## SD1-A5 identity and authorization correction

This revision supersedes the former installation-bound wording in the G1
execution contract. PLE is one installation with global accounts and one shared
published question corpus; it has no institution selector. Every authenticated
operation starts with the server-derived `ActorContext { user_id, session_id }`.
The context is resolved only from the active session and never from a route,
header, JSON body, queue payload, or browser state. It identifies the actor but
does not itself grant a course, Student, workspace, question, or grading
capability.

The protected Store and broker boundary derives and checks the complete exact
ownership chain in one transaction: `CourseId`, assignment, Student `UserId`,
`RunId`, `QuestionAttemptId`, and `AcceptedSubmissionId`. It also checks the
immutable published question reference (`QuestionId` plus its exact
`ProblemVersionRef`) and every immutable evidence reference/digest used by the
issued attempt and receipt. A foreign user, course, assignment, run, attempt,
submission, question, evidence reference, revoked membership, or stale route
returns the same concealed no-row/unauthorized result; it never broadens a
query by actor or by a current catalog lookup.

Workers do not impersonate an actor. A queue claim locks a typed
`WorkerLease` containing `JobId`, `JobLeaseToken`, `WorkerId`, exact scope,
execution generation, and manifest digest. The locked `WorkerManifest` binds
the exact course, Student, run, attempt, submission, immutable question and
evidence references, and accepted-input digest. Claim, load, completion, fail,
and recalculation brokers recheck that lease, manifest, and generation in the
same transaction; `FOR UPDATE SKIP LOCKED` permits one winner and generation
fencing makes stale workers harmless. The manifest contains references and
digests only, never browser answers, keys, feedback, or grader diagnostics.

Automated grading is deterministic and server-owned. Manual score ownership is
outside the product model and no automated-grading route, status, worker, or
receipt may introduce a manual or exemption transition.

This documentation repair adds no tests and does not claim runtime acceptance.
Future validation is split by evidence class: permanent offline Memory/Rust/
TypeScript tests cover actor-context handling, exact ownership, immutable
question/evidence references, canonical evidence, and lease/manifest/generation
state transitions; a disposable PostgreSQL oracle covers broker signatures,
forced RLS, ACLs, locked manifests, `FOR UPDATE SKIP LOCKED`, and stale-worker
fences; the canonical HTTPS browser suite covers answer-free status and
Instructor recovery plus cross-user/cross-course/foreign-target refusal; and
one-time Graphify/source, migration, SQL catalog, route, consumer, and rendered
evidence reviews document this rebase without becoming permanent tests.

## Immutable question versions and correction evidence

Every assignment item, issued attempt, accepted submission, worker manifest,
completion receipt, and grading evidence pins the exact published question
identity: public `QuestionId` plus its immutable `ProblemVersionRef` (including
the exact problem/version evidence digest). A worker, status reader, or
recalculation broker never resolves a mutable latest question. Published
question versions and their grading semantics are append-only; a correction to
the prompt, generator, answer semantics, rubric, or grader contract publishes
a new version and never edits the old version in place.

Publishing a corrected version creates replacement-impact evidence linked to
the originating improvement thread and the old/new immutable references. The
evidence is answer-free and records affected assignment references, pinned
attempt/run/submission populations, the safe impact summary, and the explicit
deterministic recalculation decision. Existing attempts remain pinned to the
old version. A decision of `preserve_pinned_history` or
`future_runs_only` leaves those attempts untouched; a decision to recalculate
any eligible existing work creates a separate generation-fenced operation
bound to the old/new references and its immutable evidence, with an auditable
Instructor action and receipt. That operation never retargets the pinned
attempt or evidence; it publishes only a separately identified derived-score
generation while preserving the original receipt. No correction silently
mutates a question, attempt, evidence record, score, or receipt.

Version-specific metrics count only accepted, server-graded attempts for the
exact published version: `accepted_server_graded_attempt_count`,
`correct_outcome_count`, and `eligible_flat_choice_selection_count`. Preview work, the answer-free
Instructor Student view, aborted attempts, and pending/ungraded work are
excluded. Privacy-thresholded rollups may aggregate these counts only when the
disclosure threshold is satisfied, must contain no Student identity, and must
label the exact formula version, question version, and evidence timestamp;
below threshold the projection is the sole insufficient-evidence state. The actionable Instructor queue keeps
the question title/public ID, correction linkage, safe impact, generation, and
next operation visible without exposing Student identity or answer material.

### Emergency forced correction and validated replacement

A Sysadmin may approve a `ForcedQuestionCorrection` only for a security or
critical-correctness defect. The operation publishes a new immutable
`QuestionId`/`ProblemVersionRef` and a closed, privacy-safe
impact/remediation manifest. It never mutates the defective version, rewrites
its grading semantics, or silently swaps any already-issued, accepted,
graded, or receipt-pinned work. Original attempts, responses, evidence,
scores, and receipts remain resolvable against their exact pinned references.

The correction commits one authoritative active-reference mapping and
generation before fan-out; from that commit, new selection and issuance no
longer resolve the defective reference. Deterministic compatibility applies
only to unissued work; idempotent generation-fenced workers materialize compatible
reference updates and remediation from the immutable manifest. This avoids an
unbounded cross-course transaction while making every new resolution follow
the committed mapping. In-progress work receives deterministic reissue or
excuse treatment. For completed work with no correct answer, the manifest
selects deterministic full-credit or exclude-and-rescale remediation. No
course Instructor approval is required after the Sysadmin decision; authorized
Instructors receive an exact-course, audited result projection and actionable
follow-up without owning manual scores.

The manifest and Sysadmin projection contain no Student identity, response,
grade, answer, or raw private evidence. They expose only thresholded impact,
old/new references, compatibility, remediation disposition, formula/version
labels, and audit metadata. The correction, active-reference commit, worker
fan-out, reissue/excuse, remediation, and any recalculation append typed
superseding correction/recalculation receipts while preserving all original
receipts. Every action is idempotent, generation-fenced, and append-only.

## Execution model

G1-W4 has one private accepted-submission execution boundary with two caller
roles. The closed `GradeAcceptedSubmission` job family reaches that boundary
directly. The generic `JobStore` families retain their established queue
paths. An exact fast-path target carries the accepted metadata tuple
`(course, assignment, student, run, attempt, submission, job, question_ref, evidence_ref)`
and never carries a response, result, feedback, score, or reason. Both generic
next-ready and exact-target entry points use one shared claim-state machine.
They return the same opaque scope-bound claim tuple
`(course, assignment, student, run, attempt, submission, job, lease_token,
execution_generation, worker, manifest_digest)` before private material is
loaded. Every answer-bearing load and outcome uses that complete claim and
manifest tuple.

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

## PostgreSQL boundary

The paired [automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md) owns the W4 PostgreSQL migration decomposition, caller roles,
function signatures, RLS and grants, transaction-held completion workflow, and
connected database validation. This execution contract keeps the cross-backend
semantics: a recovery caller and an exact fast-path caller use one leased handler,
while the database contract proves the least-privilege realization. The
implementation status ledger remains the sole migration-allocation authority.


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
terminal, exact ownership, immutable-question, and immutable-evidence guards.

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
pub struct ActorContext {
    pub user_id: UserId,
    pub session_id: SessionId,
}

pub struct ImmutableQuestionReference {
    pub question_id: QuestionId,
    pub version: ProblemVersionRef,
}

pub struct ImmutableEvidenceReference {
    pub issued_snapshot_digest: Sha256Digest,
    pub grading_source_digest: Sha256Digest,
}

pub struct WorkerLease {
    pub job: JobId,
    pub lease_token: JobLeaseToken,
    pub worker: WorkerId,
    pub generation: GradingExecutionGeneration,
}

pub struct WorkerManifest {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student: UserId,
    pub run: RunId,
    pub attempt: QuestionAttemptId,
    pub submission: AcceptedSubmissionId,
    pub question: ImmutableQuestionReference,
    pub evidence: ImmutableEvidenceReference,
    pub manifest_digest: Sha256Digest,
}

pub struct AcceptedSubmissionExecutionTarget {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student: UserId,
    pub run: RunId,
    pub attempt: QuestionAttemptId,
    pub submission: AcceptedSubmissionId,
    pub job: JobId,
    pub question: ImmutableQuestionReference,
    pub evidence: ImmutableEvidenceReference,
}

pub struct AcceptedSubmissionExecutionClaim {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student: UserId,
    pub run: RunId,
    pub attempt: QuestionAttemptId,
    pub job: JobId,
    pub lease_token: JobLeaseToken,
    pub submission: AcceptedSubmissionId,
    pub question: ImmutableQuestionReference,
    pub evidence: ImmutableEvidenceReference,
    pub execution_generation: GradingExecutionGeneration,
    pub worker: WorkerId,
    pub manifest_digest: Sha256Digest,
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
        lease: WorkerLease,
        manifest: WorkerManifest,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, StoreError>;

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        lease: WorkerLease,
        manifest: WorkerManifest,
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
requires every target identifier, immutable question/evidence reference, and
manifest digest to match the persisted `GradeAcceptedSubmission` payload. It
returns the typed scope/lease claim, and load, completion lock, commit, and fail
continue to fence every field of that claim.

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

Manual score ownership and exemption transitions are outside the automated
grading product model. The worker writes only deterministic `graded`,
`automated_exception`, or retry/pending state; no route, receipt, or status DTO
creates a manual score path.
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
apply current disclosure only when projecting that aggregate. PostgreSQL
realizes the same transaction-scoped workflow through the lock/commit
capability owned by
[automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md).

## Database capability handoff

The database-specific W4 decomposition and PostgreSQL obligations are owned by
[automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md). It preserves migrations `2026081851` through `2026081860`, the exact
caller/function/RLS/grant boundary, transaction and recovery rules, and W7b's
connected database oracle. Lanes A, C, and D consume its stable capability;
portable execution, evidence, state, and handler semantics remain here.


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

W4 extends W3's pending response into one flattened tagged union. **Current pre-WN1 behavior:**
Browser JSON uses lowerCamelCase fields and the `kind` discriminant; the WN1-A source/type matrix
assigns its complete PLE wire cutover to one C or QM closure. Portable symbolic values
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
the broad `Store` facade. It accepts `ActorContext` and proves the exact
Student, course, assignment, run, attempt, submission, and immutable
question/evidence references in one authoritative read before returning the
union. Memory reads
the completed aggregate under one lock; PostgreSQL checks the exact route
binding and receipt in one actor-scoped transaction. Both use the same closed table:

| Execution / evaluation / receipt | Read result |
| --- | --- |
| `ready`, `running`, or `retry_wait` / `automated_pending` / no completed receipt | `accepted_pending` |
| `exception` / `automated_exception` / no completed receipt | `instructor_attention` |
| `completed` / `graded` / valid immutable completed receipt | `completed` |
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
| B: SQL authority | [automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md); `schemas/migrations/2026081851_accepted_submission_execution_schema.sql` through `schemas/migrations/2026081860_accepted_submission_execution_fail.sql` | The database companion owns ten readable W4 layers, executable authority, one-winner claims, verified reads, and W7b setup. |
| A1: Login and deployment | `postgres/connection.rs`, `postgres/connection_contract.rs`, `local_stack_control/process_logins.py`, pool composition/settings, and focused tests | Exact recovery and fast-path login profiles, one membership each, bounded private pools, and constructors that keep both pools out of general stores. |
| A2: PostgreSQL execution and receipt implementation | `postgres/grading_operations.rs`, `postgres/submission_receipts.rs`, `postgres/submission.rs`, `postgres/external_tool.rs`, `postgres/feedback_data.rs`, `postgres/feedback.rs`, `postgres/row_decode.rs`, `postgres.rs`, focused PostgreSQL-gated unit tests | Type-distinct recovery/fast-path wrappers over a private shared execution core; same-transaction actor entitlement then exact ownership and immutable-reference verification; one held transaction from lock through Rust planning and commit v2. |
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
  transitions; the worker emits only deterministic graded, exception, or retry
  state and cannot originate a manual score transition.
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
- Same-transaction actor-entitlement ordering before the exact ownership and
  immutable-reference read; a different user, foreign course, foreign target, or changed route remains unavailable, and
  only the existing safe evaluation projection can reach the caller.
- Exact route-bound status authorization, state-table projection, and
  answer-free 202/status recovery with no second answer POST. Fast `Committed`
  re-reads the same route-bound completed projection (`200`); `None`,
  rescheduled, known post-acceptance error, and acknowledgement uncertainty
  return pending (`202`); terminal work re-reads as instructor attention
  (`202`). No route synthesizes a receipt or replays an accepted answer.

The connected PostgreSQL proof is owned by
[automated_grading_execution_database_contract.md](automated_grading_execution_database_contract.md).
The paired document keeps the offline deterministic, handler, route, and
learner-status criteria; the database companion owns fresh migration install,
ACL/RLS, claim/load/commit, receipt, recovery, replay, and 1830/1831
publication evidence.

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
