# API contract map

This document is the durable route-level map for PLE's same-origin HTTP API.
It tells a client or server contributor which boundary owns a route, what
identity the route trusts, and where to find the exact request and response
shape. It is not an OpenAPI replacement and deliberately does not duplicate
every generated Rust or TypeScript field.

[CONTRACTS.md](CONTRACTS.md) remains the module ownership register.
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns the detailed
authorization decision sequence and authority sources.
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) owns the Student
render-and-submit boundary and its planned cutover. [SECURITY_MODEL.md](SECURITY_MODEL.md)
owns the threat model and storage rules. This document connects those sources
at the HTTP boundary.

## Status and authority

The route registrations in [composition.rs](../crates/server/src/composition.rs)
are the executable authority for the mounted production surface. The current
entry point mounts only readiness, session resolution and logout, and the
deployment-gated seeded Live Demo account selector. The unmounted route
modules, browser clients, and generated DTOs describe retained target work;
they do not establish an available endpoint.

The product contracts below remain the required design for the Store-backed
delivery reconstruction. They become HTTP contracts only when that
reconstruction mounts them through the production composition entry point and
adds connected evidence under [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Common protocol rules

| Concern | Durable contract |
| --- | --- |
| Origin | Browser API requests use a relative same-origin path, `credentials: "same-origin"`, and `cache: "no-store"`. [request.ts](../src/api/http_client/request.ts) rejects an external base path. |
| Sessions | The server resolves a hashed opaque session cookie into one Authenticated Session for an Account and returns only `{ authenticated: true, account: { id, role } }`. `role` is the Account's one immutable Product Role; the response contains no credential, display name, membership, or role list. Request paths, query parameters, headers, and JSON select a resource for exact relationship authorization. [auth.rs](../crates/server/src/auth.rs) owns this boundary. |
| Caching | Private JSON routes return `Cache-Control: no-store`, including errors where a route applies its response middleware. The browser verifies this for sensitive appearance traffic. Immutable public asset redirects are the explicit exception. |
| JSON | **Current pre-WN1:** PLE JSON transport remains mixed and many routes use lower-camel fields directly. **Approved target:** PLE-owned JSON fields, TypeScript data-object properties, PLE query keys, and portable discriminants use direct `snake_case` generated from effective Serde. Feature decoders reject unknown and retired-camel PLE input and retain bounded-body, closed-union, numeric, and relationship checks. Registered external payloads retain owner spelling at their adapter boundary. |
| Request parsing | Mutating Rust request types use closed Serde models or canonical typed-value comparison. Unknown fields, malformed IDs, and unsupported variants refuse. |
| Pagination | Lists use opaque cursors. Clients do not use offsets and must reject a repeated cursor during traversal. |
| Object delivery | JSON carries opaque delivery IDs, never object keys, bucket names, object checksums, or signed URLs. `GET /api/assets/{id}` can redirect only an immutable, active public asset. A protected object requires body-free `POST /api/assets/{id}/delivery`, which reauthorizes the current typed delivery record, audits the decision, and returns the short-lived delivery result. |
| Error detail | Errors describe a permitted action or unavailable service without disclosing hidden account, course, Student, draft, answer, key, renderer, or object state. |

## Identity and authorization

Every authenticated request follows one order:

```text
cookie -> stored session -> AuthenticatedSession -> route resource lookup -> exact course, Student, workspace, or lease check
```

The route can use an identity in its path only after the session-derived Account
context and exact relationship constrain the lookup. A caller never supplies
an account, membership, or database object key as an authorization input.

| Scope | Authority | Normal concealed result |
| --- | --- | --- |
| Shared catalog | Authenticated approved-Instructor access plus visible lifecycle policy | Every published state appears in browse and exact lookup; the response labels `Published`, `Deprecated`, or `Archived`. |
| Course | Persisted direct `course_member` relationship | Foreign/nonmember course looks absent where disclosure is unsafe; Sysadmin alone is not course authority. |
| Assignment activity | Exact Course, Assignment, Student Record, Assignment Attempt, Issued Question, and session-derived role | Student-facing Store reads and mutations require one active Student assignment entitlement in the same authority boundary as the record lookup. A revoked Student cannot retain access through an ended Course Membership, Assignment Attempt, or Question Attempt identifier. |
| Workspace | Exact workspace owner/collaborator relationship | Student, foreign, and unshared workspaces share an absent projection. |
| Protected asset | Typed delivery record plus current persisted authorization pointer | Unknown or unauthorized delivery ID is not an object-storage lookup. |

The fuller authorization and forced account-and-relationship-scoped RLS evidence is in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security)
and [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Mounted route families

The current executable surface is intentionally small while SD1 reconstructs
the Store-backed course-delivery composition. Exact methods, request parsing,
and HTTP status behavior remain in the linked owner.

| Family | Routes | Boundary | Owner |
| --- | --- | --- | --- |
| Health | `GET /health` | Readiness only; it is not an authenticated API session probe. | [composition.rs](../crates/server/src/composition.rs) |
| Authenticated Session | `GET /api/auth/session`; `POST /api/auth/logout` | The server resolves or revokes one bounded Authenticated Session for one global Account. The browser receives an Account ID and immutable Product Role, never a credential or course authority. | [auth.rs](../crates/server/src/auth.rs) |
| Seeded Live Demo | Deployment-gated `GET/POST /api/auth/live-demo/accounts` | The selector exists only with complete disposable-demo configuration and mints the ordinary Authenticated Session for a seeded Account. It is not a product authentication provider. | [auth/live_demo.rs](../crates/server/src/auth/live_demo.rs), [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) |

## Deferred delivery families

The following product capabilities remain authoritative design targets, not
mounted routes: shared Question Catalog and lifecycle; private authoring and
QTI import; Blueprint Course and Course Instance creation; Course Roster and
Course Enrollment; assignment authoring, direct Student Accommodations, and
Student delivery; automated grading, Gradebook, Student-work inspection, and
exports; object delivery; retention; and external-tool boundaries. Their
future route composition must be Store-backed, relationship-authorized, and
tested against the disposable PostgreSQL stack before this document can list
their paths as available.

### Shared catalog and Student entitlement

The catalog is one installation-wide Instructor surface. Every authenticated
approved Instructor can browse, search, resolve, and inspect the safe published
question content referenced by any course. `Published`, `Deprecated`, and
`Archived` are all discoverable states; the response labels the state and any
retirement reason. Only `Published` (the active state) is eligible for ordinary
new assignment selection. Deprecated and Archived questions remain available
for exact historical references, evidence, provenance, and retained
assignments.

Question ID is stable within one lineage; each immutable version has exact
assignment and evidence pins. Approved Instructors can inspect version history,
lineage, and preserved improvement threads, including forks created by other
Instructors. Any approved Instructor may start a fork, but its draft remains
private to the creator until validation and publication. Star is one
vetted-Instructor-visible endorsement per Question ID; approved Instructors may see its count
and the identities of vetted Instructors who starred. Students and anonymous
callers see neither the identity list nor star state. Watch is a private
notification subscription for versions, forks, improvements, and impact
events; Students and anonymous callers see no watch state.

`QuestionChangeProposal` is the lightweight improvement command. Any vetted
Instructor submits one patch and rationale against one exact immutable base
version. The server completes publication validation and semantic/grading-
impact analysis before accepting the submission for the lineage owner's
accept/reject decision. A stale base returns a rebase-and-resubmit conflict.
Acceptance of a compatible `ModerateEdit` publishes a new immutable `QuestionVersionNumber`
under the original stable `QuestionId`, preserves canonical authorship and the
compatible CC license, records contributor credit and proposal ancestry, and
leaves every assignment and evidence `QuestionVersionReference` unchanged.
`FullFork` remains the distinct major-change path: its creator-private draft
validates before global publication with a new Question ID and exact ancestry.
`ForcedQuestionCorrection` remains the distinct Sysadmin-only emergency
replacement path below. The UI action is **Suggest an improvement**; any
GitHub analogy is documentation-only and carries no API or authorization
meaning.

Catalog evidence is version-specific. After the configured disclosure
threshold, a safe rollup may expose accepted-attempt, graded-attempt, and
correct counts plus eligible-choice selection counts for supported choice
families. Below the threshold the counts remain unavailable. The rollup never
includes raw responses, small cells, linkable cohorts, Student identities,
preview traffic, or the Instructor Student view. Course-local item analysis is
a separate exact-course projection. The evidence is keyed to the immutable
version that generated it, not to a mutable latest pointer.

`ForcedQuestionCorrection` is a Sysadmin-approved security operation. It
accepts only `security_flaw` or `critical_correctness_flaw`. It atomically
activates one authoritative mapping from the flawed version to validated
replacement content, so new selection and issuance resolve to the replacement
immediately. The old version is preserved solely as immutable historical
evidence. Replacement requires validated content and a closed, privacy-safe
impact manifest. The activated generation is handed to bounded, idempotent,
generation-fenced workers for active-binding and remediation materialization
across active Blueprint, CourseInstance, assignment, selection-pool, and
future-issuance bindings.
A deterministic compatibility check governs reissue or excuse for in-progress
work. Issued and graded evidence stays pinned to the original version;
completed work receives superseding receipts and deterministic recalculation.
There is no per-course approval step. Instructors receive audited,
course-authorized results, while Sysadmin projections contain no FERPA-bearing
course or Student records. Approval, validation, manifest, atomic advance,
reissue, excuse, superseding receipt, recalculation, and publication events are
append-only audited.

Student delivery is a separate authority path. A Student receives a question
only when the server grants an exact assignment entitlement for that
authenticated Student, active Student membership, exact course and assignment,
assignment audience and lifecycle, and current policy. Catalog visibility never
grants Student delivery. Anonymous requests receive no catalog authority and
cannot browse, search, resolve, or inspect a Question ID.

Shared question visibility does not weaken course privacy. An approved
Instructor may inspect published content used by another course, while
assignment composition, course membership, Student assignment entitlement, and
Student records remain available only through their exact course-authorized
operations.

### Instructor assignment workspace

The assignment workspace is one course-scoped resource. `GET
/api/courses/{course}/assignments/{assignment}` first checks the authenticated
direct Instructor relationship to the course and then verifies that the
assignment belongs to that exact course. A mismatched or unavailable pair has
the same concealed not-found result and returns no assignment facts. The
response carries the complete revisioned editor projection: title, ordered
fixed or pool content, Student Feedback Release Rules, Assignment activity rules, course-local teaching
settings, server-derived current state, audience, and publication readiness.

`POST /api/courses/{course}/assignments/drafts` accepts only a title and
persists an ordinary incomplete Draft with server-owned defaults. An empty
Draft is valid and reloadable; publication readiness, rather than draft
creation, requires an active deliverable position and valid policy state.

Questions owns `PUT
/api/courses/{course}/assignments/{assignment}/content`. It accepts the title
and ordered public Question-ID entries, resolves each publication under the
exact course authority, accepts only active `Published` questions for ordinary
new selection, and records the selected Question ID with its exact immutable
`QuestionVersionReference` pin. The Instructor may choose a shared question without
importing its content into the course row. A future version becomes available
only through this explicit, revision-checked update; publication and lifecycle
work never advance an assignment automatically. Policies
owns `PUT
/api/courses/{course}/assignments/{assignment}/policies`. It accepts one closed
aggregate of audience, Student Feedback Release Rules, Assignment activity rules, and course-local teaching
settings, resolves local times and group references on the server, validates
the candidate, and commits all policy-owned fields together. Both writes
require the current assignment revision in `If-Match`, advance one shared
aggregate revision, and return the complete authoritative editor projection
with its new `ETag`.

Structural content changes are refused with the current typed
`issuedLearnerWork`
conflict once immutable Student work has been issued. Ordinary stale-revision
conflicts remain retryable; neither conflict path partially changes the
assignment. The browser therefore keeps its local draft and offers a visible
reload/recovery action instead of silently overwriting another page.

`GET /api/courses/{course}/assignments/{assignment}/student-view` is a
non-mutating, `no-store` Instructor read. It uses the exact same course and
assignment authority, projects the current answer-free Student landing with
course-wide base delivery facts, and does not create enrollment, run, attempt,
submission, receipt, grade, or preview state. The shared landing presentation
is also used by ordinary Student overview; only an exact active Student
assignment entitlement creates work and supplies the server-owned gradebook
evidence.

### Instructor grading operations

The grading-operations surface is an assignment-local Instructor recovery
projection. `GET
/api/courses/{course}/assignments/{assignment}/grading-operations` resolves
the direct Instructor relationship from the session, verifies the exact course
and assignment pair, and returns a bounded `no-store` page of metadata-only
rows. **Target WN1-C6-GO1 and QM-GRADING-OPS:** `group_by=question|student`; `cursor`
and `page_size` are opaque and bounded. Rows expose safe operation state, reason, revision, next
action, grouping label, affected Student count, and trust generation. They
never expose Student responses, answer keys, feedback internals, private
source, or score values.

**Current pre-WN1:** this route currently uses `groupBy`, `student`, and `pageSize`.
The C6-GO1/QM-GRADING-OPS closure moves its parser, generated DTO, browser client, and strict
decoder together to the target PLE spelling.

`POST
/api/courses/{course}/assignments/{assignment}/grading-operations/{operation}/retry`
retries one visible operation. It requires one strong operation revision in
`If-Match`, one UUID `Idempotency-Key`, and an exactly empty body. `POST
/api/courses/{course}/assignments/{assignment}/grading-operations/recalculate`
requests assignment-wide recalculation with one strong assignment revision in
`If-Match`, one UUID `Idempotency-Key`, and the same empty-body contract. Both
commands return a `no-store` action receipt with a strong `ETag`; authorization
and exact course/assignment ownership are checked from the authenticated
session before the controlled operation reference or action is interpreted.

The browser receives only the route-bound metadata and receipt projection. The
server recovers accepted private input through the sealed worker capability; a
retry never asks the Student to submit the response again.

A grading-semantic correction is an impact/recalculation operation, not an
ordinary content edit. Its operation records the affected Question ID,
version-specific assignment and evidence pins, evaluates the permitted impact,
and uses the controlled recalculation path while preserving prior immutable
attempt and receipt evidence. A correction cannot silently change a Student's
historical answer or an assignment's pinned version.

### Identity composition and activation

The current route table is assembled by
[composition.rs](../crates/server/src/composition.rs).
It injects account and session Stores, the invitation issuer, passwordless
email delivery and rate-limit issuer, and optional WebAuthn configuration.
The browser uses this same production-shaped graph through the disposable HTTPS
gateway; no browser build or browser command selects a local credential form or
legacy login route.

`production_router_from_env` at
[composition.rs](../crates/server/src/composition.rs)
constructs persistent dependencies and composes the provider-free PLE
passwordless/account/session graph. Its eight-hour `FirstPartyHttps` policy
makes account, email-binding, and authenticated-session cookies Secure, HttpOnly, and
first-party `SameSite=Lax`; its explicit `ReviewNotRequired` gate leaves
institutional review integration optional. It mounts only the production
account and session route families listed above.

Production is a same-origin first-party application. It does not support an
embedded `SameSite=None` session mode. A future LTI integration is a separate
security design and must establish its own authenticated launch and CSRF
protocol; it must not relax these cookie or origin rules by convention.

## Browser request, method, and frame boundaries

The router owns one exhaustive typed route-method inventory. A registered
method is explicit; an unregistered method fails closed before a handler can
interpret a body. State-changing routes use their declared non-GET method and
the request's canonical same-origin protection. A route must never create a
launch, grade, mutation, or signed protected delivery from an incidental GET.

The normal unsafe-request boundary requires the canonical HTTPS `Host` and an
exact same-origin `Origin`, with duplicate session cookies refused. The sole
intentional narrow exception is the sandboxed external-activity POST: a
browser iframe may send `Origin: null` only when it presents both its ordinary
session and the distinct HttpOnly launch cookie, and the server verifies the
AEAD-bound launch context, attempt, authenticated account, and lease before proxying a
bounded activity. `Origin: null` is not a general CSRF bypass and is rejected
by every other unsafe route.

The browser decoder treats all network JSON as hostile: it bounds bytes,
checks content type, constructs null-prototype records, refuses duplicate or
unknown fields and closed-discriminant misses, and checks requested/returned
relationships before exposing a typed value. The server gives mutating
Serde models the same closed-world posture. Browser types and Wasm are
convenience projections, never authorization evidence.

Browser acceptance and developer browser entry use the visible PLE account page.
The deployment-gated seeded selector and ordinary passkey remain within this
account/session graph; no browser build or command selects an alternate
credential form or login route. Any remaining service-only harness must be
treated as browser-free infrastructure evidence and cannot become a browser
authentication path.

The established SMTP adapter is constructed only when the complete SMTP
settings and `PLE_INVITATION_TOKEN_SECRET_FILE` are configured. It implements
both invitation and passwordless email delivery. Without them, the router uses
unavailable delivery/issuer capabilities and email start fails closed; a
server-secret-only deployment can still issue an Instructor copy link for an
invitation. PLE has no mail-server container or deliverability subsystem. A
live external-provider account and its acceptance evidence remain WP-RC8 work.
Optional OIDC/SAML linking is
an integration path to an existing PLE account, not the primary identity
system.

## Mutations and replay safety

The mutation rule is intentional: a route accepts only the data that the
server cannot derive from authenticated state and the existing record.

| Mechanism | Applies to | Contract |
| --- | --- | --- |
| Strong ETag plus `If-Match` | Workspace saves/deletes, publication review and conversion, focused assignment content/policy saves, course appearance | Read returns a strong revision. A write must send that exact revision; stale state conflicts without mutation. Authorization happens before precondition evaluation when that prevents an existence oracle. |
| `Idempotency-Key` | Future Student submission | The same key and same response will represent one grading request. A retry will return the committed receipt without grading twice. This is a deferred Store-backed delivery requirement, not a mounted route. |
| Server-generated identity | Runs, attempts, publications, export jobs, upload candidates, object deliveries | Browser paths name an existing opaque record but browser bodies do not mint durable identities or choose storage paths. |
| Bytes-first promotion | Candidate banners, QTI/flat publication, exports | Objects may be written before the database transaction, but an unbound candidate is not visible content. The database commits the authoritative public/delivery pointer atomically. |

ETags are resource revisions, not general-purpose cache validators. An attempt
submission uses its attempt ID and idempotency key rather than a browser-owned
question/version/seed tuple.

## Public and private payloads

| Browser may receive | Browser must not receive |
| --- | --- |
| Prompt blocks, response controls, accessible asset descriptions, safe course appearance, disclosed feedback, opaque IDs, public catalog metadata, and policy-permitted aggregate analysis | Answer keys, expected values, hidden correct choices, private rubrics or weights, grading code, provider credentials, renderer fields, PG/QTI source, object keys, bucket names, signed URLs in JSON, authenticated-session context, database provenance, or raw provider results |

An instructor's private author preview is a distinct, authorized exception for
display-ready correct-response teaching material. It is not a Student route and
does not turn answer-bearing source or an `AnswerKey` into a browser API type.

For native assessment payload detail, including attempt-specific rendered IDs,
presentation digests, partial-credit results, and WeBWorK replay, use
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). For private
source/object/renderer rules, use [SECURITY_MODEL.md](SECURITY_MODEL.md) and
[OBJECT_STORAGE.md](OBJECT_STORAGE.md).

