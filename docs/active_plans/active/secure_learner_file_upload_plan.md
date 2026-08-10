# Plan: Secure learner file-upload responses

## Status

Planning state: decision-complete on 2026-08-10; implementation has not started. The current UI and
submission route correctly fail closed. This plan owns the dedicated upload contract required before
PLE can enable `ResponseDefinition::FileUpload` for learners.

This plan does not interrupt the current release order. It runs after the secure assessment payload
packages WP-P1 through WP-P6, object reconciliation WP-RC7, and LTI WP-RC9 are accepted. Its forward
SQL migration is `2026080912_secure_learner_uploads.sql`, following the already-reserved
object-reconciliation and LTI migrations and completing before WP-RC10 production deployment. File
upload is a required
working-codebase release capability; it is not a reason to weaken or bypass the current fail-closed
behavior.

## Context

PLE already models a file-upload response, manual grading, typed object storage, short-lived protected
delivery, retention, and a durable worker queue. Those pieces are intentionally not connected yet:

- `StudentResponse::FileUpload` still carries a raw string `object_key`;
- browser format validation merely checks that the string is nonempty;
- the learner widget displays an unavailable state; and
- the submission route returns `422` before backend or Store mutation.

That refusal is correct. A browser-supplied object key would let an untrusted client assert storage
ownership, tenant, learner, attempt, file type, and lifecycle state. None of those claims is safe to
accept from the browser.

The secure boundary is a server-created upload record plus repeated authenticated authorization. The
browser receives one opaque `LearnerUploadId`, streams bytes to a same-origin PLE route, and later
submits only that ID. PLE owns the physical object keys, checksum, observed media type, inspection
state, delivery grant, attempt binding, and retention.

The design follows the defense-in-depth controls in the
[OWASP file upload guidance](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html):
allowlisted types, generated storage names, bounded size, authorization, storage outside the web
root, malware inspection, and CSRF protection. S3/MinIO transport integrity supplements, but never
replaces, PLE's authoritative SHA-256 record.

## Objectives

- Enable one-file learner responses without exposing raw object keys or storage credentials.
- Bind every upload to exactly one tenant, learner, question attempt, course, presentation digest,
  and server-owned response definition.
- Keep bytes non-deliverable until server-side type validation and malware inspection pass.
- Atomically consume one accepted upload when the attempt enters `needs_manual_grading`.
- Preserve idempotent submission, timing, retention, RLS, replica safety, and access logging.
- Keep the learner wire small: one upload ID in the answer body, with no repeated filename, checksum,
  MIME type, object path, learner, course, version, seed, or backend.
- Give learners an accessible upload, progress, replacement, failure, and recovery flow.

## Design philosophy

The upload ID is a handle, not authority. Authority comes from the authenticated session, forced RLS,
attempt ownership and lifecycle, the persisted presentation digest, the closed upload state machine,
and the atomic final submission transaction.

CRC16 remains the compact consistency ID for rendered selectable items. It is not used for uploaded
bytes. PLE stores a full SHA-256 digest for every candidate and durable learner object. A UUID-sized
upload handle occurs only once in a submission, so shortening it would not produce meaningful latency
or bandwidth savings.

The first implementation uses a same-origin PLE streaming route. It does not issue browser-facing
S3 credentials or presigned write URLs. This is the smallest auditable security boundary and works
across replicas because all durable state lives in PostgreSQL and object storage. A later direct-to-S3
transport may reuse the same upload record and state machine only after separate S3/MinIO policy,
checksum, expiry, CORS, and replay evidence. It must not change the final answer contract.

## Scope

- One file per `ResponseDefinition::FileUpload` question.
- A closed learner-upload identity, policy, state machine, Store contract, Memory/PostgreSQL backends,
  and forced-RLS migration.
- Authenticated same-origin create, byte-stream, status, abandon, submit, and protected-download
  routes.
- Typed temporary and durable learner-upload object keys.
- Streaming size enforcement and SHA-256 calculation without buffering the complete file in API
  memory.
- A complete worker family for file inspection, promotion, and atomic durable finalization.
- First-release file profiles for PDF, UTF-8 plain text, PNG, and JPEG.
- Existing manual-grading integration, learner/instructor authorized retrieval, retention,
  reconciliation, multi-replica operation, accessibility, and documentation.

