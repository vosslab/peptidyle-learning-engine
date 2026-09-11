# Code architecture

## Overview

Peptidyle Learning Engine (PLE) is a question-agnostic learning platform. A
SolidJS browser client renders answer-free server reader data and results. Rust owns
identity, authorization, course and assignment state, grading, and durable
records. PostgreSQL is the production persistence authority; the in-memory
Store is a deterministic contract-test adapter, not a runtime fallback.

The course model has one reusable source aggregate and one delivery aggregate:

```text
BlueprintCourse (reusable, revisioned, answer-free source)
  +- ordered BlueprintModule
     \`- ordered BlueprintAssignment
        \`- exact published QuestionRevisionReference pins

CourseInstance (private teaching aggregate)
  +- immutable BlueprintCourse parent and applied revision
  +- copied Blueprint Revision Content and resolved delivery settings
  \`- Students, releases, deadlines, accommodations, grades, and activity
```

`BlueprintCourse` has no Students, live deadlines, releases, accommodations,
grades, or delivery settings. Drafts are private to their owner and authorized
workspace collaborators. An explicitly published `BlueprintCourseView` is visible and
reusable by every vetted Instructor. `CourseInstance` is private to its
current equal Teaching Team Members and enrolled Students. Every instance has exactly
one immutable Blueprint parent and records the applied Blueprint revision.

The former product-level Alpha/Blueprint split is not part of this architecture.
Current terminology corrections name the exact legacy source, generated output,
or immutable historical SQL that they replace.

## System shape

```text
browser
  SolidJS application + answer-free Rust WebAssembly
        |
        | same-origin HTTPS and strict browser-safe JSON
        v
gateway/load balancer -> Rust API -> PostgreSQL
                          |          |
                          |          \`-> forced RLS and Authorization Checks
                          v
                     workers and typed object domains
                          |
                          \`-> Question Backends and public-asset publisher
```

The browser and Wasm facade format values, validate non-secret response shape,
and display server decisions. They never receive answer keys, grading payloads,
private source, or authority to create a course record. The API authenticates
and preflights the active session before decoding protected path, query, or body
values. PostgreSQL rechecks Account, workspace, course, Student, and worker lease
scope under forced row-level security.

## Course boundaries

### BlueprintCourse

`crates/question_model/src/blueprint_course.rs` owns the reusable meaning.
The aggregate contains a title, reviewed Question Authorship/publication state, one strong
revision, ordered modules, ordered Blueprint Assignments, policy defaults,
relative schedule defaults, evidence context, and public Question ID members.
The Store resolves each public Question ID to an exact immutable publication pin
before committing a complete Blueprint Revision Content replacement.

Relative availability, due, and close values retain calendar-day and local-wall-
clock meaning. They are defaults, not live deadlines. A semantic no-op keeps the
revision; a stale or invalid replacement changes nothing. Publishing and later
controlled parent updates are explicit actions and never silently tether a
CourseInstance to a moving source.

### CourseInstance

`CourseInstance` is created or updated through the separate Blueprint-operation boundary.
`crates/question_model/src/blueprint_operations.rs` owns source observations,
target-term schedule resolution, previews, commands, Course Origin and Assignment Source Record
creation, receipts, and
Apply Blueprint Update semantics. Create Course from Blueprint copies reusable
Blueprint Revision Content, policy/theme defaults, reviewed offsets, and normalized manifests
into an exact destination `CourseId`; it never copies Students, invitations,
Course Memberships, accommodations, Assignment Attempts, Student Responses, grades, or issued
evidence.

New assignments added to a BlueprintCourse propagate to daughter instances as
unreleased Assignments. An Instructor explicitly releases each new delivery
assignment through the CourseInstance boundary. Delivery edits remain private
to the instance. Untouched imports may fast-forward before issued work;
divergent assignments require explicit selected copying or a new source-derived
assignment. Copy Course for New Term and Shift Course Dates are separate Course Instance operations.

## Major components

