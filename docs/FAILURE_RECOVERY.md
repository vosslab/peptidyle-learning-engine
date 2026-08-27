# Failure and recovery contract

PLE has intentionally small, stateless API replicas, but its correctness does not depend on a
request reaching the same process twice. This document tells a learner, browser,
or operator whether to retry, reload, stop, or require repair after an outcome
is known or becomes uncertain. It deliberately does not repeat the transaction,
compare-and-swap, lease, generation, object, or prefetch mechanics that make an
outcome safe; those belong to [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md)
and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md).

It describes implemented code-bound behavior and labels planned operational work. It does not
claim automatic failover, a recovery objective, managed point-in-time recovery, or production high
availability. Those require the release evidence in
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

The canonical live-demo browser path uses these same ordinary application contracts. Its seeded
people and records are fictional live data, and regeneration is a disposable reset; it is not a
second recovery or product path.

## Authority status

**Current authority.** Route-safe error projection, attempt-scoped submission
receipts, no-store responses, and bounded private-backend failures determine
the current caller-visible outcomes described below.

**Required for new work.** Each capability must classify failures as committed,
rejected, retryable, or indeterminate; preserve enough durable evidence to
resolve an indeterminate request; and define the safe learner/operator action.
Its race-safety mechanism is specified separately in
[CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md).

**Planned boundaries.** Presentation-digest recovery awaits its accepted
payload package; object reconciliation is WP-RC7; managed point-in-time
recovery and production failover are WP-RC10 deployment work. None is current
automatic recovery behavior.

## Recovery rule

Every state-changing path falls into one of four outcomes:

| Outcome       | Meaning                                                                                                   | Caller behavior                                                                                                   |
| ------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Committed     | The requested durable effect is known to be visible.                                                      | Show the returned receipt or current projection.                                                                  |
| Rejected      | Validation, authorization, lifecycle, or immutable-state rules refused the request before a valid effect. | Do not retry unchanged automatically; correct the visible condition or reload.                                    |
| Retryable     | A dependency or serializable transaction was unavailable before a known commit.                           | Retry only through the operation's documented idempotency or lease boundary.                                      |
| Indeterminate | The client lost contact while a commit might have happened.                                               | Preserve the request identity, query/retry through its durable receipt, and never create a second logical action. |

The application must never turn an indeterminate result into a new attempt, seed, response,
worker effect, object identity, or user-visible grade. Nor may it replace an integrity failure with
best-effort data. PLE either reconstructs the exact durable state or fails closed.

## Error and HTTP boundary

`StoreError` in
[contracts/store.rs](../crates/learning-data-access/src/contracts/store.rs) is deliberately
backend-neutral. It classifies a persistence result; it is not a browser error schema and does not
authorize exposing its attached diagnostic text.

| Store result                    | Durable meaning                                                         | Normal recovery                                                                                       |
| ------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `NotFound`                      | No visible record exists in the active scope.                           | Treat as absent; routes may also use it to conceal a foreign record.                                  |
| `AlreadyExists`                 | Immutable identity or first-writer boundary already exists.             | Resolve the existing immutable record only when the operation defines exact replay. Otherwise reload. |
| `TenantMismatch` or `Forbidden` | Caller context lacks the required tenant, ownership, or role.           | Stop. Do not reveal whether another tenant owns the record.                                           |
| `Conflict`                      | A compare-and-swap, lifecycle, or immutable-state precondition changed. | Reload the authoritative projection and ask the user to review before retrying.                       |
| `RetryableTransaction`          | PostgreSQL aborted the whole serializable/deadlock transaction.         | Retry only at the owner-defined transaction or idempotent command boundary.                           |
| `TimedOut`                      | The database-authoritative attempt deadline already passed.             | Stop the submission path and reload the current attempt or summary.                                   |
| `InvalidRecord` or `RunModel`   | Trusted code or accepted wire data violated a model rule.               | Do not retry unchanged; return the bounded, route-approved validation message.                        |
| `Unavailable`                   | A bounded dependency is unavailable.                                    | Preserve input and retry the same logical operation after recovery.                                   |

HTTP routes project this classification narrowly. For example,
[run/support.rs](../crates/server/src/run/support.rs) maps a missing run record to `404`, a run
conflict or expired attempt to `409`, malformed accepted input to `422`, and storage or backend
unavailability to `503`. It sends `Cache-Control: no-store` error responses. Other routes may use
different public wording or concealment. In particular, an owner-scoped run lookup returns not
found for a nonowner rather than confirming that the attempt exists. A new route must copy the
relevant boundary's concealment rule instead of exposing a raw `StoreError` or making one global
status mapping.

Browser errors contain a stable short message only. They never contain SQL, object keys, bucket
names, signed URLs, checksums not already public, tenant identities, leases, renderer/provider
state, source archives, answer keys, or raw backend errors.

## Submission and attempt recovery

The durable attempt is the authority for learner, course, assignment, question version, seed,
timing, and grading backend. A replica reconstructs that state from PostgreSQL; the browser cannot
recover an uncertain submission by issuing a different attempt.

- A learner supplies one `Idempotency-Key` header for a submission. The validated key is bounded,
  visible ASCII and is stored with tenant-scoped submission evidence in
  [contracts/runs.rs](../crates/learning-data-access/src/contracts/runs.rs).
- The Store first atomically persists the accepted response, immutable issued-work witness, pending
  evaluation, execution, and ready job. An exact retry returns the same durable acceptance rather
  than creating another grading operation. A changed response or incompatible replay conflicts.
- After `202 Accepted`, the browser uses the route-bound submission-status read. The sealed worker
  commits the result, attempt/run/enrollment transitions, projections, and completed receipt in one
  transaction; status reads converge on that receipt without re-grading.
