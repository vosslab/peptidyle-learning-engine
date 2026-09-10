# API contract map

This is PLE's durable route-level map. It identifies the presently implemented
same-origin HTTP surface and separates it from retained product contracts. It
does not replace generated Rust and TypeScript shapes.

[CONTRACTS.md](CONTRACTS.md) owns module ownership,
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns detailed
authorization decisions, and [SECURITY_MODEL.md](SECURITY_MODEL.md) owns
security and storage rules.

## Status and authority

[composition.rs](../crates/server/src/composition.rs) is the executable
authority for the production route surface. Its current entry point provides
health, session resolution/logout, deployment-gated seeded Live Demo account
entry, Instructor Question Library, M6 private Question authoring
and publication, M7 Blueprint Course lifecycle, M8 Course Instance creation,
M9 Course Roster Import, M10 Assignment Workspace and release, M11 Student
Assignment Access and issuance, M12 native controls and asset delivery, M13
native submission and status, M14 WeBWorK delivery and grading, M15 Gradebook,
M16 Instructor Accounts, M17 scoped support operations, M18 protected Course
Invitation export, current Course member summary and public-reference
navigation, Course Appearance, and the Student course and Assignment landing
routes. Route
modules absent
from server composition, generated DTOs, browser clients, schemas, and models
retain product design; none establishes an available HTTP endpoint.

## Implemented Server Routes

