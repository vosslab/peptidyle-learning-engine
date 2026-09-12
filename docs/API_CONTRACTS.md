# API contract map

This is PLE's durable, route-level HTTP contract. It describes the public
surface implemented by the server today, not a roadmap, a database catalog, or
a Live Demo provisioning protocol. [composition.rs](../crates/server/src/composition.rs)
composes the executable route surface; the linked route modules own the details.

[CONTRACTS.md](CONTRACTS.md) owns module boundaries,
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns authorization
decisions, and [SECURITY_MODEL.md](SECURITY_MODEL.md) owns credentials, storage,
and deployment security.

## Common protocol

All browser calls use relative same-origin URLs, `credentials: "same-origin"`,
and `Cache-Control: no-store`. The server stores only a hash of the opaque,
host-only, Secure, HttpOnly, `SameSite=Lax` session cookie. Unsafe requests
require the canonical same-origin `Origin` and reject ambiguous cookies.

An opaque public reference locates a candidate only; session-derived authority
and the stored relationship decide access. Concealed resources, malformed
opaque references, and callers outside the applicable relationship receive the
same no-store `404`. Browser decoders accept only their closed response shapes.
Responses never expose private object locations or checksums, source bytes,
answer keys, raw renderer or grader data, credentials, or internal database IDs.

An `If-Match` header carries one quoted qualified Edit Number. A missing
precondition is `428`; an invalid header is `400`; a stale competing write is
`412`; an invalid resulting record or confirmation is `422`; and an invalid
lifecycle transition is `409`. Temporary dependency failure is `503`.

## Session and Question authoring

| Surface | Route | Contract |
| --- | --- | --- |
| Health | `GET /health` | Reports bounded readiness only. |
| Session | `GET /api/auth/session`, `POST /api/auth/logout` | Resolves the browser-safe session projection or revokes the presented session. |
| Optional seeded-demo entry | `GET` / `POST /api/auth/live-demo/accounts` | When the installation includes configured demo accounts, lists the bounded eligible persona set or establishes an ordinary session for one selected persona. It grants no authority itself. |
| Question Library | `GET /api/questions/search`, `GET /api/questions/by-id/{questionId}`, `GET /api/questions/by-id/{questionId}/detail` | Instructor-only answer-free discovery and current published Question detail. |
| Exact Question Revision | `GET /api/questions/by-id/{questionId}/revisions/{revisionNumber}` | Resolves one immutable `QuestionRevisionReference`, including when its lineage is archived, for an authorized Instructor. |
| Question availability | `POST /api/questions/by-id/{questionId}/archive`, `POST /api/questions/by-id/{questionId}/restore` | Archive requires the exact lineage Availability Edit Number and `{ "confirmationTitle": "..." }`; restore requires the exact Edit Number. Each returns current availability and its new Edit Number. |
| Draft Question | `GET` / `POST /api/authoring/drafts`; `GET` / `PUT /api/authoring/drafts/{draftQuestionReference}/source` | An Instructor owns private canonical source through their Authoring Workspace. Source save uses its quoted Draft Edit Number ETag. |
| Question publication | `POST /api/authoring/drafts/{draftQuestionReference}/publish`, `POST /api/authoring/drafts/{draftQuestionReference}/publish-revision` | Validates the owned Draft and creates immutable published Question content. The first route creates a stable lineage; the second requires an exact parent `QuestionRevisionReference` and reviewed reason, then creates its successor Revision. Responses expose no private source binding. |

Question availability belongs to the stable lineage; publishing another
`QuestionRevisionReference` does not reset it. Ordinary selection admits only
Available Question lineages. Existing exact references remain resolvable.

## Blueprint Courses

| Surface | Route | Contract |
| --- | --- | --- |
| Browse | `GET /api/course-blueprints` | Lists Available reusable Blueprint Courses for an active Instructor. |
| Create | `POST /api/course-blueprints` | Creates one stable Blueprint Course and its private current Draft. The `201` response contains that Draft and its ETag; creation does not create a Blueprint Revision. |
| Current Blueprint Course | `GET /api/course-blueprints/{blueprintCourseReference}` | Returns the authorized answer-free aggregate: current availability, Availability Edit Number, optional latest immutable `BlueprintRevisionReference`, and the owner Draft when applicable. |
| Draft save | `PUT /api/course-blueprints/{blueprintCourseReference}/draft` | Replaces current Draft content with the exact Draft Edit Number `If-Match`; a changed save advances the Draft ETag. |
| Publish | `POST /api/course-blueprints/{blueprintCourseReference}/publish` | Requires the Draft ETag and returns the newly accepted immutable `BlueprintRevisionReference`. A replay with the same request receipt converges; a deliberate accepted publication records a Revision. |
| Exact Blueprint Revision | `GET /api/course-blueprints/{blueprintCourseReference}/revisions/{revision}` | Returns one immutable exact Blueprint Revision for an authorized Instructor, including after the lineage is archived. |
| Availability | `POST /api/course-blueprints/{blueprintCourseReference}/archive`, `POST /api/course-blueprints/{blueprintCourseReference}/restore` | Archive requires the lineage Availability Edit Number and title confirmation; restore requires the exact Edit Number. Both return availability and its new Edit Number. |

