# Data classification

This reference classifies the information PLE handles so contributors can make
safe storage, API, cache, logging, and deletion decisions before adding a new
field or artifact. It complements the enforcement details in
[SECURITY_MODEL.md](SECURITY_MODEL.md), the API and module register in
[CONTRACTS.md](CONTRACTS.md), and the concrete object rules in
[OBJECT_STORAGE.md](OBJECT_STORAGE.md). It does not grant an exception to any
of those contracts.

Classification follows the information's meaning, not its representation. A
UUID, checksum, object ID, or opaque handle can still be sensitive when it
links a learner to a protected record. A value copied from a private object to
a log, cache, URL, or browser field retains its original classification.

## Decision procedure

Classify a new datum before choosing its Rust type, table, object key, API
projection, or telemetry event.

1. Decide whether it is answer-bearing or could reveal a correct answer before
   a learner submits. If so, it is server-only grading material.
2. Decide whether it identifies or describes a learner, course member, or
   tenant-owned teaching activity. If so, it is a FERPA educational record and
   receives the repository's radioactive handling discipline.
3. Decide whether it is immutable shared teaching content, tenant-private
   authoring/provenance content, or a protected artifact. Do not infer
   delivery permission from its bucket.
4. Give browser code only the narrowest projection needed for the visible
   operation. The authenticated attempt, not browser-provided tenant or
   question information, determines grading authority.
5. State the retention owner and deletion behavior when the datum is created.
   A record without a deletion owner does not become exempt from retention.

## Classification matrix

"Browser exposure" means ordinary learner-facing and public browser contracts.
The explicit instructor-only canonical-source and author-preview routes remain
narrow exceptions described in [SECURITY_MODEL.md](SECURITY_MODEL.md).
"Implemented" describes current code; "planned" names a committed work
package rather than claiming that capability has shipped.