| Surface                | Route                               | Current boundary                                                                                                                                                              | Owner                                                  |
| ---------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Health                 | `GET /health`                       | Bounded readiness: `200` with no unavailable dependencies, or `503` naming only closed unavailable categories; the disposable topology checks database, object store, renderer, and worker, while the gateway names a missing API | [composition.rs](../crates/server/src/composition.rs)  |
| Session                | `GET /api/auth/session`             | Resolves the presented opaque session cookie into a browser-safe Authenticated Session response                                                                               | [auth.rs](../crates/server/src/auth.rs)                |
| Session                | `POST /api/auth/logout`             | Revokes the presented session and clears its cookie                                                                                                                           | [auth.rs](../crates/server/src/auth.rs)                |
| Seeded Live Demo entry | `GET /api/auth/live-demo/accounts`  | Lists surviving members of the deployment's closed seeded-persona set and a bounded unavailable count, with no Account ID, unavailable-persona detail, or authorization claim | [live_demo.rs](../crates/server/src/auth/live_demo.rs) |
| Seeded Live Demo entry | `POST /api/auth/live-demo/accounts` | Resolves one closed seeded persona and issues the ordinary Authenticated Session                                                                                              | [live_demo.rs](../crates/server/src/auth/live_demo.rs) |
| Question Library | `GET /api/questions/search` | Requires the stored current Instructor role, resolves only M4 Published Question source bindings server-side, and returns a bounded, answer-free filtered page. Student and anonymous requests receive the same `404` concealment. | [question_library.rs](../crates/server/src/question_library.rs) |
| Question Library | `GET /api/questions/by-id/{question_id}` | Resolves one currently published stable Question ID after the same Instructor boundary. Object records, source bindings, checksums, and private source bytes remain server-only. | [question_library.rs](../crates/server/src/question_library.rs) |
| Question Library | `GET /api/questions/by-id/{question_id}/detail` | Returns the browser-safe current Question Details presentation after the same Instructor boundary; no response, answer, feedback, source, or grading fields cross the route. | [question_library.rs](../crates/server/src/question_library.rs) |
| My Question Drafts | `GET /api/authoring/drafts` | Requires the stored current Instructor role and returns only each private Draft Question's opaque `D-` reference, positive Edit Number, and bounded discovery metadata. Student and anonymous requests receive the same `404` concealment. | [authoring.rs](../crates/server/src/authoring.rs) |
| My Question Drafts | `POST /api/authoring/drafts` | Requires the same Instructor boundary and canonical PLE Question JSON. The server resolves or creates the caller's Authoring Workspace, stores source privately, and returns only the opaque Draft Question Reference and Edit Number. | [authoring.rs](../crates/server/src/authoring.rs) |
| Draft Question source | `GET /api/authoring/drafts/{draft_question_reference}/source` | Resolves private canonical PLE Question JSON only after the current Instructor's exact Authoring Workspace relationship. It returns a no-store source payload and strong Edit Number ETag; no workspace UUID, Draft Question UUID, object address, or checksum crosses the route. | [authoring.rs](../crates/server/src/authoring.rs) |
| Draft Question source | `PUT /api/authoring/drafts/{draft_question_reference}/source` | Requires canonical same-origin `Origin`, the exact Edit Number `If-Match`, current Instructor workspace authority, and valid PLE Question JSON. A stale Edit Number returns `409`; the server binds new private immutable bytes. | [authoring.rs](../crates/server/src/authoring.rs) |
| Question Publication | `POST /api/authoring/drafts/{draft_question_reference}/publish` | Requires the same exact Edit Number and authorized current Draft Question. The server validates the private source, copies it into a Question Revision-owned object, and returns only the stable Question ID; the published DTO contains no draft identity or source path. | [authoring.rs](../crates/server/src/authoring.rs) |
| Blueprint Courses | `GET /api/course-blueprints` | Requires the stored current Instructor role and lists only published reusable Blueprint Courses using closed, answer-free Blueprint Course Read Access classifications. Student and anonymous requests receive the same `404` concealment. | [blueprint_course.rs](../crates/server/src/blueprint_course.rs) |
| Blueprint Courses | `POST /api/course-blueprints` | Requires the same Instructor boundary; resolves selected available Questions to exact Question Revision References and atomically publishes an initial immutable Blueprint Revision. The response is answer-free and carries its revision ETag. | [blueprint_course.rs](../crates/server/src/blueprint_course.rs) |
| Blueprint Course | `GET /api/course-blueprints/{blueprint_course_reference}` | Resolves one published Blueprint Course for its Blueprint Course Owner or another active Instructor and returns only closed reader data; no account, source, checksum, response, answer, or feedback fields cross the route. | [blueprint_course.rs](../crates/server/src/blueprint_course.rs) |
| Blueprint Course | `PUT /api/course-blueprints/{blueprint_course_reference}` | Requires the exact Blueprint Course Owner, canonical same-origin `Origin`, and current Blueprint Revision `If-Match`. It atomically publishes a successor immutable Blueprint Revision; a stale ETag returns `412`. | [blueprint_course.rs](../crates/server/src/blueprint_course.rs) |
| Course Instances | `GET /api/course-instances` | Requires an active Instructor Course Membership and lists only the caller's current Course Instances. Student, anonymous, and Sysadmin requests receive the same `404` concealment. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course Instance | `POST /api/course-instances` | An Active Instructor creates a Course Instance for themself from one exact Available published Blueprint Revision and Course Term. An active Sysadmin may create it only by naming an active Instructor. The atomic result stores immutable Course Origin, Course Schedule Revision 1, initial Instructor Course Membership and event, and audit evidence; it creates no Student Record or Assignment. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course Instance | `GET /api/course-instances/{course_instance_reference}` | Requires the caller's direct active Instructor Course Membership and returns the Course Term, immutable source reference, and initial Teaching Team. A Sysadmin creator gains no ambient Course access. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course member summary | `GET /api/courses/{course}` | Requires an authenticated active Course Membership and returns the answer-free Course summary needed to compose a scoped Course route. Anonymous, nonmember, ended-membership, and malformed-Course requests receive concealed no-store `404` responses. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course navigation | `GET /api/navigation/{course_instance_reference}` | Resolves a public `C-` Course reference to its internal Course ID only for an authenticated active Course Member. It returns a no-store concealed `404` for an unavailable or unauthorized reference. | [navigation.rs](../crates/server/src/navigation.rs) |
| Course Appearance | `GET /api/courses/{course}/appearance` | Requires an authenticated active Course Membership and returns the current answer-free aggregate `{ theme, banner }` reader result. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Course Theme | `PUT /api/courses/{course}/appearance` | Requires the current Instructor Product Role and exact active Instructor Course Membership; replaces only the scalar current theme. The current implementation returns the aggregate shape with `banner: null`, even when a banner exists; this is a known defect against the aggregate reader contract. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Course Banner upload | `POST /api/courses/{course}/appearance/banner-uploads` | Requires the same exact active Instructor Course Membership; verifies bounded image bytes and stages one temporary Course-bound upload for later independent promotion. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Course Banner promotion or removal | `PUT` / `DELETE /api/courses/{course}/appearance/banner` | Requires the same exact active Instructor Course Membership; promotes one staged upload or clears the current banner independently of the Course Theme. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Course Banner delivery | `POST /api/course-banners/{banner}/delivery`; `POST /api/course-banners/{banner}/delivery/card` | Requires an authenticated active Course Member for the current opaque banner reference and returns the authorized hero or card WebP rendition. | [course_appearance.rs](../crates/server/src/course_appearance.rs) |
| Course Instance creation | `GET /api/course-instance-creation/instructors` | Requires the stored active Sysadmin role and returns bounded active Instructor `U-` references for the sole creation-assignment choice; it returns no account names or emails. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course Roster | `GET /api/course-instances/{course_instance_reference}/roster` | Requires the caller's direct active Instructor Course Membership and returns only that Course Instance's course-scoped roster email, roster ID, and pending/active state. Student, anonymous, foreign-course, and Sysadmin callers receive the same `404` concealment. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Roster Import | `POST /api/course-instances/{course_instance_reference}/roster` | Requires the same direct Instructor authority. The bounded atomic import resolves or creates each Student Account by immutable Student Authentication Email, records only a pending Course Invitation and course-scoped roster profile, and creates no Student Record, membership, Assignment, or Student work. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Invitation claim | `POST /api/course-instances/{course_instance_reference}/roster/claim` | Requires the authenticated active Student who is the pending invitation target. It creates or reuses the exact Student Record and creates an active Student Course Membership, without Assignment or Student-work state. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Roster access revocation | `POST /api/course-instances/{course_instance_reference}/roster/{roster_id}/revoke` | Requires the direct current Instructor. It records an immutable ended Student Course Membership or revoked pending Course Invitation and retains protected educational records. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Assignment Question Picker | `GET /api/course-instances/{course_instance_reference}/assignment-question-picker` | Requires the caller's direct active Instructor Course Membership and lists a bounded answer-free set of currently Available Published Questions. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Course Assignments | `GET /api/course-instances/{course_instance_reference}/assignments` | Requires the caller's direct active Instructor Course Membership and lists every Assignment in that Course Instance by public Assignment Reference, title, lifecycle status, and Edit Number. The projection contains no Student identity, work, response, answer, or grading state. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Assignment Workspace | `POST /api/course-instances/{course_instance_reference}/assignments` | Requires the same direct Instructor authority and creates one Unreleased Course-owned Assignment without Question selection, Student Record, Assignment Access, or Student work. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Assignment Workspace | `GET`/`PUT /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}` | Loads or saves current answer-free authored content only for a direct Instructor. Saves require the exact strong Assignment Edit Number `If-Match`; stale content returns `412`. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Assignment Release Validation | `GET /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/release-validation` | Calculates whether the current Assignment has releasable Available Published Questions without creating a Revision or Student work. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Assignment Preview | `GET /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/preview` | Returns a direct-Instructor-only, answer-free Assignment Preview of title, instructions, and selected Question descriptions. It has no Student identity, Student work, response, answer, feedback, or delivery state. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Assignment Release | `POST /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/release` | Requires the exact strong Assignment Edit Number `If-Match`, validates the current selection, and atomically creates the next immutable Assignment Revision for later M11 delivery. | [assignment_release.rs](../crates/server/src/assignment_release.rs) |
| Student Assignment Access | `GET /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/access` | Requires the authenticated Student's exact active Student Record for the Course. It returns only the server-calculated start decision for the released Assignment at authoritative time. | [assignment_delivery.rs](../crates/server/src/assignment_delivery.rs) |
| Student Assignment start | `POST /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/start` | Requires the same exact active Student Record, refuses unavailable, closed, attempt-limit, and due/late-rejected starts before issue, and atomically starts or resumes the released Assignment Revision. It issues or resumes one full answer-free QuestionPresentation pinned to the exact Question Revision, with a 53-bit OS-random public QuestionSeed, nonce, title, prompt, and response format. The no-store response has no internal locator, source, checksum, reproduction data, answer, or grading data. | [assignment_delivery.rs](../crates/server/src/assignment_delivery.rs) |
| Student submission and status | `POST` / `GET /api/course-instances/{course_instance_reference}/assignments/{assignment_reference}/presentations/{presentation_nonce}/submissions` | Requires the Student's exact issued Question Presentation. Submission accepts only the applicable response format. Status is bound to that nonce and returns only the presentation nonce and closed grading state; it does not return correctness, points, Answer Keys, source, private feedback, or raw grader data. | [assignment_delivery.rs](../crates/server/src/assignment_delivery.rs) |
| Question asset delivery | `GET /api/assets/{asset_id}` | Requires the authorized Student presentation relationship before returning an immutable public-asset redirect. | [question_asset_delivery.rs](../crates/server/src/question_asset_delivery.rs) |
| Student course landing | `GET /api/student/course-instances`, `GET /api/student/course-invitations`, `GET /api/course-instances/{course_instance_reference}/assignment-landing` | Requires the Student's exact active or pending Course relationship and returns only the safe Course or released-Assignment landing projection. | [live_student_course_landing.rs](../crates/server/src/live_student_course_landing.rs) |
| Gradebook | `GET /api/course-instances/{course_instance_reference}/gradebook` | Requires the current Course Instructor and returns the answer-free immutable Gradebook projection. | [live_gradebook.rs](../crates/server/src/live_gradebook.rs) |
| Instructor Accounts | `GET` / `POST /api/instructor-accounts`; `POST /api/instructor-accounts/{reference}/deactivate`; `POST /api/instructor-accounts/{reference}/reactivate` | Requires the current Sysadmin role and rechecks that authority in the Store. | [instructor_account.rs](../crates/server/src/instructor_account.rs) |
| Scoped support | `POST /api/course-instances/{reference}/support-capabilities`; `POST /api/course-instances/{reference}/support-capabilities/{capability_id}/revoke`; `GET /api/support-capabilities/{capability_id}/course-roster` | A direct Instructor issues or revokes one Course-scoped capability; a Sysadmin reads only its registered minimal roster projection. | [support_capability.rs](../crates/server/src/support_capability.rs) |
| Course Invitation export | `GET /api/course-instances/{course_instance_reference}/invitation-export` | Requires the current direct Instructor and returns a no-store JSON attachment of only pending, unexpired Student Course Invitations for that Course in the existing mailer shape. The browser downloads the attachment only; this route does not send mail. | [invitation_export.rs](../crates/server/src/invitation_export.rs) |

