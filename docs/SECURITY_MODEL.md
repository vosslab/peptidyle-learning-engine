# Security model

Peptidyle keeps grading authority on the server. The browser may determine
whether a response is structurally ready to submit, but it never receives an
answer key or makes a correctness decision.

This is the cross-cutting enforcement model. It names the boundaries that
must hold across routes, storage, workers, adapters, and browser code. The
specialized durable contracts own their detailed data shapes and operations:

- [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md) classifies protected data
  and its permitted projections.
- [USER_ROLES.md](USER_ROLES.md) owns the closed Student, Instructor, and
  Sysadmin human-role model.
- [DATABASE_TENANCY.md](DATABASE_TENANCY.md) owns PostgreSQL roles, RLS, and
  transaction-local tenant context.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md) owns typed keys, delivery grants, and
  object/database reconciliation.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) owns the learner
  render, response, digest, and rendered-item wire contract.
- [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md) owns caller-visible recovery and
  evidence-preserving failure handling.

The active release plan distinguishes implemented behavior from an accepted
target that has not yet crossed every HTTP, storage, and browser boundary.

## Grading boundary

| Browser-safe surface                           | Server-only surface                |
| ---------------------------------------------- | ---------------------------------- |
| Input schema and browser-side response state   | `grading::AnswerKey`               |
| Parameter generation from a supplied seed      | Expected numeric values            |
| Response-format validation                     | Correct choice IDs and ordering    |
| Timer display and pure state transitions       | Accepted text and private rubrics  |
| Correctness and point results after disclosure | Checkers and correctness decisions |

The browser-safe model explains the input shape and public grading policy. The
current compatibility envelope may expose a numeric tolerance or that exactly
two choices are required. The reserved compact learner presentation must not
expose tolerance; [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)
owns that cutover. The expected number and the two correct choice IDs remain
in `crates/grading`.

`crates/grading` is the browser-excluded authority for checkers, answer keys,
and correctness decisions. It is not the only server-only component that
handles protected answer-bearing material. The native flat-source compiler
parses canonical author source and splits it into answer-free public content
and private key/feedback material. `crates/learning-data-access` then validates
and carries that private material as an opaque grading payload, bound to the
public definition, for authorized staging, publication, and grader retrieval.
It does not expose the canonical bytes through browser-facing stores, routes,
generated contracts, or the Wasm closure.

Ungraded content has no `AnswerKey`; it does not use a browser-safe placeholder
key. Native H5P remains ungraded practice because its own evaluation runs in
the browser. The authenticated author-role-only flat-source `GET`/`PUT` route
is the narrow exception for an instructor's own canonical source. It uses
`Cache-Control: no-store` and a strong ETag, exposes no signed object URL or
checksum, and does not widen learner, public, non-author, or Wasm contracts.

`grading::grade(question, response, key)` repeats browser-safe format
validation before consulting the key. Its generic all-or-nothing checker owns
numeric exact, absolute, relative, and significant-figure comparisons;
multiple-choice set comparison; declared short-text matching; and exact
ordering. It returns only correctness and points. Partial-credit questions are
referred to a capable backend or explicit private rubric, and file uploads are
referred for manual review; the generic checker fails explicitly instead of
inventing either policy.

## Format validation

`domain::validation::validate_response_format` checks only student-controlled
structure:

- response kind matches the definition;
- numeric input is finite;
- selection count, uniqueness, and IDs are valid;
- short text fits its character limit;
- ordering is an exact permutation of the displayed items; and
- the legacy upload placeholder is nonempty.

This function has no answer-key parameter and cannot determine correctness.
The browser calls it through `wasm_bridge::validate_response_format`; the
server repeats it before grading because client validation is a convenience,
not an authority. The browser-safe validator can only reject an empty legacy
upload placeholder; it cannot establish that a string names an authorized
object. The server therefore refuses every file-upload submission before
backend or Store mutation. The server-issued upload capability and metadata
binding workflow are not implemented. [secure_learner_file_upload_plan.md](active_plans/active/secure_learner_file_upload_plan.md)
owns that future path. It requires file size, profile, checksum, ownership,
learner, and attempt binding to be checked from server-owned object metadata,
never from browser claims.

## Compile-time closure

The shipped workspace dependency closure of `wasm_bridge` is exactly:

```text
wasm_bridge
+-- domain
|   `-- question_model
`-- question_model
```

It contains no `grading` crate. `tests/test_crate_boundaries.py` resolves the
normal, build, and target-specific local dependency tables conservatively and
fails if any other workspace crate enters this closure. Including build
dependencies matters because a build script could otherwise embed secret data
without becoming a runtime dependency.

Run the closure gate with the repository Python environment:

```bash
source source_me.sh && python3 -m pytest tests/test_crate_boundaries.py
```

## Export allowlist

`tests/e2e/e2e_wasm_export_allowlist.mjs` builds the current bridge, processes it
with the lockfile-matched `wasm-bindgen` tooling, and compares every export name
and kind with a committed allowlist. Its disposable processed module lives
inside a temporary output directory.

The reviewed application exports are currently:

- `bridge_version`;
- `timer_verdict`;
- `validate_assignment_config`; and
- `validate_response_format`;
- `preview_native_draft`; and
- `verify_presentation_descriptor`.

The allowlist also names the exact memory, table, allocator, and lifecycle
exports required by `wasm-bindgen`. A new Rust export fails the gate until a
reviewer determines that it is key-free and deliberately updates the list.
An answer-bearing export is rejected rather than added.

`timer_verdict` is safe in the browser because its inputs are already disclosed
timer policy and server timestamps, and its output cannot reveal an answer.
The server still supplies the authoritative evaluation timestamp and decides
whether to accept a submission; browser time remains display-only.

`validate_assignment_config` receives only question definitions and backend
capability declarations already shown to an instructor. Its violations name a
question version and a missing capability, never an answer or grading key. The
server independently calls the same domain function before publication.

`preview_native_draft` receives an unversioned draft workspace projection and
a seed. It produces only title, prompt, and response material for native
drafts; other adapters return an explicit `offlinePreview` unavailability
result. The shared materializer lives in `domain`, while native adapter key
derivation remains in its server-only crate. The bridge therefore cannot
construct an answer key, provenance, published identity, grade, or score.

`verify_presentation_descriptor` recomputes only the deterministic descriptor
for already disclosed public envelope and asset-binding data. It returns a
consistency result and cannot issue an attempt, accept a submission, resolve a
durable mapping, or disclose a key. The server retains the full SHA-256 digest
and decides whether a request belongs to that presentation.

Run the export gate directly:

```bash
node tests/e2e/e2e_wasm_export_allowlist.mjs
```

The crate-closure check remains in the fast repository gate. The export allowlist
is an explicit non-browser E2E because it builds the Rust target and runs bindgen.

## Authentication and tenant derivation

Production authentication is PLE-owned and provider-free: a short-lived,
single-use email ceremony creates or restores the opaque PLE account, and
WebAuthn passkeys are optional additional credentials for that same account.
PLE stores no password verifier. Email remains the recovery authority: loss or
revocation of a passkey returns the learner to email sign-in, while a signed-in
email change requires control of the current account. The canonical browser
entry uses seeded production authentication through the ordinary visible email
and passkey flows; it has no local credential form, alternate credential transport, or
alternate browser authentication path.

Authentication uses one `__Host-` opaque session cookie. The raw 256-bit
credential is generated from the operating-system random source, marked
HttpOnly, and never enters browser `localStorage`, logs, or PostgreSQL. The
cookie has no `Max-Age` or `Expires` attribute, so ordinary authentication is
limited to the browser session. Shared session storage contains only its
SHA-256 hash and database-authoritative creation, bounded expiration, and
revocation state.

Production cookies are `Secure; HttpOnly; SameSite=Lax; Path=/` and have no
`Domain` attribute. The `__Host-` prefix makes those host-only constraints
browser-enforceable. The canonical browser path uses the HTTPS gateway. There
is no production embedded `SameSite=None` mode: a future LTI integration must
introduce and review a separate browser/session design rather than weaken the
first-party session contract.

Every unsafe cookie-authenticated request must present the exact canonical
HTTPS `Host` and same `Origin`; duplicate or malformed cookie inputs are
rejected. The sole narrow exception is the external-tool form POST with
`Origin: null`: it additionally requires the exact single session and an
AEAD-protected, HttpOnly, `SameSite=Strict` launch cookie. The API does not
grant credentialed cross-origin CORS. These checks supplement, rather than
replace, `SameSite=Lax`.

