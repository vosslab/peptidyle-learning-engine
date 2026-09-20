# API contract map

This is PLE's target route-level HTTP contract. It follows
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Generic teaching objects are Assessments.
Assignment appears only in the three Assessment Type names. Blueprint lifecycle
states are Private, Public, and Archived. [composition.rs](../crates/server/src/composition.rs)
composes the currently executable route surface.

[CONTRACTS.md](CONTRACTS.md) owns module boundaries,
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns authorization
decisions, and [SECURITY_MODEL.md](SECURITY_MODEL.md) owns credentials, storage,
and deployment security.

## Common protocol

All browser calls use relative same-origin URLs, `credentials: "same-origin"`,
and `Cache-Control: no-store`. The server stores only a hash of the opaque,
host-only, Secure, HttpOnly, `SameSite=Lax` session cookie. Unsafe requests
require the canonical same-origin `Origin` and reject ambiguous cookies.

An opaque public ID locates a candidate only; session-derived authority
and the stored relationship decide access. Concealed resources, malformed
opaque IDs, and callers outside the applicable relationship receive the
same no-store `404`. Browser decoders accept only their closed response shapes.
Responses never expose private object locations or checksums, source bytes,
answer keys, raw renderer or grader data, credentials, or internal database IDs.
Account, Course Instance, Assessment, Blueprint Course, Attempt, and similar
objects emit JSON `id` (or nested `courseId` / `assessmentId`). HTTP path
parameters that carry those public IDs are named `course_instance_id`,
`assessment_id`, `blueprint_course_id`, and `account_id`. Composite Question
Revision pins remain `{ questionId, revisionNumber }`.

An `If-Match` header carries one quoted qualified Edit Number. A missing
precondition is `428`; an invalid header is `400`; a stale competing write is
`412`; an invalid resulting record or confirmation is `422`; and an invalid
lifecycle transition is `409`. Temporary dependency failure is `503`.

## Session and Question authoring

| Surface                    | Route                                                                                                                                 | Contract                                                                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Health                     | `GET /health`                                                                                                                         | Reports bounded readiness only.                                                                                                                                                                                                                                                                                                                                                                                                |
| Session                    | `GET /api/auth/session`, `POST /api/auth/logout`                                                                                      | Resolves the browser-safe session projection or revokes the presented session.                                                                                                                                                                                                                                                                                                                                                 |
| Optional seeded-demo entry | `GET` / `POST /api/auth/live-demo/accounts`                                                                                           | When the installation includes configured demo accounts, lists the bounded eligible persona set or establishes an ordinary session for one selected persona. It grants no authority itself.                                                                                                                                                                                                                                    |
| Question Library           | `GET /api/questions/search`, `GET /api/questions/by-id/{questionId}`, `GET /api/questions/by-id/{questionId}/detail`                  | Active vetted Instructors and Sysadmins receive answer-free discovery and current published Question detail.                                                                                                                                                                                                                                                                                                                    |
| Exact Question Revision    | `GET /api/questions/by-id/{questionId}/revisions/{revisionNumber}`                                                                    | Resolves one immutable `QuestionRevisionTuple`, including when its lineage is archived, for an authorized Instructor.                                                                                                                                                                                                                                                                                                      |
| Question Bloom correction  | `POST /api/questions/by-id/{questionId}/revisions/{revisionNumber}/bloom`                                                            | An active vetted Instructor corrects one exact Question Revision with the complete Bloom pair and its expected classification Edit Number. A successful response returns that same exact Revision and current pair.                                                                                                                                                                                                                   |
| Question availability      | `POST /api/questions/by-id/{questionId}/archive`, `POST /api/questions/by-id/{questionId}/restore`                                    | Archive requires the exact lineage Availability Edit Number and a clear confirmation; the current `confirmationTitle` request field is an implementation shape, not a Human Guidance requirement. Restore requires the exact Edit Number. Each returns current availability and its new Edit Number.                                                                                                                                  |
| Draft Question             | `GET` / `POST /api/authoring/drafts`; `GET` / `PUT /api/authoring/drafts/{draftQuestionId}/source`                                    | An Instructor owns private canonical source through their Authoring Workspace. `draftQuestionId` is the native private UUID route value. Source save uses its quoted Draft Edit Number ETag.                                                                                                                                                                                                                                      |
| Question publication       | `POST /api/authoring/drafts/{draftQuestionId}/publish`, `POST /api/authoring/drafts/{draftQuestionId}/publish-revision`               | Validates the owned Draft and creates immutable published Question content. Publication copies the author-declared educational Question Type from the Draft source binding to the Published Question Revision. The first route creates a stable lineage; the second requires an exact parent `QuestionRevisionTuple` and reviewed reason, then creates its successor Revision. Responses expose no private source binding.   |