## Non-goals

- The browser does not grade file content or decide whether inspection passed.
- CRC16, filename extensions, browser MIME headers, `Content-Length`, and client-computed checksums
  are not security proof.
- The first release does not accept HTML, SVG, JavaScript, executables, generic ZIP archives, audio,
  video, encrypted documents, DOCX, ODT, or macro-enabled office formats.
- Inline rendering of an original learner upload is out of scope. Accepted objects download as
  attachments; separately generated safe previews require a later content-disarm contract.
- Multiple files, folders, resumable multipart upload, direct browser-to-S3 upload, and instructor
  annotation are later additive capabilities.
- Malware inspection does not prove content is pedagogically appropriate or harmless. It is one
  layer in a closed validation pipeline.
- Exact throughput or latency targets are not invented before representative pilot evidence.

## Current state summary

| Boundary | Current behavior | Required change |
| --- | --- | --- |
| Question model | File response stores raw `object_key` text | Replace with typed `LearnerUploadId` |
| Browser widget | Accessible unavailable state | Add native file input, upload state, replace, and submit |
| Run submission | Refuses all file uploads before mutation | Accept only atomically consumable `ready` upload IDs |
| Object store | Generic temporary and student-record keys | Add attempt-bound candidate and durable upload keys |
| Object metadata | `StudentRecord` implies export category | Add a distinct learner-submission category |
| Object write | `PutObject` owns a complete `Vec<u8>` | Add a bounded streaming write contract |
| Worker | No upload-inspection family | Add handler and committer before the family is claimable |
| Manual grading | Pending evaluation is implemented | Bind its response to the consumed upload record |
| Delivery | Protected 5-minute student-record URLs exist | Authorize the exact learner/instructor upload grant |
| Retention | Student records archive/delete at 100/365 days | Include consumed upload records and bytes |
| Reconciliation | WP-RC7 owns object/database agreement | Register candidate and durable upload object classes |

## Architecture boundaries and ownership

### Question model

`crates/question_model` owns `LearnerUploadId`, the browser-safe upload status projection, and the
typed file-upload response. It does not know a bucket name, S3 key, scanner, access grant, or storage
credential.

Replace:

```rust
StudentResponse::FileUpload { object_key: String }
```

with:

```rust
StudentResponse::FileUpload { upload: LearnerUploadId }
```

The secure assessment wire chooses this response shape only after loading an attempt whose issued
definition is `FileUpload`. The browser never sends `kind`.

### Domain

`crates/domain` owns pure upload-policy validation and legal state transitions. It validates
definition limits against an injected platform policy and validates the final typed response shape.
It cannot inspect bytes, query a database, read a clock, or authorize a learner.

### Object storage

`crates/objects` owns two new semantic keys:

- `LearnerUploadCandidate { tenant, attempt, upload, object }` in `temp-processing`; and
- `LearnerUpload { tenant, attempt, upload, object }` in `student-records`.

Both paths derive only from typed server-generated IDs. No original filename enters a path. The
candidate is never signable. The durable object uses `ObjectCategory::LearnerSubmission`, not the
existing export category.

The object contract adds a bounded streaming put operation. It accepts a typed destination and an
async byte stream, enforces the server-selected maximum while reading, calculates SHA-256, and
returns authoritative size and metadata. The Memory backend may collect bounded bytes; S3/MinIO must
stream. An incomplete or over-limit stream leaves no accepted object record.

### Learning data access

`crates/learning-data-access` owns the upload record, transition commands, Memory/PostgreSQL parity,
RLS, queue payload, atomic consumption with manual submission, download authorization, retention,
and reconciliation references.

### Server

`crates/server` owns same-origin routes, session resolution, attempt ownership, request-body
streaming, platform file profiles, media inspection, scanner composition, worker registration,
protected download projection, and safe error mapping.

The worker registry may claim `InspectLearnerUpload` only when both its real handler and atomic
committer are registered. A missing scanner configuration keeps the family unregistered and leaves
uploads unavailable; it does not consume and fail jobs repeatedly.

### Browser

