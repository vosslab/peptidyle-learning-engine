# Failure and recovery contract

PLE has intentionally small, stateless API replicas, but its correctness does not depend on a
request reaching the same process twice. This document tells a student, browser,
or operator whether to retry, reload, stop, or require repair after an outcome
is known or becomes uncertain. It deliberately does not repeat the transaction,
compare-and-swap, lease, generation, object, or prefetch mechanics that make an
outcome safe; those belong to [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md)
and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md).

It describes implemented code-bound behavior and labels planned operational work. It does not
claim automatic failover, a recovery objective, managed point-in-time recovery, or production high
availability. Those require the release evidence defined by
[ROADMAP.md](ROADMAP.md) and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

The canonical live-demo browser path uses these same ordinary application contracts. Its seeded
people and records are fictional live data, and regeneration is a disposable reset; it is not a
second recovery or product path.

## Authority status

**Current authority.** Route-safe error result, attempt-scoped Question
Submission Receipts, no-store responses, and bounded private-backend failures determine
the current caller-visible outcomes described below.

**Required for new work.** Each capability must classify failures as committed,
rejected, retryable, or indeterminate; preserve enough durable evidence to
resolve an indeterminate request; and define the safe student/operator action.
Its race-safety mechanism is specified separately in
[CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md).

**Planned boundaries.** Question Presentation Checksum recovery awaits its accepted
payload migration; Object Storage Checks remain planned; managed point-in-time
recovery and production failover remain deployment work. None is current
automatic recovery behavior.

## Recovery rule

Every state-changing path falls into one of four outcomes:

| Outcome       | Meaning                                                                                                   | Caller behavior                                                                                                   |
| ------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Committed     | The requested durable effect is known to be visible.                                                      | Show the returned receipt or current Assignment Attempt Summary.                                                  |
| Rejected      | Validation, authorization, lifecycle, or immutable-state rules refused the request before a valid effect. | Do not retry unchanged automatically; correct the visible condition or reload.                                    |
| Retryable     | A dependency or serializable transaction was unavailable before a known commit.                           | Retry only through the operation's documented record/revision/Receipt or lease boundary.                          |
| Indeterminate | The client lost contact while a commit might have happened.                                               | Preserve the request identity, query/retry through its durable receipt, and never create a second logical action. |

The application must never turn an indeterminate result into a new attempt, seed, response,
worker effect, object identity, or user-visible grade. Nor may it replace an integrity failure with
best-effort data. PLE either reconstructs the exact durable state or fails closed.

## Imperfect-data recovery

The four outcomes above remain the classification for every state-changing path. The following
rules determine how PLE keeps unaffected work available when a specific item is imperfect:

- **Salvageable:** Normalize or reconstruct only from authoritative facts. Preserve original
  evidence when needed, and never guess identity, Product Role, authority, credential state, or a
  committed outcome.
- **Clean retry:** Discard only ephemeral attempt state and repeat the same logical operation.
  Preserve durable operation identity where an earlier commit may exist.
- **Irrecoverable item:** Quarantine, revoke, or omit that exact item and continue the batch,
  page, demo, or unrelated capability.
- **Security-sensitive loss:** Fail the affected credential or operation closed while keeping
  unrelated Accounts, personas, routes, and services available.
- **Disposable Live Demo data:** Its owner may perform one clean, owner-scoped regeneration when
  corrupt disposable state could explain the result. A repeated deterministic failure becomes a
  reported defect or deferred capability rather than an endless reset loop.
- **Persistent or production-like data:** Never delete or recreate it merely to make a gate pass.

## Error and HTTP boundary

`StoreError` is deliberately backend-neutral. It classifies a persistence
result; it is not a browser error schema and does not authorize exposing its
attached diagnostic text.

