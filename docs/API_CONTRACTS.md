# API contract map

This document is the durable route-level map for PLE's same-origin HTTP API.
It tells a client or server contributor which boundary owns a route, what
identity the route trusts, and where to find the exact request and response
shape. It is not an OpenAPI replacement and deliberately does not duplicate
every generated Rust or TypeScript field.

[CONTRACTS.md](CONTRACTS.md) remains the module ownership register.
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns the detailed
authorization decision sequence and authority sources.
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) owns the learner
render-and-submit boundary and its planned cutover. [SECURITY_MODEL.md](SECURITY_MODEL.md)
owns the threat model and storage rules. This document connects those sources
at the HTTP boundary.

## Status and authority

The route registrations in [crates/server/src/composition/router.rs](../crates/server/src/composition/router.rs)
are the executable authority for what is mounted. Route modules own their
extractors, limits, authorization, and HTTP status mapping. The corresponding
browser method in [src/api/client.ts](../src/api/client.ts) and strict decoder
under [src/api/decoders.ts](../src/api/decoders.ts) own the browser-facing
shape. Rust types under `crates/question_model/` and generated files under
`generated/api/` own shared value definitions.

Route paths below describe the current implementation unless a row says
**target**. The planned compact learner response in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) is not silently
substituted for the current tagged `StudentResponse` route contract.

## Common protocol rules

| Concern | Durable contract |
| --- | --- |
| Origin | Browser API requests use a relative same-origin path, `credentials: "same-origin"`, and `cache: "no-store"`. [src/api/http_client/request.ts](../src/api/http_client/request.ts) rejects an external base path. |
| Sessions | The server resolves a hashed opaque session cookie and derives `TenantContext` from the stored session. Request paths, query parameters, headers, and JSON never select a tenant. [crates/server/src/auth.rs](../crates/server/src/auth.rs) owns this boundary. |
| Caching | Private JSON routes return `Cache-Control: no-store`, including errors where a route applies its response middleware. The browser verifies this for sensitive appearance traffic. Immutable public asset redirects are the explicit exception. |
| JSON | Browser JSON is decoded from `unknown` with bounded bodies, content-type checks, closed discriminants, bounded numeric values, and relationship checks. A TypeScript type assertion is not a decoder. |
| Request parsing | Mutating Rust request types use closed Serde models or canonical typed-value comparison. Unknown fields, malformed IDs, and unsupported variants refuse. |
| Pagination | Lists use opaque cursors. Clients do not use offsets and must reject a repeated cursor during traversal. |
| Object delivery | JSON carries opaque delivery IDs, never object keys, bucket names, object checksums, or signed URLs. `GET /api/assets/{id}` can redirect only an immutable, active public asset. A protected object requires body-free `POST /api/assets/{id}/delivery`, which reauthorizes the current typed delivery record, audits the decision, and returns the short-lived delivery result. |
| Error detail | Errors describe a permitted action or unavailable service, not hidden tenant, course, draft, answer, key, renderer, or object state. |

## Identity and authorization

Every authenticated request follows one order:

```text
cookie -> stored session -> TenantContext -> route resource lookup -> role or relationship check
```

The route can use an identity in its path only after the session-derived tenant
context constrains the lookup. A caller never supplies `tenantId`, `userId`,
or a database object key as an authorization input.

| Scope | Authority | Normal concealed result |
| --- | --- | --- |
| Shared catalog | Visibility and lifecycle policy | Hidden or archived versions do not appear in browse results. |
| Course | Persisted direct `course_member` relationship | Foreign/nonmember course looks absent where disclosure is unsafe; Sysadmin alone is not course authority. |
| Assignment/run/attempt | Owning course, enrollment, and session-derived learner or instructor role | Learner-facing Store reads and mutations take the actor and require an active `Student` course membership in the same authority boundary as the record lookup. A revoked learner cannot retain access through an old enrollment, run, or attempt identifier. |
| Workspace | Tenant-owned author/collaborator ACL | Student, foreign, and unshared workspaces share an absent projection. |
| Protected asset | Typed delivery record plus current persisted authorization pointer | Unknown or unauthorized delivery ID is not an object-storage lookup. |

