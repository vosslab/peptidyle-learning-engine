# Concurrency contracts

PLE runs stateless API and worker replicas against shared PostgreSQL and
S3-compatible object storage. This document defines the durable atomicity and
race rules that keep those replicas correct when requests overlap, a process
dies, or a client retries. It complements
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md),
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#intended-database-model),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), and the frozen public/API register in
[CONTRACTS.md](CONTRACTS.md).

This is an implementation contract, not a claim that production deployment or
every operational recovery workflow is complete. Student and operator actions
after a failure belong in [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md). Status in
this document means:

- **Implemented**: current Memory/PostgreSQL and server owners provide the
  named behavior.
- **Required**: every new mutating path must follow this rule, even where a
  broader package has not yet been accepted.
- **Planned**: the boundary is specified but awaits its named release package
  and acceptance evidence.

## Authority status

**Current authority.** The current applied schema and mounted Authenticated Session
boundary establish database and Account facts. Store-backed course, authoring,
activity, worker, and object operations remain deferred; this document
specifies the concurrency rules they must satisfy when composed.

**Required for new work.** A mutation must name its transaction boundary,
idempotency or compare-and-swap rule, and lock order before it can expose a
result. A worker must name its lease and, when output can be superseded, its
generation fence.

**Planned boundaries.** The general Object Storage Check and Repair capability
remains unimplemented.
Managed failover and recovery objectives are deployment work; this
document does not claim either is implemented.

## Authority model

No API replica, browser tab, worker process, or object-store listing is a
correctness authority. PostgreSQL records are authoritative for exact course,
Student, workspace, and operation state; typed object records and checksums bind
PostgreSQL metadata to bytes. The browser can retry an authenticated request, while the server resolves its Account,
advance a revision, renew a lease, replace a receipt, or make a pending
operation final.

| State or decision                              | Authoritative owner                                                      | Status      | Main implementation owner                                                                                                                             |
| ---------------------------------------------- | ------------------------------------------------------------------------ | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Account identity and row access                | `AuthenticatedSession`, transaction-local forced PostgreSQL RLS          | Implemented | [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security), [connection.rs](../crates/learning-data-access/src/postgres/connection.rs) |
| Mutable authoring and assignment state         | Revisioned PostgreSQL rows                                               | Planned     | Future Store-backed authoring and Course Instance composition                                                                                         |
| Student submission outcome                     | Attempt-scoped idempotency and append-only evidence                      | Planned     | Future Store-backed Student delivery composition                                                                                                      |
| Background work ownership                      | PostgreSQL Job row plus opaque lease token                               | Planned     | Future Store-backed Job composition                                                                                                                   |
| Current analytic projection                    | Assignment/timing generation plus an active lease                        | Planned     | Future Store-backed scoring and analysis composition                                                                                                  |
| Published Question Revision                    | Immutable revision rows created from an exact Draft Question Edit Number | Planned     | Future Store-backed published Question composition                                                                                                    |
| Cross-system object inventory Check and Repair | Object Storage Check and Repair job                                      | Planned     | [release completion plan](active_plans/active/release_completion_plan.md)                                                                             |

## Account-scoped transactions and retries

### Transaction boundary

Every PostgreSQL operation on protected data begins a new transaction from a
server-derived `AuthenticatedSession`. The store sets `LOCAL ROLE` and the session Account setting
before querying, and commits or rolls back before returning the connection to the
pool. A pooled connection carries authority only for its current request. The complete
forced-RLS and role rule is in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security).

Required rules for a new Store mutation:

- Bind every protected read and write to the trusted authenticated Account context and preserve
  exact course, Student, workspace, and typed operation relationships.
- Authorize the exact Account relationship within the same transaction as the protected mutation.
- Commit all relational effects that define one outcome together, or leave no
  final relational effect. Do not split one receipt, revision update, and
  queue insertion across independently committed transactions.
- Treat database time as authoritative for leases, expiry, timing, and
  durable ordering. A replica clock is not an authority.

### Bounded retry scope

`retry_transaction` in
[connection.rs](../crates/learning-data-access/src/postgres/connection.rs)
replays an entire fresh transaction at most three times for PostgreSQL
serialization failure (`40001`) or deadlock (`40P01`). It does not retry a
single statement inside an aborted transaction, connection failures, or an
ambiguous commit.

| Operation inside a retry closure                                                      | Rule                                                                                                                                                | Reason                                                                      |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| New transaction, authorization, reads, inserts, updates, and commit                   | Allowed                                                                                                                                             | The full operation can be repeated after PostgreSQL aborts it.              |
| Deterministic validation and construction from command data                           | Allowed                                                                                                                                             | It has no externally visible effect.                                        |
| Object-store put/copy/delete, renderer call, email, HTTP callback, or message publish | Not allowed before a replayable commit                                                                                                              | Repeating it can duplicate an external effect or leave an ambiguous result. |
| Random ID, nonce, or receipt generation                                               | Generate before retry only when its value is intentionally the same across retries; otherwise persist a durable idempotency key before side effects | A retry must converge on one logical operation, not create multiple ones.   |