| Store result                                 | Durable meaning                                                                                                                | Normal recovery                                                                                                                  |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `NotFound`                                   | No visible record exists in the active scope.                                                                                  | Treat as absent; routes may also use it to conceal a foreign record.                                                             |
| `AlreadyExists`                              | Immutable identity or first-writer boundary already exists.                                                                    | Resolve the existing immutable record only when the operation defines exact replay. Otherwise reload.                            |
| `Forbidden`                                  | Caller context lacks the required Account relationship, course/Student ownership, workspace relationship, capability, or role. | Stop. Preserve concealment of a foreign Account's record.                                                                        |
| `Conflict`                                   | A compare-and-swap, lifecycle, or immutable-state precondition changed.                                                        | Reload the authoritative Student Question Attempt View or Assignment Attempt Summary and ask the user to review before retrying. |
| `RetryableTransaction`                       | PostgreSQL aborted the whole serializable/deadlock transaction.                                                                | Retry only at the owner-defined transaction boundary with its exact operation facts.                                             |
| `TimedOut`                                   | The database-authoritative attempt deadline already passed.                                                                    | Stop the submission path and reload the current attempt or summary.                                                              |
| `InvalidRecord` or Assignment Activity Rules | Trusted code or accepted wire data violated an Assignment policy rule.                                                         | Do not retry unchanged; return the bounded, route-approved validation message.                                                   |
| `Unavailable`                                | A bounded dependency is unavailable.                                                                                           | Preserve input and retry the same logical operation after recovery.                                                              |

HTTP routes project this classification narrowly. For example,
The deferred Assignment Attempt route maps a missing attempt record to `404`, an Assignment Attempt conflict
or expired attempt to `409`, malformed accepted input to `422`, and storage or backend
unavailability to `503`. It sends `Cache-Control: no-store` error responses. Other routes may use
different public wording or concealment. In particular, a Student-owned Assignment Attempt lookup returns not
found for a nonowner rather than confirming that the attempt exists. A new route must copy the
relevant boundary's concealment rule instead of exposing a raw `StoreError` or making one global
status mapping.

Browser errors contain a stable short message only. They never contain SQL, Object Addresses, bucket
names, signed URLs, checksums not already public, Account or course identities, leases, renderer/provider
state, source archives, answer keys, or raw backend errors.

## Submission and attempt recovery

The durable attempt is the authority for student, course, assignment, question revision, seed,
timing, and grading backend. A replica reconstructs that state from PostgreSQL; the browser cannot
recover an uncertain submission by issuing a different attempt.

- The exact Question Attempt is the submission identity. The deferred Store records one accepted
  submission and receipt for it.
- The Store first atomically persists the accepted Question Submission and its Question Submission
  Receipt, pending evaluation, execution, and ready job. The submission remains bound through its
  Question Attempt to the immutable Issued Question and private Question Attempt Reproduction
  Details. An exact retry returns the same durable acceptance rather than creating another grading
  operation. A changed response or incompatible replay conflicts.
- After `202 Accepted`, the browser uses the route-bound submission-status read. The sealed worker
  commits the Grading Result, Question Attempt/Assignment Attempt/enrollment transitions, Assignment Attempt Summary, and Automated Grading Receipt in one
  transaction; an authorized status reader derives policy-redacted Student Feedback without re-grading.
- If the browser loses the response, it preserves the same request body and retries that exact
  Question Attempt after connectivity returns. It must not create a new attempt merely because the
  outcome was unknown.
- If a deadline has elapsed, the Store refuses the response. The browser reloads the attempt or
  summary; client clocks never extend a deadline.
- A returned conflict means the student must reload the durable state. This is particularly
  important after another tab, a timed auto-submit, or an instructor policy change changes the
  attempt lifecycle.

The accepted payload redesign in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) adds an attempt-bound Question Presentation
Checksum and Presentation Response Item References. Its planned mismatch response is a fail-closed `409`: PLE does not
grade or mutate the attempt, preserves the editable draft only in memory, reloads the same
presentation, and restores the draft only when its schema and Presentation Response Item References still match. Until that
work item lands, no current browser client exposes a Question Attempt or Question Submission route. The retained
contract must not treat Question Attempt Reproduction Details as client authority.

## Replica and cache recovery

API replicas have no correctness-bearing process memory. The shared PostgreSQL session store,
authenticated Account context, attempts, Question Submissions, Question Submission Receipts, and shared S3-compatible object store
allow a surviving replica to resume an authorized attempt. The exact topology and evidence are in
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).