Question availability belongs to the stable lineage; publishing another
`QuestionRevisionTuple` does not reset it. Ordinary selection admits only
Published, non-archived Question lineages. Existing exact references remain resolvable.

Active Instructors and Sysadmins receive answer-free Question and Pool Library
read projections. A Question projection carries its exact Revision's required
two-value Bloom Classification and independent classification Edit Number. A
Pool projection carries the current Pool's own pair and classification Edit
Number; member Question pairs never substitute. The `questionRevisionTuple` field identifies the exact resolved Revision on an
exact Question-detail route. `GET /api/question-pools` and
`GET /api/question-pools/{questionPoolId}` are the product Pool reads. JSON
carries sibling `questionPoolId` and `questionPoolEditNumber` fields; there
is no Pool Pin wrapper. Exact immutable Question Revision pins remain
`{ questionId, revisionNumber }`.

Pool discovery accepts optional exact `bloom_cognitive_process` and
`bloom_knowledge_dimension` query parameters. Each combines with every other applied Pool
predicate. The opaque continuation is bound to both values, and the response carries all six
Cognitive Process counts plus all four Knowledge Dimension counts from the complete filtered Pool
set, including zeros and an empty page. Page position affects only `items`; the counts and rows come
from the same authorized filtered SQL relation. These predicates and counts use the current Pool's
own pair, never a member Question's pair.

The corresponding Pool correction route is
`POST /api/question-pools/{questionPoolId}/bloom`. Bloom lives on the
current Pool. Both Question and Pool correction routes
accept only the complete `cognitiveProcess`, `knowledgeDimension`, and
`expectedClassificationEditNumber` command. The classification Edit Number is
the pair's CAS precondition, not a content Revision or a lineage token. Either
route first checks a stale expected number and returns `412`; only a current
request may then return an unchanged pair without advancing it. A changed pair
advances its classification Edit Number once. Active vetted Instructor
authority is based on current exact Library read access, not target ownership.
Sysadmins retain the read projection but cannot use either correction route.

The client does not retry or merge a `412`. It reloads only the same exact
Question Revision or current Pool, retains the Instructor's draft pair for
comparison, and requires an explicit later Save. Corrections do not create a
Question content Revision, change Pool member Question Revision pins, or alter retained
Assessment or Student Work evidence.

`GET /api/questions/search` accepts optional exact `bloom_cognitive_process` and
`bloom_knowledge_dimension` filters. Each is independent and combines with every
other active normalized Question Library predicate. Saved `QuestionSearchFilter`
values, browser URL handoff, and opaque cursors retain both filters and the
existing sort; a cursor is valid only for that exact normalized query. The route
does not change the established `titleAscending` and `publishedNewest` sorts.
Its facets report the whole matching set, not one cursor page, with all six
Cognitive Process and all four Knowledge Dimension values in guide order,
including zero counts and an empty result.

## Blueprint Courses