The seeded routes are present when at least one valid, unambiguous deployment
mapping survives. The five-persona set remains closed, but a configured demo
Account can be unavailable without removing the surviving entries; the bounded
unavailable count discloses no Account or mapping detail. Seeded entry replaces
identity verification for the known demo personas. It supplies no Product Role,
Course Membership, Student record, or authority; the server derives those facts
from stored PLE records whenever a future route needs them.

## Implemented Server Route protocol rules

| Concern         | Current contract                                                                                                                                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Origin          | Browser requests use a relative same-origin path, `credentials: "same-origin"`, and `cache: "no-store"`. [request.ts](../src/api/http_client/request.ts) rejects an external base path.                                               |
| Session         | The server stores only a hash of the opaque cookie token. `GET /api/auth/session` returns browser-safe `{ authenticated, account }` information, with one immutable Account Product Role and no credential, membership, or role list. |
| Cookies         | The deployed session cookie is host-only, Secure, HttpOnly, first-party, and `SameSite=Lax`.                                                                                                                                          |
| Unsafe requests | The production browser boundary requires canonical HTTPS host and exact same-origin `Origin`; duplicate session cookies are refused.                                                                                                  |
| Caching         | Private Server Route JSON responses use `Cache-Control: no-store`.                                                                                                                                                                    |
| Error detail    | A route returns only the information permitted by the current boundary and does not disclose hidden Account, course, Student, answer, source, renderer, or object state.                                                              |