A Blueprint Revision contains exact `QuestionRevisionReference` pins. A Course
Assignment records `BlueprintAssignmentSource`: its stable Blueprint Assignment
Reference plus the exact Blueprint Revision Reference. That source is
provenance, not a third Revision family.

## Courses and roster

| Surface | Route | Contract |
| --- | --- | --- |
| Course Instances | `GET` / `POST /api/course-instances`; `GET /api/course-instances/{courseInstanceReference}` | An active Instructor creates or reads a Course Instance from an exact Available `BlueprintRevisionReference`. Creation supplies current Course Term dates and establishes the assigned Instructor relationship; it creates no Assignment or Student Work. A Sysadmin creating for another Instructor receives no ambient Course authority. |
| Course summary and navigation | `GET /api/courses/{course}`, `GET /api/navigation/{courseInstanceReference}` | Returns answer-free scoped Course identity only for an active Course Member. |
| Course appearance | `GET` / `PUT /api/courses/{course}/appearance`; `POST /api/courses/{course}/appearance/banner-uploads`; `PUT` / `DELETE /api/courses/{course}/appearance/banner`; `POST /api/course-banners/{banner}/delivery`; `POST /api/course-banners/{banner}/delivery/card` | Course Members read the answer-free appearance aggregate; an active Course Instructor changes theme or banner. Delivery rechecks Course membership. |
| Instructor profile | `GET` / `PATCH /api/instructor-profile`; `GET` / `POST /api/instructor-profile/thumbnail`; `POST /api/instructor-profile/thumbnails/{thumbnail}/delivery` | An active Instructor reads or updates only their profile and private thumbnail; replacement validates and normalizes bounded image bytes before delivery. |
| Course creation choice | `GET /api/course-instance-creation/instructors` | A Sysadmin receives bounded active Instructor public references for assigned-Instructor selection. |
| Roster | `GET` / `POST /api/course-instances/{courseInstanceReference}/roster`; `POST /api/course-instances/{courseInstanceReference}/roster/claim`; `POST /api/course-instances/{courseInstanceReference}/roster/{rosterId}/revoke` | Direct Course Instructors import or revoke the course-scoped roster. The invited Student claims only their own invitation. Import creates neither Assignment nor Student Work. |
| Invitation export | `GET /api/course-instances/{courseInstanceReference}/invitation-export` | A direct Instructor receives a no-store attachment with only pending, unexpired invitations; this route does not send mail. |

Course Term is current Course Instance state. The public API has no Course
Schedule Revision endpoint, field, receipt, or compatibility alias.

## Current Assignments

| Surface | Route | Contract |
| --- | --- | --- |
| Source choices and picker | `GET /api/course-instances/{courseInstanceReference}/assignment-source-choices`, `GET /api/course-instances/{courseInstanceReference}/assignment-question-picker` | A direct Course Instructor sees answer-free source choices and currently Available Question Revision pins. |
| Assignment list and due-soon | `GET /api/course-instances/{courseInstanceReference}/assignments`, `GET /api/assignments/due-soon` | Returns current answer-free Assignment summaries with `status` and `editNumber`; due-soon independently rechecks current Course membership. |
| Create and workspace | `POST /api/course-instances/{courseInstanceReference}/assignments`; `GET` / `PUT /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}` | Creates, reads, or saves one stable current Assignment. A save requires the Assignment Edit Number ETag and stores ordered normalized Entries that pin exact Question Revisions. An unchanged retry returns the existing aggregate and ETag without mutation. |
| Inline save | `PUT /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/inline` | Updates the closed title/due projection using the Assignment Edit Number ETag. |
| Validation and preview | `GET /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/release-validation`, `GET /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/preview` | Computes current release blockers or returns an answer-free direct-Instructor preview without creating Student Work. |
| Release | `POST /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/release` | Requires the Assignment Edit Number ETag and a release-valid current Assignment. `200` returns the current Assignment aggregate with Released status and its new ETag. |
| Unrelease impact | `GET /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/unrelease-impact` | For a Released Assignment only, returns its current confirmation title, required Edit Number, and aggregate Attempt, submission, and grade counts. It contains no Student identity, response, or grade detail. |
| Unrelease | `POST /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/unrelease` | Requires a direct Teaching Team member, Released status, exact Assignment Edit Number ETag, and `{ "confirmationTitle": "exact current title" }`. `200` returns the current Unreleased Assignment, new ETag, and aggregate deletion counts. PostgreSQL atomically locks the Assignment, removes its rooted Student Work, rebuilds affected statistics, and records a redacted audit event. |