| Data class | Examples and authoritative owner | Storage and access owner | Browser exposure | Retention and deletion | Current state |
| --- | --- | --- | --- | --- | --- |
| Shared public teaching content | Immutable published question identity, browser-safe prompt, response definition, licensed metadata, and public assets. `question_model`, catalog publication, and `ObjectKey::ProblemAsset` own the shape. | Shared catalog tables and the physically separate `PublicAssets` bucket. Only `Ready` public registry records are CDN-visible. | Safe published presentation and logical `AssetId` values may be sent. The browser never receives a bucket, physical object key, arbitrary signed URL, or hidden publication pair. | Not learner records. Publication is immutable; every correction publishes a new Question ID and fresh hidden evidence. Retention deletion must not follow course references into shared content. | Implemented code boundary; deployed CDN/IAM/Object Lock evidence is separate. |
| Tenant teaching definitions | Course settings, instructor membership, assignment definitions, assignment-to-problem references, and course content such as banners. `learning-data-access` and `server` own authorization and lifecycle. | Tenant-scoped PostgreSQL rows under `FORCE ROW LEVEL SECURITY`; course content uses its approved typed object class. `TenantContext` comes only from the authenticated session. | Only authorized route projections, normally `Cache-Control: no-store`. Browser state contains no tenant-selection authority. | These definitions are not automatically part of the learner-record purge. The default frozen course disposition retains assignment definitions and instructor membership; an explicit deletion disposition may delete assignment definitions. Shared content remains outside either path. | Implemented retention distinction; see [RETENTION_POLICY.md](RETENTION_POLICY.md). |
| Learner educational records (FERPA; radioactive) | Learner enrollment and course/group membership, assignment summaries, runs, attempts, submissions, feedback, grades, exports, student-record assets, retention receipts, and learner access/audit evidence. `learning-data-access` and `server` own authorization and lifecycle. | Tenant-scoped PostgreSQL rows under `FORCE ROW LEVEL SECURITY`; protected artifacts use `student-records`. Direct Student ownership or direct Instructor membership is normally required in addition to the server-derived tenant. Sysadmin has only audited roster support and coarse retention capabilities, never general student-record authority. | Only the exact authorized teaching or roster-support projection, normally `Cache-Control: no-store`. Exclude it from general logs, analytics, URLs, and browser persistence. | Course lifecycle notifies, archive-fences access, then deletes the course-owned learner graph according to the institution policy. Shared content, drafts, anonymous aggregates, and normally assignment definitions survive. Application deletion is immediate; backup expiry and recovery objectives remain undeployed WP-RC10 work. | Implemented application retention lifecycle; see [RETENTION_POLICY.md](RETENTION_POLICY.md). |
| Answer-bearing grading material | Answer keys, accepted values, rubrics, partial-credit weights, checker configuration, canonical flat private material, and private feedback sidecars. `crates/grading` owns correctness decisions; adapters own engine-specific private material. | Separate private grading tables/grants and server-only adapter/grader paths. Canonical source objects live in `PrivateContent`; source classification prevents delivery. | Never in learner, public, generated TypeScript, Wasm, browser cache, URL, log, trace, or ordinary DTO. A reviewed author-only source route is not a learner contract. | Retained with the immutable content/provenance required to reproduce an authorized grade; it is never deleted as part of a learner-record purge. | Implemented server-only and Wasm-closure boundary. |
| Account and authentication data | Authentication email, account label, opaque `UserId`, passkey public credential/state, and account ceremonies. The dedicated auth capability owns it separately from course Stores. | Global PLE account tables are restricted to `ple_auth`; course Stores and instructors do not receive them. | Only the account owner receives the minimum account-management projection. A course-linked copy becomes FERPA data even when the global value is not. | Account lifecycle and security retention, not automatic course retention, owns the global record. Course snapshots follow course retention. | Implemented passwordless/passkey separation. |
| Credentials and secrets | Opaque session credentials, database URLs, object-store credentials, provider authentication, signing/encryption keys, and deployment secrets. Auth and deployment composition own them. PLE stores no password verifier. | Host-only HttpOnly cookie for the raw ordinary session credential; server-side hashed session record; deployment secret storage and process configuration for other secrets. | Raw credentials, secrets, and connection strings never enter JSON, local storage, URLs, logs, traces, generated code, images, or repository examples. | Session records expire or revoke under authentication policy. Deployment secrets follow institutional rotation and revocation procedures, not course retention. | Implemented account-session and secret boundary; production secret-manager delivery is deployment work. |
| Private provider replay and session state | WeBWorK field/value replay mapping, renderer identity, iMathAS launch correlation, provider handles, source bytes, result tokens, and launch cookies. The adapter/server boundary owns it. | Attempt-bound private persistence and server-held sessions; renderer and provider calls receive trusted server-built requests only. | The learner receives a safe attempt envelope and result projection, never upstream field names, values, source, token, credential, or provider session state. | Retained only as tenant-owned attempt evidence while needed for replay, grading, and course retention. It is removed with the associated learner record unless a separate immutable shared source/provenance rule applies. | WeBWorK replay persistence is implemented for the supported path; iMathAS remains contracted-provider work. |
| Protected assets and student artifacts | Institution-only published assets, course banners, student exports, annotated exams, and future Student-upload submissions. Object storage and the asset-delivery registry own typed object identity and delivery grants. | Restricted published assets and banners use `PrivateContent`; student artifacts use `StudentRecords`. Every protected delivery first resolves an opaque delivery ID through PostgreSQL authorization. | Authorized readers make `POST /api/assets/{id}/delivery` and receive a short-lived URL after an audit event. Browser JSON and markup contain no raw object key or durable signed URL; temporary objects are never served. | Course banners follow course-content lifecycle. Student artifacts are retention-bound and are revoked/deleted through the exact typed cleanup manifest. | Implemented for banners, exports, restricted assets, and protected delivery. |
| Student upload candidates and final uploads | One file response bound to tenant, Student, attempt, course, response definition, and presentation digest. The planned upload record, worker, and object contract own it. | Planned candidate bytes in non-deliverable `temp-processing`, then inspected durable submissions in `student-records` with server-computed SHA-256. The browser receives only `StudentUploadId`. | Current file-response submissions fail closed. The future same-origin flow does not give the browser an object key, storage credential, presigned write URL, authoritative MIME type, or client checksum. | Planned candidate cleanup follows short processing lifetime. Consumed durable submissions follow Student-record archive/delete; rejected or abandoned candidates are not deliverable. | Planned in [secure Student file-upload plan](active_plans/active/secure_student_file_upload_plan.md); it is not enabled. |
| Anonymous aggregate statistics | Cohort-gated item difficulty, timing, and discrimination statistics associated with shared problem versions. `MOD-STATS` owns the aggregate boundary. | Shared aggregate tables, identity-free and separate from course records. A course-local item-analysis projection is a different tenant-owned record. | Catalog disclosure is suppressed below the deployment-wide k-anonymity floor. No learner, tenant, course, raw response, or per-student score is included. | Survives learner-record deletion because it is generated before purge and contains no identifying record. It is not reconstructed from deleted attempts. | Implemented aggregation and disclosure boundary; current reports distinguish anonymous aggregates from course-local analysis. |
| Logs, diagnostics, and audit evidence | Security audit events, protected-delivery authorization, bounded worker/job evidence, errors, and operational diagnostics. The producing server or worker owns its event shape. | Tenant-scoped audit evidence remains under RLS when it records educational activity; deployment observability follows operations controls. Queue payloads remain bounded identifiers and generations. | Browser-facing errors are coarse and safe. Logs, traces, console output, telemetry, and diagnostic attachments must omit answers, keys, raw responses, object URLs, credentials, provider tokens, and session values. | Educational audit evidence follows the associated tenant record lifecycle. Operational logs require a documented deployment retention policy and must not be used as an undeclared record archive. | Implemented application audit controls; production observability retention is deployment work. |

