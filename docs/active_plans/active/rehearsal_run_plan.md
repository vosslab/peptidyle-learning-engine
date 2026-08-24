# Plan: WP-PROF-T4 rehearsal runs

## Status

WP-PROF-T4 is the sole current professor package. This focused plan supplies its execution
detail; [professor_capability_architecture_plan.md](professor_capability_architecture_plan.md)
remains the scope and dependency authority, while
[implementation_status.md](../implementation_status.md) remains the sole current-package and
migration-allocation authority. The approved architecture decision for this plan is recorded in
the private execution report `resume_presentation_authority.architect.report.md`.

The package begins from the accepted non-mutating WP-PROF-T3 preview plane. It implements the
one permitted mutating preview-plane case: an Instructor-owned rehearsal against a teaching or
Alpha assignment. It is a production-stack multi-request workflow, never a mock, a test student,
or an ordinary learner run marked with a flag.

## Purpose and boundary

An Instructor uses a closed, identity-free `PreviewSubject` to take an assignment under a chosen
policy context and chosen moment before a learner sees it. The rehearsal uses real delivery,
timing, rendering, and deterministic server-owned grading, while remaining structurally outside
the educational-record pipeline.

The subject is the accepted T3 value only. A synthetic subject contains selected group roles,
compatible modifier values, and a moment. A learner-derived subject contains only resolved policy
values and source-layer labels; T3 has already authorized and audited that record read. The
rehearsal receives neither a learner locator nor any identifying field.

The live demonstration remains the real application described by
[LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md). Its seeded Instructor may use rehearsal through the
ordinary authenticated production path over ordinary live courses, assignments, published
questions, and deterministic graders. The workflow retains only its immutable execution and audit
evidence outside learner records; it does not create a second content source or application.

## Contract

### Dedicated aggregate

T4 introduces a physical `RehearsalRun` aggregate with distinct `RehearsalRunId` and
`RehearsalAttemptId`, private issue/grading receipts, frozen items, and a focused
`RehearsalStore`. It has separate Memory and PostgreSQL implementations, commands, and response
types.

T4 does not add a rehearsal mode to `AssignmentRun`, does not extend the ordinary `RunStore`, and
does not project rehearsal into learner data. The only shared execution seam is a small
context-neutral helper for deterministic selection and trusted `RunBackend` issue, render, and
grade preparation. Rehearsal authorization and persistence stay rehearsal-specific.

### Immutable evidence and mutable state

Rehearsal evidence is a binding aggregate and persistence contract. Each selected item appends its
frozen item identity, version, and canonical content digest before delivery. Each accepted submission
appends the submitted response and grading receipt as private rehearsal-local evidence. An accepted
response is bounded to the server-approved response shape and size; its receipt is bounded to the
server-produced grading outcome, feedback projection basis, backend receipt reference, timestamps,
and canonical digest material. The private evidence is never a browser or public-corpus payload.

Evidence rows are append-only and tamper-evident. The server computes a canonical digest for every
evidence record and hash-chains it to the previous rehearsal-local evidence digest, beginning from a
canonical run genesis digest. Store APIs expose append and verified read operations, not evidence
replacement, update, or deletion. PostgreSQL enforces the same shape with constrained columns,
append-only roles/privileges, and mutation/deletion triggers; Memory enforces it through its closed
Store implementation. Digest verification fails closed when persisted evidence is altered or its
chain is incomplete. These controls are concrete database and Store defenses, not a claim that a
database superuser cannot change bytes.

Issue capabilities, capability revocation, and lifecycle transitions are separate non-answer-bearing
state records. They may append a transition or revoke an unsubmitted capability during discard,
expiry, stale-revision invalidation, or source-context removal, but cannot rewrite accepted
evidence.

Rehearsal is a retained, independently verifiable tenant-owned archive, not a learner record and
not a public archive. It retains immutable historical tenant, course, assignment, direct-Instructor
membership, assignment revision, subject-fingerprint, frozen-item, response-definition, receipt,
and genesis/evidence-chain provenance. Those values support logical deterministic reproduction and
verification without a live assignment source. A later catalog-purge capability must explicitly
preserve the frozen version material that this proof needs.

Student-record deletion and a removal that leaves all rehearsal source bindings intact do not fence
the archive: rehearsal contains no learner identity, enrollment, or learner-record projection. An
authorized removal of its assignment, course, or direct Instructor membership instead atomically
fences each matching active rehearsal before deleting the source context. The fence is a private
Store/SQL capability under the existing authorized source-removal transaction, not a browser route
or a T4 retention worker. It locks and verifies each aggregate, frozen inventory, evidence chain,
and claim history; appends all required source-context claim revocations; transitions the run; and
clears its active-run projection before the source delete can commit. Any invalid aggregate, claim,
or transition aborts the entire source-removal transaction.

`rehearsal_run` has no live source foreign keys or cascades to course, assignment, membership,
enrollment, revision, policy, or learner tables. Those bindings are historical provenance, not
authorization references. Rehearsal-internal parent relationships use `ON DELETE RESTRICT`, and
evidence and claim transitions are append-only with guarded update/delete operations. Normal
read, resume, issue, and submit routes still require live direct-Instructor/course/assignment
authorization and conceal a fenced archive. T4 adds no archive UI, reader, export, deletion API,
TTL, or worker. A future tenant-retention evidence reader and tenant-erasure workflow own audited
post-removal access and erasure policy; tenant deletion remains refused while retained rehearsal
evidence exists.