The SolidJS widget owns file selection, progress, retry, replacement, status polling, keyboard and
screen-reader behavior, and final submission of the upload ID. It never constructs an object path,
derives a tenant/course/learner binding, or treats a local digest as authoritative.

## Capability contract

### Server-issued record

Creating an upload requires all of these checks before the request body can contain file bytes:

1. authenticate the session;
2. RLS-load the attempt and owning run;
3. prove the actor owns the attempt;
4. prove the run and attempt are active;
5. reproduce the exact issued envelope and require `ResponseDefinition::FileUpload`;
6. require the submitted presentation digest to equal the attempt binding;
7. intersect the question's allowed extensions with the platform allowlist; and
8. calculate an expiry no later than the attempt's server-owned submission window.

The returned `LearnerUploadId` is randomly generated with at least UUIDv4 entropy. It is not a
bearer credential: every later route repeats authenticated ownership and binding checks. The record
contains no reusable secret and no presigned URL.

### Expiry

An issued upload capability lasts 15 minutes and never past the attempt deadline plus its effective
server-owned grace. An active untimed attempt may request a fresh capability after expiry. Expiry
does not close the attempt and never changes grading policy.

Uploaded bytes continue through an already-queued inspection even if the issuance window expires.
An unconsumed accepted upload becomes abandoned when the attempt closes, or after 24 hours for an
otherwise active untimed attempt. These are initial operational defaults, exposed through one
validated server policy rather than copied across modules. Permanent tests inject short policies and
assert behavior; they do not freeze the default numbers.

### Upload state machine

The durable states are:

```text
issued -----> uploaded -----> ready -----> consumed
  |               |             |
  +-> expired     +-> rejected  +-> abandoned
  +-> abandoned
```

- `issued`: no accepted object record exists.
- `uploaded`: immutable temporary bytes and authoritative SHA-256/size exist; inspection is queued.
- `ready`: inspection accepted the bytes and an immutable student-record object exists.
- `consumed`: the exact upload is bound to the submitted attempt requiring manual grading.
- `rejected`: type, structure, or malware inspection failed; no delivery is possible.
- `expired`: no upload completed before capability expiry.
- `abandoned`: the learner replaced/cancelled it or the attempt closed before consumption.

There is no persisted `receiving` state. A replica streams to the immutable candidate key while the
row remains `issued`, then atomically records metadata and queues inspection. If the process dies
before that transaction, the bytes are an unreferenced temporary object for WP-RC7 reconciliation.
An identical retry converges by verifying complete object metadata and bytes; different bytes for an
existing upload ID return `409` and require a new capability.

Every transition compares a positive `state_revision`. Illegal, stale, or terminal transitions fail
without mutation. The browser cannot set a state.

## Network contract

### Create capability

```http
POST /api/attempts/{attemptId}/uploads
Content-Type: application/json
```

```json
{
  "presentationDigest": "pd1_...",
  "filename": "lab-report.pdf",
  "sizeBytes": 48211
}
```

`filename` is bounded display metadata only. It is decoded once, rejects control characters, path
separators, bidi controls, empty names, and excessive UTF-8 bytes, and is never used as a path.
`sizeBytes` is an early usability and quota check; the stream count remains authoritative.

The response is `201`, `Cache-Control: no-store`:

```json
{
  "uploadId": "...",
  "contentPath": "/api/attempts/.../uploads/.../content",
  "statusPath": "/api/attempts/.../uploads/...",
  "expiresAt": "...",
  "maxBytes": 26214400
}
```

The server returns same-origin paths, never an S3/MinIO URL, bucket, key, credential, checksum, or
tenant/learner identifier. The exact default `maxBytes` above is illustrative; the response carries
the authoritative effective policy.

### Stream bytes

```http
PUT /api/attempts/{attemptId}/uploads/{uploadId}/content
Content-Type: application/octet-stream
```

The API reads a bounded stream, not `to_bytes`. It applies per-session and deployment-wide concurrent
upload limits, aborts immediately after the effective byte cap, computes SHA-256, and writes only the
server-owned candidate key. A supplied `Content-Length` or browser MIME type may cause an early
refusal but never relaxes streaming validation.