## Boundary rules by storage medium

### PostgreSQL

- Shared immutable catalog content has no tenant row ownership; tenant-owned
  educational records carry `tenant_id` and are accessed through a
  server-derived `TenantContext`.
- Forced RLS, transaction-local tenant context, and narrow roles are the
  access boundary. A browser header, URL component, or JSON field never
  supplies tenant authority.
- A grading-reader connection is a separate least-privilege capability. The
  ordinary application Store does not acquire grading-read access by changing
  role inside a request.
- A table or JSON envelope does not make an answer-bearing value browser-safe.
  Private grading payloads stay opaque outside their authorized grader path.

### Object storage

- `PublicAssets`, `PrivateContent`, `StudentRecords`, and `TempProcessing` are
  distinct buckets because their delivery, encryption, IAM, and lifecycle
  policies differ. A bucket is a physical enforcement domain; it still does
  not independently authorize a browser request.
- `ObjectKey` derives physical paths from typed server IDs. Callers do not
  build raw storage paths, and a browser-provided string never becomes an
  object key.
- Source, provenance, renders, and institution-only assets are durable objects
  in `PrivateContent` while remaining private because they can contain
  answer-bearing material. The only CDN domain is `PublicAssets`, and it holds
  only `Ready` public `ProblemAsset` objects. `TempProcessing` is never
  delivered.
- Every object record carries a server-computed SHA-256, verified media type,
  semantic category, provenance, and creation time. These integrity fields do
  not change the authorization rule for the object's contents. Production
  writes require SSE-KMS; per-domain encryption at rest does not make a public
  asset private and does not replace RLS, IAM, or route authorization.

### Browser, caches, and URLs

- The default browser persistence is in-memory UI state only. Session tokens,
  answer-bearing values, object keys, grades, and provider state are not
  stored in `localStorage`, `sessionStorage`, or persistent browser caches.
- Render payloads are intentionally richer than submissions. **Currently**, a
  tagged `StudentResponse` body includes `kind` alongside the response value;
  the server still derives the expected family, grading backend, seed,
  ownership, and policy from the attempt. The accepted **target** submission
  sends the attempt-bound route, idempotency key, presentation digest, and a
  family-minimal type-free answer. See
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for the
  cutover boundary.