| Surface                   | Route                                                                                                                                                                                                | Contract                                                                                                                                                                                                 |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Browse                    | `GET /api/course-blueprints`                                                                                                                                                                         | Lists Public Blueprint Courses for a vetted Instructor. Explicit historical discovery may include Archived Blueprint Courses. Private Blueprint Courses remain owner-only.                              |
| Create                    | `POST /api/course-blueprints`                                                                                                                                                                        | Validates complete reusable content and atomically creates one Private Blueprint Course with Revision 1. The `201` response contains that Revision and its ETag.                                         |
| Create from Course Instance | `POST /api/course-instances/{course_instance_id}/course-blueprints`                                                                                                                            | A current Course Instructor supplies only the new Blueprint names and classification. The server atomically copies current reusable Assessment structure into a distinct actor-owned Private Revision 1, records the unchanged source Course Instance as its first Adoption, forks each Course-owned Pool with identical exact ordered pins, and returns the complete owner-visible Blueprint with `201` and `no-store`. |
| Current Blueprint Course  | `GET /api/course-blueprints/{blueprint_course_id}`                                                                                                                                              | Returns the authorized answer-free aggregate: current Revision, its Revision ETag, and separate opaque lineage metadata ETag for short name, long name, and lifecycle state.                             |
| Save                      | `PUT /api/course-blueprints/{blueprint_course_id}`                                                                                                                                              | Replaces reusable content with the exact current Revision `If-Match`. A changed Save creates the next Revision; a canonical no-op returns the current Revision with `changed: false`.                    |
| Exact Blueprint Revision  | `GET /api/course-blueprints/{blueprint_course_id}/revisions/{revision_number}`                                                                                                                         | Returns one immutable exact Blueprint Revision for an authorized Instructor, including after the lineage is archived.                                                                                    |
| Metadata and lifecycle    | `PUT /api/course-blueprints/{blueprint_course_id}/metadata`; current archive and restore routes                                                                                                  | Rename and lifecycle changes use the lineage metadata ETag without creating a Revision. The target lifecycle is Private, Public, and Archived. The current route set does not yet express the complete make-Public and permitted return-to-Private workflow.             |

A Blueprint Revision contains exact `QuestionRevisionTuple` pins. A Course
Instance Assessment records its stable Blueprint Assessment reference plus the
exact Blueprint Revision Tuple. That source is
provenance, not a third Revision family. A newly added Blueprint Assessment is
automatically copied to daughter Course Instances as an Unreleased Course
Instance Assessment. New Blueprint Revisions must also be offered to daughter
Course Instructors for review and approval, without silently applying changes
to existing Assessments. Source-fork update discovery and Blueprint Course
Change Proposals are required workflows, but Human Guidance does not choose
their exact route names or wire shapes.

## Courses and roster

| Surface                       | Route                                                                                                                                                                                                                                                             | Contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Course Instances              | `GET` / `POST /api/course-instances`; `GET /api/course-instances/{course_instance_id}`                                                                                                                                                                       | An active Instructor creates an empty Course Instance or adopts an exact current Public Blueprint Revision. Adoption atomically establishes the Course Term, first equal Instructor membership, and every Blueprint Assessment as a fresh Course Instance Assessment with exact Question Revision pins, reusable settings, Unreleased state, and unset dates. Creation starts the maximum six-month Active lifetime. A Sysadmin creating for another Instructor receives no ambient Course authority. |
| Course summary and navigation | `GET /api/course-instances/{course_instance_id}/summary`, `GET /api/navigation/{course_instance_id}`                                                                                                                                                          | Returns answer-free scoped Course identity only for an active Course Member.                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Course appearance             | `GET` / `PUT /api/course-instances/{course_instance_id}/appearance`; `POST /api/course-instances/{course_instance_id}/appearance/banner-uploads`; `PUT` / `DELETE /api/course-instances/{course_instance_id}/appearance/banner`; `POST /api/course-banners/{course_banner_id}/delivery` | Course Members read the answer-free appearance aggregate; an active Course Instructor changes theme or banner. Delivery rechecks Course membership.                                                                                                                                                                                                                                                                                                                                               |
| Account Settings              | `GET` / `PUT /api/account/settings`                                                                                                                                                                                                                               | Every signed-in Product Role reads or replaces only its own closed `{ "timeZone": "exact IANA name" }` preference. The request has no Account, Course, or Product Role selector. PostgreSQL derives the active Account and atomically validates and changes the exact installed IANA name. A change affects display and later Instructor wall-clock date entry, never a stored instant. This surface has no passkey, email, TOTP, recovery, Account-status, or session control; required passwordless authentication and multiple Student passkeys remain Accounts-and-roles work. |
| Instructor profile            | `GET` / `PATCH /api/instructor-profile`; `GET` / `POST /api/instructor-profile/thumbnail`; `POST /api/instructor-profile/thumbnails/{thumbnail}/delivery`                                                                                                         | An active Instructor reads or updates only their profile and private thumbnail; replacement validates and normalizes bounded image bytes before delivery. The projection displays the current Account Settings time zone and links to `/account-settings`; it does not update the zone. |
| Course creation choice        | `GET /api/course-instance-creation/instructors`                                                                                                                                                                                                                   | A Sysadmin receives bounded active Instructor public references for assigned-Instructor selection.                                                                                                                                                                                                                                                                                                                                                                                                |
| Roster                        | `GET` / `POST /api/course-instances/{course_instance_id}/roster`; `POST /api/course-instances/{course_instance_id}/roster/claim`; `POST /api/course-instances/{course_instance_id}/roster/{roster_id}/revoke`                                       | Equal Course co-Instructors may bulk add Students through roster import and remove one Student at a time; PLE provides no bulk Student-removal workflow. An invited Student claims only their own invitation. A newly created Student Account receives the inviting Instructor's Account Time Zone as its one-time default at acceptance; an existing Account or prior Student choice is unchanged. Enrollment creates neither Assessment nor Student Work. |
| Invitation export             | `GET /api/course-instances/{course_instance_id}/invitation-export`                                                                                                                                                                                           | A direct Instructor receives a no-store attachment with only pending, unexpired invitations; this route does not send mail.                                                                                                                                                                                                                                                                                                                                                                       |