Passwordless email and passkey ceremonies persist only hashed/serialized
server state. Email challenges are atomic, browser-bound, short-lived, and
single-use. Quotas consume keyed, non-reversible composite identities (for
the normalized email or credential flow and a coarse client network) before
ceremony persistence. The server trusts `X-Forwarded-For` only from an
explicit CIDR allowlist; it accepts one bounded canonical chain and otherwise
uses the transport peer or a fail-closed shared bucket. IPv4 /24 and IPv6 /56
aggregation bounds abuse without turning a campus NAT into per-device
tracking.

## Encryption and secret boundary

Managed PostgreSQL, object storage, backups, and deployment volumes use
provider-managed encryption at rest with scoped KMS keys. This is the durable
baseline for source, protected records, and image objects. PLE does **not**
blanket-encrypt public published content in the application: immutable public
objects need CDN delivery and integrity comes from their canonical SHA-256
binding, publication authority, and object-store immutability. Application
encryption is selective: AEAD protects secrets that must be stored and later
used, such as external-tool launch state. Keys, database URLs, SMTP credentials,
and provider credentials stay in deployment secret storage and least-privilege
runtime roles, never tracked configuration or browser DTOs.

## Contracted iMathAS provider

iMathAS is disabled unless its complete contracted-provider configuration is
present. The server accepts only a deployment-owned HTTPS base URL, bounded
private transport, immutable-source revalidation, and the contracted
scored-embed profile. Generic hosted MyOpenMath remains refused because it
does not establish immutable execution and server-grade binding.

Provider authentication, launch signing, result verification, correlation,
and launch-state encryption are server-only settings. The attempt-scoped
launch cookie is HttpOnly, Secure, SameSite=Strict, and AEAD-protected.
Provider handles, JWTs, source bytes, result tokens, and grades never enter
URLs, shell HTML, browser messages, logs, or DTOs. Provider reachability is
not a startup or health dependency; an outage is question-local.

The optional block starts with `PLE_IMATHAS_PROVIDER_KEY`; when present it
requires `PLE_IMATHAS_BASE_URL`, `PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS`,
`PLE_IMATHAS_MAX_TRANSPORT_BYTES`, `PLE_IMATHAS_MAX_SNAPSHOT_BYTES`,
`PLE_IMATHAS_MAX_RESULT_BYTES`, `PLE_IMATHAS_LAUNCH_TTL_MILLIS`,
`PLE_IMATHAS_LAUNCH_STATE_SECRET`, `PLE_IMATHAS_CORRELATION_SECRET`,
`PLE_IMATHAS_LAUNCH_SIGNING_SECRET`, and
`PLE_IMATHAS_RESULT_VERIFICATION_SECRET`. Private provider authentication is
optional but paired: `PLE_IMATHAS_PROVIDER_AUTH_HEADER_NAME` must be exactly
`x-ple-provider-auth` and is valid only with nonempty
`PLE_IMATHAS_PROVIDER_AUTH_VALUE`. Secret values belong only in deployment
secret storage, never tracked examples.

## Published QTI runtime

QTI stays unsupported unless `PLE_QTI_RUNTIME_ENABLED=1` and a nonempty
`PLE_GRADER_DATABASE_URL` are both present. Partial, malformed, or unreachable
configuration fails startup before router construction. The grader URL uses the
dedicated `ple_grading_reader` login and constructs a separate bounded pool. It is
never the normal application pool, never acquired through `SET ROLE`, and is
injected only into the QTI backend's `QtiGradingStore` boundary.

The normal application store and object store resolve only immutable published
source, artifact, and asset evidence. The QTI backend reparses the exact
checksum-pinned archive before a private grading lookup, and the dedicated pool
can return only committed published bindings visible to the current tenant.
Disabled QTI has no registry capability or run dispatch; non-QTI and foreign
dispatches do not reach the grader. Connection strings and grading payloads are
not included in errors, Debug output, browser DTOs, TypeScript, or WASM.

## Student-record retention boundary