The shared ledger reserves the ordered T4 migration sequence before code:
`2026081811_rehearsal_runs.sql`, `2026081812_assignment_mutator_authority.sql`, then
`2026081813_rehearsal_source_fence_integration.sql`, and
`2026081814_assignment_definition_capabilities.sql`, followed by
`2026081815_rehearsal_start_intent.sql`, and
`2026081816_course_group_mutator_capabilities.sql`, followed by
`2026081817_learner_work_source_preparation.sql`,
`2026081818_course_creation_authority.sql`,
`2026081819_course_grade_control_capabilities.sql`, and
`2026081820_scoring_commit_source_preparation.sql`, followed by
`2026081821_rehearsal_operation_idempotency.sql`, then
`2026081822_rehearsal_delivery_material.sql`. The first creates only the rehearsal namespace
and forced RLS. It binds tenant, course, assignment, direct Instructor-course membership, and
assignment revision, but has no foreign key to enrollment or ordinary activity records. The second
owns the assignment-mutation authority substrate and private stale-rehearsal invalidator. The third
owns direct-Instructor and retention source-removal fence integration. The fourth completes the
actor-authorized assignment-definition command family: closed create and complete normalized
replacement, its normal-path application caller cutover, and retirement of the incomplete scalar
replacement from the second migration. This remains one unaccepted T4 package with no callable
compatibility or fallback layer. Their mutable allocation state remains solely in the shared ledger;
accepted migrations remain unchanged.

The fifth migration owns the explicit start-intent capability. It retires the ten-argument
rehearsal-start function and admits one twelve-argument function with
`start_new_after_completion` and a Store-verified latest-owner-run witness. It retains one live,
durable operation: a successful call resumes the locked active run, terminalizes and replaces it
for a changed subject, or creates the explicitly requested post-completion run. It is not a
read-only rehearsal view, and it does not create a parallel browser-only state or a client-visible
decision token. The eleventh migration owns the durable start receipt and idempotent public start
broker, then retires the directly callable 1815 start entry from `ple_app`; 1815 remains the
start-intent decision rather than the durable operation protocol.

The sixth migration owns dedicated execute-only course-group mutation broker capabilities and exact
affected-assignment witnesses for invalidation. It does not restore direct `ple_app` DML; its
allocation is reserved and remains unaccepted.

The seventh migration owns typed learner-work and entitlement source-preparation witnesses, including
the immutable issued-question source/execution snapshot described below. It closes ordinary-work
regressions caused by the deliberate `1812` and `1814` revocations while preserving direct
learner-artifact writes after Rust verifies the bounded witness. The eighth owns
the closed course-provisioning authority required after `1817` revokes direct source DML: ordinary
Instructor and Sysadmin calls are session-bound, while Base Course installation uses a separately
attested installer capability. It does not restore broad `ple_app` DML or introduce an Instructor
platform role. The ninth owns isolated grade-scheme replacement and export-audit capabilities. The
tenth owns isolated scoring-worker preparation and finalization capabilities. The eleventh owns the
durable non-submission rehearsal protocol: start/delivery/discard receipt roots, delivery operation
events and safe screen receipts, broker authority, and forward stale/source/terminal revocation
integration. `2026081811_rehearsal_runs.sql` remains the existing 999-line rehearsal aggregate
migration; `2026081815_rehearsal_start_intent.sql` remains start-intent preparation. The durable
protocol is only `2026081821_rehearsal_operation_idempotency.sql`. These forward-only brokers never
restore broad `ple_app` source-row update authority and remain unaccepted.

The twelfth migration owns canonical immutable rehearsal delivery material. A successful
Store-owned start/freeze transaction writes one append-only answer-free source snapshot and one
forced-RLS grader-only private-execution sibling for every selected frozen attempt, then verifies
the complete frozen inventory before committing its start receipt. The source snapshot contains the
checksummed `IssuedQuestionSnapshotV1` and exact family witness; the private sibling contains the
checksummed closed execution contract sufficient for deterministic issue, render, and grade. It
adds the dedicated source-freeze and route-scoped verification broker authority, the Store-owned ordered-progression
decision, exact delivery-to-frozen-attempt binding, and atomic final-item completion. It grants no
`ple_app` private-sibling read and preserves `1811`, `1815`, and `1821` as separate aggregate,
decision, and operation owners. Material is immutable rehearsal evidence, not a cache or a
live-demo sidecar. Missing, malformed, unsupported, or inconsistent material fails closed;
delivery never reconstructs it from the mutable catalog, renderer, provider, client progress
state, or a compatibility path.

### Rehearsal delivery material

`RehearsalDeliveryMaterialStore` is the separate server-only facade for route-scoped verification. It
is not added to `RehearsalStore`, ordinary `Store`, or `CatalogStore`. Its
`verify_rehearsal_delivery_material_from_route` command validates the complete immutable material
inventory for early fail-closed feedback and returns only `Result<(), StoreError>`. It exposes no
candidates, binding digest, locator, IDs, source payload, selection, private material, or JSON to
routes or browser DTOs.

The verification transaction checks the active rehearsal, assignment revision, frozen inventory,
answer-free source snapshots, private siblings, and checksums before delivery proceeds. The later
atomic Store claim independently authorizes and locks the route, verifies the relevant persisted
material and bindings, derives the lowest unresolved frozen ordinal, and mints the opaque issue
handle. Replay, pending, stale, foreign, discarded, fenced, and source-removed paths perform no
new claim, material execution, or backend effect. Selection authority exists only in that Store
claim; verification never preloads or selects a candidate.