| Component            | Canonical owner                                         | Responsibility                                                                                                                                                                                      |
| -------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question model       | `crates/question_model/`                                | BlueprintCourse tree, typed references, exact question identities, assignment meaning, Blueprint-operation contracts, previews, and browser-safe reader types.                                      |
| Domain               | `crates/domain/`                                        | Pure timing, policy, disclosure, Assignment Attempt, scoring, generation, and validation behavior without database or wall-clock reads.                                                             |
| Grading              | `crates/grading/`                                       | Answer-bearing checkers and correctness decisions; server-only and outside the Wasm dependency closure.                                                                                             |
| Learning data access | `crates/learning-data-access/src/`                      | Current Account Session, Assignment Workspace, Course, and Student Attempt Store boundaries. Student delivery reads direct caller-bound Attempt context, progress, and one presentation; it saves canonical working input and finalizes the whole Attempt into immutable submission and grading work. |
| PostgreSQL modules   | `crates/learning-data-access/src/postgres/`             | Current connection, migration, and Store implementations. Assignment delivery invokes narrow session-authorized procedures for public `R-n` context/progress/presentation, saved-response persistence, and final Attempt submission. |
| Server               | `crates/server/src/`                                    | Current HTTP/cookie boundary, Course and Workspace routes, and `assignment_delivery.rs` Student routes. The delivery router authenticates the Student, validates typed references, reproduces one presentation, converts only presentation-scoped response references, and emits no-store projections. |
| Generated contracts  | `crates/project-tools/src/tsgen.rs` -> `generated/api/` | Derivative TypeScript DTOs generated from Rust contract roots; generated files are not hand-edited.                                                                                                 |
| Browser              | `src/`                                                  | Application Shell, strict decoding, route/page state, Assignment Workspace, Course, and Student delivery. `assignment_attempt_page.tsx` renders one selected Question at a time from the direct Attempt context/progress/presentation contract and keeps saved working input distinct from final submission. |
| Object storage       | `crates/objects/`                                       | Typed keys, checksums, image ingress, and the `public-assets`, `private-content`, `student-records`, and `temp-processing` domains.                                                                 |
| Adapters             | `crates/adapters/`                                      | Bounded PLE, iMathAS, and WeBWorK Question Backends, QTI Import, and H5P Package support behind the shared Question operations.                                                                     |

The implemented interface inventory also includes self-only Instructor Profile and thumbnail handling in
[`crates/server/src/instructor_profile.rs`](../crates/server/src/instructor_profile.rs) and
[`src/pages/instructor_profile_page.tsx`](../src/pages/instructor_profile_page.tsx); the
cross-Course Due Soon reader and page in
[`crates/server/src/assignment_release.rs`](../crates/server/src/assignment_release.rs) and
[`src/pages/assignments_due_soon_page.tsx`](../src/pages/assignments_due_soon_page.tsx); and the
selected completed-Attempt history reader in
[`crates/server/src/assignment_delivery/history.rs`](../crates/server/src/assignment_delivery/history.rs).
The history reader resolves only the owned completed Attempt's pinned source evidence and returns
independently disclosed teaching content without regrading.

The current server composition is
[`crates/server/src/composition.rs`](../crates/server/src/composition.rs).
`production_router_from_env()` constructs PostgreSQL Stores for sessions,
Question Library, authoring/publication, Blueprint and Course Instances, Course
appearance, roster, Instructor accounts, support, invitations, Gradebook,
Student Course landing, Assignment Workspace/release, Student delivery, native
PLE submission status, and Question Asset delivery. It binds Question Library,
authoring, delivery, and the private WeBWorK adapter to server-only object
handling. The router merges health, Account Session, deployment-gated Live
Demo, each listed owned route family, then applies the browser cookie boundary
and HTTP security headers. Native PLE and WeBWorK workers have separate
composition paths with dedicated database logins and no HTTP listener.

## Persistence ownership

The current Learning Data Access inventory is intentionally focused: Account
Session, authentication ceremony and email, Assignment Attempt, Question
Source, workspace Question Source object records, reusable Blueprint Course
lifecycle, Course Instance creation and initial Teaching Team, Course Roster
Import/claim/revocation, direct-Instructor Assignment Workspace/release,
Student delivery, grading status, pagination, and iMathAS Question Backend
Session contracts, Memory support, and PostgreSQL modules. M7 stores published reusable content as
immutable Blueprint Revisions with exact Question Revision References, closed
Blueprint Course Read Access, and checksum-verified content. M8 atomically
creates a Course Instance from an exact Available published Blueprint Revision,
retaining Course Origin, Course Schedule Revision 1, and the initial Instructor
Course Membership. M9 separately imports a pending Course Invitation, then
creates the exact Student Record and Student Course Membership only when its
authenticated target claims it. The Assignment Workspace stores Course-owned
authoring and release separately from Student work. Its answer-free Assignment
Preview has no Student identity, response, answer, feedback, or delivery state.
Student Assignment Access authorizes the signed-in Student's exact active
Student Record for public Course and Assignment references, calculates access
at authoritative time, and starts or resumes a public `R-n` Attempt. That
Attempt owns caller-bound context, answer-free progress, and one selected
Question presentation. The server resolves source pins and objects privately;
its presentation binding retains only the nonce and descriptor checksum. A
response save records canonical working input for one issued position, and one
explicit finalization turns all saved positions into immutable submission and
grading work. Presentation and acknowledgement projections expose no Answer
Key, grading input, feedback, or grade. Blueprint-operation persistence remains
future work with its own Store, PostgreSQL/RLS authority, service routes, and
browser integration.

