# Authorization contracts

This document records how PLE decides who may perform an operation. It is the
durable authorization companion to [SECURITY_MODEL.md](SECURITY_MODEL.md),
[DATABASE_TENANCY.md](DATABASE_TENANCY.md), and [CONTRACTS.md](CONTRACTS.md).
The [active implementation plan](active_plans/implementation_plan.md) remains
the source of truth for scope and release acceptance.

Authorization is deliberately distinct from authentication, input validity,
and grading. A well-formed request, visible identifier, or role label is not
authority by itself.

## Decision sequence

Every protected request follows this order:

```text
opaque session cookie
        |
        v
resolve shared session and PLE-established subject
        |
        v
derive TenantContext on the server
        |
        v
authorize actor for the exact resource and action
        |
        v
check request shape, revision, lifecycle, and domain rules
        |
        v
perform Store work under tenant RLS and least-privilege capability
```

The server resolves the session before it constructs `TenantContext` or opens a
tenant-owned Store operation. A tenant ID in a path, header, cookie extension,
or JSON body is never authority. `TenantContext` has no `Default`; its only
production constructor is the authenticated-session boundary in
`crates/learning-data-access/src/rls.rs`.

Authorization also precedes conditionals. Course and workspace authorization
comes before `If-Match`, body parsing where possible, publication capability
checks, and expensive external work. A malformed revision or request body must
not become a membership, resource-existence, or tenant-discovery oracle.

The closed human-role vocabulary is defined in
[USER_ROLES.md](USER_ROLES.md). `Sysadmin` is platform authority, not ambient
course authority. Reading, grading, exporting, or otherwise using FERPA
records as teaching data still requires direct Instructor membership in the
exact course. Sysadmin-only retention transitions are the narrow payload-free
exception documented in [RETENTION_POLICY.md](RETENTION_POLICY.md). The other
explicit exception is audited roster support: its closed operations expose or
change only the roster state needed to help an Instructor and do not grant
grade, response, run, export, item-analysis, or ordinary course authority.

## Separate decisions

| Control | Question answered | It does not prove |
| --- | --- | --- |
| Authentication | Which PLE account ceremony established this session subject? | Permission for a particular course or record |
| Tenant derivation | Which tenant owns the protected operation? | Course, workspace, or asset permission |
| Authorization | May this actor perform this action on this resource now? | That content is well-formed or pedagogically valid |
| Structural validation | Does input satisfy the closed API and domain shape? | That the actor owns the target or response is correct |
| Lifecycle and revision | Is this state transition allowed and current? | Membership or grade correctness |
| Grading | Is an authorized response correct and worth these points? | Browser authority or a reusable permission |

For example, a learner may be authorized to submit only their own active
attempt, while the server still rejects an invalid response shape, stale
presentation binding, expired timing state, changed idempotency replay, or
unsupported question capability.

## Session and tenant authority

PLE uses one opaque `__Host-` first-party session credential. The server stores only its
SHA-256 hash with database-authoritative expiry and revocation state. The raw
credential is HttpOnly and does not enter browser storage, logs, DTOs, or
PostgreSQL. Missing, malformed, expired, revoked, and unknown credentials have
the same unauthenticated result.

Production email/passwordless and optional passkey ceremonies establish the
PLE account; no production password verifier or generic identity provider
establishes this session. A `SessionSubject` carries tenant, authenticated
`UserId`, display name, and coarse roles. The authenticated user is not an
`EnrollmentId` or `StudentId`; run and record access binds the user to a
persisted enrollment rather than equating identifiers.

The database receives only the server-derived tenant through transaction-local
state. `PostgresStore` starts a transaction, uses `SET LOCAL ROLE ple_app`, and
sets `ple.tenant_id` with `set_config(..., true)`. Pool reuse cannot carry a
previous request's authority. The normal application pool cannot use this
mechanism to obtain grader access.

## Authority sources

| Resource | Authority source | Important limit |
| --- | --- | --- |
| Tenant-owned record | Session-derived tenant plus forced RLS | Tenant match alone does not grant user access |
| Course | Direct `course_member` row | Neither a coarse Instructor role nor Sysadmin status grants every course; audited Sysadmin roster support is a separate narrow capability |
| Assignment write | Exact direct course Instructor | References must also be visible, published, and lifecycle-valid |
| Learner run, attempt, summary, and submission | Enrollment owned by the authenticated user **and active Student membership** | A removed learner cannot retain read/write access; course staff cannot submit as the learner |
| Workspace draft | Persisted workspace owner/collaborator ACL | Student routes do not construct authoring repositories |
| Catalog publication | Eligible role, ownership, and review policy | Browser scope, problem ID, and adapter declaration are not authority |
| Asset delivery | Immutable registry scope plus session, tenant, and user grant where required | Logical `AssetId` never grants object-key access |
| Worker job | Tenant context, opaque lease, family, generation, and broker grant | Process name or queue row alone is not permission |
| Grading material | Separately injected restricted grader capability | Normal Store, browser, and Wasm never receive it |