- If the browser loses the response, it preserves the same request body and idempotency key, then
  retries that exact logical submission after connectivity returns. It must not create a new key
  merely because the outcome was unknown.
- If a deadline has elapsed, the Store refuses the response. The browser reloads the attempt or
  summary; client clocks never extend a deadline.
- A returned conflict means the learner must reload the durable state. This is particularly
  important after another tab, a timed auto-submit, or an instructor policy change changes the
  attempt lifecycle.

The accepted payload redesign in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) adds an attempt-bound presentation
digest and rendered item IDs. Its planned mismatch response is a fail-closed `409`: PLE does not
grade or mutate the attempt, preserves the editable draft only in memory, reloads the same
presentation, and restores the draft only when its schema and rendered IDs still match. Until that
work package lands, current clients must use the implemented attempt and idempotency boundary and
must not treat current provenance fields as client authority.

## Replica and cache recovery

API replicas have no correctness-bearing process memory. The shared PostgreSQL session store,
tenant context, attempts, submissions, idempotency receipts, and shared S3-compatible object store
allow a surviving replica to resume an authorized attempt. The exact topology and evidence are in
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).

- A gateway removes an unready API replica from rotation. A replica's readiness checks database
  schema compatibility and object-store bucket access, not optional question-backend reachability.
- A native question, course read, or authentication request can continue when an optional private
  renderer is down. The renderer-backed question itself returns a bounded `503`; PLE does not
  pretend it graded or substitute another question.
- A process crash after an attempt or submission commit is recovered by reading durable state. A
  process crash before commit leaves no receipt and can be retried through the normal owner path.
- Immutable render and asset caches accelerate delivery but are never correctness authority. Cache
  keys bind immutable version and seed; entries contain only safe public render data. A miss may
  rerender privately. A reproduction, provenance, or checksum disagreement fails closed rather
  than serving a near match.

## Prefetch and cache recovery

Prefetch is optional acceleration, never a learner attempt or an offline queue.
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
Tenant-visible inspection exposes only coarse state and count, not lease tokens
or failure text. Operators use the authorized job boundary to investigate a
dead job and choose a documented repair; they do not make a stale worker's
output current.

Workers log only `StoreError` categories and aggregate pass counts in
[worker/runtime.rs](../crates/server/src/worker/runtime.rs). Diagnostics must not serialize a raw
error object because it may contain identifiers or dependency-specific text.

## Effectful external-tool dispatch

An external question engine can receive an effectful POST after PLE has sent
bytes but before PLE receives a valid response. Retrying that request as though
nothing happened could duplicate an upstream action or make PLE and the
provider disagree about the attempt. The external-tool activity lease therefore
uses a durable pre-dispatch fence:

1. while the exact activity lease is still valid, PLE atomically records an
   indeterminate marker bound to the lease-token hash before the provider POST;
2. it sends the one server-built provider request; and
3. it clears that exact marker only after a valid, accepted response has been
   processed.

A timeout, transport error, malformed response, process crash, or lease-loss
after step 1 leaves the marker in place. Reclaim, relaunch, grade finalization,
and normal revocation reject that attempt rather than issuing another effectful
provider POST. The browser receives bounded unavailable/conflict behavior and
must not auto-retry with a new launch. This is deliberately conservative: it
preserves at-most-once local dispatch rather than guessing whether the external
side effect occurred.

The marker is durable evidence, not an automatic recovery protocol. Resolving
an indeterminate external result requires an authorized operator procedure and
provider-specific evidence that can establish the outcome without replaying
the POST. Until such a procedure is designed and tested for a provider, the
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

WP-RC7 object reconciliation is planned, not implemented. Until that package
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
and [DATABASE_TENANCY.md](DATABASE_TENANCY.md).

- Applied migration files are immutable. Repair uses a forward migration or a deliberate recovery
  procedure; it never edits a checksum already recorded in a durable database.
- An incompatible or dirty ledger is an operator incident. Do not fabricate SQLx ledger rows,
  disable verification, or route traffic around the readiness boundary.
- PostgreSQL-major changes preserve the old volume, restore into a new clean cluster, verify the
  migration ledger, logical data, roles/grants/RLS, application writes, and broker calls, then
  retain the old volume until recovery is accepted. See
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).
- A local logical restore exercise is evidence for local recovery only. Managed point-in-time
  recovery, backup retention, KMS, numerical recovery objectives, and production failover remain
  WP-RC10 deployment work.

Any connected validation of backup, restore, deletion, migration, or fault behavior is a dated
disposable exercise, recovery drill, or controlled fault injection against fictional live-demo
data. Such exercises do not claim that the corresponding production operation is deployed.

## Diagnostic minimization

Recovery needs enough evidence to classify an operation, but diagnostics are not a second
browser-facing data channel.

- Browser responses may contain a status, a short stable message, and a route-safe identifier
  already visible to the caller. They never contain object keys, buckets, manifests, signed URLs,
  leases, provider payloads, source bytes, answer keys, raw responses, SQL, credentials, or a
  foreign tenant's existence.
- Durable audit and access records remain tenant-owned and retention-bound. Store the minimum
  operation identity, actor/tenant scope, reason category, and time needed for investigation.
- Worker and server logs use stable error categories such as `unavailable` or `conflict`; attach
  protected correlation data only in the authorized operator boundary and never copy it into an
  HTTP response.
- Before adding a diagnostic field, decide whether it is necessary to recover a correct durable
  state. If not, omit it. A checksum or presentation digest supports consistency diagnosis; it is
  not authentication, transport security, or permission to reveal protected content.

## Change checklist

A new mutation, worker, cache, backend, or storage capability must state its recovery boundary
before implementation:

1. Specify which failures reject, retry, remain indeterminate, or fail closed.
2. Define the public status, preserved learner input, and concealment rule separately from
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