The current Question Model owns Blueprint-operation contracts. M7 implements
reusable Blueprint Course creation, reader access, and successor-revision
publication. M8 implements the narrower live Course Instance bootstrap from one
exact Available published Blueprint Revision. The remaining Blueprint-operation
persistence, PostgreSQL/RLS authority, service routes, and browser integration
remain future work. When implemented, the Store boundary must own Fork Blueprint
Course, Copy Assignment from Blueprint, Apply Blueprint Update, Copy Course for
New Term, and Shift Course Dates, plus the full operation contract for Create
Course from Blueprint. It must use each exact operation identity and request
checksum, preserve immutable receipts separately from repairable current read
results, and keep Assignment Import Repair bounded to derived state.

The checked-in pre-production migration sequence and forward allocation rule are documented in
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md). The immutable
`2026081837_blueprint_alpha_curriculum.sql` and accepted successor migrations
are historical implementation evidence and are not edited to disguise the old
split.

## Future Blueprint-operation data flow

```text
Instructor creates or edits BlueprintCourse
  -> question_model validates ordered tree and relative defaults
  -> future Store resolves public Question IDs to exact published pins
  -> future PostgreSQL/RLS boundary authorizes the exact operation
  -> future strict server DTO and generated TypeScript declaration
  -> future browser sends a request-retry-bound command
  -> server resolves term/zone and DST corrections
  -> atomic apply binds one Blueprint revision to one CourseId
  -> CourseInstance owns delivery, student records, release, and grading state
```

The retained contracts select nested Blueprint assignments by typed reference
plus module and assignment positions. Copy Assignment from Blueprint targets an
existing exact CourseInstance; Create Course from Blueprint creates a new
CourseInstance; Fork Blueprint Course creates an independent BlueprintCourse
with immutable source lineage. Apply Blueprint Update, Copy Course for New
Term, and Shift Course Dates retain their distinct operation contracts. The
future boundary must preserve CourseInstance and FERPA authority limits.

## Assessment and delivery boundary

Published Questions remain shared Question Library content. Course assignments, Student
records, Assignment Attempts, Question Attempts, responses, grades, accommodations, and issued evidence
are exact CourseInstance records. The normal assessment flow is:

```text
published immutable question
  -> Course Instance Assignment and Assignment Access
  -> caller-owned, answer-free Assignment Attempt presentation
  -> durable canonical saved response for each issued position
  -> whole-Attempt immutable submissions
  -> sealed worker grading and receipt
  -> current policy-controlled score and Student Feedback result
```

`crates/grading/` remains server-only. `crates/wasm/` may validate response
format and timing inputs, but it cannot grade or authorize. iMathAS and WeBWorK
Question Backend execution is server-side, and its output is treated as untrusted input.

## Browser and runtime topology

`src/application_shell.tsx` owns the one persistent Application Shell, content
origin, skip-link/focus boundary, and the mounted Ribbon. `src/ribbon/` owns
the ordered catalog and schema, capability-registry admission, scope resolution,
selected/pending presentation, and fixed-row rendering. The catalog may retain a
future destination's designed identity, but the registry admits it only after a
complete usable path is backed; a route identity, fixture, or label does not
claim a Browser Surface, Service, Server Route, or authorization grant.

`src/api/` decodes every response from `unknown`; `src/features/` and
`src/pages/` own visible page behavior beneath the shell; `crates/wasm/` provides
the one browser-safe Rust bridge. The production `dist/` artifact is served
through the same-origin gateway in the disposable live-demo owner. PostgreSQL,
MinIO, API, worker, gateway, and private renderer compose the local
production-shaped topology.

