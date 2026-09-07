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
health, session resolution/logout, the deployment-gated seeded Live Demo
account selector, Instructor Question Library, M6 private Question authoring
and publication, M7 Blueprint Course lifecycle, and M8 Course Instance
creation and initial Teaching Team, and M9 Course Roster Import, invitation
claim, and access revocation. Route modules absent
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
| Course Instance creation | `GET /api/course-instance-creation/instructors` | Requires the stored active Sysadmin role and returns bounded active Instructor `U-` references for the sole creation-assignment choice; it returns no account names or emails. | [course_instance.rs](../crates/server/src/course_instance.rs) |
| Course Roster | `GET /api/course-instances/{course_instance_reference}/roster` | Requires the caller's direct active Instructor Course Membership and returns only that Course Instance's course-scoped roster email, roster ID, and pending/active state. Student, anonymous, foreign-course, and Sysadmin callers receive the same `404` concealment. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Roster Import | `POST /api/course-instances/{course_instance_reference}/roster` | Requires the same direct Instructor authority. The bounded atomic import resolves or creates each Student Account by immutable Student Authentication Email, records only a pending Course Invitation and course-scoped roster profile, and creates no Student Record, membership, Assignment, or Student work. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Invitation claim | `POST /api/course-instances/{course_instance_reference}/roster/claim` | Requires the authenticated active Student who is the pending invitation target. It creates or reuses the exact Student Record and creates an active Student Course Membership, without Assignment or Student-work state. | [course_roster.rs](../crates/server/src/course_roster.rs) |
| Course Roster access revocation | `POST /api/course-instances/{course_instance_reference}/roster/{roster_id}/revoke` | Requires the direct current Instructor. It records an immutable ended Student Course Membership or revoked pending Course Invitation and retains protected educational records. | [course_roster.rs](../crates/server/src/course_roster.rs) |

The seeded routes are present when at least one valid, unambiguous deployment
mapping survives. The five-persona set remains closed, but a configured demo
Account can be unavailable without removing the surviving entries; the bounded
unavailable count discloses no Account or mapping detail. The selector replaces
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

Private imports; Course Instance operations beyond M8 creation and initial Teaching
Team; Course Roster Import delivery beyond M9's pending invitation/claim boundary;
assignment workspace; Student delivery and
Question submission; automated grading; Gradebook; Student-work inspection;
object delivery; Course Retention; and iMathAS Question Backend browser
boundaries are retained Store-backed product requirements. Server composition
currently provides none of those HTTP routes. M7 supplies reusable Blueprint
Course create, list, load, and successor-revision operations; M8 supplies only
Course Instance creation from an exact Blueprint Revision plus its initial
Teaching Team; and M9 supplies direct-Instructor roster import, target Student
claim, and access revocation. Blueprint Operations and all Assignment teaching
delivery remain deferred.

When implemented, each route uses the session-derived Account plus exact stored
relationships. Course and Assignment references locate a resource only after
that authority check; they do not grant authority. Student delivery requires an
allowed Assignment Access decision for the exact Student, Course, and
Assignment. Answer Keys, Question Graders, Question Source data, object
addresses, credentials, raw backend results, and Question Attempt Reproduction
Details remain outside browser responses.

## Instructor assignment workspace

The future workspace is course-scoped and checks the authenticated Instructor's
exact relationship to the Course before resolving the Assignment. It owns
authoring content, delivery rules, and answer-free Student inspection through
separate explicit operations. A future Student view never creates an Assignment
Attempt, Question Attempt, Question Submission, grade, or preview state.

Structural content edits that conflict with issued Student activity use the
typed recovery contract `SuccessorAssignmentRevisionRequired`. It carries the
immutable `baseRevision` pinned by existing Student work. Visible guidance calls
the outcome a **Successor Assignment Revision**. The server-owned successor
creation command and its Server Route remain future work, so this document
does not claim a currently available edit-recovery endpoint.

### Future Student delivery and grading

The future Student route supplies only the Question presentation and controls
authorized for the exact Assignment Attempt. An accepted submission creates one
receipt. Status and recovery flows remain answer-free and preserve the accepted
private response rather than asking the Student to resubmit it. Future grading
operations expose bounded metadata and authorized recovery actions without
revealing Student responses, Answer Keys, private feedback internals, private
source, or raw grading input.

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

The local stack's current aggregate acceptance is service evidence. A passing
aggregate does not prove a visible production-browser course, workspace,
delivery, submission, or grading journey. Production browser restoration is a
separate release-blocking requirement; see
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) and
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

## Change control

A future Server Route may exist only with its Rust owner, Store and authorization
boundary, browser client and strict decoder, appropriate focused evidence, and
an update to this route map. A compatibility adapter has a named removal
condition and preserves server ownership rather than making a browser-supplied
authority field authoritative.