- A gateway removes an unready API replica from rotation. A replica's readiness checks database
  schema compatibility and object-store bucket access, not optional question-backend reachability.
- A PLE Question, course read, or authentication request can continue when an optional private
  renderer is down. The renderer-backed question itself returns a bounded `503`; PLE does not
  pretend it graded or substitute another question.
- A process crash after an attempt or submission commit is recovered by reading durable state. A
  process crash before commit leaves no receipt and can be retried through the normal owner path.
- Immutable render and asset caches accelerate delivery but are never correctness authority. Cache
  keys bind immutable version and seed; entries contain only safe public render data. A miss may
  rerender privately. A reproduction, Question Attempt Reproduction Details, or checksum disagreement fails closed rather
  than serving a near match.

## Prefetch and cache recovery

Prefetch is optional acceleration, never a student attempt or an offline queue.
On a prefetch failure, mismatch, or browser teardown, discard the in-memory
candidate and reload the current server-issued attempt. A submitted answer is
never recovered by promoting a browser cache entry. The atomic reservation and
promotion rules are in [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md);
cache scope and withholding rules are in
[CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md).

## Worker outcomes

Workers may scale without producing duplicate visible effects because their
lease and generation rules are owned by
[CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md). From a recovery
perspective, a transient or timed-out job receives its bounded retry/backoff;
a permanent or exhausted job becomes `Dead`; and an unavailable or unknown
finalization remains recoverable rather than being immediately repeated.
Account-visible inspection exposes only coarse state and count, not lease tokens
or failure text. Operators use the authorized job boundary to investigate a
dead job and choose a documented repair; they do not make a stale worker's
output current.

Workers log only `StoreError` categories and aggregate pass counts. Diagnostics
must not serialize a raw error object because it may contain identifiers or
dependency-specific text.

## Effectful iMathAS Question Backend dispatch

An iMathAS Question Backend can receive an effectful POST after PLE has sent
bytes but before PLE receives a valid response. Retrying that request as though
nothing happened could duplicate an upstream action or make PLE and the
backend disagree about the attempt. The iMathAS Question Backend activity lease therefore
uses a durable pre-dispatch fence:

1. while the exact activity lease is still valid, PLE atomically records an
   indeterminate marker bound to the lease-token hash before the backend POST;
2. it sends the one server-built backend request; and
3. it clears that exact marker only after a valid, accepted response has been
   processed.

A timeout, transport error, malformed response, process crash, or lease-loss
after step 1 leaves the marker in place. Reclaim, relaunch, grade finalization,
and normal revocation reject that attempt rather than issuing another effectful
backend POST. The browser receives bounded unavailable/conflict behavior and
must not auto-retry with a new launch. This is deliberately conservative: it
preserves at-most-once local dispatch rather than guessing whether the external
side effect occurred.

The marker is durable evidence, not an automatic recovery protocol. Resolving
an indeterminate iMathAS Result requires an authorized operator procedure and
backend-specific evidence that can establish the outcome without replaying
the POST. Until such a procedure is designed and tested for a backend, the
attempt remains fenced. Read-only grade/result retrieval must remain
structurally side-effect-free; it may not be used as a hidden dispatch retry.

## Object and provider outcomes

Typed object storage is bytes-first and checksum-verified. Its authoritative
identity and cross-system state rules are in
[STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) and
[OBJECT_STORAGE.md](OBJECT_STORAGE.md). This table states recovery actions,
not object commit mechanics.

| Condition                                    | Required behavior                                                                           |
| -------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Immutable put finds exact existing bytes     | The owner may reuse it only where its transaction contract explicitly defines exact replay. |
| Immutable put finds different existing bytes | Refuse; never overwrite or reinterpret the key.                                             |
| Read is missing or checksum mismatches       | Fail closed, withhold delivery or grading input, retain the database evidence, and alert.   |
| Database record exists but bytes do not      | Treat as a broken reference, not as a reason to delete the record or return a substitute.   |
| Bytes exist without a database record        | Treat as an orphan; do not serve it.                                                        |
| Object-store dependency is unavailable       | Return bounded unavailable behavior and leave durable metadata/leases recoverable.          |