Every mutation requires the configured PLE origin and, when present, same-origin Fetch Metadata. The
server does not rely on `SameSite` cookies alone, because future embedded LTI sessions require an
explicit origin-bound anti-CSRF check. Capability creation also enforces durable per-learner,
per-attempt, and per-tenant active-upload and byte quotas so another API replica cannot bypass them.

The S3/MinIO backend aborts an incomplete multipart upload on disconnect, over-limit, cancellation,
or backend failure. Bucket lifecycle also expires incomplete multipart uploads as a backstop. No
partially completed upload may produce an `ObjectRecord`.

Success returns `202` with the upload status. An exact completed replay returns the same status. A
different replay returns `409`. No scanner result is returned synchronously.

### Read or abandon status

```http
GET /api/attempts/{attemptId}/uploads/{uploadId}
DELETE /api/attempts/{attemptId}/uploads/{uploadId}
```

The safe status projection contains only upload ID, state, display filename, authoritative size,
accepted media profile when known, and a stable learner-facing rejection code. It never returns a
physical object ID/key, checksum, scanner signature, raw scanner output, course/tenant/learner ID, or
download URL.

Delete is idempotent for `issued`, `uploaded`, or `ready`; it cannot delete `consumed`. Cleanup of
bytes is asynchronous and exact-key only.

### Submit answer

The existing compact submission route receives:

```json
{
  "presentationDigest": "pd1_...",
  "answer": { "upload": "..." }
}
```

`QuestionAttemptId` remains in the path and `Idempotency-Key` remains in the header. The body does not
repeat `kind`, filename, size, type, checksum, object ID/key, learner, tenant, course, question,
version, seed, or grading state.

File-upload submissions use assessment request contract version `2`; ordinary flat and WeBWorK
answers remain version `1`. The new migration widens the internal idempotency-version constraint and
adds `PresentationDescriptorV2` only for file-upload attempts. V2 extends the closed response schema
with the public file limit and allowed profile labels; it does not expose storage or scanner state.

The final Store command locks the attempt and upload row in one transaction and verifies:

- exact tenant, actor, attempt, course, and presentation digest binding;
- attempt timing and active lifecycle;
- upload state is `ready` and `consumed_at` is null;
- durable key, object ID, size, SHA-256, observed profile, and inspection policy match the row;
- the profile and size satisfy the exact issued response definition; and
- the idempotency key either replays the identical canonical request or is unused.

It then inserts the immutable submission, creates/updates the existing `needs_manual_grading`
evaluation, marks the attempt submitted, marks the upload `consumed`, and records the protected
delivery binding atomically. A changed idempotent replay returns `409` before any transition.

## File policy and inspection

### Allowlist ownership

The platform owns a closed `LearnerUploadProfileV1` allowlist. An instructor's
`accepted_extensions` may narrow it but can never add a profile. Publication refuses an extension
outside the platform allowlist instead of creating a question learners cannot safely submit.

The first profiles are:

| Profile | Extension | Required validation |
| --- | --- | --- |
| PDF | `pdf` | PDF signature and bounded structural parse; reject encryption and active launch actions |
| Plain text | `txt` | Valid UTF-8, no NUL, bounded line and total length |
| PNG | `png` | Exact signature, CRC/structure, bounded dimensions and decoded allocation |
| JPEG | `jpg`, `jpeg` | Exact marker structure, bounded dimensions and decoded allocation |

The original filename and `Content-Type` are hints. Acceptance requires one and only one profile to
match verified bytes. Ambiguous polyglots, trailing active payloads, malformed structures, decompression
bombs, and files that match no profile are rejected.

### Malware scanner

A production `UploadInspector` composes deterministic profile validation with a private `clamd`
client. The scanner container is digest-pinned, has no public port, runs without unnecessary
capabilities, and is reachable only by the worker network. Scanner definitions and engine version are
recorded as bounded inspection metadata; raw scanner messages are not stored or returned.

Scanner unavailable, timed out, malformed reply, or size-limit result is retryable and leaves the
upload non-deliverable. A positive malware result is terminal `rejected`. A worker never marks an
upload ready merely because the scanner is absent.