These Profile rows inventory current routes; they do not define the complete
product capability. Every signed-in Product Role requires Profile and Account
Settings plus a Profile menu containing Sign Out. Profile Settings owns avatar
selection and Profile-image work: Students choose only from PLE-provided
avatars and never upload; Instructors and Sysadmins may choose a provided
avatar or add a Profile image. Missing Student and Sysadmin Profile routes are
implementation gaps, not permission to omit those workflows.

The current Course Appearance hero/card delivery routes record implementation
shape only. They do not preserve the superseded 6:1 page-width hero or 5:2 card
crop as product requirements. The target is one responsive 5:1 Course banner,
shown small and centered, with 1280 by 256 pixels as the recommended authoring
size. The product contract has no separate card crop or rendition family.

Course Term is current Course Instance state. The public API has no Course
Schedule Revision endpoint, field, receipt, or compatibility alias.

## Current Assessments

| Surface                      | Route                                                                                                                                                                                                       | Contract                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Source choices and picker    | `GET /api/course-instances/{course_instance_id}/assessment-question-picker`                                           | A direct Course Instructor sees answer-free source choices and currently Published, non-archived Question Revision pins. |
| Assessment list and due-soon | `GET /api/course-instances/{course_instance_id}/assessments`, `GET /api/assessments/due-soon`                                                                                                          | Returns current answer-free Assessment summaries with Assessment Type, release status, due date, and `editNumber`; due-soon independently rechecks current Course membership.                                                                                                                                                                                                                |
| Create and edit              | `POST /api/course-instances/{course_instance_id}/assessments`; `GET` / `PUT /api/course-instances/{course_instance_id}/assessments/{assessment_id}`                                         | Creates, reads, or saves one stable current Assessment. A save requires the Assessment Edit Number ETag and stores ordered Questions and Question Pools that pin exact Question Revisions. An unchanged retry returns the existing aggregate and ETag without mutation.                                                                                                                     |
| Inline save                  | `PUT /api/course-instances/{course_instance_id}/assessments/{assessment_id}/inline`                                                                                                              | Updates the closed title/due projection using the Assessment Edit Number ETag.                                                                                                                                                                                                                                                                                                             |
| Assessment Properties save  | `PUT /api/course-instances/{course_instance_id}/assessments/{assessment_id}/policies`                                                                                                            | Updates whole-Assessment instructions, dates, scoring, Attempts, late work, and disclosure settings using the Assessment Edit Number ETag. Ordered Questions and Question Pools remain unchanged. The `/policies` path is the Assessment Properties save surface.                                                                                                                            |
| Validation and preview       | `GET /api/course-instances/{course_instance_id}/assessments/{assessment_id}/release-validation`, `GET /api/course-instances/{course_instance_id}/assessments/{assessment_id}/student-view` | Computes current release blockers or returns an answer-free direct-Instructor preview without creating Student Work.                                                                                                                                                                                                                                                                       |
| Release                      | `POST /api/course-instances/{course_instance_id}/assessments/{assessment_id}/release`                                                                                                            | Requires the Assessment Edit Number ETag and a passing automated, interactive Assessment Release Validation. Validation covers required Questions/settings, point and Attempt/time ranges, date order, and a due date at least 24 hours ahead that does not exceed the Course Instance's six-month Active limit. `200` returns the current Assessment aggregate with Released status and its new ETag. |
| Unrelease impact             | `GET /api/course-instances/{course_instance_id}/assessments/{assessment_id}/unrelease-impact`                                                                                                    | For a Released Assessment only, returns its current confirmation title, required Edit Number, and aggregate affected Student Work counts. It contains no Student identity, response, or grade detail.                                                                                                                                                                                       |
| Unrelease                    | `POST /api/course-instances/{course_instance_id}/assessments/{assessment_id}/unrelease`                                                                                                          | Requires an equal Course co-Instructor, Released status, exact Assessment Edit Number ETag, and `{ "confirmationTitle": "exact current title" }`. `200` returns the current Unreleased Assessment and new ETag. PostgreSQL atomically locks the Assessment and permanently removes its Student Work.                                                                                       |