Student records are FERPA data and treated as radioactive: tenant-owned,
course-scoped, minimized, and excluded from general logs and analytics;
reusable published content is not. Every
learner-facing Store and PostgreSQL path checks the same course-retention access predicate, so
archive cannot be bypassed through runs, summaries, feedback, exports, external tools, or protected
StudentRecord assets. Instructor/Sysadmin retention views expose only coarse
lifecycle, fixed notification copy, and a strong revision-not learner, object,
job, lease, or generation identity. This payload-free lifecycle authority is
the only Sysadmin exception to direct Instructor membership for FERPA-bearing
course state other than the separately audited, closed roster-support
capability. Roster support never grants grade export, item analysis, responses,
runs, or general course access.

Only the scheduler creates a closed retention job binding. The broker-owned prepare and commit
functions require the exact tenant, course, stage, generation, job, and active lease. They persist a
typed StudentRecord object manifest before delivery revocation. The worker refuses foreign-tenant or
non-StudentRecord keys and treats an already absent object as idempotent success. Permanent deletion
then removes only relationally course-owned learner rows and changes the lifecycle to deleted after
residual checks pass. Shared published content, drafts, and anonymous aggregates are outside that
delete authority.

The complete lifecycle, retained/deleted table classes, and honest backup limitation are documented
in [RETENTION_POLICY.md](RETENTION_POLICY.md).

The authentication cookie has no analytics, advertising, tracking, or
preference purpose. Nonessential storage, including `localStorage`, requires a
separate consent path. Persistent `remember me` behavior is not part of the
ordinary session contract and requires explicit user choice plus a
jurisdiction-specific compliance review before implementation.

Before authentication, the PostgreSQL `ple_auth` role can see only the
`auth_session` row matching the presented one-way hash. Resolving that row is
the only production path that constructs `TenantContext`; tenant values from
URLs, headers, or JSON never establish RLS context. Missing, malformed,
unknown, expired, and revoked credentials all return the same unauthenticated
response.

## Author-preview boundary

The ordinary browser/WASM draft preview remains key-free. A separate
`GET /api/workspaces/{workspace}/author-preview` route exists only after an
explicit instructor action. It resolves the stored draft through the same
owner/collaborator binding as workspace editing, requires the exact saved
strong `If-Match` revision, and returns the same absent result for students,
foreign tenants, and unshared workspaces. Responses are `no-store`.

The author route never serializes `AnswerKey`, grading material, source
locator, object key, provider credential, or published identity. A supported
native family may supply only display-ready correct-response and rationale
content through its server-only adapter seam. External sources and native
families without a reviewed presentation return an explicit unavailable state;
they do not invent answer material. The editor saves before requesting this
view, rejects a mismatched response ETag, and keeps author-preview data out of
browser persistence. Student routes deny the authoring surface before its
repository or author-preview client is constructed.

## Catalog publication boundary

Every catalog route resolves the session before deriving `TenantContext`.
Request paths and bodies cannot select another tenant. Institution-visible
metadata and payloads are protected by forced PostgreSQL RLS and an exact
tenant/problem/version grant; the context-free store methods expose public
content only.

The browser supplies a workspace identifier and requested publication scope,
but never a new `ProblemId` or a backend capability declaration. The server
loads the tenant-owned draft, resolves capabilities from its trusted adapter
registry, returns the complete capability-violation list, and generates fresh
published identities for a new work, correction, or derivative. The store
compares and locks the same draft before committing metadata, immutable
payload, visibility grant, and draft deletion in one transaction.

Public publication requires Instructor or Sysadmin authority plus the
institution's optional review gate. Institution publication permits an
Instructor or Sysadmin. Post-publication transitions require both an eligible
role and author ownership. Database privileges permit only the lifecycle fields
to change; published identity, scope, payload, capabilities, metadata,
authorship, and lineage cannot be updated or deleted by the application role.

The no-drift contract requires every content change, including an
original-owner correction, to publish a new Question ID with optional explicit
provenance. Existing assignments retain their exact references until an
Instructor makes a deliberate, revision-checked replacement; no lifecycle or
background action may select a successor for them. The server resolves only the
Question ID chosen by the Instructor for a revision-checked replacement; it
does not select a successor or internal publication pair on the browser's
behalf. The accepted WP-R2 boundary enforces this contract.