### Ordered assignment progression

The frozen material set is the rehearsal assignment. Its zero-based ordinal is contiguous and
authoritative. The Store derives progress from immutable frozen material, delivery operations,
submission claims, accepted evidence, and run lifecycle; it does not persist a mutable
`current_ordinal`, accept a browser item index, or consult a rehearsal catalog.

- The next item is the lowest frozen ordinal without accepted evidence.
- Exactly one item may be unresolved at a time. Prepared, dispatched, issued,
  submission-pending, or expired work prevents selection of a later item.
- Resume and exact replay return the persisted answer-free screen or pending state without a
  backend call. Retry retains the same frozen ordinal and snapshot in a new issue cycle only after
  the previous unsubmitted capability is explicitly expired or abandoned.
- The route-bound submission broker derives the sole current issued item and its presentation
  digest. The browser submits a bounded response and idempotency key, never a rehearsal attempt ID.
- An accepted item is terminal. The final accepted submission appends its immutable evidence and
  receipt, proves accepted coverage of every frozen ordinal, and transitions the rehearsal to
  `completed` in the same transaction.

`2026081822_rehearsal_delivery_material.sql` owns this progression authority because lawful next
selection depends on its frozen ordinal and material binding. `2026081821` remains the generic
append-only idempotency/event/receipt protocol; no additional migration, parallel progress table,
or compatibility path is introduced. WP-PROF-T5 may later choose a concrete deterministic pool
variation before freezing, while the T4 progression contract remains unchanged.

### Rehearsal timing authority

Ordinary issued-attempt timing is learner-record authority. Rehearsal never reads or writes learner
attempts, effective-policy receipts or current pointers, or timer jobs. Instead, `1822` persists an
immutable `RehearsalTimingWitnessV1` for each dispatched issue cycle. Its binding contains the
persisted `PreviewSubject` fingerprint, the frozen `QuestionDefinition` snapshot digest,
`rehearsal_run.started_at`, database/server `issued_at`, the derived deadline, and the derived
grace, plus the selected `PerQuestion`, `PerAttempt`, or `SubjectLimit` deadline source. The Store
creates and verifies that witness in the same serialized transaction that changes `prepared` to
`issueDispatched`; it never rereads current catalog or ordinary timing rows.

- `PerQuestion` derives from `issued_at`; `PerAttempt` and the subject time limit derive from
  `rehearsal_run.started_at`; `Untimed` has no question deadline.
- If both a question deadline and subject time limit exist, the earlier deadline wins. Question
  grace applies only when the question deadline wins. An untimed question with no subject limit
  never expires.
- The selected preview moment's `available`, `due`, and `close` values decide whether Start is
  eligible. They never become a later rehearsal deadline or arbitrary rehearsal TTL.
- The Store appends exactly one immutable `issueDispatched -> expired` event only after the
  deadline plus inclusive grace. Expiry and a first submission claim use the same serialization
  boundary, so exactly one lawful outcome wins.
- Retry is route-, revision-, and idempotency-bound. It accepts no attempt identity, consumes only
  a current generation whose immutable selected deadline source is `PerQuestion`, and creates the
  next issue cycle for the same frozen ordinal, snapshot, private material, and seed. Expiry whose
  source is `PerAttempt` or `SubjectLimit` returns the visible, replayable `RunTimeExhausted` state
  without a successor or screen. A per-question retry prepared before a run-wide cap is rechecked
  at dispatch; if that cap has elapsed, the Store appends and replays the same pre-dispatch terminal
  state without issuing a screen. Continue never advances an expired or run-exhausted generation.

The timing witness is immutable rehearsal evidence rather than a rehearsal content sidecar or a
policy cache. No arbitrary TTL, mutable current ordinal, timer worker, or new migration is added.
Malformed fingerprints or snapshots, overflow, a negative grace, or an impossible deadline fail
closed as `Unavailable` without an ordinary timing read, catalog fallback, or reissue path.

### Issued question snapshot

After the active `1818` completion handoff, T4 adds one bounded ordinary-learner dependency:
`IssuedQuestionSnapshotV1`. It is a closed, immutable, server-only source/execution witness owned
by an issued attempt and by a durable prefetch reservation until atomic promotion. It applies only
to newly issued pre-production data. There is no legacy reader, backfill, catalog fallback, or
reissue path.

The V1 payload contains its schema version, the complete `QuestionDefinition`, and exactly one
family witness: native physical-asset bindings only when no specialized issued contract owns them;
QTI source artifact; external source artifact and integration-profile identity; or no extra witness
for flat and WebWork. `question.problem` and `question.version` must match the attempt and run item.
The canonical complete payload, including schema version, receives a SHA-256 checksum. Migration
`2026081817_learner_work_source_preparation.sql` adds non-null attempt payload and checksum columns
with the existing checksum discipline. `PrefetchedQuestion` embeds the same V1 snapshot in its
already checksummed payload; promotion compares the complete snapshot and writes it while consuming
the reservation in one transaction. No second prefetch table or parallel checksum columns are added.

The snapshot supplies the common immutable definition; it never replaces specialized private
authority. `IssuedFlatGradingContract` remains the flat key/grading authority, and the WebWork
contract plus replay state remain the WebWork source, renderer, and private form-mapping authority.
Receipt presentation and grading envelopes remain answer-free presentation authority, and attempt
provenance remains corroborating provenance. Specialized contracts must agree with the snapshot
question and fail closed on disagreement. The decoder recomputes the checksum, validates identity,
and returns `Unavailable` for missing, malformed, substituted, or mismatched data before a first
grade backend call.