An Assessment is one stable mutable aggregate. Release is a status transition,
not an Assessment Revision. Released saves remain valid only when the resulting
current Assessment passes release validation; they affect future Attempts.
Existing Attempts interpret their retained evidence. The public surface has no
Assessment Revision, Assessment Revision Entry, successor-revision, or released
Assessment Revision Tuple field.

## Student delivery, grading, and Gradebook

Completed history may include `backendAnswerReview: "available"` on an issued
WeBWorK Question only when its independent `question_answer` decision permits
disclosure. Withheld markers are absent; native `questionAnswer` stays unchanged.
The browser derives
`GET /api/assessment-attempts/{assessment_attempt}/questions/{position}/answer-review-document`
from the validated Attempt reference and positive position. No query, body,
source, seed, response, URL, or disclosure flag is accepted.

That route requires the owning Student's current Course membership, ordinary
Work visibility, committed Assessment Submission, and copied answer policy.
Quiz/Exam disclosure also requires completion by the current Course's Students.
The same completed-history decision runs before source/render I/O and again
after rendering; a changed roster or access discards the result. Unknown,
foreign, unsubmitted, archived/deleted, wrong-backend, and withheld positions
return concealed `404` with no renderer call. Authorized source or renderer
failure returns generic sandboxed `503` HTML. Successful output contains only
backend `renderedHTML`, with UTF-8 HTML, `no-store`, `nosniff`, `no-referrer`, and
response-level `sandbox allow-scripts` CSP. The existing opaque preview frame
receives only bounded resize messages and exposes Retry correct answer through
a fresh history read. Protected answer images stay inline inside the document;
there are no public review-only assets, save/submit calls, or grading writes.