## Deferred teaching routes

Private imports; Course Instance operations beyond current creation and roster
boundaries; Assignment policy beyond the current fixed-question authoring and
release boundary; individual Student-work inspection; Course Retention; and
iMathAS Question Backend browser boundaries remain retained Store-backed product
requirements. Server composition currently provides none of those later HTTP
routes. The current implementation supplies reusable Blueprint
Course create, list, load, and successor-revision operations; M8 supplies only
Course Instance creation from an exact Blueprint Revision plus its initial
Teaching Team; M9 supplies direct-Instructor roster import, target Student
claim, and access revocation; M10 supplies the narrow direct-Instructor
Assignment Workspace and immutable release; Student-only Assignment Access,
issuance, native submission, recovery, and supported WeBWorK delivery; the
answer-free Gradebook; narrowly scoped account and support operations; and the
direct-Instructor no-store invitation attachment. These current route claims
specify the boundary exercised by M19; they do not replace the separate
connected production-browser evidence recorded for that acceptance.

When implemented, each route uses the session-derived Account plus exact stored
relationships. Course and Assignment references locate a resource only after
that authority check; they do not grant authority. Student delivery requires an
allowed Assignment Access decision for the exact Student, Course, and
Assignment. Answer Keys, Question Graders, Question Source data, object
addresses, credentials, raw backend results, and Question Attempt Reproduction
Details remain outside browser responses.