An Assignment is one stable mutable aggregate. Release is a status transition,
not an Assignment Revision. Released saves remain valid only when the resulting
current Assignment passes release validation; they affect future Attempts.
Existing Attempts interpret their retained evidence. The public surface has no
Assignment Revision, Assignment Revision Entry, successor-revision, or released
Assignment Revision reference field.

## Student delivery, grading, and Gradebook

| Surface | Route | Contract |
| --- | --- | --- |
| Assignment access | `GET /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/access` | The exact active Student Course relationship receives the current start decision, answer-free Assignment facts, any active Attempt reference, and authorized prior-Attempt history. |
| Start or resume | `POST /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/start` | At authoritative time, starts or resumes one eligible Attempt. It returns an answer-free presentation built from retained Attempt and Issued Question evidence: exact Question Revision, stored Question Seed, issue position, and retained presentation/asset binding. Current Assignment configuration governs new Attempt eligibility; it does not reinterpret an existing Attempt. |
| Attempt reads | `GET /api/assignment-attempts/{assignmentAttemptReference}/student-progress`, `GET /api/assignment-attempts/{assignmentAttemptReference}/context`, `GET /api/assignment-attempts/{assignmentAttemptReference}/student-question?position={position}`, `GET /api/assignment-attempts/{assignmentAttemptReference}/history` | An owning active Student reads only their authorized progress, retained presentation, and policy-authorized history. The server reproduces pinned content; it does not regrade or substitute current configuration. |
| Responses, submission, and status | `PUT /api/assignment-attempts/{assignmentAttemptReference}/responses/{position}`, `POST /api/assignment-attempts/{assignmentAttemptReference}/submission`, `GET /api/course-instances/{courseInstanceReference}/assignments/{assignmentReference}/presentations/{presentationNonce}/submissions` | An exact issued presentation accepts only its applicable response format. Whole-Attempt submission is separate; status remains bound to its nonce and exposes no answer key, raw grader input, private source, or private feedback internals. |
| Question assets | `GET /api/questions/{questionId}/revisions/{revisionNumber}/assets/{assetId}` | Delivers an immutable asset only after the authorized retained presentation relationship. |
| Student landing | `GET /api/student/course-instances`, `GET /api/student/course-invitations`, `GET /api/course-instances/{courseInstanceReference}/assignment-landing` | Returns only safe active/pending Course or Released Assignment landing data. |
| Gradebook | `GET /api/course-instances/{courseInstanceReference}/gradebook` | A current Course Instructor receives the answer-free course Gradebook projection. |

## Administration and support

| Surface | Route | Contract |
| --- | --- | --- |
| Instructor accounts | `GET` / `POST /api/instructor-accounts`; `POST /api/instructor-accounts/{reference}/deactivate`; `POST /api/instructor-accounts/{reference}/reactivate` | Sysadmin-only account lifecycle; the Store repeats authorization. |
| Scoped support | `POST /api/course-instances/{reference}/support-capabilities`; `POST /api/course-instances/{reference}/support-capabilities/{capabilityId}/revoke`; `GET /api/support-capabilities/{capabilityId}/course-roster` | A direct Instructor manages a bounded Course-scoped capability; a Sysadmin reads only its minimal registered roster projection. |

## Excluded compatibility surface

Only Question Revision and Blueprint Revision name immutable published content.
The API contains no Course Schedule Revision, Assignment Revision, Question
Change Proposal Revision, Course Retention Plan Revision, or compatibility
reader for any of those shapes. Course Retention and Question Change Proposal
have no public persistence or endpoint until a complete product boundary is
implemented. Live Demo provisioning and reports are installation concerns, not
HTTP API contracts.

## Change control

A new route earns inclusion only when it has a server owner, a Store and
authorization boundary, a closed browser decoder where browser-visible, and
focused evidence for its durable behavior. The route map is updated with that
change; it does not preserve retired names as aliases.