| Surface                           | Route                                                                                                                                                                                                                                                                                                                    | Contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Assessment access                 | `GET /api/course-instances/{course_instance_id}/assessments/{assessment_id}/access`                                                                                                                                                                                                                           | The exact active Student Course relationship receives answer-free Assessment facts, any active Attempt reference, authorized prior-Attempt history, and the server-owned availability decision. It carries the Assessment Type, dates, effective time and Attempt limits, Student display time zone, current Start Decision, and public reason without exposing accommodation identity.                                                                                                                                                                                                                                                                                                                               |
| Start or resume                   | `POST /api/course-instances/{course_instance_id}/assessments/{assessment_id}/start`                                                                                                                                                                                                                           | At authoritative time, starts or resumes one eligible Assessment Attempt. Resume across reload or another authenticated session returns that same Attempt and its answer-free presentation built from retained Attempt and Issued Question evidence. Current Assessment settings govern new Attempt eligibility; they do not reinterpret an existing Attempt.                                                                                                                                                                                                                                                                                                                                                       |
| Attempt reads                     | `GET /api/assessment-attempts/{assessment_attempt}/student-progress`, `GET /api/assessment-attempts/{assessment_attempt}/context`, `GET /api/assessment-attempts/{assessment_attempt}/student-question?position={position}`, `GET /api/assessment-attempts/{assessment_attempt}/history` | An owning active Student reads only their authorized progress, retained presentation, and policy-authorized history. Context returns `expiresAt`, `timerRemainingMilliseconds`, and `displayTimeZone` from one authoritative read. Reads do not finalize Student Work, call a Question Backend, or substitute current configuration. The server-owned expiry instant refuses later changes; background execution finalizes saved work for expired Attempts that need no further Student request. |
| Backend document                  | `GET /api/assessment-attempts/{assessment_attempt}/questions/{position}/document`                                                                                                                                                                                                                                | An owning active Student receives the exact immutable WeBWorK backend document for that issued position. The response is same-origin HTML with `no-store`, document CSP, and same-origin CORP. Foreign, malformed, native, incomplete, and unavailable positions are concealed. The route returns document bytes only; it exposes no source, renderer endpoint, credential, or grading data.                                                                                                                                                                                                                                                                                                                                                                                                         |
| Backend preview document          | Current authorized Instructor WeBWorK preview routes                                                                                                                                                                                                                                                                      | Returns a no-write, answer-free renderer document in an opaque script-only sandbox. The preview omits only the renderer's parent-frame telemetry loader because an opaque frame intentionally has no `frameElement`; Student delivery retains the exact renderer document. Its sole outbound bridge message is exactly `{kind: "ple.webwork.preview.resize", version: 1, height: int}` to the canonical parent origin. The parent accepts it only from origin `null`, the exact iframe `contentWindow`, and the exact three-key shape with safe `height` 160 through 1200. No preview ready, capture, form, or response message exists. An unexpected or ambiguous matching loader is concealed rather than broadly rewriting backend HTML. |
| Opaque preview dependencies       | `GET /styles/ple_embed.css`, `GET /ple_bridge.js`, and the fixed local font paths referenced by the embed stylesheet                                                                                                                                                                                                     | The canonical browser gateway publicly serves only these PLE-owned dependencies to an opaque Instructor preview. The reads do not consult PLE credentials or emit cookies. All use `no-store` and cross-origin CORP; fonts alone also use anonymous `Access-Control-Allow-Origin: *` with no credential grant. A missing exact file returns 404 instead of the browser application shell.                                                                                                                                                                                                                                                                                                                                              |
| Backend assets                    | `GET /api/webwork-assets/{prefix}/{path}`                                                                                                                                                                                                                                                                                | This public GET boundary delivers bounded renderer bytes only from the `webwork2_files` and `pg_files` namespaces. It does not consult PLE session authority, forward `Cookie` or `Authorization` to the private renderer, or return a renderer `Set-Cookie`. It rejects unsafe paths, redirects, unsupported media, and oversized responses. Installation-owned `webwork2_files`, `pg_files/js`, and `pg_files/node_modules` assets use cross-origin CORP; generated and otherwise unclassified `pg_files` assets retain same-origin CORP. Fonts in the public installation namespaces additionally use anonymous `Access-Control-Allow-Origin: *` with no credential grant. `webwork2_files` may be cached for one day; `pg_files` are `no-store`. |
| Responses and submission          | `PUT /api/assessment-attempts/{assessment_attempt}/responses/{position}`, `POST /api/assessment-attempts/{assessment_attempt}/submission`, current presentation-status route                                                                                                                             | A save accepts one complete Question response in its declared format and remains editable while the Assessment Attempt is open. Whole-Assessment submission finalizes every complete saved response together as Student Work. Other Questions remain visibly unanswered, receive zero credit, and count as incorrect without being sent to a backend. Backend-owned response payloads stay opaque to PLE. Status exposes no answer key, grader input, private source, or private backend state. |
| Submission result                 | `POST /api/assessment-attempts/{assessment_attempt}/submission`                                                                                                                                                                                                                                                   | Submission returns the Assessment result using one immutable credit fraction for each complete saved response evaluated by its Question Backend. When PLE requests a grading outcome, the backend returns it without a deferred grading state; failed evaluation cannot produce a completed submission. |
| Question assets                   | `GET /api/questions/{questionId}/revisions/{revisionNumber}/assets/{assetId}`                                                                                                                                                                                                                                            | Delivers an immutable asset only after the authorized retained presentation relationship.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Student landing                   | `GET /api/student/course-instances`, `GET /api/student/course-invitations`, current Course landing route                                                                                                                                                                                                                  | Returns safe Course and Coursework data. Each item uses its specific Assessment Type label and shows due date and completion status. A released item may remain listed before availability so the Student can see the server-owned reason and schedule. |
| Gradebook                         | `GET /api/course-instances/{course_instance_id}/gradebook`                                                                                                                                                                                                                                                          | A current Course co-Instructor receives the answer-free Course Gradebook projection. Scores are calculated on read from stored immutable credit fractions and current Question point values; when multiple Attempts are submitted, the highest Assessment Attempt score is used. It exposes no response, failure reason, Job, lease, or grading action. |
| Gradebook point export | `GET /api/course-instances/{reference}/gradebook/export?format=csv` or `format=tsv` | The same current Course co-Instructor authorization and Gradebook projection produce the fixed point-only attachment described below. |