Submission, replay, and external activity retain this order:

1. Authenticate and parse the complete typed route binding.
2. Establish live tenant, active Student membership, course access, effective policy, and opaque
   attempt/run/enrollment ownership under the existing `1817` preparation lock/witness.
3. Read only enough idempotency/receipt state to recognize an exact replay; return that receipt
   under current disclosure policy without catalog, snapshot, renderer, provider, or grader reads.
4. For a first effect only, decode and validate the issued snapshot, then load family-specific
   issued authority.
5. Perform one backend operation and atomically persist its outcome, receipt, and successor or
   prefetch-promotion state.

Live authorization, disclosure, deadlines, revocation, assignment policy, launch-proof expiry, and
broker lease state are deliberately not snapshotted. A valid snapshot never grants access by itself.
The following issued-attempt timing rule governs ordinary learner attempts only; it does not govern
or supply authority to rehearsal. For an active issued learner attempt, the existing locked
attempt-local authored deadline and non-negative grace are the immutable definition-derived timing
baseline. PostgreSQL reads
`attempt_timing_current.authored_deadline` and `authored_grace_seconds`; Memory reads the equivalent
`MemoryAttemptTiming` values. They are established at issuance and remain distinct from current live
course, group, accommodation, assignment, entitlement, and revocation policy. Re-resolution uses
that stored baseline with current restrictions; it never reloads a catalog `QuestionDefinition`,
reuses the mutable effective timer as authored authority, or adds a V1 timing field, table, or
migration allocation. A missing, malformed, negative-grace, or impossible baseline fails closed as
`Unavailable` without a catalog fallback or timer-job mutation.
Native non-flat grading uses the V1 definition and captured physical bindings; native flat adds the
existing flat contract; WebWork cross-checks V1 but grades from its issued contract and replay
mapping; QTI resolves its captured archive artifact through the existing private grader; and
external/iMathAS validates its broker rows and source/profile against V1 before resolving only the
captured artifact. Exact replay is receipt-only for every family. No post-issue path may read current
catalog definitions, asset relations, mappings, imported payloads, provider locators, or renderer
state to reconstruct issued work.

Memory and PostgreSQL persist the same typed V1 value and make identical decisions for identity
mismatch, absent or corrupt data, cross-attempt substitution, withdrawn catalog visibility, first
grade, and exact replay. The existing `ple_prepare_attempt_work` capability and row lock remain the
sole PostgreSQL preparation path; no direct `ple_app` authority is restored. The snapshot is never a
browser DTO, receipt field, response-cache entry, Wasm input, URL, client-storage value, or safe
debug payload. Diagnostics may use only the existing safe policy for an attempt ID, family, and
checksum prefix; payloads and source locators remain redacted.

### Issued-attempt evidence authority

The binding architecture decision is one expanded, broker-first, route-bound read for all
learner-owned issued-attempt private evidence. `RunStore`/`Store` keeps the authority-bearing
method name and takes the complete trusted route tuple:

```rust
async fn read_issued_attempt_evidence(
    context: TenantContext,
    actor: UserId,
    binding: LearnerWorkRoutingBinding,
    attempt: QuestionAttemptId,
) -> Result<IssuedAttemptRead, StoreError>;
```

`IssuedAttemptRead` is server-only, non-serializable, and redacting in `Debug`. It has exactly
these lifecycle branches:

- `Active(ActiveIssuedAttemptEvidence)` contains the common immutable receipt-evidence tuple,
  the active flat/WebWork contracts, and active WebWork replay authority.
- `Submitted { evidence, receipt }` contains the common tuple and only the checked immutable,
  answer-free `SubmittedQuestionReceipt` needed to render the submitted question.
- `TerminalWithoutReceipt { evidence, status }` contains the common tuple and the known terminal
  status when no learner question-delivery receipt exists.

The common tuple is the validated presentation binding, answer-free `ReceiptPresentationSnapshot`,
and server-only grading envelope. The submitted receipt does not expose a full
`SubmissionRecord`, learner response, grading result, feedback, summary, or other disclosure
material. Narrow accessors expose common receipt evidence to trusted Base Course validation,
active presentation only for `Active`, and submitted presentation only for `Submitted`. No
browser DTO, Wasm input, or public serialization exposes the aggregate or grading material.

#### One transaction and current lifecycle

PostgreSQL begins the tenant transaction and calls `ple_prepare_attempt_work` before any protected
source, evidence, or receipt read. The broker validates the exact tenant, learner, course,
assignment, run, attempt, active Student membership, and ownership witness. Under that one state
snapshot it then reads the immutable attempt identity/lifecycle row, hydrates the common tuple once,
and derives the current lifecycle from the relational `attempt_status` plus `submitted_at` witness.
The immutable issuance payload is evidence of issuance only: its raw `in_progress` fields must not
override the current relational lifecycle. A shared decoder checksum-validates immutable identity,
requires issuance-shaped raw fields, overlays the allowlisted relational status and timestamp, and
fails closed on malformed payload, unknown status, impossible timestamp, or witness mismatch.