## Instructor Assignment Workspace

M10's current Course-scoped workspace checks the authenticated Instructor's
direct active Course Membership before resolving the Assignment. It owns only
fixed Available Published Question selection, title/instructions, release
validation, an answer-free Assignment Preview, and immutable release. The
Preview is not the retained Student View Scenario contract: it has no Student
identity, Student work, response, answer, feedback, delivery, grade, or preview
state. Assignment policy and a Student View Scenario remain separate future
operations.

The M10 direct-Instructor save compares the exact Assignment Edit Number and
resolves its Course-local wall-time due value against the latest Course Schedule
Revision. Release snapshots that resolved due value and its Late Work Rule into
the immutable Assignment Revision; later Student start uses that snapshot.

## Student Assignment Access and initial issuance

M11 accepts only public `C-` and `A-` references and requires the signed-in
Student's exact active Student Record for that Course. The server evaluates the
released snapshot at authoritative time and refuses a new issue before an
unavailable, closed, attempt-limited, or due/late-rejected Assignment can create
Student work. A successful start atomically creates or resumes one Assignment
Attempt and issues or resumes one full answer-free QuestionPresentation pinned
to the exact Question Revision. It carries a 53-bit OS-random public
QuestionSeed, nonce, title, prompt, and response format. The server resolves
the source pin and S3 object privately. Its immutable one-to-one
QuestionPresentation binding retains only nonce and full descriptor checksum;
reproduction details remain private Question Attempt/source-binding facts, and
resume reproduces the same public presentation. It exposes no internal locator, source,
checksum, reproduction data, answer, grading input, feedback, submission, or
grade; every response is `no-store`. Native response controls and submission
are implemented under their separate Student-only route boundary.

Structural content edits that conflict with issued Student activity use the
typed recovery contract `SuccessorAssignmentRevisionRequired`. It carries the
immutable `baseRevision` pinned by existing Student work. Visible guidance calls
the outcome a **Successor Assignment Revision**. The server-owned successor
creation command and its Server Route remain future work, so this document
does not claim a currently available edit-recovery endpoint.

### Student delivery and grading boundary

The Student route supplies only the Question presentation and controls
authorized for the exact Assignment Attempt. An accepted submission creates one
receipt. Status and recovery flows preserve the accepted private response rather
than asking the Student to resubmit it. Status is limited to its bound
presentation nonce and grading state; Student Feedback is a separate
policy-evaluated projection. Current grading operations do not reveal Student
responses, Answer Keys, private feedback internals, private source, or raw
grading input.

### Future mutation safety

Future mutation routes accept only values the server cannot derive from
authenticated state and stored records. Strong revisions protect concurrent
edits. The exact operation identity, Request Checksum, and accepted Receipt
govern a repeated write request and grant no authority. Browser input never
creates server-owned durable identity or chooses storage paths.

## Browser and acceptance boundary

The browser treats network JSON as hostile: route-specific decoders bound the
body, verify content type and closed shape, reject unknown values, and confirm
returned relationships before exposing a typed value. Browser types and Wasm
represent data; they do not establish authorization.

The local stack aggregate and each M12-M18 focused gate retain their narrower
claims. M19 separately accepted the single connected production-browser journey
on 2026-09-07; see [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) and
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

## Change control

A future Server Route may exist only with its Rust owner, Store and authorization
boundary, browser client and strict decoder, appropriate focused evidence, and
an update to this route map. A compatibility adapter has a named removal
condition and preserves server ownership rather than making a browser-supplied
authority field authoritative.