The worker reads only the typed temporary object, independently verifies its database metadata and
SHA-256, applies profile checks and malware scanning, then writes the same verified bytes under the
pre-minted durable key. A replayed durable write is accepted only when every byte and metadata field
matches. The committer atomically records `ready`; cleanup of the temporary copy follows. Failed
cleanup is visible to reconciliation and never changes the ready verdict.

## Persistence contract

`2026080912_secure_learner_uploads.sql` creates `learner_upload` with at least:

- `tenant_id`, `upload_id`, `attempt_id`, `attempt_occurred_at`, `course_id`, and `learner_id`;
- full 32-byte `presentation_digest`;
- pre-minted candidate and durable object IDs plus protected delivery ID;
- bounded display filename and declared byte count;
- authoritative size, 32-byte SHA-256, observed profile, and media type after upload;
- inspection policy/engine/definition versions and stable rejection code;
- state, positive state revision, created/expiry/uploaded/inspected/ready/consumed timestamps; and
- cleanup claim metadata needed for idempotent exact-key deletion.

The primary key is `(tenant_id, upload_id)`. A composite foreign key binds the partitioned attempt by
`(tenant_id, attempt_id, attempt_occurred_at)`. Unique constraints protect candidate object, durable
object, and delivery identities. A partial unique index permits at most one consumed upload per
attempt. Check constraints enforce state-specific all-or-none columns and exact digest widths.

A course-binding trigger derives `course_id` from the attempt. The learner ID is derived from the
owned run/enrollment, not supplied as a trusted request field. The table uses forced RLS. The
application receives only the statements needed by authenticated route transitions; the queue broker
receives claim access; the retention broker receives exact tenant-scoped select/delete. No role gets
an RLS bypass.

The migration also:

- widens assessment request contract versions from `(0,1)` to `(0,1,2)`;
- adds `PresentationDescriptorV2` as an allowed attempt descriptor only when the issued response is
  file upload;
- adds `InspectLearnerUpload` to the closed job payload/check constraints;
- extends delivery metadata with an attempt-bound learner-upload kind;
- adds retention fences, access-log scope, and object-reconciliation references; and
- updates the read-only migration ledger projection.

No historical row is rewritten to invent an upload. The current server has never accepted one.

## Manual grading and delivery

A ready upload is not a grade. Successful submission creates the existing
`needs_manual_grading` evaluation with no correctness or credit. The instructor grading projection
adds only safe metadata and one protected same-origin download route. It does not expose the object
key, checksum, scanner details, or learner-supplied path.

The learner may retrieve their own consumed file, and an authorized course instructor may retrieve it
for grading. Every request reauthorizes tenant, course, role, learner/attempt scope, and retention
state before issuing the existing maximum five-minute student-record URL. Responses remain
`Cache-Control: no-store`, `Pragma: no-cache`, and `Referrer-Policy: no-referrer` and are delivered as
attachments with `X-Content-Type-Options: nosniff`. Temp and rejected objects are never signable.

Original filenames are used only as a sanitized `Content-Disposition` display suggestion. The
ASCII fallback is generated server-side; CR/LF, path, control, and quoting injection are impossible.

## Retention and reconciliation

- Unconsumed candidates and durable objects are short-lived and deleted by exact typed key.
- Consumed uploads are student records and follow the course's archive/delete policy: notify at 30
  days, archive at 100 days, permanently delete at 365 days by default.
- Retention first revokes delivery, then deletes the exact durable object and learner-upload row in
  the established work-set order.
- WP-RC7 inventory understands both new key classes. It never deletes after one observation, never
  follows a browser string, and cancels deletion when a valid reference appears.
- A database reference to missing or checksum-mismatched bytes makes delivery and grading review
  unavailable and raises an operational alert. The database record is not deleted to hide damage.
- Rejected malware bytes are never promoted. Their temporary object is quarantined only long enough
  for bounded retry/diagnosis and then deleted without a delivery grant.

## Multi-replica and failure behavior

No upload state lives only in API memory. One replica may create the capability, another receive the
bytes, a worker inspect them, and a third accept the final submission.