The browser formats every decision instant in the supplied `displayTimeZone`, identifies that zone
to the Student, and displays the server-owned Start Decision and public reason directly. Landing and
pre-start pages share this presentation; browser time is never used to recompute permission.

### Student invitation context

`GET /api/student/course-invitations` returns only the signed-in active Student's pending,
unexpired invitations without an active Student Course Membership. Each closed item carries
`reference`, `shortName`, `longName`, `instructorDisplayName`, and `term` with inclusive `startDate`
and `endDate`. The display name comes from the server-controlled verified identity of the Course's
assigned Instructor. It exposes no Authentication Email, Account identity, internal Course or
invitation identity, or roster. Responses use `no-store`; other product roles remain concealed.

The invitation index and exact-reference detail page show the Course name, Instructor, and term
before acceptance. The detail page resolves context from that same self-only list and exposes
Accept only when the exact pending invitation is available. Missing and foreign invitations share
the unavailable state. Acceptance still uses the separate authorized Course Roster transaction.

### Gradebook point export

The required `format` query accepts only lowercase `csv` or `tsv`. Missing, duplicate,
malformed, unsupported, and unknown query fields return `400` without an attachment. Format
validation discloses no Course facts. Each valid download resolves an ordinary Instructor session
and reads the existing authorized Course Gradebook projection once. Students, unrelated Instructors,
Sysadmins or support capabilities without ordinary Instructor access, expired sessions, and
unavailable Courses receive the same concealed `404` as Gradebook. Ordinary retention visibility
applies; this route does not grant archived Student Work recovery.