`local_stack_control/` owns typed lifecycle, readiness, leases, scoped cleanup,
and acceptance composition. The disposable API readiness route checks the
database, object store, renderer, and worker; its public body identifies only a
closed unavailable category, while the worker's socket stays private and is not
an HTTP or Job-execution surface. It is operational infrastructure, not a
second application architecture. `deploy/opentofu/` describes the AWS target
but does not prove that production activation has occurred.

## Testing and verification

Validation follows [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md):

- Current permanent Rust and Node tests protect implemented contracts, strict
  DTO decoding, answer-free browser reader data, and deterministic Application
  Shell/Ribbon behavior.
- Current disposable PostgreSQL/RLS oracles prove the applied schema and
  service authority boundaries. The M5 saved-response and context SQL oracles
  pass; its keyboard, reload, timer, and browser axe journey remain pending.
  The M5 real-stack runner separately proves
  Instructor Question Library browse/detail plus Student and anonymous
  concealment. M6 proves private Instructor Draft Question creation, exact
  Edit Number save conflict, publication, and Question Library handoff. M7
  proves reusable Blueprint Course Owner/Active Instructor read access,
  successor-revision preservation, and visible creation/publication. M8 proves
  the exact Blueprint Revision source, immutable Course Origin, initial Assigned
  Instructor Course Membership, no ambient Sysadmin Course access, and visible
  Course Instance creation/Teaching Team. M9 proves Course Roster Import,
  invitation claim, Student Record/current Student Course Membership, immediate
  revocation, and visible Instructor roster projection. These focused runners
  are not a complete browser-owner or teaching-workflow claim.
- A future Blueprint-operation implementation requires focused contract,
  PostgreSQL/RLS, generated-contract, and browser acceptance evidence for its
  exact operations before the visible workflow is treated as current.
- Graphify, source inventories, migration registration, generated-file
  regeneration, and screenshot publication are one-time implementation
  evidence. They are not recurring pytest or Node gates.
- Rendered screenshots and independent visual/accessibility review remain
  separate human evidence; artifact counts and pixel identity are not behavior
  gates.

A focused documentation check may validate Markdown links, ASCII compliance,
and whitespace. Documentation success does not close an unrun required runtime,
database, browser, or human-review gate.

## Dependency order

The dependency sequence names the capability that each implementation layer
must establish:

```text
foundational decisions and inventory
  -> domain and authorization contracts
     -> PostgreSQL schema, RLS, and protected functions
        -> Store implementations
           -> server, workers, objects, and adapters
              -> API, generated TypeScript, browser, and Live Demo
                 -> connected real-stack evidence and release closure
```

Within the future Blueprint-operation work, preserve the existing Rust
Question Model contracts, then implement the Store, the allocated SQL epoch,
PostgreSQL/RLS parity, server routes and policy, generated TypeScript, strict
browser decoders and clients, and finally the visible workflow. Run focused
permanent tests at each implemented boundary; connected PostgreSQL, Playwright,
screenshot, and human-review evidence follows the coherent implementation.

## Extension points

- Add reusable course meaning in the Rust Question Model BlueprintCourse
  contracts.
- Add Blueprint-operation persistence through the future Store with an exact
  source, destination, revision, authorization, preview, apply, request-retry,
  and receipt contract.
- Add PostgreSQL schema only through the status-owned forward migration
  allocation; preserve applied migrations as history.
- Add server routes under the owning capability module and register them through
  the composition root and route policy.
- Regenerate `generated/api/` from Rust after contract changes, then update
  strict decoders and typed browser clients.
- Add browser behavior in the owning API client, feature, page, or component;
  keep CourseInstance delivery decisions server-authoritative.
- Add Question Backend behavior through an adapter with a safe public
  Question Presentation and a server-only grading handoff.

## Known gaps

- The current working source still contains paired legacy reusable-curriculum
  symbols and routes. Their removal spans Question Model, Store, SQL, server,
  generated output, and browser boundaries.
- The fresh pre-production migration epoch and its exact per-file Migration
  Allocation Registry remain owned by [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md);
  this document does not allocate migrations.
- Production AWS activation, SMTP delivery attestation, institutional
  FERPA/legal sign-off, and human pilot acceptance remain separate release
  evidence classes.