Catalog browse responses contain hot browser-safe metadata only. They expose a
backend family but no native family name, WeBWorK path, QTI package identifier,
H5P package identifier, prompt, response definition, or answer-bearing value.
Deprecated and archived questions are hidden from browse, but exact authorized
internal lookup remains available for historical records. A deprecated Question
ID may be deliberately selected for a new assignment when its lifecycle policy
permits it; archived content is blocked. This lifecycle behavior does not add a
successor, "latest" resolution, or automatic
assignment replacement.

## Course authorization boundary

Every course route resolves the shared session before constructing
`TenantContext`; no request may choose a tenant. A coarse instructor role may
create a course, but access to an existing course comes from a tenant-owned
`course_member` row. `Sysadmin` is not a course membership variant and never
substitutes for direct Instructor membership when accessing general FERPA
records. Its closed roster-support exception records actor/course/action/time
and exposes only roster operations needed to help an Instructor.

Course and membership tables use forced tenant RLS. Nonmembers receive the same
not-found response as absent courses, limiting identity disclosure. Students
may list and resolve assignments in their courses but receive a forbidden
response for assignment creation. Assignment writes validate each selected
Question ID against catalog visibility and lifecycle state; no question payload,
answer key, or grading code is copied into the course row or returned by browse.

Assignment creation and focused replacement accept Question IDs, while ordinary
update retains its assigned item identities and changes assignment-owned fields.
Request JSON cannot supply a tenant, course, assignment ID, hidden publication
pair, workspace draft, capability declaration, source, or question payload.
The server resolves each Question ID through tenant-visible catalog state,
accepts published and lifecycle-permitted deprecated questions but not archived
content, and uses the persisted immutable capability declaration with
`validate_assignment_config`. The browser may display the returned safe title,
Question ID, and capability violations, but it is never the capability authority.

Assignment edits use a positive strong revision ETag. Course authorization is
resolved before the `If-Match` precondition, so malformed or missing revisions
cannot become a membership or tenant oracle. Memory performs replacement under
one write lock; PostgreSQL binds tenant, course, assignment, and revision in
the update transaction and locks every selected version against a concurrent
lifecycle transition. Stale writes conflict without changing the stored
assignment. Direct course instructors may mutate;
students receive forbidden and unrelated or foreign courses remain absent.
All success and error responses are `no-store`.

## Assignment export boundary

Assignment exports are created from an authenticated course-management route
with an exact empty body. Authorization is resolved before the body is read, so
request fields cannot select versions, formats, filenames, object identities,
or recipients and cannot become a course or tenant oracle. The Store freezes
the assignment title and ordered immutable version references, the requester,
one opaque manifest, and four server-generated private object identities before
it enqueues one closed export job.

The Store repeats session resolution and exact Instructor membership under the
course-roster lock. The browser cannot name `requested_by` or turn a stale
route decision into an export: revocation racing export creation causes the
transaction to fail.

The worker resolves only that frozen manifest under its tenant context and
builds the standard and accessible DOCX/PDF bundle from browser-safe published
question presentation and immutable capabilities. It never loads an answer
key, private grader state, source locator, or provider credential. Published
figures are rechecked against their exact asset binding and checksum. Output is
written bytes-first to tenant `StudentRecord` keys; an exact immutable object
may be reused after a pre-commit crash, while different existing bytes refuse.

PostgreSQL makes the four delivery rows, requester-only ACLs, ready status, and
worker completion visible in one active-lease transaction. The request and
artifact tables force tenant RLS, broker functions have narrow grants and no
public execution, and permanent or exhausted jobs expose only a coarse failed
state. Browser status contains delivery IDs, stable filenames, and media types,
never object keys, manifests, leases, source refs, failure details, or signed
URLs. Downloads continue through the protected asset route and its audit log.

## Run authorization and grading boundary

The presentation model and its server-persisted binding are implemented, but
the current learner HTTP route still accepts the broader tagged
`StudentResponse` body, including the browser-supplied response `kind`. The
current route rederives and validates the expected family from the attempt;
`kind` is therefore not submission authority. The accepted
[secure grading payload plan](active_plans/decisions/secure_question_grading_payload_plan.md)
owns a future atomic wire cutover to authenticated attempt ID, idempotency
key, presentation digest, and a family-minimal, type-free answer. That target
also introduces CRC16 rendered-item IDs and a SHA-256-backed presentation
digest to detect inconsistent presentation state. Neither target value
authenticates the learner or grades an answer. All component scoring and
partial credit remain server-owned in both contracts.