Object Storage Checks are planned, not implemented. Until that capability
is accepted, operators preserve the evidence and repair the backing store;
application code must not silently delete mismatched records.

Private iMathAS and WeBWorK communication is a question-local dependency. It uses bounded private
transport and server-held credentials. A timeout or outage returns a safe `503` for the affected
question, does not put credentials or source in browser diagnostics, and does not make API
readiness fail for unrelated native work. Effectful external activity follows the pre-dispatch
fence above rather than an automatic POST retry. The adapter cache and reproducibility rules are
defined in [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md).

## Schema refusal and restore

Application startup verifies the embedded SQLx epoch through a restricted migration-state
projection. `Pending`, `Modified`, `Dirty`, unknown, or unavailable migration state does not become
an application write. The API reports safe dependency/schema state through readiness and workers
refuse schema-incompatible draining. Migration code and the operator-only status/migrate/verify
commands are in [postgres/migrations.rs](../crates/learning-data-access/src/postgres/migrations.rs)
and [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#fresh-migration-epoch).

- Applied migration files are immutable. Repair uses a forward migration or a deliberate recovery
  procedure; it never edits a checksum already recorded in a durable database.
- An incompatible or dirty ledger is an operator incident. Do not fabricate SQLx ledger rows,
  disable verification, or route traffic around the readiness boundary.
- PostgreSQL-major changes preserve the old volume, restore into a new clean cluster, verify the
  migration ledger, logical data, roles/grants/RLS, application writes, and protected database-function calls, then
  retain the old volume until recovery is accepted. See
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).
- A local logical restore exercise is evidence for local recovery only. Managed point-in-time
  recovery, backup retention, KMS, numerical recovery objectives, and production failover remain
  deployment work.

Any connected validation of backup, restore, deletion, migration, or fault behavior is a dated
disposable exercise, recovery drill, or controlled fault injection against fictional live-demo
data. Such exercises do not claim that the corresponding production operation is deployed.

## Diagnostic minimization

Recovery needs enough evidence to classify an operation, but diagnostics are not a second
browser-facing data channel.

- Browser responses may contain a status, a short stable message, and a route-safe identifier
  already visible to the caller. They never contain Object Addresses, buckets, manifests, signed URLs,
  leases, provider payloads, source bytes, answer keys, raw responses, SQL, credentials, or a
  foreign Account's course, Student, workspace, or record existence.
- Durable audit and access records remain course/Student-owned and retention-bound. Store the minimum
  operation identity, authenticated Account, exact target scope, reason category, and time needed for investigation.
- Worker and server logs use stable error categories such as `unavailable` or `conflict`; attach
  protected correlation data only in the authorized operator boundary and never copy it into an
  HTTP response.
- Before adding a diagnostic field, decide whether it is necessary to recover a correct durable
  state. If not, omit it. A Checksum, such as a Question Presentation Checksum, supports consistency diagnosis; it is
  not authentication, transport security, or permission to reveal protected content.

## Change checklist

A new mutation, worker, cache, backend, or storage capability must state its recovery boundary
before implementation:

1. Specify which failures reject, retry, remain indeterminate, or fail closed.
2. Define the public status, preserved student input, and concealment rule separately from
   internal errors.
3. State how an indeterminate request finds its existing durable outcome; link the atomicity
   mechanism to [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md).
4. State when a cache, prefetch, or provider result must be discarded and reloaded from the
   durable authority.
5. Define the operator escalation boundary for missing, mismatched, or unavailable dependencies.
6. Add behavior-focused tests for the recovery path only when it is stable, deterministic, and
   meaningful under [PYTEST_STYLE.md](PYTEST_STYLE.md). Use disposable live checks for real
   migrations, RLS, object stores, containers, or replica recovery rather than turning those
   environment probes into fragile permanent tests.

The active implementation and release plans remain the source of truth for package order and
acceptance. This document makes their failure behavior easier to find; it does not authorize a
feature before its work package is accepted.