The authorization result is action-specific. Viewing a course does not imply
permission to edit assignments, read every learner record, publish catalog
content, request an export, promote an upload, or obtain a signed URL.

## Course and educational records

Course membership is an explicit tenant-owned relationship between an
authenticated `UserId` and a course. Its durable membership roles are
`Student` and `Instructor`. `Sysadmin` is deliberately not a course role and
does not replace direct Instructor membership for general FERPA access.

Nonmembers receive the same not-found response as an absent or foreign course.
This concealment applies to course-scoped assignments and learner records when
revealing existence would disclose protected educational information. Students
may list and work assignments in their own courses, but cannot create or alter
them. Direct course instructors may mutate course definitions only after exact
course authorization succeeds.

The same boundary applies to gradebook and run history. Learner-scoped Store
methods recheck both enrollment ownership and an active `Student` membership
inside the Store transaction, so a route authorization check cannot outlive a
concurrent roster revocation. Direct course instructors may read permitted,
explicitly instructor-scoped historical projections, but a non-owner does not
gain learner submission authority. Archive and deletion state adds a second
fence: retained definitions can remain visible to authorized instructors while
learner records, exports, external-tool records, and student-record assets
remain closed.

## Workspace and author preview

Unpublished drafts are tenant-owned workspace records, not shared catalog
content. Editing, saving, deletion, collaborator grants, QTI upload/review,
conversion, publication, and explicit author preview use the stored owner or
collaborator binding and the exact strong draft revision where revisioned.

Author preview is an instructor action, not a student rendering shortcut. The
server resolves the same workspace ACL before it reads the stored draft,
requires the exact saved revision, returns `no-store`, and gives students,
foreign tenants, and unshared workspaces the same absent result. The projection
remains answer-free except for a reviewed server-side presentation; it never
serializes an answer key, private rubric, source key, provider credential, or
published identity.

## Catalog and publication

Published problem versions are shared, immutable content. Drafts, publication
tenant grants, and course assignments are tenant-owned. A course cannot create
shared publication authority merely by referencing a version.

Catalog routes first resolve session and tenant. Context-free Store methods
expose only public material; institution-visible metadata and payloads require
forced RLS and an exact tenant/problem/version grant. The browser can request a
workspace and desired scope, but cannot mint a `ProblemId`, assert an adapter
capability, or select a private source.

The server loads the authorized draft, obtains capabilities from the trusted
adapter registry, and commits immutable payload, visibility grant, and draft
transition together. Public publication requires Instructor or Sysadmin
authority and any configured review gate. Institution publication permits an
Instructor or Sysadmin. Post-publication lifecycle transitions
also require eligible authority and author ownership. Published identity, scope,
payload, capabilities, metadata, authorship, and lineage are immutable to the
application role.

## Attempt and grading

An issued attempt binds the learner's tenant, enrollment, course assignment,
published version, seed, timing state, and grading backend. The authenticated
attempt is the primary learner submission authority; a question ID, browser
`kind`, choice label, seed, or response checksum is not.

The current learner route accepts a tagged `StudentResponse`, including its
browser-supplied `kind`, after the attempt route binding. The server derives
the expected response family from that attempt and rejects a shape mismatch;
the submitted tag is not authority. The accepted payload cutover will send
only response evidence the server cannot derive, plus the idempotency key and
presentation-consistency values. The server reauthorizes ownership, rechecks
timing and response shape, then loads answer-bearing material only through the
selected private grader. Partial credit, correctness, feedback policy, and
score persistence are server decisions. See
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

CRC16 rendered-item IDs and a descriptor digest belong to that accepted target
wire contract; they are not required by the current tagged-response route.
When deployed, they will detect stale or mismatched render state. They are
integrity diagnostics, not authentication, authorization, transport security,
or grading authority. A valid CRC will not allow a different learner to
submit.

The dedicated QTI and flat-question grader capability is injected through the
restricted `PostgresGraderStore`/grader-pool boundary. It uses a separate
least-privilege database login and tenant-scoped transactions. The ordinary
Store, API pool, object store, client contract, mock payloads, and Wasm closure
do not receive answer tables or a way to grade a response.

## Asset delivery

Browser markup refers to a logical `AssetId`, never a bucket, physical object
key, source package, or signed URL. `/api/assets/{id}` resolves the ID through
the database-authoritative immutable delivery registry.