The current tagged render `ResponseDefinition` also exposes some
grading-adjacent metadata, including numeric tolerance and short-text match
mode. That metadata does not disclose an expected answer, but it is broader
than rendering requires. The target render projection retains only public
input constraints and displayed units while keeping tolerances, normalization
rules, answer keys, weights, and rubrics server-only.

Run reads and mutations require the authenticated `UserId` stored on the
enrollment **and an active `Student` course membership at the Store/DB
boundary**; they never infer authorization by equating that identity with
`StudentId`. This is repeated for learner run, enrollment, summary, attempt,
prefetch, feedback-release, issuance, submission, and external-tool paths.
PostgreSQL checks it in the same transaction with the roster lock, and the
in-memory Store uses the corresponding atomic lock. Course instructors retain a
separate, explicitly authorized historical-record projection after removal;
that Instructor authority never leaks into a learner-scoped Store method.
Direct course instructors may read enrollment history and
summaries, but only the enrollment owner may start or submit a run. Nonowners
receive not found so record existence is not disclosed.

Each newly issued attempt receives an operating-system-random seed. Resuming
an unresolved attempt returns its stored seed and provenance, and the store
locks the run so only one unresolved question exists at a time. Server-owned
database timestamps determine issue time, deadline, response arrival, and run
completion.

Next-question prefetch stores a tenant-owned, server-only reservation without an
attempt ID, timer, response, grade, or public answer. Its browser projection is
answer-free, while the reservation retains checksummed private grading authority
for the exact issued question so first grade never reconstructs from current
catalog or renderer state. The Store binds it to the owned active predecessor
and first unattempted assignment position. Only submission promotion creates
the successor attempt and records either its immutable `nextIssued` descriptor
or durable `nextPending` state in the predecessor's receipt. Replay reads that
state instead of deriving a new successor from
current run state; initial owner-scoped recovery alone may heal the sole
committed-but-unlinked predecessor after a process failure.

The prefetch response contains only the safe envelope and an exact descriptor.
Its rendered hash remains backend-owned because a backend such as WeBWorK may
cover sanitized markup in addition to the shared envelope. The route still
requires exact parameter hash, full provenance, version, and seed reproduction.
The browser caches this projection in memory only, aborts it on route teardown,
warms at most 12 deduplicated same-origin logical asset routes, and advances
from it only after an exact `nextIssued` receipt match. No prefetch envelope or
descriptor enters `localStorage` or `sessionStorage`.

The server first validates browser-visible rendered IDs and response shape
against the checksummed issued public snapshot, then translates those IDs and
validates the result against the server-only grading envelope before calling
the injected grader. Native and WeBWorK first grading additionally require
their matching issued private grading contracts, so neither path reloads a
current catalog definition or grader view. The idempotency table retains the
original public learner response; the translated private response is grade-only. Submission persistence
rejects malformed point values and atomically commits the response, grade
event, run and enrollment transitions, and summary. The idempotency table is
insert-only for the application role; an exact retry returns its first
committed receipt, while a changed key or response conflicts.

The current attempt DTO is answer-free but broader than the learner needs: it
still carries version, seed, parameter hash, provenance, implementation IDs,
and source/asset identifiers. Feedback policy redacts answer-bearing material,
not that complete DTO. The payload plan's minimal learner descriptor,
digest-bound type-free response body, and compact receipt are accepted target
work, not the current HTTP contract. Until that atomic cutover, clients must
not treat current provenance fields or the tagged response `kind` as
submission authority. Policy-permitted results may contain correctness and
points, but never an answer key, expected value, private rubric, or checker
state. Full teaching feedback uses an explicit sanitized disclosure DTO; it
never serializes the server-only key as a shortcut.

## External-tool indeterminate-effect boundary

An external tool is an untrusted, potentially effectful service. Before PLE
dispatches a provider `POST`, the Store atomically records a pre-dispatch
marker tied to the current, unexpired activity-lease token. Only a valid
provider response can clear that exact marker. A timeout, I/O failure, process
death, lease expiry, or later launch leaves the attempt permanently
indeterminate and fail-closed: it cannot be reclaimed, relaunched, graded, or
finalized automatically. Read-only provider retrieval is structurally a GET;
the browser has no generic provider proxy. The learner receives a generic
accessible recovery message directing them to the instructor, rather than
details that could disclose provider state or invite a duplicate action.