For relational `in_progress`, the capability validates active-only flat/WebWork contracts and
replay evidence. For relational `submitted`, it loads and validates the immutable submission
receipt in the same transaction, cross-checks its attempt, run, and presentation against the common
tuple, and reduces it to `SubmittedQuestionReceipt`. A submitted WebWork read therefore succeeds
from its checked immutable receipt after private active replay state has been deleted; it never
requires active replay. Active WebWork requires both its contract and replay and fails closed when
either is absent. `auto_submitted`, `needs_manual_grading`, `cleared`, and `exempt` return
`TerminalWithoutReceipt` unless a later contract explicitly supplies an immutable learner receipt.
Missing, corrupt, or inconsistent submitted evidence returns `Unavailable`; it never falls back to
active evidence, a current catalog, renderer, replay mapping, or grader state. A valid terminal
attempt without a delivery receipt maps to the existing delivery-state conflict.

Memory follows the same explicit-binding, tenant, actor, membership, lifecycle-overlay, receipt,
and tuple-integrity rules and makes the same `NotFound` versus `Unavailable` decisions. The
capability is the sole question-delivery authority. Retire nested question-GET uses of
`learner_get_question_attempt`, `authorized_run`, `owned_assignment_for_run`, and
`submission_record`; a separate unbound preflight or submitted-receipt authorization call is not
permitted. Distinct non-question history or prefetch consumers may retain their methods only with
documentation that they are not question-delivery authority.

#### Route and authorization boundary

`get_attempt_question` authenticates, constructs `LearnerWorkRoutingBinding::new(course,
assignment)` from its typed route, and calls the capability once. It performs no question-GET
preflight lookup and creates no attempt-ID-to-route resolver, so authorization and disclosure share
one transaction rather than a two-transaction TOCTOU boundary. The active branch renders only the
answer-free snapshot; the submitted branch renders only the immutable receipt presentation.
Wrong course or assignment, foreign actor or tenant, unknown attempt, inactive or revoked Student
membership, and revoked live access map to the existing concealed 404 with `no-store` and no
envelope. Missing or corrupt authoritative evidence maps to unavailable/server failure, while a
known terminal state without a delivery receipt uses the existing state conflict.

#### Migration, database authority, and ownership

No SQL migration is approved. This repair reuses the already allocated
`2026081817_learner_work_source_preparation.sql`; it must not amend an existing T4 migration,
restore direct `ple_app` table privileges, or add an installer-only bypass. PostgreSQL may retain
tenant-RLS-scoped plain `SELECT` for bounded post-broker hydration. The protected proof is that
direct `ple_app` `FOR UPDATE` or other source-graph locking and source DML are denied, while the
execute-only `1817` broker remains the sole source-graph locking/mutation-preparation path. With no
tenant context, RLS returns no rows; the proof must not assert a false blanket no-`SELECT` model.

Ownership order is one atomic slice: (1) the learning-data-access owner changes the result
contract, Memory parity, PostgreSQL hydration, lifecycle decoder, and receipt cross-check; (2) the
server/Base Course owner adopts the result, removes route preflights, and keeps one evidence read;
(3) the test owner adds Memory and isolated PG17 conformance before the completion-oracle rerun.

The focused gates are: Memory and isolated PG17 parity for valid active, valid submitted receipt,
terminal without receipt, wrong route, foreign actor/tenant, unknown attempt, and revoked/inactive
Student; raw issuance `in_progress` overlaid by relational/witness `submitted`, with tampering of
either lifecycle source failing closed; active WebWork contract-plus-replay and submitted WebWork
receipt-only behavior after replay deletion; receipt absence/corruption/presentation mismatch
returning `Unavailable` without fallback; nested GET using only the route-bound Store method with
answer-free active/submitted responses and concealed cross-route denial; Base Course validating
both branches from one common result with mismatch rollback; and PG17 authorization proving
tenant-RLS plain hydration is available while direct learner-graph locking/source DML are denied
and the execute-only broker succeeds. Rerun the isolated four-case completion oracle and
serializable barrier only after these focused gates pass. These focused gates precede T4 acceptance
and do not replace the complete final-material-tree Validation suite below.

### Lifecycle and invalidation

There is one active rehearsal for each tenant, course, assignment, and direct Instructor-course
membership. A transactional partial unique constraint enforces that rule.

- Start atomically resumes an active rehearsal for the same assignment revision and exact canonical
  subject fingerprint.
- Starting with a different subject at the same revision discards the existing active rehearsal and
  creates a new one.
- A completed rehearsal is terminal. Starting another requires an explicit user action, carried by
  the server-owned `start_new_after_completion` intent and verified against the locked latest run.
- An explicit discard is terminal and clears only rehearsal-local browser state.
- A changed assignment revision atomically transitions every active rehearsal for that assignment
  to `DiscardedStaleRevision`, revokes unsubmitted private issue capability, and returns an
  answer-free `412 Precondition Failed` contract.
- Authorized source-context removal atomically transitions every matching active rehearsal to
  `DiscardedSourceContextRemoved`. Every `Prepared` or `GradingDispatched` submission claim receives
  one append-only `RevokedSourceContextRemoved` claim transition; completed claims and accepted
  evidence remain unchanged. This terminal state is neither revision invalidation nor Instructor
  discard. A revoked claim cannot be reclaimed or completed, and old handles fail closed.

Assignment-definition mutation performs that invalidation within its existing strong revision
transaction. Rehearsal start, read, issue, submit, and resume repeat the locked revision check, so
an old-revision issue or grade cannot win a concurrent mutation. There is no arbitrary rehearsal
TTL. A rehearsal is resumable while its owner remains a direct authorized Instructor and the frozen
revision remains current. `1822` derives any operational deadline from the immutable timing witness,
not from the selected preview moment or an ordinary learner record. Normal issue expiry produces a
typed recoverable `Expired` outcome; Retry issues the same frozen item in a new cycle.
Course, assignment, and direct-Instructor-membership removal take the separate source-removal fence
under the same source locks before their delete. Student deletion and a source operation that does
not remove one of those bindings do not take that fence.