| Asset class | Authorization and delivery | Deliberate refusal |
| --- | --- | --- |
| Public catalog asset | Immutable CDN redirect; no authentication or signing call | Cannot expose a private source or student record |
| Institution content | Authenticated tenant and exact protected registry binding | Catalog lookup cannot bypass visibility/RLS |
| Student record | Authenticated tenant, authorized-user grant, and retention fence | Missing and unauthorized both return not found |
| Current course banner | Current persisted pointer and authorized course access | Candidate and superseded banners are not deliverable |
| Source, cache, temp-processing | Never direct delivery targets | Typed object storage refuses to sign them |

Protected delivery writes an audit event before requesting a short-lived signed
URL. It sends the redirect in headers with `no-store`; signed URLs never enter
JSON, markup, browser storage, logs, or database records. A public asset's
immutable cache policy does not apply to a protected object.

## Private providers and workers

WeBWorK, iMathAS, QTI runtime, and other private render/grading providers are
server-side dependencies. The browser receives a safe rendered projection or
same-origin launch handle, not upstream credentials, renderer cookies, source
paths, answer mappings, provider handles, launch tokens, result tokens, or
private grader payloads.

Provider configuration and secrets are deployment-owned. A provider outage is
question-local and fails that operation closed; it is not authority to fall
back to a browser checker or unrelated backend. Optional providers register
only with complete validated configuration and their least-privilege capability.
QTI runtime also requires its separate grader database URL; disabled QTI has no
dispatch capability.

Workers have no browser session authority and are not HTTP targets. A worker
acts only through a tenant-bound claim with an opaque current lease token, the
expected job family, a supported handler/committer pair, and generation fences.
PostgreSQL broker grants and RLS restrict job types; stale or foreign lease
completion fails. Queue payloads carry bounded IDs and generations, never
names, raw responses, answer keys, grades, or object URLs.

An external provider `POST` is separately fenced: the Store writes a marker
for the exact active activity lease before dispatch and clears it only after a
valid result. Any ambiguous outcome remains fenced and blocks reclaim,
relaunch, grading, and finalization. A new lease or browser retry is never
authority to repeat an indeterminate upstream effect.

## Database enforcement

Authorization has application and database layers:

- Server routes establish actor authority and concealment behavior.
- Store methods require explicit `TenantContext` and bind actor, course,
  enrollment, active membership, workspace, or delivery identities as
  appropriate; learner-scoped reads are Store authority, not route convention.
- PostgreSQL enables and forces RLS on tenant-owned tables. Policies compare
  the row tenant with `ple_current_tenant()` and add resource predicates.
- Roles are narrow, `NOINHERIT`, `NOSUPERUSER`, and `NOBYPASSRLS`; the
  application identity is not a table-owner or superuser bypass.
- Broker and security-definer functions expose only narrowly granted actions.

RLS establishes tenant isolation but does not replace resource authorization.
A tenant row match does not establish course membership, workspace
collaboration, learner ownership, retention access, or delivery ACL.
Conversely, route checks cannot compensate for missing RLS because a future
path may be wrong.

Export creation is also Store authority: it takes the resolved session-token
hash, locks the course roster, and resolves the current direct Instructor
itself before freezing a job. Request fields cannot choose an actor, recipient,
object, or export input, and a concurrently revoked Instructor cannot win the
race.

## Error, audit, and change rules

Responses distinguish only cases an authorized caller needs to recover from.
Missing or unauthorized protected resources normally share an absent result.
Students receive forbidden for an action whose category is already clear, such
as assignment creation in their own course. Revision conflicts and validation
errors become distinct only after relevant authorization succeeds.

Successful protected asset delivery creates an audit event. Other educational
record, export, retention, and broker flows retain dedicated tenant-owned audit
or lifecycle evidence. Audit records identify the tenant, actor, and durable
resource; they never contain session credentials, signed URLs, answer keys,
private grader state, or browser-provided authority.

Before changing a protected route, Store method, worker action, or provider
integration, verify that:

- session resolution precedes tenant and resource authority;
- browser input cannot choose a tenant or elevate scope;
- the exact actor-resource rule and concealment result are tested;
- authorization, structural validation, and lifecycle checks stay independent;
- PostgreSQL work uses transaction-local context and forced RLS;
- answer keys, provider secrets, object keys, and signed URLs remain outside
  browser DTOs and Wasm; and
- permanent tests cover stable behavior, while fresh-database RLS, provider,
  and multi-replica checks remain named live/disposable acceptance evidence.

## Related references

- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines answer secrecy, session,
  publication, course, run, asset, and provider boundaries.
- [DATABASE_TENANCY.md](DATABASE_TENANCY.md) defines tenant ownership,
  transaction-local context, forced RLS, roles, and retention.
- [CONTRACTS.md](CONTRACTS.md) maps owners and implemented module boundaries.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines learner
  render, response, and grading wire contracts.
- [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md) defines stateless replicas,
  shared sessions, object storage, and worker leases.
- [RETENTION_POLICY.md](RETENTION_POLICY.md) defines archive and deletion of
  tenant-owned educational records.