An operation that needs an external effect uses a durable prepare/claim/commit
protocol. The worker first obtains a fenced lease, performs a bounded external
preparation, then commits only if the same lease is still active. Course-banner
promotion is the implemented bytes-first example; object-inventory repair is
the planned generalization.

## Revisions and immutable publication

### Compare-and-swap edits

Mutable instructor resources use positive revisions. The browser receives a
strong ETag and returns it in `If-Match`; it does not send a revision in a JSON
body. The future Store-backed authoring, Course Instance, and Course Appearance
operations parse exactly one strong revision, check it against the stored row,
and return a conflict for a stale edit.

Required behavior:

- Read responses that support mutation expose the current strong revision.
- A mutation locks or conditionally updates the exact resource, verifies the
  expected revision and authorization, then advances the revision once.
- A stale request must return conflict without replacing the newer value or
  emitting a second downstream job.
- A request that may be retried also needs an idempotency receipt when its
  result is not safely inferred from the current revision alone.

### Publication race

Publication consumes one exact workspace draft revision and mints a new
immutable published question with a fresh Question ID and hidden exact pair. It
locks the draft row, checks its payload and revision, checks publisher
ownership, then writes the immutable published-Question facts in the same transaction. A
concurrent edit or a second publication request cannot silently publish a
different draft. The publication and assignment-reference constraints are
described in [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

Published content is never corrected in place. A correction publishes a new
question; existing assignments and issued attempts retain their pinned exact
evidence. This makes a race observable as an explicit conflict or a new
immutable publication, never as changed historical question content.

## Attempts, submissions, and convergence

### Attempt identity

An issued `QuestionAttemptId` binds the authenticated Student Account, exact course,
Assignment Attempt Question Pool Item, immutable Question Revision, seed, timing state, and
grading backend. It is the primary response authority. The browser sends the
minimal Student response plus an `Idempotency-Key`; it does not choose an
Account, course, key, seed, grading backend, or question kind. The exact browser
boundary is [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

### Submission idempotency

`submission_idempotency` is keyed by the exact `QuestionAttemptId`, and stores the
bound idempotency key plus the replayable result. The submit path accepts an
existing receipt only when the request identity/fingerprint agrees; a different
logical request for the same attempt conflicts. A transport retry therefore
returns the original authorized outcome rather than creating a second
submission or grading attempt.

| Situation                                                  | Required result                                                                                   | Implemented owner                                                          |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Client times out after submission reaches PLE              | Retry with the same key; return the stored receipt/outcome                                        | Deferred Student delivery Store                                            |
| Two replicas receive the same submission                   | One durable receipt wins; the other converges on the same receipt or conflicts on differing input | Deferred Student delivery Store                                            |
| Same attempt, different request/key/fingerprint            | Conflict; never overwrite response evidence                                                       | Deferred Student delivery Store                                            |
| Retry after a server-side failure before a receipt commits | No final submission exists; ordinary retry rules apply                                            | [connection.rs](../crates/learning-data-access/src/postgres/connection.rs) |

### Predecessor and successor receipt

Continued practice and retry behavior must converge even when the browser
retries or two replicas handle adjacent requests. A submitted predecessor has
one immutable `submission_next_attempt` receipt, keyed by the predecessor's exact
predecessor attempt. The receipt contains either the exact successor attempt or
an explicit `None` result. `ON CONFLICT DO NOTHING` lets concurrent finalizers
race safely; a losing finalizer must accept only the exact same stored result.

`question_prefetch` is similarly bound to the exact Course Instance/Student
Record attempt, predecessor, and
assignment position. It is valid only before the predecessor is submitted and
only when its full issued tuple-capability, binding, public snapshot, and
server-only Question Grading Input agrees with its protected columns. A later
attempt cannot reuse an old prefetch as a new attempt. The future PostgreSQL
and in-memory Stores must implement the same receipt rule.

## Leases and generation fences

### Worker leases

Workers claim jobs through PostgreSQL with a newly generated opaque lease
token. A complete/fail/commit operation verifies all of the following before
making its result current:

- the job is still `leased`;
- its token equals the claimant's token;
- its lease has not expired according to PostgreSQL time; and
- the worker command matches the claimed job's bounded payload.

A stale worker cannot complete or publish after another worker reclaims the
job. Crash recovery occurs by bounded lease expiry and retry/backoff, not by a
replica remembering what another process did. The fresh baseline's queue
ownership is recorded in [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

### Generation fences

Some workers calculate replaceable projections. A lease says _which worker may
act_; a generation says _which logical input remains current_. Timing,
Assignment Scoring, and Assignment Analysis each carry a positive generation.
Assignment Question Analysis rows belong to their Assignment Analysis for that
same Scoring Generation. The worker locks the current owner row and publishes
only when the requested generation still equals that row's generation. A
superseded job completes as superseded rather than overwriting a newer
projection.

This dual fence prevents an old calculation from becoming current after an
Assignment Content change, accepted-submission completion, authorized
attempt support, or a timer adjustment. The future scoring and auto-submit
Stores must enforce the same generation check.

### iMathAS Question Backend Sessions and Result Exchanges

An iMathAS Question Backend Session is server-created and bound to the exact
course/Student/attempt scope, expiry-bound, and revocable. Its random bearer token is stored only as a
hash; backend state is encrypted before persistence. The browser-visible
embed is presentation-only and cannot grade itself.

An iMathAS Result Exchange is separately idempotent, lease-fenced, and
indeterminate-safe. It binds the attempt/version/seed/Source Object Checksum,
iMathAS Response Checksum, backend correlation, and idempotency key before verification.
Before an effectful backend POST, the holder must atomically prove the exact
launch-token hash and an unexpired authoritative lease, then write the durable
pre-dispatch marker. A crash or ambiguous outcome leaves that marker in place:
no retry, new claim/launch, grade, finalization, or revocation may guess that
the backend did or did not act. A valid verified outcome clears the marker
only in the same final persistence transition. Grade retrieval is a
structurally safe GET-only operation, never a fall-through backend action.
Only the holder of the active lease can move it from `verifying` to `ready_to_commit`;
a verified token then binds the final commit. `failed` records a safe failure code,
and `cancelled` records its terminal time; neither permits a retained backend result.
This is a deferred iMathAS Question Backend Store and adapter requirement.

## Cross-system commit boundaries

Object bytes and relational metadata cannot commit atomically in one database
transaction. This section defines the race-safe commit boundary; the
caller/operator outcome and repair actions are in
[FAILURE_RECOVERY.md](FAILURE_RECOVERY.md), and the storage identity contract
is [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md).

### Planned Course Banner Upload promotion

The future Course Banner Upload capability writes normalized, checksummed bytes
under a temporary non-signable identity first. Its Store must bind the upload to
one Course Instance, Account, expiry, and Object Reference before a
revision-checked promotion can save a current Course Banner. The durable record
will remember consumption and cleanup state so competing cleaners cannot delete
another Course's object or undo a current pointer. [OBJECT_STORAGE.md](OBJECT_STORAGE.md)
records the current typed-object boundary; it does not claim that durable
promotion exists today.

### Planned Object Storage Check and Repair fence

The general Object Storage Check job must compare typed
database records with bucket inventory, repair only evidence-backed
prepare/promote/cleanup states, and record every decision. It must not treat a
bucket listing as permission to expose, delete, or recreate an object. Until
that package is accepted, new cross-system workflows must provide their own
bounded recovery state and must fail closed when the relational and object
facts disagree.

## Locking and deadlock discipline

PostgreSQL detects a deadlock and maps it to the bounded full-transaction retry
above, but retry is a safety net, not an excuse for arbitrary lock order.

Required ordering for a new multi-row mutation:

1. Establish `AuthenticatedSession` and authorize the exact course, Student, workspace,
   or leased operation target.
2. Lock the highest shared owner first: Course/Assignment or Assignment Attempt, as applicable.
3. Lock its direct child next: enrollment or assignment item.
4. Lock attempt, receipt, candidate, or projection rows last, in stable ID or
   assignment-position order when there is more than one.
5. Acquire an external lease before preparation; re-check it inside the final
   account-and-relationship-scoped transaction before publishing an effect.

Existing Assignment Attempt/prefetch paths follow Assignment Attempt, enrollment, predecessor Question Attempt, then
prefetch/receipt order. Existing scoring paths lock the assignment owner before
staging/current projection rows. New code that needs a different order must
document why it cannot use this hierarchy and add a focused concurrent
behavior test. Do not hold a database row lock while making an unbounded
network call.

## Review checklist

Before accepting a new mutating API, Store method, worker, or object workflow,
verify all applicable points:

- [ ] Account and exact relationship authority is reconstructed server-side inside its database
      transaction.
- [ ] The mutation has one durable authority and a clear conflict result.
- [ ] Retries cover only a complete replayable transaction; external effects
      are outside that retry or fenced by a durable receipt/lease.
- [ ] A stale ETag, lease, generation, predecessor, or idempotency key cannot
      overwrite the newer result.
- [ ] Concurrent equal requests converge on one receipt; different requests
      conflict without destroying evidence.
- [ ] Object bytes have a typed identity, checksum, lifecycle row, and bounded
      recovery behavior.
- [ ] Locks follow the documented hierarchy, and no lock is held across an
      unbounded network dependency.
- [ ] Memory behavior remains a conformance model; PostgreSQL remains the
      production authority and receives a focused live/RLS oracle where needed.

The package-level acceptance authority remains
[implementation_plan.md](active_plans/implementation_plan.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).