### Delivery, grading, and records

T4 admits deterministic native server-owned grading and the existing trusted renderer path only
through a rehearsal-specific execution capability over Store-minted frozen work. The capability
shares the ordinary native adapter and presentation primitives, but it does not manufacture a
learner `QuestionAttempt`, call an ordinary learner `RunStore`, or reread mutable catalog or asset
state. Native and flat question families are admitted only when their exact frozen family witness
and private execution sibling agree; every other family fails closed until its isolated rehearsal
contract exists. The browser gets an answer-free presentation envelope, response definition,
public timing and disclosure data, and server-projected feedback.

Each dispatched generation commits one closed, immutable issued-execution artifact before its
screen receipt. The artifact is rehearsal execution and audit evidence, never a question source,
cache, alternate bank, or simulated content row. It retains the exact generated grading envelope,
parameter hash, attempt provenance, answer-free presentation snapshot and binding, plus digests of
the sole frozen source/private siblings and Store-minted generation. It does not duplicate the
frozen question snapshot. Canonical bounded bytes are append-only: exact replay resumes the same
artifact without invoking the adapter again, while substitution or a different second result fails
closed. Current V1 admits only no-asset native and flat execution; asset-bearing rehearsal remains
unsupported until frozen physical rendition bindings and opaque rehearsal asset tokens have an
explicit versioned contract.

The Store persists only rehearsal-local execution evidence: the immutable artifact, validated
`RehearsalActiveScreenV1`, timing witness, accepted evidence, and receipts. The screen's
server-derived full 32-byte commitment covers the complete typed
`RehearsalQuestionPresentationV1`; the browser receives only the commitment's 128-bit public token
as a stale-screen check. Persistence reads, replay, delivery claims, and submission claims rehydrate
the artifact and typed presentation, rederive the full commitment, and fail closed on any mismatch.
A route or browser never supplies authoritative screen JSON, full commitment, artifact, source
snapshot, durable item mapping, or grading metadata.

Submission recovery uses a separate sealed grader preparation keyed by the already-authorized
route and submission idempotency key. After `gradingDispatched`, a retry rehydrates the same request,
claim-to-delivery binding, issued artifact, frozen source, and private grading sibling; it never
reclaims a new logical operation or depends on a cached browser handle. The canonical presentation
mapper translates rendered response IDs back to durable grading IDs only inside this sealed
boundary. Exact completion races return the immutable receipt, while a merely prepared concurrent
claim returns pending without calling a grader.

The following outcomes are part of the closed contract:

- An unsupported external backend, including iMathAS until its isolation contract exists, returns
  `422 rehearsal_delivery_unsupported` before delivery.
- A manual or otherwise non-deterministically gradeable question is unsupported rehearsal source
  material. Start fails closed before creating the rehearsal run, frozen material, delivery
  capability, accepted-response evidence, or receipt. T4 has no `NeedsManualGrading` outcome,
  manual-grading queue, or compatibility path.
- Upload response families are rejected before upload body or object acceptance. A later dedicated
  package must own rehearsal-upload authorization, immutable evidence, retention, and deletion.

The database and Store design make the following ordinary traces unrepresentable for rehearsal:
enrollment, learner run, learner attempt, submission, current score, gradebook summary,
item-analysis observation, catalog contribution, export row, worker job, and learner audit change.
This is the package's core privacy and integrity invariant, not a downstream filter.

## Access and browser contract

Routes are namespaced below the Instructor course/assignment delivery-check surface and use opaque
public rehearsal references. Server code authorizes the actor, direct Instructor membership, exact
tenant/course/assignment binding, and route capability before decoding a protected reference,
subject, answer, or upload body. Requests and responses use closed strict decoders, bounded
content-type and size checks, `Cache-Control: no-store`, standard secure errors, idempotency, and
concealed denial behavior.

The browser owns only a namespaced response draft and idempotency key. It may perform response
format validation but never grading. A stale `412` clears rehearsal-local drafts and results,
preserves the non-authoritative preview builder draft, and offers Reload latest assignment revision.

Every visible state says "Rehearsal", displays a role/group-only subject summary, and offers
native keyboard Start, Resume, Submit, Continue, Retry, Discard, and Return controls. The focused
page reuses response and feedback presentation, not `RunPage`, and is compact at 1280 by 800 while
remaining usable at 800 by 1280, 393 by 852, and 800 by 800. Student and outsider direct-route
denial occurs before protected transport or page mount.

## Security and privacy controls

The implementation applies the approved ASVS access-control, input-validation, error-handling,
transport, privacy, file, and extension controls. T4 evidence specifically establishes:

- direct-Instructor, exact-course authorization before protected decoding and concealed foreign
  record behavior;
- closed, bounded, answer-free transport and `no-store` responses;
- server-only deterministic grading, protected provider credentials, and no browser grading
  capability, key, checker source, private renderer metadata, learner identity, score projection,
  or UUID;
- transactional revision invalidation, idempotent submission, and recoverable expiry outcomes;
- forced-RLS tenant isolation for every durable rehearsal row; and
- explicit refusal of unsupported external delivery and uploads before those surfaces are reached.

