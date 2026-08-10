# Security model

Peptidyle keeps grading authority on the server. The browser may determine
whether a response is structurally ready to submit, but it never receives an
answer key or makes a correctness decision.

## Grading boundary

| Browser-safe surface                           | Server-only surface                |
| ---------------------------------------------- | ---------------------------------- |
| `ResponseDefinition` and `StudentResponse`     | `grading::AnswerKey`               |
| Parameter generation from a supplied seed      | Expected numeric values            |
| Response-format validation                     | Correct choice IDs and ordering    |
| Timer display and pure state transitions       | Accepted text and private rubrics  |
| Correctness and point results after disclosure | Checkers and correctness decisions |

The browser-safe model explains the input shape and public grading policy. For
example, it may reveal a numeric tolerance or that exactly two choices are
required. The expected number and the two correct choice IDs remain in
`crates/grading`.

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
- an uploaded response carries a server-issued object reference.

This function has no answer-key parameter and cannot determine correctness.
The browser calls it through `wasm_bridge::validate_response_format`; the
server repeats it before grading because client validation is a convenience,
not an authority. The current server refuses every file-upload submission
before backend or Store mutation because the server-issued upload capability
and metadata-binding workflow do not exist yet. When that workflow is added,
file size, extension, checksum, ownership, learner, and attempt binding must be
checked against server-owned object metadata rather than trusted from the
browser.

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

`tests/test_wasm_export_allowlist.mjs` builds the current bridge, processes it
with the lockfile-matched `wasm-bindgen` tooling, and compares every export name
and kind with a committed allowlist. Its disposable processed module lives
under ignored `generated/wasm-export-check/`.

The reviewed application exports are currently:

- `bridge_version`;
- `timer_verdict`;
- `validate_assignment_config`; and
- `validate_response_format`.
- `preview_native_draft`.

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

Run the export gate directly:

```bash
node --test tests/test_wasm_export_allowlist.mjs
```

Both boundary gates run from `./check_codebase.sh`.

## Authentication and tenant derivation

Authentication uses one host-only opaque session cookie. The raw 256-bit
credential is generated from the operating-system random source, marked
HttpOnly, and never enters browser `localStorage`, logs, or PostgreSQL. The
cookie has no `Max-Age` or `Expires` attribute, so ordinary authentication is
limited to the browser session. Shared session storage contains only its
SHA-256 hash and database-authoritative creation, bounded expiration, and
revocation state.

The normal HTTPS cookie policy is `Secure; SameSite=Lax`. LTI embedding must
explicitly select `SameSite=None; Secure`, and plain HTTP requires the named
local-development mode. Credential-provider implementations own their
protocol's replay and CSRF checks; the generic login route also bounds a
presentation body to 64 KiB.

`SameSite=None` is not a CSRF defense. Ordinary browser mutations use
same-origin JSON requests and the server must not add permissive credentialed
CORS. Before embedded LTI mode is composed for production, its state-changing
requests require an origin-bound anti-CSRF mechanism in addition to the LTI
launch protocol's state, nonce, and replay validation.

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

Student records are tenant-owned and course-scoped; reusable published content is not. Every
learner-facing Store and PostgreSQL path checks the same course-retention access predicate, so
archive cannot be bypassed through runs, summaries, feedback, exports, external tools, or protected
StudentRecord assets. Manager retention views expose only coarse lifecycle, fixed notification
copy, and a strong revision-not learner, object, job, lease, or generation identity.

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
registry, returns the complete capability-violation list, and generates a
fresh problem identifier for a new work or fork. The store compares and locks
the same draft before committing metadata, immutable payload, visibility
grant, and draft deletion in one transaction.

Public publication requires a publisher or administrator role plus the
institution's optional review gate. Institution publication permits an
instructor, publisher, or administrator. Post-publication transitions require
both an eligible role and author ownership. Database privileges permit only
the lifecycle fields to change; published identity, scope, payload,
capabilities, metadata, authorship, and lineage cannot be updated or deleted by
the application role.

Catalog browse responses contain hot browser-safe metadata only. They expose a
backend family but no native family name, WeBWorK path, QTI package identifier,
H5P package identifier, prompt, response definition, or answer-bearing value.
Deprecated and archived versions are hidden from browse, but exact authorized
lookup remains available for historical records. Deprecation remains usable by
an exact new reference; archival additionally blocks new assignments.

## Course authorization boundary

Every course route resolves the shared session before constructing
`TenantContext`; no request may choose a tenant. A coarse instructor role may
create a course, but access to an existing course comes from a tenant-owned
`course_member` row. Tenant administrators may inspect every course through a
separate effective-authority path. Administrator authority is not a storable
membership variant, so a membership write cannot accidentally manufacture
tenant-wide access.

