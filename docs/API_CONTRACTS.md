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

The route registrations in [router.rs](../crates/server/src/composition/router.rs)
are the executable authority for what is available. Route modules own their
extractors, limits, authorization, and HTTP status mapping. The corresponding
browser method in [client.ts](../src/api/client.ts) and strict decoder
under [decoders.ts](../src/api/decoders.ts) own the browser-facing
shape. Rust types under `crates/question_model/` and generated files under
`generated/api/` own shared value definitions.

Route paths below describe the current implementation unless a row says
**target**. The planned compact Student response in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) is not silently
substituted for the current tagged `StudentResponse` route contract.

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
| Assignment/run/attempt | Exact course, assignment, Student membership, and session-derived role | Student-facing Store reads and mutations require one active Student assignment entitlement in the same authority boundary as the record lookup. A revoked Student cannot retain access through an old enrollment, run, or attempt identifier. |
| Workspace | Exact workspace owner/collaborator relationship | Student, foreign, and unshared workspaces share an absent projection. |
| Protected asset | Typed delivery record plus current persisted authorization pointer | Unknown or unauthorized delivery ID is not an object-storage lookup. |

The fuller authorization and forced account-and-relationship-scoped RLS evidence is in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security)
and [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Route families

The map names route groups instead of copying each DTO. Exact method, path,
body limit, status response, and Rust type remain in the linked owner.

| Family | Routes | Identity and payload boundary | Owner |
| --- | --- | --- | --- |
| Health | `GET /health` | Readiness only; it is not an authenticated API session probe. | [router.rs](../crates/server/src/composition/router.rs) |
| Auth/session | `GET /api/auth/session`; `POST /api/auth/logout`; deployment-gated seeded-persona `GET/POST /api/auth/live-demo/accounts` | The currently mounted routes resolve, revoke, or create the one bounded Authenticated Session carried in the host-only `__Host-ple_session` cookie. The visible seeded-persona selector is available only with complete disposable-demo configuration. Email-code and passkey ceremonies remain required product capabilities, with their private schema roots in place; their replacement adapters are not yet mounted. | [auth.rs](../crates/server/src/auth.rs), [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md), [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) |
| Catalog (**target SD1-B3**) | `GET /api/problems`, `/search`, `/by-id/{reference}`, `/by-id/{reference}/detail`, `GET /api/taxonomy` | Only an authenticated approved Instructor may browse, search, resolve, or inspect published question content. The stable Question ID exposes its immutable version history and exact ancestry without exposing private evidence. All published lifecycle states are discoverable and exactly resolvable. The safe projection excludes private response/grading material, credentials, internal publication pairs, and Student records. | [routes.rs](../crates/server/src/catalog/routes.rs) |
| Catalog lifecycle (**target SD1-B3**) | `POST /api/problems/{workspace}/publish`, `POST /api/problems/by-id/{reference}/deprecate`, `/archive` | A same-lineage correction or compatible improvement keeps the Question ID and publishes a new immutable version under original-lineage stewardship. A major objective, task, or Question Type change is a creator-private fork draft that validates before global publication with a new Question ID and exact source ancestry. A Sysadmin-approved `ForcedQuestionCorrection` accepts only `security_flaw` or `critical_correctness_flaw`, atomically maps the flawed version to its validated replacement so new selection and issuance resolve to the replacement immediately, and preserves the old version solely as immutable historical evidence. `Published` is the active state for ordinary new assignment selection; `Deprecated` and `Archived` remain discoverable and resolvable but are not ordinarily selectable. Publication has no scope field or branch. | [routes.rs](../crates/server/src/catalog/routes.rs) |
| Course and assignment | `GET/POST /api/courses`, `GET /api/courses/{course}/assignments`, `GET /api/courses/{course}`, `/gradebook`; exact-course Instructor workspace `GET /api/courses/{course}/assignments/{assignment}`, `POST .../drafts`, `PUT .../{assignment}/content`, `PUT .../{assignment}/policies`, and `GET .../{assignment}/student-view` | Course comes from the route plus the authenticated direct-course relationship. Workspace reads and writes require the exact course/assignment pair; the workspace read returns the complete Instructor editor projection, while focused content and policy writes each replace only their owned slice and return the complete authoritative projection. Every focused write uses the shared assignment revision and `If-Match`; stale or issued-work conflicts preserve the page's entered values for visible recovery. | [routing.rs](../crates/server/src/course/routing.rs), [workspace.rs](../crates/server/src/course/assignments/workspace.rs) |
| Assignment Student delivery (**target SD1-E**) | Student `GET /api/assignments/{assignment}/student` and `/summary` | Ordinary Student routes remain separate key-free projections. They deliver content only after exact server-side entitlement binds the authenticated Student, active Student membership, course, assignment, audience, lifecycle, and current policy. They never consult the shared Instructor catalog and do not authorize an Instructor Student view. | [student.rs](../crates/server/src/course/assignments/student.rs) |
| Instructor grading operations | `GET /api/courses/{course}/assignments/{assignment}/grading-operations`; `POST .../grading-operations/{operation}/retry`; `POST .../grading-operations/recalculate` | Direct-Instructor-only, exact course/assignment authority derived from the authenticated session and active direct course membership. The GET is a bounded, answer-free, metadata-only, `no-store` projection grouped within the assignment by question or Student. Retry and recalculation are `no-store`, body-free commands: retry requires the operation revision in `If-Match`, recalculation requires the assignment revision, and both require one UUID `Idempotency-Key`. Neither route accepts Student responses, answer keys, scores, or operation state from the browser. | [grading_operations.rs](../crates/server/src/course/grading_operations.rs) |
| Course groups and membership policy | `GET/POST /api/courses/{course}/groups`; `GET/PUT/DELETE /api/courses/{course}/groups/{group}`; `GET/PUT /api/courses/{course}/group-purpose-policies/{purpose}`; `GET /api/courses/{course}/group-membership-warnings` | Exact-course direct Instructors manage bounded, cursor-paged groups and the five purpose policies. Strong revisions protect mutations; referenced groups cannot be deleted or changed to an incompatible purpose. Multiple Section membership reports its actual warning disposition and count without blocking; other default purposes allow it. Responses are no-store and expose only safe `G-` and `M-` references and display labels. | [course router](../crates/server/src/course/routing.rs), [question-model facade](../crates/question_model/src/lib.rs) |
| Assignment access modifiers and preview | `PUT/DELETE /api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}`; `PUT/DELETE .../group-accommodations/{group}`; `PUT/DELETE .../individual-policy-exceptions/{student}`; `GET .../policy-preview/{student}`; `GET /api/courses/{course}/student-targets` | Exact-course direct Instructors mutate M2, M3, and M4 through assignment `If-Match`; authorization and reference resolution precede body parsing. Course-local times are resolved by the server through the course IANA zone. Each mutation atomically re-evaluates active-attempt S5 entitlement and S3 effective policy. The server-owned preview is a closed denied/allowed union: denial leaks no policy, while allowance carries safe local values, course zone, and ordered `G-`/`M-` provenance labels without internal IDs, clocks, or raw policy inputs. | [course router](../crates/server/src/course/routing.rs), [question-model facade](../crates/question_model/src/lib.rs) |
| Course roster | `GET /api/courses/{course}/roster`; invitation create/revoke/redeem; enrollment-policy replace; member revoke; roster-import preview/commit | Direct Instructors own the workflow. Sysadmin support uses the exact list/invite/policy/revoke/import capability; the Store records account/course/action/time for each support disclosure or change. Invitation claim resolves the authenticated PLE account and atomically creates a canonical membership episode plus its protected roster profile; assignment receipts remain evaluator-owned and materialize only on a qualifying event. | [roster.rs](../crates/server/src/course/roster.rs), [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) |
| Teaching authority and co-instructors | `PUT/DELETE /api/teaching/instructor-approvals/{account}`; `GET/POST /api/courses/{course}/co-instructor-invitations`; `GET /api/courses/{course}/co-instructor-targets`; `DELETE /api/courses/{course}/co-instructor-invitations/{invitation}`; `GET /api/account/co-instructor-invitations`; `POST /api/account/co-instructor-invitations/{invitation}`; `GET /api/courses/{course}/instructors`; `DELETE /api/courses/{course}/instructors/{membership}` | A live Sysadmin session may maintain global Instructor approval, which is eligibility evidence and never ambient course authority. Exact-course direct Instructors search approved display-only targets, invite without email, revoke pending invitations, list instructors, and remove only when another direct Instructor remains. The authenticated target alone lists and accepts or declines its pending invitations; acceptance rechecks approval and mints one direct membership. Strong ETags/`If-Match`, no-store responses, authorization-before-reference/body ordering, and opaque `U-`/`M-`/`CI-` references preserve CAS and non-enumeration boundaries without UUID or email disclosure. | [course router](../crates/server/src/course/routing.rs), [database structure](DATABASE_STRUCTURE.md) |
| Course grade scheme | `GET/PUT /api/courses/{course}/grade-scheme` | Accepted WP-INST-S6. Direct Instructors read the current total-points or weighted-categories scheme and assignment settings. The read projection includes current server-owned assignment titles; the title-free write body replaces exact current assignment settings. `PUT` requires one strong `If-Match` revision and returns `412` on a stale revision. Completion-based grading is not a route mode. | [gradebook.rs](../crates/server/src/course/gradebook.rs), [course_gradebook.rs](../crates/learning-data-access/src/course_gradebook.rs) |
| Calculated Gradebook and selection | `GET /api/courses/{course}/gradebook`; `GET /api/courses/{course}/gradebook/selection` and `/gradebook/students/{membership}/assignments/{assignment}/runs` | Direct-Instructor-only, same-origin Fetch Metadata, no-store projections. The server owns roster-first course calculation, closed assignment/Student/operation filtering, exact run choice, structural continuation, and reload-required states. Selection exposes bounded public references and safe labels; it never exposes a Student response or grader material. | [course Gradebook](../crates/server/src/course/gradebook.rs), [Gradebook Store](../crates/learning-data-access/src/course_gradebook.rs) |
| Audited Student work | `GET /api/courses/{course}/gradebook/students/{membership}/assignments/{assignment}/runs/{run}` | Direct-Instructor-only and same-origin Fetch Metadata protected. One no-store response contains server-owned Student and assignment labels, immutable submitted responses, solution-free visible feedback, issued presentation evidence, and a closed return context. The PostgreSQL broker verifies the full public composite and immutable evidence and atomically records the successful Student-record access and metadata audit facts. | [course Gradebook](../crates/server/src/course/gradebook.rs), [inspection Store](../crates/learning-data-access/src/student_work_inspection.rs) |
| Course grade totals | `GET /api/courses/{course}/gradebook-totals` | Direct-Instructor-only, no-store compact summary projection. The server calculates every row from one scheme snapshot and maintained assignment summaries; the browser never recomputes totals and receives only the protected display label plus a score or closed unavailable reason. Roster ID, email, and raw Student summaries remain outside this browser contract. | [gradebook.rs](../crates/server/src/course/gradebook.rs), [course_grade.rs](../crates/domain/src/course_grade.rs) |
| Course grade export | `POST /api/courses/{course}/grade-export.csv` | Direct-Instructor-only synchronous CSV with an empty request body, bounded to 500 active-student rows. Rows contain the selected mode, four-decimal rounding, course total or explicit unavailable status, and ephemeral roster email/display name. The response is no-store and exposes an opaque export ID; durable `course_total_export_audit` metadata is PII-free. | [gradebook.rs](../crates/server/src/course/gradebook.rs), [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) |
| Course appearance | `GET/PUT /api/courses/{course}/appearance`, `POST /api/courses/{course}/appearance/banner-candidates` | Safe theme/banner projection; candidate upload uses raw bounded raster bytes and returns an opaque candidate receipt. Current appearance update uses strong `If-Match`. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Workspace | `GET /api/workspaces`, `GET/PUT/DELETE /api/workspaces/{workspace}`, `POST /publication-validation`, `GET /publication-diff` | Private authoring draft surface. Mutation and publication review use the strong workspace revision ETag. | [workspace.rs](../crates/server/src/workspace.rs) |
| Private author presentation | `GET /api/workspaces/{workspace}/author-preview?seed=...` | Instructor-only teaching display; it may return rendered correct-response material but never an answer key or reusable grading contract. | [author_preview.rs](../crates/server/src/author_preview.rs) |
| Flat authoring | `GET/PUT /api/workspaces/{workspace}/flat-question`, `POST /api/problems/{workspace}/flat-question-publish` | The narrow author route accepts answer-bearing source only after author authorization. Generic workspace and Student routes remain answer-free. | [flat_question_publication.rs](../crates/server/src/flat_question_publication.rs) |
| QTI profile | `GET/PUT /api/workspaces/{workspace}/qti-imports/{import}`, `POST /items/{item}/convert-flat`, `POST /api/problems/{workspace}/qti-publish` | Archive bytes and conversion/provenance stay private. Browser reports, acknowledgements, and converted draft handoff remain answer-free. | [qti_profile_import.rs](../crates/server/src/qti_profile_import.rs) |
| Runs and attempts | `POST /api/courses/{course}/assignments/{assignment}/runs`; nested `POST .../attempts/{attempt}/prefetch-next` and `/submissions`; `GET .../attempts/{attempt}/submission-status`; `GET /api/runs/{run}`, `/summary`, `/attempts`, `GET /api/attempts/{attempt}`, `/question` | The nested course/assignment pair is a server-verified routing assertion, never Student authority or a JSON field. An authenticated attempt binds Student, run, assignment position, immutable version, seed, timing, lifecycle, and backend. Submission acceptance returns an answer-free `202 accepted_pending` projection; the route-bound status read converges on pending, instructor-attention, or the immutable completed receipt. | [routes.rs](../crates/server/src/run/routes.rs) |
| Automated grading recovery | `GET /api/courses/{course}/assignments/{assignment}/grading-operations`; `POST .../grading-operations/{operation}/retry`; `POST .../grading-operations/recalculate` | **Target WN1-MG:** automated grader exceptions receive an answer-free Instructor recovery surface. The browser requests retry or assignment-wide recalculation through revisioned, empty-body commands, and the server routes accepted private input through its worker capability before publishing the current Gradebook result. Student feedback and scores remain policy-projected. **Current pre-WN1:** the source still contains a human-grade endpoint; WN1-MG retires that product path together with its route, Store, and route-policy closure. | [grading_operations.rs](../crates/server/src/course/grading_operations.rs) |
| External tool | Nested `GET/POST /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch`, plus bounded `/activity` and `/submission` children | The typed nested route is verified before provider work and is never accepted in JSON or provider data. Only the POST creates a server-held, one-attempt launch session. The GET returns an inert same-origin shell; it cannot create, renew, or reveal a provider launch. Activity requires the session-bound launch proof. Provider material, replay state, field names, credentials, and raw provider outcomes remain server-held. | [external_tool.rs](../crates/server/src/run/external_tool.rs) |
| Item analysis | `GET /api/courses/{course}/assignments/{assignment}/item-analysis` | Instructor-only aggregate projection. It excludes Student, attempt, raw response, answer, and object identity. | [item_analysis.rs](../crates/server/src/item_analysis.rs) |
| Export | `POST /api/assignments/{assignment}/exports`, `GET /api/exports/{export}` | Creation requires an exactly empty body. The server freezes the assignment and delivery plan; status returns safe identifiers and progress, not object keys or manifests. | [export.rs](../crates/server/src/export.rs) |
| Retention | `GET /api/courses/{course}/retention`, `POST /end`, `/archive`, `/delete`, `PATCH /extend` | A current course Instructor or a Sysadmin using the narrow audited capability may read, end, archive, or delete the exact course retention record. Only the authorized retention capability may extend it. Unauthorized foreign-course requests remain non-enumerating and cannot expose Student data or worker lease state. The browser renders only the server-owned notification copy and closed action outcome; it does not derive retention timing or cleanup state. | [retention.rs](../crates/server/src/retention.rs) |
| Assets | `GET /api/assets/{id}`, `POST /api/assets/{id}/delivery` | GET is deliberately public-only: it redirects an active immutable public asset and never signs private content. POST is deliberately protected: it rechecks the session-derived authorization pointer, records the authorization decision, and then creates the bounded private delivery. Pending public assets are unavailable from both paths until the dedicated publisher has verified and activated them. | [asset.rs](../crates/server/src/asset.rs) |
| Browser validation fallback | `POST /api/validation/response-format`, `/timer`, `/assignment-capabilities` | Authenticated, key-free pure validation only. It never grades, authorizes publication, establishes server time, or replaces server grading. | [validation.rs](../crates/server/src/validation.rs) |

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
Acceptance of a compatible `ModerateEdit` publishes a new immutable `VersionId`
under the original stable `QuestionId`, preserves canonical authorship and the
compatible CC license, records contributor credit and proposal ancestry, and
leaves every assignment and evidence `ProblemVersionRef` unchanged.
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
fixed or pool content, disclosure policy, run policies, course-local teaching
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
`ProblemVersionRef` pin. The Instructor may choose a shared question without
importing its content into the course row. A future version becomes available
only through this explicit, revision-checked update; publication and lifecycle
work never advance an assignment automatically. Policies
owns `PUT
/api/courses/{course}/assignments/{assignment}/policies`. It accepts one closed
aggregate of audience, disclosure, run policies, and course-local teaching
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

**Current pre-WN1:** this route currently uses `groupBy`, `learner`, and `pageSize`.
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

The route table describes the shared application router assembled by
[router.rs](../crates/server/src/composition/router.rs).
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
| `Idempotency-Key` | Student submission | The same key and same response represent one grading request. A retry returns the committed receipt without grading twice. The exact owner is [routes.rs](../crates/server/src/run/routes.rs). |
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