| Failure | Required result |
| --- | --- |
| API dies during stream | No accepted row transition; orphan candidate is reconciled |
| API dies after object write | Exact retry verifies and converges, or reconciliation owns orphan |
| Duplicate byte PUT | Same bytes return existing status; different bytes return `409` |
| Scanner unavailable | Upload stays non-deliverable and job retries with bounded backoff |
| Worker dies after promotion | Replay verifies exact durable object before commit |
| Database commit fails after object write | Durable orphan is reconciled; no upload becomes ready |
| Temporary cleanup fails | Ready record remains valid; cleanup/reconciliation retries |
| Learner replaces file | New upload ID; old unconsumed upload becomes abandoned |
| Attempt closes during upload | Final transition/submission refuses; cleanup owns bytes |
| Foreign tenant/learner guesses ID | Concealed `404`; no object read, write, or state disclosure |
| Ready object later mismatches checksum | Delivery and grading review fail closed; alert is recorded |

## Browser and accessibility behavior

The widget uses a native `<input type="file">` with an associated label and help text showing allowed
profiles and the effective size limit. The primary no-mouse path is platform-native:

- Tab and Shift+Tab move through choose-file, upload/replace, submit, and return controls;
- Space activates the focused native file chooser or button;
- the operating-system file picker remains native and is not replaced by a custom drop zone; and
- status changes use a concise `role="status"` live region without moving focus.

Drag-and-drop may be added only as a secondary path. It cannot be the only upload action. Enter-to-
submit and Escape-to-return remain widget extensions under the existing no-mouse contract.

The widget distinguishes selecting, uploading, checking, ready, rejected, expired, offline, and
submitted states in text, not color alone. It preserves the selected local `File` only in memory;
page refresh cannot silently recreate a browser file handle. After refresh, PLE reloads server upload
status. If bytes were never accepted, the learner is clearly asked to choose the file again.

The browser never stores file bytes, upload URLs, or object metadata in `localStorage` or
`sessionStorage`. It may retain only the attempt/upload IDs needed to query same-origin status while
the active attempt remains compatible.

## Milestone plan

| Milestone | Work packages | Outcome |
| --- | --- | --- |
| M1 | WP-FU1, WP-FU2 | Closed identity, state, schema, RLS, and Store parity |
| M2 | WP-FU3, WP-FU4 | Bounded streaming upload, inspection, and promotion |
| M3 | WP-FU5, WP-FU6 | Atomic submission, manual grading, delivery, browser, and release evidence |

### Work package WP-FU1: Model and policy

- Owner: `rust-code-expert`, independently reviewed by a security reviewer.
- Depends on: accepted WP-P1 through WP-P6, WP-RC7, and WP-RC9.
- Files: question-model identity/response/presentation modules, domain validation/state modules,
  generated contracts, Wasm bridge only for browser-safe format/state validation.
- Behavior: add `LearnerUploadId`, `PresentationDescriptorV2`, closed profiles, pure policy/state
  transitions, and typed response; remove raw object-key input from every learner contract.
- Success: unknown fields, wrong descriptor, invalid transition, unsupported profile, and forged raw
  object material cannot construct an accepted response.
- Validation: focused Rust/Wasm vectors, strict Clippy, generated binding freshness, and independent
  secret-boundary review.

### Work package WP-FU2: Persistence and RLS

- Owner: `postgresql-expert`.
- Depends on: WP-FU1, accepted WP-RC7, and accepted WP-RC9 migration ledger.
- Files: `2026080912_secure_learner_uploads.sql`, Store contract, Memory/PostgreSQL upload owners,
  queue payload, retention/reconciliation and conformance tests.
- Behavior: persist exact bindings and transitions, forced RLS, least grants, atomic queueing, and
  exact retention references.
- Success: foreign tenant/actor, mismatched attempt/digest/course, illegal state columns, duplicate
  consumed upload, stale revision, and broken object identities refuse in both backends.
- Validation: fresh/no-op migration, SQL constraint probes, Memory/PostgreSQL conformance, RLS role
  matrix, retention/reconciliation integration, backup/restore verification, independent PostgreSQL
  review.

### Work package WP-FU3: Streaming ingress

- Owner: `rust-code-expert` for server/object transport.
- Depends on: WP-FU1 and WP-FU2.
- Files: object streaming trait and Memory/S3 implementations; server upload routes, limits,
  settings, and safe response/error contracts; API client/decoder owners.