Course and membership tables use forced tenant RLS. Nonmembers receive the same
not-found response as absent courses, limiting identity disclosure. Students
may list and resolve assignments in their courses but receive a forbidden
response for assignment creation. Assignment writes validate every exact
problem/version reference against catalog visibility and lifecycle state; no
question payload, answer key, or grading code is copied into the course row or
returned by browse.

Assignment creation and replacement accept only a title, an ordered list of
immutable problem/version references, and the four assignment-level
`RunPolicies`. Request JSON cannot supply a tenant, course, assignment ID,
workspace draft, capability declaration, source, or question payload. The
server resolves each reference through tenant-visible catalog state, accepts
published and deprecated versions but not archived versions, and uses the
persisted immutable capability declaration with `validate_assignment_config`.
The browser may display the returned safe title/reference/capability
violations, but it is never the capability authority.

Assignment edits use a positive strong revision ETag. Course authorization is
resolved before the `If-Match` precondition, so malformed or missing revisions
cannot become a membership or tenant oracle. Memory performs replacement under
one write lock; PostgreSQL binds tenant, course, assignment, and revision in
the update transaction and locks every selected version against a concurrent
lifecycle transition. Stale writes conflict without changing the stored
assignment. Direct course instructors and tenant administrators may mutate;
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

This section describes the implemented route. Before WP-RC5, the accepted
[secure grading payload plan](active_plans/decisions/secure_question_grading_payload_plan.md)
atomically narrows the learner wire to authenticated attempt ID, idempotency
key, presentation digest, and a family-minimal answer. Its CRC16 rendered-item
IDs and SHA-256 presentation digest detect inconsistent presentation state;
they do not authenticate the learner or grade. All component scoring and
partial credit remain server-owned.

Run mutations require the authenticated `UserId` stored on the enrollment;
they never infer authorization by equating that identity with `StudentId`.
Course instructors and tenant administrators may read enrollment history and
summaries, but only the enrollment owner may start or submit a run. Nonowners
receive not found so record existence is not disclosed.

Each newly issued attempt receives an operating-system-random seed. Resuming
an unresolved attempt returns its stored seed and provenance, and the store
locks the run so only one unresolved question exists at a time. Server-owned
database timestamps determine issue time, deadline, response arrival, and run
completion.

Next-question prefetch stores a tenant-owned, key-free reservation without an
attempt ID, timer, response, grade, or answer. The Store binds it to the owned
active predecessor and first unattempted assignment position. Only submission
promotion creates the successor attempt and atomically records its immutable
link in the predecessor's receipt. Replay reads that link instead of deriving a
new successor from current run state; a bounded owner-scoped pending lookup may
heal the sole committed-but-unlinked predecessor after a process failure.

The prefetch response contains only the safe envelope and an exact descriptor.
Its rendered hash remains backend-owned because a backend such as WeBWorK may
cover sanitized markup in addition to the shared envelope. The route still
requires exact parameter hash, full provenance, version, and seed reproduction.
The browser caches this projection in memory only, aborts it on route teardown,
warms at most 12 deduplicated same-origin logical asset routes, and advances
from it only after an exact `nextIssued` receipt match. No prefetch envelope or
descriptor enters `localStorage` or `sessionStorage`.

The server repeats structural response validation before calling the injected
grading backend. Submission persistence rejects malformed point values and
atomically commits the response, grade event, run and enrollment transitions,
and summary. The idempotency table is insert-only for the application role;
an exact retry returns its first committed receipt, while a changed key or
response conflicts.

The current attempt DTO is answer-free but broader than the learner needs: it
still carries version, seed, parameter hash, provenance, implementation IDs,
and source/asset identifiers. Feedback policy redacts answer-bearing material,
not that complete DTO. WP-P1 through WP-P6 in the secure grading payload plan
replace it before WP-RC5 with the minimal learner descriptor, presentation
digest, type-free response body, and compact receipt. Until that atomic
cutover, clients must not treat current provenance fields as submission
authority. Policy-permitted results may contain correctness and points, but
never an answer key, expected value, private rubric, or checker state. Full
teaching feedback uses an explicit sanitized disclosure DTO; it never
serializes the server-only key as a shortcut.

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

## Other controls

WP-C6 proves the source and WebAssembly boundary. MOD-API-AUTH, MOD-API-CAT,
MOD-API-COURSE, MOD-API-RUN, MOD-SCHEMA, MOD-STO, and MOD-OBJ now add
authentication, catalog, course, and run authorization, PostgreSQL answer-table
grants, forced tenant row-level security, and signed object URLs. Later work
packages still add authorization for asset routes, sanitized supplied markup,
content security policy, and browser network-trace inspection. None of those
controls weakens the crate boundary established here.