These controls preserve the durable decisions in
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md): student-linked data stays protected, public evidence
contains no answers or FERPA records, and the canonical visible workflow remains keyboard-first.

## Module ownership

Keep each authored source below 1000 lines and create focused modules rather than broadening the
ordinary run, Store, or page owners.

| Owner area | Planned responsibility |
| --- | --- |
| `question_model` | Closed rehearsal IDs, lifecycle, request, receipt, and strict serde shapes |
| `domain` | Pure lifecycle, subject fingerprint, revision, idempotency, and no-projection invariants |
| `learning-data-access` | Rehearsal aggregate contracts and `RehearsalDeliveryMaterialStore`; Memory/PostgreSQL source-freeze, route-scoped verification, Store-owned claim/progression, immutable timing-witness, expiry/retry, per-generation issued-execution artifact, sealed crash-recoverable submission preparation, and atomic-completion parity; SQL migration, RLS, grader-only private material, append-only evidence, and invalidation transactions |
| ordinary run issuance | `IssuedQuestionSnapshotV1`, canonical serialization/checksum, attempt/prefetch promotion, and Memory/PostgreSQL preparation parity; active-attempt timing reads its existing authored baseline |
| `server` | Direct-Instructor route composition, coordinator injection, route-scoped material verification, rehearsal-specific Native/Flat issue and grade capabilities over ordinary adapter primitives, exact artifact issue/resume, DTO conversion, no-store/error mapping, and typed unsupported/unavailable execution outcomes |
| generated API and Solid | Closed TypeScript decoders, rehearsal client, namespaced attempt state, focused Instructor page |
| real-stack evidence | Browser journey, Python declaration, screenshot manifest entries, privacy-safe corpus publication |

## Dependency order

1. Freeze the closed question-model, domain, Store, lifecycle, error, backend-support, and wire
   contracts after the shared allocation is recorded.
2. Implement Memory/PostgreSQL parity and the ordered twelve-migration persistence boundary:
   rehearsal/RLS, assignment-mutation authority substrate, source-removal fence integration, then
   actor-authorized assignment-definition create and complete normalized replacement with the
   incomplete scalar replacement retired, followed by explicit rehearsal start intent and
   execute-only course-group mutation capabilities with exact affected-assignment witnesses,
   learner-work source preparation with `IssuedQuestionSnapshotV1`, closed course-provisioning
   authority, grade-control capabilities, scoring-commit preparation, the durable rehearsal
   operation protocol, and immutable delivery material. The operation protocol forward-adds
   idempotent start/delivery/discard roots, events, receipts, and revocation integration; the
   material migration forward-adds source snapshots, grader-only private siblings, source-freeze
   brokers, and route-scoped verification. Neither amends the existing 1811 aggregate or permits a
   mutable-catalog fallback.
3. After the `1818` completion handoff, implement the snapshot contract in one storage-contract
   batch (`1817`, `contracts/runs.rs`, issued contracts, and attempt issuance), then one data-access
   parity batch (Memory/PostgreSQL issuance, prepared-attempt hydration, and the active-attempt
   course-policy call site), then one backend consumer batch (run submission, prefetch, external
   tool, QTI, and iMathAS). Preserve flat and WebWork specialized issued contracts; move replay
   recognition before snapshot hydration; remove post-issue catalog hydration; and replace the
   PostgreSQL timing reread with its locked existing authored deadline/grace baseline. The
   course-policy owner does not add a parallel timing payload or snapshot field.
4. Keep ordinal selection and opaque issue-handle minting in the atomic Store claim. Implement the
   rehearsal-specific execution capability, per-generation issued-execution artifact, canonical
   rendered-response translation, and sealed crash-recoverable submission preparation before
   adding server routes and generated closed DTOs. Reuse ordinary adapter primitives without
   constructing learner attempts or widening ordinary learner Store authority.
5. Implement the Instructor rehearsal page and recovery behavior; add the one canonical real-stack
   journey and the corresponding corpus entries.
6. Obtain independent architecture, security/privacy, HCI, and documentation reviews. Resolve every
   P0-P3 finding before package acceptance.

## Validation and acceptance

Focused validation proves the behavioral boundary before broad final validation:

- qmodel/domain closed-shape, lifecycle, fingerprint, revision, and idempotency tests;
- Memory/PostgreSQL RehearsalStore conformance, including assignment revision invalidation,
  evidence-chain verification, and refusal of accepted-evidence update, delete, or replacement;
- fresh-chain PostgreSQL/RLS proof through 1822 that 1811 remains unchanged at 999 lines, 1821
  alone establishes durable start/delivery/discard replay, broker-only mutation, append-only
  receipt/event integrity, and stale/source/terminal revocation integration, and 1822 alone
  establishes immutable rehearsal source snapshots, forced-RLS grader-only private siblings,
  source-freeze brokers, and the unit-returning route-scoped verification seam plus its independent
  atomic Store claim, including lowest-unresolved-ordinal derivation and opaque issue-handle minting;
  it also proves source-owned timing/artifact verifier chains, grader-only artifact preparation and
  commit, forced-RLS append-only issued artifacts, exact commit/replay/conflict behavior, required
  artifact binding before screen or submission persistence, and no application artifact authority;
- Memory/PostgreSQL source-removal conformance: student-only deletion preserves active rehearsal
  evidence; assignment/course/direct-Instructor removal fences matching runs atomically, appends
  required claim revocations, retains completed evidence, removes only the active-run projection,
  and leaves chain verification green; aggregate corruption prevents both fence and source removal;
  normal routes conceal fenced archives; and a private test-only verified reader proves retained
  evidence without querying a removed source;