The fuller authorization and RLS evidence is in
[DATABASE_TENANCY.md](DATABASE_TENANCY.md) and [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Route families

The map names route groups instead of copying each DTO. Exact method, path,
body limit, status response, and Rust type remain in the linked owner.

| Family | Routes | Identity and payload boundary | Owner |
| --- | --- | --- | --- |
| Health | `GET /health` | Readiness only; it is not an authenticated API session probe. | [crates/server/src/composition/router.rs](../crates/server/src/composition/router.rs) |
| Auth/session | production `POST /api/auth/passwordless/email/start` and `/complete`; account-email start/complete; passkey registration/authentication start/complete; passkey list/revoke; account course list/select; `GET /api/auth/session`; `POST /api/auth/logout`; local development additionally has legacy `POST /api/auth/login` | PLE-owned accounts use email as the canonical sign-in path and optional passkeys as shortcuts. Email and passkey completion issue a bounded HttpOnly `ple_account_session`; invitation claim or course selection then derives a tenant-scoped `ple_session` from an authorized relationship. Production composes this provider-free graph with an eight-hour `FirstPartyHttps` policy and does not mount the legacy route. | [crates/server/src/auth.rs](../crates/server/src/auth.rs), [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) |
| Catalog | `GET /api/problems`, `/search`, `/by-id/{reference}`, `/by-id/{reference}/detail`, `GET /api/taxonomy` | Browse, exact-ID lookup, and detail are browser-safe Question-ID projections. Source, private response/grading material, credentials, internal publication pairs, and student records are excluded. | [crates/server/src/catalog/routes.rs](../crates/server/src/catalog/routes.rs) |
| Catalog lifecycle | `POST /api/problems/{workspace}/publish`, `POST /api/problems/by-id/{reference}/deprecate`, `/archive` | Publication mints one new immutable question with a fresh Question ID and hidden exact publication evidence. Lifecycle actions resolve the Question ID under authorization and retain protected historical evidence. | [crates/server/src/catalog/routes.rs](../crates/server/src/catalog/routes.rs) |
| Course and assignment | `GET/POST /api/courses`, `GET/POST /api/courses/{course}/assignments`, `GET /api/courses/{course}`, `/gradebook`, `GET /api/assignments/{assignment}`, `PUT /api/courses/{course}/assignments/{assignment}`, `POST .../items`, `DELETE .../items/{item}`, `PUT .../items/{item}/question` | Course comes from path plus membership. Create and focused item mutations select Question IDs; ordinary update preserves the selected items while changing title, order, points, delivery, scoring, or policies. Item mutations use assignment `If-Match`; a revision conflict preserves the caller's input for reload and retry. | [crates/server/src/course/routing.rs](../crates/server/src/course/routing.rs) |
| Course roster | `GET /api/courses/{course}/roster`; invitation create/revoke/redeem; enrollment-policy replace; member revoke; roster-import preview/commit; `POST /api/courses/{course}/assignments/{assignment}/grade-export.csv` | Direct Instructors own the workflow. A Sysadmin may use only the closed list/invite/policy/revoke/import support operations; the Store records actor/course/action/time for each support disclosure or change. Invitation claim resolves the authenticated PLE account and atomically reconciles membership, assignment enrollments, and empty summaries. Grade export remains direct-Instructor-only, synchronous, no-store, and excludes global account IDs. | [crates/server/src/course/roster.rs](../crates/server/src/course/roster.rs), [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) |
| Course appearance | `GET/PUT /api/courses/{course}/appearance`, `POST /api/courses/{course}/appearance/banner-candidates` | Safe theme/banner projection; candidate upload uses raw bounded raster bytes and returns an opaque candidate receipt. Current appearance update uses strong `If-Match`. | [crates/server/src/course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Workspace | `GET /api/workspaces`, `GET/PUT/DELETE /api/workspaces/{workspace}`, `POST /publication-validation`, `GET /publication-diff` | Private authoring draft surface. Mutation and publication review use the strong workspace revision ETag. | [crates/server/src/workspace.rs](../crates/server/src/workspace.rs) |
| Private author presentation | `GET /api/workspaces/{workspace}/author-preview?seed=...` | Instructor-only teaching display; it may return rendered correct-response material but never an answer key or reusable grading contract. | [crates/server/src/author_preview.rs](../crates/server/src/author_preview.rs) |
| Flat authoring | `GET/PUT /api/workspaces/{workspace}/flat-question`, `POST /api/problems/{workspace}/flat-question-publish` | The narrow author route accepts answer-bearing source only after author authorization. Generic workspace and learner routes remain answer-free. | [crates/server/src/flat_question_publication.rs](../crates/server/src/flat_question_publication.rs) |
| QTI profile | `GET/PUT /api/workspaces/{workspace}/qti-imports/{import}`, `POST /items/{item}/convert-flat`, `POST /api/problems/{workspace}/qti-publish` | Archive bytes and conversion/provenance stay private. Browser reports, acknowledgements, and converted draft handoff remain answer-free. | [crates/server/src/qti_profile_import.rs](../crates/server/src/qti_profile_import.rs) |
| Runs and attempts | `POST /api/runs`, `GET /api/runs/{run}`, `/summary`, `/attempts`, `GET /api/attempts/{attempt}`, `/question`, `POST /prefetch-next`, `POST /api/submissions/{attempt}` | An authenticated attempt binds learner, run, assignment position, immutable version, seed, timing, lifecycle, and backend. The current submission response is typed; the planned compact contract is owned separately by the assessment payload design. | [crates/server/src/run/routes.rs](../crates/server/src/run/routes.rs) |
| Instructor grading | `GET/PUT /api/attempts/{attempt}/manual-grade`, `POST /feedback-release`, `GET /api/grading/summaries/{enrollment}` | Instructor actions use the course relationship resolved from the attempt/enrollment. Learners receive only policy-projected feedback and score. | [crates/server/src/run/routes.rs](../crates/server/src/run/routes.rs) |
| External tool | `POST /api/attempts/{attempt}/external-tool/launch`, inert `GET /api/attempts/{attempt}/external-tool/launch`, and bounded `/launch/*` activity children | Only the POST creates a server-held, one-attempt launch session. The GET returns an inert same-origin shell; it cannot create, renew, or reveal a provider launch. Activity requires the session-bound launch proof. Provider material, replay state, field names, credentials, and raw provider outcomes remain server-held. | [crates/server/src/run/external_tool.rs](../crates/server/src/run/external_tool.rs) |
| Item analysis | `GET /api/courses/{course}/assignments/{assignment}/item-analysis` | Instructor-only aggregate projection. It excludes learner, attempt, raw response, answer, and object identity. | [crates/server/src/item_analysis.rs](../crates/server/src/item_analysis.rs) |
| Export | `POST /api/assignments/{assignment}/exports`, `GET /api/exports/{export}` | Creation requires an exactly empty body. The server freezes the assignment and delivery plan; status returns safe identifiers and progress, not object keys or manifests. | [crates/server/src/export.rs](../crates/server/src/export.rs) |
| Retention | `GET /api/courses/{course}/retention`, `POST /end`, `/archive`, `/delete`, `PATCH /extend` | Instructor-only tenant-owned record-retention control. It cannot expose another tenant's learner data or worker lease state. | [crates/server/src/retention.rs](../crates/server/src/retention.rs) |
| Assets | `GET /api/assets/{id}`, `POST /api/assets/{id}/delivery` | GET is deliberately public-only: it redirects an active immutable public asset and never signs private content. POST is deliberately protected: it rechecks the session-derived authorization pointer, records the authorization decision, and then creates the bounded private delivery. Pending public assets are unavailable from both paths until the dedicated publisher has verified and activated them. | [crates/server/src/asset.rs](../crates/server/src/asset.rs) |
| Browser validation fallback | `POST /api/validation/response-format`, `/timer`, `/assignment-capabilities` | Authenticated, key-free pure validation only. It never grades, authorizes publication, establishes server time, or replaces server grading. | [crates/server/src/validation.rs](../crates/server/src/validation.rs) |

### Identity composition and activation

The route table describes the shared application router assembled by
[`crates/server/src/composition/router.rs`](../crates/server/src/composition/router.rs).
It injects account and session Stores, the invitation issuer, passwordless
email delivery and rate-limit issuer, and optional WebAuthn
configuration. Local development additionally layers its `IdentityProvider`
and legacy login route over that graph. This makes the account, passwordless,
passkey, invitation, and roster route families independently testable without
granting their route modules process-local authority.

`production_router_from_env` at
[`crates/server/src/composition.rs`](../crates/server/src/composition.rs)
constructs persistent dependencies and composes the provider-free PLE
passwordless/account/session graph. Its eight-hour `FirstPartyHttps` policy
makes account, email-binding, and tenant-session cookies Secure, HttpOnly, and
first-party `SameSite=Lax`; its explicit `ReviewNotRequired` gate leaves
institutional review integration optional. It does not read
`PLE_AUTH_PROVIDER`, `PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH`, or
`PLE_LOCAL_AUTH_FILE`, and it does not mount `/api/auth/login`.

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
AEAD-bound launch context, attempt, actor, and lease before proxying a
bounded activity. `Origin: null` is not a general CSRF bypass and is rejected
by every other unsafe route.

The browser decoder treats all network JSON as hostile: it bounds bytes,
checks content type, constructs null-prototype records, refuses duplicate or
unknown fields and closed-discriminant misses, and checks requested/returned
relationships before exposing a typed value. The server gives mutating
Serde models the same closed-world posture. Browser types, mocks, and WASM
are convenience projections, never authorization evidence.

The separately callable local-development launcher requires
`PLE_AUTH_PROVIDER=local-file`, the explicit development flag, and an
operator-owned identity file. That provider serves legacy `POST /api/auth/login`
and issues a tenant-scoped `ple_session`; it does not bootstrap a PLE account.
Only a binary explicitly built with the `local-development-auth` feature may
select that launcher, and it does so only when
`PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH=1`.

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
| Strong ETag plus `If-Match` | Workspace saves/deletes, publication review and conversion, assignment changes, course appearance | Read returns a strong revision. A write must send that exact revision; stale state conflicts without mutation. Authorization happens before precondition evaluation when that prevents an existence oracle. |
| `Idempotency-Key` | Learner submission | The same key and same response represent one grading request. A retry returns the committed receipt without grading twice. The exact owner is [crates/server/src/run/routes.rs](../crates/server/src/run/routes.rs). |
| Server-generated identity | Runs, attempts, publications, export jobs, upload candidates, object deliveries | Browser paths name an existing opaque record but browser bodies do not mint durable identities or choose storage paths. |
| Bytes-first promotion | Candidate banners, QTI/flat publication, exports | Objects may be written before the database transaction, but an unbound candidate is not visible content. The database commits the authoritative public/delivery pointer atomically. |

ETags are resource revisions, not general-purpose cache validators. An attempt
submission uses its attempt ID and idempotency key rather than a browser-owned
question/version/seed tuple.

## Public and private payloads

| Browser may receive | Browser must not receive |
| --- | --- |
| Prompt blocks, response controls, accessible asset descriptions, safe course appearance, disclosed feedback, opaque IDs, public catalog metadata, and policy-permitted aggregate analysis | Answer keys, expected values, hidden correct choices, private rubrics or weights, grading code, provider credentials, renderer fields, PG/QTI source, object keys, bucket names, signed URLs in JSON, tenant context, database provenance, or raw provider results |

An instructor's private author preview is a distinct, authorized exception for
display-ready correct-response teaching material. It is not a learner route and
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
5. related records in a composed screen agree on tenant, course, run, attempt,
   version, and seed where relevant.

The mock API implements the same `ApiClient` surface and checks serialized
fixtures, so browser tests do not create a second unchecked protocol. The
implementation is in [src/api/http_client.ts](../src/api/http_client.ts),
[src/api/http_client/request.ts](../src/api/http_client/request.ts),
[src/api/http_client/response.ts](../src/api/http_client/response.ts), and
[src/api/mock/handlers.ts](../src/api/mock/handlers.ts).

Server error status is part of the contract, but a client must not use a
difference between `404`, `403`, malformed input, or a conflict as proof that
another tenant's resource exists. Route modules deliberately choose concealed
not-found responses where that distinction would leak ownership.

## Versioning and change control

PLE currently uses stable `/api/...` paths, not a global `/v1` URL prefix.
Versioning is therefore explicit at the boundary that evolves:

- immutable published questions, their protected exact evidence, and deterministic seeds identify
  delivered educational content;
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
strict decoder, mock handler/fixture, focused behavior test, this map when the
route-level rule changes, and [CONTRACTS.md](CONTRACTS.md) when module
ownership changes. Keep a compatibility adapter only when an active consumer
needs it, name its removal condition, and do not make browser-supplied copies
of server-owned authority fields authoritative during a migration.
