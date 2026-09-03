# API contract map

This is PLE's durable route-level map. It identifies the presently mounted
same-origin HTTP surface and separates it from retained product contracts. It
does not replace generated Rust and TypeScript shapes.

[CONTRACTS.md](CONTRACTS.md) owns module ownership,
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) owns detailed
authorization decisions, and [SECURITY_MODEL.md](SECURITY_MODEL.md) owns
security and storage rules.

## Status and authority

[composition.rs](../crates/server/src/composition.rs) is the executable
authority for the production route surface. Its current entry point provides only
health, session resolution/logout, and the deployment-gated seeded Live Demo
account selector. Route modules absent from server composition, generated DTOs, browser clients,
schemas, and models retain product design; none establishes an available HTTP
endpoint.

## Mounted routes

| Surface                | Route                               | Current boundary                                                                                | Owner                                                  |
| ---------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Health                 | `GET /health`                       | HTTPS readiness response                                                                        | [composition.rs](../crates/server/src/composition.rs)  |
| Session                | `GET /api/auth/session`             | Resolves the presented opaque session cookie into a browser-safe Authenticated Session response | [auth.rs](../crates/server/src/auth.rs)                |
| Session                | `POST /api/auth/logout`             | Revokes the presented session and clears its cookie                                             | [auth.rs](../crates/server/src/auth.rs)                |
| Seeded Live Demo entry | `GET /api/auth/live-demo/accounts`  | Lists the deployment's closed seeded-persona set, with no Account ID or authorization claim     | [live_demo.rs](../crates/server/src/auth/live_demo.rs) |
| Seeded Live Demo entry | `POST /api/auth/live-demo/accounts` | Resolves one closed seeded persona and issues the ordinary Authenticated Session                | [live_demo.rs](../crates/server/src/auth/live_demo.rs) |

The seeded routes are present only when their complete deployment configuration
is supplied. The selector replaces identity verification for the five known
demo personas. It supplies no Product Role, Course Membership, Student record,
or authority; the server derives those facts from stored PLE records whenever a
future route needs them.

## Mounted protocol rules

| Concern         | Current contract                                                                                                                                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Origin          | Browser requests use a relative same-origin path, `credentials: "same-origin"`, and `cache: "no-store"`. [request.ts](../src/api/http_client/request.ts) rejects an external base path.                                               |
| Session         | The server stores only a hash of the opaque cookie token. `GET /api/auth/session` returns browser-safe `{ authenticated, account }` information, with one immutable Account Product Role and no credential, membership, or role list. |
| Cookies         | The deployed session cookie is host-only, Secure, HttpOnly, first-party, and `SameSite=Lax`.                                                                                                                                          |
| Unsafe requests | The production browser boundary requires canonical HTTPS host and exact same-origin `Origin`; duplicate session cookies are refused.                                                                                                  |
| Caching         | Private mounted JSON responses use `Cache-Control: no-store`.                                                                                                                                                                         |
| Error detail    | A route returns only the information permitted by the current boundary and does not disclose hidden Account, course, Student, answer, source, renderer, or object state.                                                              |

## Deferred teaching routes

Question Library and lifecycle; private authoring and imports; Blueprint Course
and Course Instance operations; roster, invitation, and enrollment; assignment
workspace; Student delivery and Question submission; automated grading;
Gradebook; Student-work inspection; object delivery; Course Retention; and
iMathAS Question Backend browser boundaries are retained Store-backed product
requirements. Server composition currently provides none of these HTTP routes.

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
creation command and its mounted route remain future work, so this document
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
edits. A Request Retry Token binds the exact Request Checksum and accepted
Receipt, returns that Receipt for a repeated write request, and grants no
authority. Browser input never creates server-owned durable identity or chooses
storage paths.

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

Mounting a future route requires its Rust owner, Store and authorization
boundary, browser client and strict decoder, appropriate focused evidence, and
an update to this route map. A compatibility adapter has a named removal
condition and preserves server ownership rather than making a browser-supplied
authority field authoritative.