- Behavior: authorize before reading bytes, stream to typed temporary storage, bound aggregate and
  per-upload resources, compute SHA-256, converge exact retries, queue inspection atomically.
- Success: over-limit, truncated, disconnected, duplicate, changed replay, expired, closed attempt,
  and foreign actor cases leave no accepted upload or hidden in-memory authority.
- Validation: focused offline stream tests, Axum route security tests, MinIO transport E2E, multi-
  replica create/upload/status path, and memory-allocation observation as one-time evidence.

### Work package WP-FU4: Inspection and promotion

- Owner: worker `rust-code-expert`; scanner/container owner for pinned `clamd` composition.
- Depends on: WP-FU2 and WP-FU3.
- Files: upload profile inspectors, scanner client, complete worker family/registry, object promotion,
  container files, launcher settings, and operational docs.
- Behavior: validate exact bytes, scan privately, promote only accepted content, commit idempotently,
  and keep every failure non-deliverable.
- Success: valid PDF/text/PNG/JPEG becomes ready; malformed, active, encrypted, polyglot, oversized,
  decompression-bomb, malware, scanner-outage, and wrong-object cases do not.
- Validation: deterministic profile corpus in focused Rust tests; disposable scanner/MinIO E2E;
  container topology/pin checks; worker crash/replay oracle; independent upload-security review.

### Work package WP-FU5: Submission and grading

- Owner: server and learning-data-access `rust-code-expert`.
- Depends on: WP-FU2 and WP-FU4.
- Files: run submission decoder/route, atomic Store command, manual-grading projection, asset delivery,
  access logging, retention and route tests.
- Behavior: consume one exact ready upload in the same transaction that records the idempotent manual
  submission; authorize learner/instructor attachment delivery.
- Success: exact retry returns the same receipt; changed retry conflicts; wrong state/binding/digest,
  expired attempt, second consumed upload, foreign course, and damaged object fail before mutation or
  delivery.
- Validation: Memory/PostgreSQL submission parity, timing/idempotency/manual-grade tests, protected-
  download header and authorization tests, retention delete path, independent server-security review.

### Work package WP-FU6: Browser and release closure

- Owner: `solid-js-expert`, reviewed by `human-interact-expert`; integration owner closes evidence.
- Depends on: WP-FU3, WP-FU4, and WP-FU5.
- Files: file-upload response widget, attempt state, HTTP client/decoders, mock test double, Playwright,
  E2E, deployment/usage/security/accessibility/status/changelog docs.
- Behavior: accessible select/upload/check/replace/submit/recover journey with no storage authority in
  browser state; production stack includes a private pinned scanner and bounded worker family.
- Success: complete keyboard path, refresh recovery, retry/replacement, rejection help, manual-grading
  handoff, multi-replica flow, no private network trace, and object cleanup pass.
- Validation: focused TypeScript/Node tests, built-browser Playwright, live PostgreSQL/MinIO/scanner
  E2E, full repository gate, and independent HCI/security/docs reviews.

## Acceptance criteria and gates

The capability is accepted only when all are true:

- no browser request or persisted learner response contains a raw object key;
- every route authenticates and rechecks tenant/learner/attempt binding before object access;
- every accepted object has authoritative size, SHA-256, observed profile, and inspection evidence;
- temp/rejected bytes are never deliverable and ready bytes use only the durable typed key;
- final submission consumes the ready upload and creates manual-grading state atomically;
- exact idempotent retries converge and changed retries fail before mutation;
- forced RLS, least grants, course retention fences, reconciliation, and access logs include uploads;
- scanner outage fails closed without losing the learner's status or consuming incomplete jobs;
- one API replica can issue, another receive, a worker inspect, and another submit/download;
- the native keyboard journey works without drag-and-drop or widget shortcuts;
- no object path, checksum, scanner output, signed URL, credential, or foreign scope enters browser
  storage, logs, analytics, receipts, or error messages; and
- an independent security reviewer reports no P0/P1 finding.

## Test and verification strategy

Permanent tests follow the repository checklist: offline, deterministic, behavior-focused, fast for
the pytest lane, and independent of incidental module names, exact collection counts, default
timeouts, or tunable byte limits. Inputs are inline or written under `tmp_path`. Keep:

- pure state/policy/profile parser behavior;
- typed object-key and metadata invariants;
- Store conformance and idempotency;
- route authorization and non-mutation failures;
- worker replay and promotion semantics; and
- browser accessibility and safe network shape.

Use explicit E2E scripts, not permanent pytest, for real PostgreSQL roles, MinIO streaming, scanner
daemon, multi-replica, container outage, memory observation, and cleanup/reconciliation. Treat one-
time profiler output, scanner compatibility probes, and representative throughput measurements as
implementation evidence; remove their temporary code after recording results. When in doubt, remove
a fragile test.

## Risk register

| Risk | Trigger | Mitigation | Owner |
| --- | --- | --- | --- |
| Memory or storage exhaustion | Many large concurrent streams | Streaming, aggregate semaphore/quota, hard effective cap, early abort | Ingress owner |
| MIME/polyglot bypass | Extension/header differs from bytes | Closed profile parsers, single unambiguous match, malware scan | Inspector owner |
| Scanner becomes availability dependency | `clamd` slow or unavailable | Durable queued state, bounded retry, clear status, never mark ready | Worker owner |
| TOCTOU between status and submit | Upload changes after validation | Immutable keys plus one atomic ready-to-consumed transaction | Store owner |
| Cross-tenant object access | Guessed upload/delivery ID | Session, forced RLS, ownership joins, concealed failures | Security reviewer |
| Orphan objects after crash | Object write precedes DB commit | Typed keys, idempotent convergence, WP-RC7 reconciliation | Object owner |
| Dangerous instructor download | Malicious active content | Attachment delivery, `nosniff`, scan, no inline original preview | Delivery owner |
| Scope expansion to office/video | Requests appear during implementation | Keep first profile closed; require a separate CDR/preview decision | Manager |
| Plan/migration drift | Feature implemented before prerequisites | Release-plan dependency and migration-ledger checks | Integrator |

## Rollout and release checklist

1. Keep the current `422 file upload submissions are unavailable` behavior through WP-FU1 through
   WP-FU5 development.
2. Apply migration `2026080912` only after migrations `0908` through `0911` are accepted and present.
3. Deploy the scanner and complete worker registration before enabling capability creation.
4. Run fresh/no-op migration, RLS, MinIO, scanner, multi-replica, retention, and browser gates in a
   disposable environment.
5. Enable file-upload question publication only after the server, worker, and browser report the
   same closed profile version.
6. Enable one pilot course first; record redacted counts, bytes, inspection duration, rejection
   codes, and queue age without filenames or content.
7. Preserve an immediate configuration kill switch that disables new capability issuance while
   leaving existing ready/consumed records retrievable under retention policy.
8. Accept the package only after independent security, PostgreSQL, HCI, and documentation review.

Rollback disables new issuance and drains or safely expires existing unconsumed uploads. It never
deletes consumed learner records, rewrites manual grades, or rolls back the forward migration.

## Documentation close-out requirements

Update README limitations only after acceptance. Until then, link its fail-closed statement to this
plan. On completion update `SECURITY_MODEL.md`, `ASSESSMENT_PAYLOAD_DESIGN.md`, `OBJECT_STORAGE.md`,
`CONTRACTS.md`, `DATABASE_STRUCTURE.md`, `RETENTION_POLICY.md`, `NO_MOUSE_ACCESSIBILITY_CONTRACT.md`,
`CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, `INSTALL.md`, `USAGE.md`, container guidance, release
status, and `CHANGELOG.md` with verified behavior and exact live evidence.

## Final decisions

- The browser submits a typed upload ID, never an object key.
- The upload ID is not authentication; session, RLS, attempt ownership, lifecycle, and digest bind it.
- File bytes use SHA-256; CRC16 remains only a rendered-item consistency identifier.
- Initial transport is same-origin PLE streaming, not a presigned browser write.
- Temp bytes are never delivered; only inspected durable student-record bytes are signable.
- Initial profiles are PDF, plain text, PNG, and JPEG.
- Accepted file submissions always enter manual grading.
- Implementation follows WP-RC7 and the reserved migration ledger, then completes before production
  deployment and final release acceptance.