Success is `200` with `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, and
`Content-Disposition: attachment; filename="ple_<canonical-course-reference>_grades.csv"`
(or `.tsv`). CSV uses `text/csv; charset=utf-8`; TSV uses
`text/tab-separated-values; charset=utf-8`. The canonical reference alone supplies the filename.
Existing Gradebook store-error statuses are retained; unexpected serialization failure returns a
generic `503` without partial bytes. Errors, including framework extraction failures, are no-store
and carry no attachment. The browser checks status, no-store, media type, nosniff, and exact
attachment filename before consuming bytes. There is no request body or Accept-based negotiation.

The file has exactly these seven columns in this order:

| Column | Value |
| --- | --- |
| `roster_id` | Course-local roster ID |
| `roster_name` | Instructor-provided roster label |
| `assessment_id` | Canonical Assessment ID |
| `assessment_title` | Assessment title |
| `status` | `not_started`, `in_progress`, `expired_submitting`, or `submitted` |
| `points_earned` | Selected submitted Attempt's earned points, or blank |
| `points_possible` | Selected submitted Attempt's possible points, or blank |

There is one row per active Student and released Assessment, including no-Attempt rows, sorted by
exact `roster_id` then canonical `assessment_id` in ascending ASCII byte order. An empty
authorized Course produces the header alone. No Attempt maps to `not_started`; an unsubmitted
Attempt maps to `in_progress` or, with the expiry flag, `expired_submitting`; a completed Attempt
maps to `submitted`, including when its score is absent. Both point cells are blank when no score
exists. A score on unsubmitted evidence fails closed. The selected submitted Attempt can coexist
with a later open Attempt; the exported status describes the selected evidence.

Point values are finite, nonnegative, locale-independent shortest round-trip decimal numbers
without display rounding; exponent notation is permitted and numeric zero is `0`. Zero earned
points remain distinct from blank. Bonus work may have zero possible points and extra credit may
exceed possible points. Scores follow current point edits through the existing score read without
backend interaction. PLE exports points; the Instructor handles weighting and percentages in the
home LMS.

Both files use UTF-8 without BOM, quote every cell with double quotes, double embedded quotes, and
end every record with CRLF. CSV uses commas; TSV is a quoted tab-delimited dialect with identical
escaping. Embedded commas, tabs, CR, and LF remain inside quoted cells. Before quoting, text cells
receive an apostrophe prefix when they begin with `=`, `+`, `-`, `@`, tab, CR, LF, or NUL, or when
removing leading whitespace and ASCII controls reveals `=`, `+`, `-`, or `@`. The original text
follows unchanged. This includes roster IDs. Numeric cells come only from validated numbers. The
safety marker may be visible outside spreadsheets; import roster IDs as text to preserve leading
zeros and avoid date/number conversion. Protection is not guaranteed after another application
strips the marker or re-saves a file.

Only the seven fields above are exported: no Question content, answers, responses, per-Question
outcomes, email, account IDs, private UUIDs, accommodations, invitations, tokens, Jobs, failure
reasons, categories, weights, percentages, or Course totals. The explicit Gradebook download
buttons fetch fresh bytes and revoke the temporary object URL. Export creates no server file,
history, reusable URL, email, external LMS transfer, browser persistence, payload log, or additional
retention record. Pending and failure feedback is accessible in-page; an error body is never saved
as a grade file.

## Administration and support

| Surface             | Route                                                                                                                                                                                                            | Contract                                                                                                                        |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Instructor accounts | `GET` / `POST /api/instructor-accounts`; `POST /api/instructor-accounts/{reference}/deactivate`; `POST /api/instructor-accounts/{reference}/reactivate`                                                          | Sysadmin-only account lifecycle; the Store repeats authorization.                                                               |
| Scoped support      | `POST /api/course-instances/{reference}/support-capabilities`; `POST /api/course-instances/{reference}/support-capabilities/{capabilityId}/revoke`; `GET /api/support-capabilities/{capabilityId}/course-roster` | A direct Instructor manages a bounded Course-scoped capability; a Sysadmin reads only its minimal registered roster projection. |

## Excluded compatibility surface

Question Revision names immutable published Question content; Blueprint Revision
names immutable saved Blueprint content.
The API contains no Course Schedule Revision, Assessment Revision, Question
Change Proposal Revision, Course Retention Plan Revision, or compatibility
reader for any of those shapes. Human Guidance defines Course-retention product
behavior, but the current route map does not yet provide the complete Instructor
notice, archival, recovery, or permanent-deletion workflow. Question Change
Proposal likewise has no public persistence or endpoint until a complete product
boundary is approved. Live Demo provisioning and reports are installation
concerns, not HTTP API contracts.

## Change control

A new route earns inclusion only when it has a server owner, a Store and
authorization boundary, a closed browser decoder where browser-visible, and
focused evidence for its durable behavior. The route map is updated with that
change; it does not preserve retired names as aliases.