- fresh PostgreSQL/RLS evidence that compares ordinary traces before and after rehearsal and proves
  zero enrollment, run, attempt, submission, summary/gradebook, analysis, contribution, export,
  and ordinary-job change, while database constraints and mutation/deletion defenses preserve
  rehearsal-local accepted evidence; this also proves cross-tenant source-fence refusal, forced
  RLS/no direct read, no source foreign key or cascade, internal `RESTRICT` relationships,
  append-only mutation guards, retention-broker authority, and tenant-delete refusal while retained
  rehearsal data exists;
- server authorization-before-decode, strict decoder, no-store, concealment, key-free response,
  deterministic native issue/grade, explicit unsupported-backend tests, and atomic start refusal
  for manual or otherwise non-deterministically gradeable source material with no rehearsal record,
  evidence, receipt, Store selection, material execution, backend call, or ordinary learner trace;
- focused issued-snapshot Memory parity for every configured family: issue a complete contract,
  withdraw catalog visibility, prove a first effect, and prove exact replay without catalog or
  backend access; PostgreSQL fresh migration, forced-RLS/grant, checksum, and cross-row-tamper
  conformance must prove the same matrix before that gate is green;
- prefetch reservation/promotion/crash-recovery behavior proving byte-for-byte V1 identity and
  atomic reservation consumption without a new attempt or timer; live authorization denials for
  revoked membership, route/tenant/run mismatch, denied policy, and expired external proof despite
  a valid snapshot; answer-free HTTP/decoder gates proving no snapshot, source artifact, locator,
  answer, mapping, or provider state reaches the browser;
- active-attempt timing parity for timed, untimed, per-question, and per-attempt definitions: after
  catalog withdrawal, Memory and PostgreSQL recalculate from the locked authored deadline/grace and
  current live exceptions; due, close, assignment-limit, membership, entitlement, and revocation
  outcomes retain their current authority; missing or corrupt authored timing fails closed as
  `Unavailable` with no catalog read or timer-job mutation; and exact replay remains receipt-only
  without hydrating timing or snapshot state;
- rehearsal source-material parity: every selected attempt atomically receives exactly one frozen
  evidence row, answer-free snapshot, and grader-only private sibling; independently tampered
  question/version/response/content/seed/algorithm, public checksum, private checksum, and family
  witness fail closed before Store selection, material execution, or backend work; source edit,
  replacement, or removal after
  start preserves reproducibility from frozen material or fences the rehearsal, never selecting
  current catalog content; and replay, pending, stale, and foreign routes invoke neither Store
  selection, material execution, nor backend;
- ordered-progression Memory/PostgreSQL parity: frozen ordinals are exact and contiguous; the Store
  selects only the lowest unresolved ordinal; one open delivery or submission prevents later-item
  selection; concurrent Continue and Submit produce one backend effect and one accepted evidence
  entry; resume returns the exact persisted screen; expiry and Retry retain the same frozen item;
  and the last accepted submission atomically commits its receipt, accepted-item integrity entry,
  complete coverage proof, and `completed` run lifecycle;
- rehearsal timing-witness Memory/PostgreSQL parity: each issue cycle binds the persisted subject
  fingerprint, frozen snapshot digest, run start, and database/server issue time; untimed work with
  no subject limit remains resumable; per-question time anchors to issue, per-attempt and subject
  limits anchor to run start, and the earlier deadline with its lawful grace wins; selected-moment
  due and close affect only preview eligibility; expiry and first submission serialize to one
  outcome; only per-question expiry permits Retry; per-attempt, subject-limit, and cap-elapsed-before-
  dispatch paths return one persisted, replayable `RunTimeExhausted` state without a screen; lawful
  Retry preserves the frozen ordinal, seed, snapshot, and private digests while changing only the
  cycle and screen capability; and every trace proves zero learner attempt, effective-policy
  receipt/current-pointer, timer-job, learner-run, submission, score, and gradebook effects;
- browser-free external, renderer, and replica service oracles proving no provider or renderer call
  on replay, plus one canonical production HTTPS journey that creates state through visible UI,
  withdraws content through an authorized visible workflow, and completes issued work without
  disclosure leakage; this is connected evidence, never a parallel mock application;
- canonical real-stack browser behavior for start, resume, submit, retry, stale revision, discard,
  keyboard operation, Student/outsider direct-route denial without protected transport, and the
  project viewport matrix; and
- screenshot capture and automated visual re-review for the private Instructor corpus. Visual proof
  supplements, rather than replaces, the persistent database invariant.

Acceptance requires all focused evidence, the independent reviews, and the complete final-material
tree Validation suite in [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md):

```text
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
./all_test.sh
./capture_screenshots.sh
git diff --check
```

The connected production browser invocation and its origin/cleanup receipts are required. No
required connected lane may be skipped. `./capture_screenshots.sh` publishes corpus changes;
`./all_test.sh` validates the final material tree without replacing that publication evidence.

## Post-acceptance handoff

After all acceptance gates are green on the final material tree, record the result in
[implementation_status.md](../implementation_status.md) and
[CHANGELOG.md](../../CHANGELOG.md), then advance only the sole global current-package handoff
defined there. WP-PROF-T5 remains the next professor dependency candidate; this plan does not
implement item pools, discovery, curricula, grading operations, external delivery isolation, or
uploads.