- Protected route responses use `no-store`. A signed object URL is an
  authorization result returned only by the protected delivery POST; it is
  not a durable browser datum.
- Presentation digests and rendered-item CRC16 identifiers are planned
  consistency data, not credentials or grading keys. They will not authorize
  an action or authenticate a learner; the current tagged response contract
  does not depend on them.

## Classification-specific design rules

### Shared content is not automatically public

Published prompt material and problem assets can be browser-safe, while a
published source archive, private render state, or grading binding can remain
private. Store a durable shared object only when it supports immutable
identity, reproduction, provenance, or approved delivery. Add delivery through
the asset registry, never by exposing the underlying object.

Public catalog assets are published through a transactional outbox. The catalog
transaction stores a `Pending` registry record and a closed publisher job; the
dedicated publisher copies only an allowlisted private workspace image into
`PublicAssets`, verifies its SHA-256, and lease-conditionally changes the
record to `Ready`. A `Pending` record is not browser-visible. This prevents a
browser-controlled key, arbitrary private source, or pre-commit public object
from becoming a public asset.

### Educational records are owned by the tenant, not by a browser session

Treat this entire class as radioactive. The rule is broader than direct PII:
an opaque attempt ID, timing event, response, score, or delivery audit becomes
FERPA data when it links a student to a course activity.

The [database radioactive table map](DATABASE_TENANCY.md#radioactive-table-map)
classifies current PostgreSQL relations as especially radioactive or
radioactive by stable linkage. The distinction prioritizes incident response;
it does not weaken controls for linked records. Derived query results and
persistent database copies inherit the highest classification of their inputs.

An attempt ID, run ID, object-delivery ID, or upload ID is an opaque locator.
Every use still rechecks the authenticated actor, tenant, course membership or
attempt ownership, lifecycle state, and operation-specific binding. Opaque IDs
reduce accidental disclosure; they do not replace authorization.

### Uploads remain a planned security class until the whole path exists

The current raw `object_key` file-response placeholder is intentionally
unusable for learner submissions. Enabling an upload widget, accepting a
browser-supplied key, or issuing a direct object-store write URL before the
server-issued record, inspection worker, atomic consumption, and reconciliation
path are accepted would violate this classification contract.

### Statistics must stay anonymous in both shape and disclosure

Removing a learner identifier is insufficient when a small cohort can identify
the learner indirectly. Aggregate computation happens while records exist,
publication enforces the k-anonymity threshold, and the retained result holds
no tenant or learner identifier. Course-specific item analysis remains a
tenant record even when it uses aggregate arithmetic.

## Change checklist

Before merging a new data path, answer these questions in its owning plan or
contract test:

1. Which matrix row owns the datum, and does it need a new explicit class?
2. Who authorizes read, write, delivery, and deletion?
3. Does an authenticated attempt or another server-owned record already derive
   information the browser would otherwise resend?
4. Can browser, Wasm, generated TypeScript, a URL, a cache, a log, or a trace
   reveal more than the approved projection?
5. Which immutable identity, checksum, version, or provenance binding proves
   that the server is acting on the intended data?
6. Which retention stage, object manifest, and reconciliation path remove the
   datum or deliberately preserve it?
7. Is the behavior implemented and validated, or is it a named future work
   package that must remain fail-closed today?

## Related references

- [Security model](SECURITY_MODEL.md) defines the grading, authentication,
  browser, provider, and delivery enforcement boundaries.
- [Contracts](CONTRACTS.md) names the public module and route contracts.
- [Database tenancy](DATABASE_TENANCY.md) defines RLS, transaction context,
  roles, and tenant educational-record ownership.
- [Object storage](OBJECT_STORAGE.md) defines typed object keys, delivery, and
  reconciliation status.
- [Retention policy](RETENTION_POLICY.md) defines the course lifecycle and
  backup limitation.
- [Assessment payload design](ASSESSMENT_PAYLOAD_DESIGN.md) defines the
  render-to-submission data boundary.
- [Secure Student file-upload plan](active_plans/active/secure_student_file_upload_plan.md)
  defines the unimplemented upload security path.