## Asset delivery boundary

Browser markup carries an internal logical `AssetId`, never a bucket name,
physical key, or signed URL. `/api/assets/{id}` resolves the identifier through
the database-authoritative immutable registry. The registry accepts only a
`ProblemAsset` whose problem, version, asset, object, bucket, and category all
agree, or a tenant-matching `StudentRecord`; source packages, render caches,
and `temp-processing` objects cannot be registered for this route.

`WorkspaceSource` and `ProblemSource` are never direct delivery targets and the
typed object contract refuses to sign either key. This includes compact PLE
flat-question JSON as well as QTI, iMathAS, and other answer-bearing sources.
An instructor preview or export must use a separate authorized projection that
redacts or deliberately includes private material for that operation; it must
not expose the source object URL.

Globally public catalog assets redirect to the configured immutable CDN URL
without authentication or an object-store signing call. Institution content
and student records require the opaque HttpOnly session. Forced RLS limits the
candidate row, the store checks the authenticated user where the record has an
explicit user grant, and missing or unauthorized protected objects both return
not found. Every successful protected authorization appends an audit event
before requesting the signed URL. The event includes tenant, actor, delivery
ID, object ID, bucket, and database timestamp, but never the cookie or URL.

The object backend selects the lifetime from the typed bucket, and the route
rejects a result that exceeds 60 minutes for `content` or 5 minutes for
`student-records`. Protected redirects use `Cache-Control: no-store`,
`Pragma: no-cache`, and `Referrer-Policy: no-referrer`; public redirects use an
immutable cache policy and checksum ETag. Signed URLs are response headers
only and must not enter JSON, application logs, browser storage, or persisted
markup.

## Diagnostics and observability

Diagnostics preserve enough evidence to investigate a boundary failure without
becoming another delivery path. Browser responses carry only short,
route-approved messages. They do not contain raw SQL, object keys, bucket
names, signed URLs, tenant identities, leases, source archives, provider state,
answer keys, or raw backend errors.

Server and worker diagnostics use bounded error categories and safe record
identities only where an operator needs correlation. Credentials, raw session
cookies, provider launch values, renderer fields, source bytes, private grading
payloads, raw learner uploads, and raw student answers are never general log
fields. A new diagnostic must use the [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)
class of each field and record the appropriate authorization and retention
owner before it is emitted.

Security-relevant delivery authorization appends an audit event before a
protected object URL is requested. Worker and retry diagnostics preserve only
the evidence necessary to recover through their durable receipt or lease
boundary. [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md) defines the required
public outcome and retry behavior; it is not acceptable to reveal a hidden
cause merely to make a support response more convenient.

## Placement rule

Place new code according to the information it needs and the decision it
makes:

- Put response parsing and structural validation in `crates/domain` when the
  result is independent of a correct answer.
- Put expected values, accepted answers, grading rubrics, partial-credit
  weights, and correctness decisions in `crates/grading`.
- Expose a domain function through `crates/wasm` only when all inputs and
  outputs are safe for a student to inspect.
- Return correctness and points through server-controlled feedback policy;
  never return the key or checker state.

When uncertain, ask whether the value would help a student infer the correct
response before submission. If yes, it belongs on the server-only side.

## Verification and change control

Security controls require evidence at the boundary they claim to protect.
Wasm closure and export-allowlist tests prove browser exclusion; Memory tests
prove pure and Store behavior; live PostgreSQL tests prove migrations, roles,
grants, and forced RLS; private renderer checks prove a provider protocol; and
browser traces prove what a learner-facing page actually receives. No one
class substitutes for another. [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md)
defines those limits, while the active release plan names the required gate for
each work package.

When adding or changing a path that handles protected data, update its owning
contract and verify all of the following: the data classification, authenticated
authorization, RLS and transaction boundary where applicable, server-only
grading boundary, retention/deletion owner, browser projection, diagnostic
redaction, and recovery behavior. The narrowest relevant security test runs
first; the package's full acceptance gate then verifies the integrated claim.