## Decoder and error contract

The HTTP client treats a successful status as untrusted until it verifies:

1. expected JSON content type and a bounded response body;
2. exact closed response shape through a route-specific decoder;
3. bounded primitive values and known discriminants;
4. returned identity matches the requested route identity; and
5. related records in a composed screen agree on the authenticated account, exact course/Student or
   workspace relationship, run, attempt, version, and seed where relevant.

Browser acceptance uses the production `ApiClient` over the disposable HTTPS
gateway. Narrow browser-free unit tests check serialized decoder and transport
fixtures without creating a second browser runtime. The implementation is in
[http_client.ts](../src/api/http_client.ts),
[request.ts](../src/api/http_client/request.ts),
[response.ts](../src/api/http_client/response.ts), with
focused decoder and serialization tests under `tests/`.

Server error status is part of the contract, but a client must not use a
difference between `404`, `403`, malformed input, or a conflict as proof that a
foreign account's course, Student, workspace, or object exists. Route modules
deliberately choose concealed not-found responses where that distinction would
leak ownership.

## Versioning and change control

PLE currently uses stable `/api/...` paths, not a global `/v1` URL prefix.
Versioning is therefore explicit at the boundary that evolves:

- immutable published questions, their protected exact evidence, and deterministic seeds identify
  delivered educational content. A stable Question ID names one lineage,
  immutable versions carry exact evidence, and assignments and attempts pin
  the version they received;
- semantic change classes determine whether an original-lineage steward may
  publish a same-ID version or whether a major objective, task, or response-
  family change requires a private fork draft and a new Question ID.
  Transport-size limits protect request handling; semantic classification
  determines compatibility;
- strong numeric resource revisions protect concurrent workspace, assignment,
  course-appearance, QTI-review, and conversion updates;
- closed Rust/TypeScript discriminants version a DTO by adding a reviewed,
  decoder-supported variant rather than silently accepting unknown values;
- content media types, payload schema versions, checksums, and upcasters belong
  to the persistence/content owner, described in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); and
- the assessment response cutover must use the explicit contract version and
  migration in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md),
  not an unannounced route behavior change.

Changing a route requires updating its Rust owner, browser `ApiClient` method,
strict decoder, focused browser-free serialization/decoder tests, this map when the
route-level rule changes, and [CONTRACTS.md](CONTRACTS.md) when module
ownership changes. Keep a compatibility adapter only when an active consumer
needs it, name its removal condition, and do not make browser-supplied copies
of server-owned authority fields authoritative during a migration.
