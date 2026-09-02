# Assessment lifecycle

This document is the durable map of how one assessment item moves through PLE.
It connects authoring, immutable publication, course activity, grading, and
privacy cleanup without redefining their detailed contracts. The active release
plan remains the source of truth for package status and acceptance evidence.

## Status and scope

The ownership, publication, activity, grading, and retention semantics below
are the durable platform design. The precise minimal student payload described
in "Submit, grade, and project" is the accepted WP-P1 through WP-P6 target
contract, not a claim that the current broader student DTO has already been
replaced. [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) labels
the current boundary separately from the target cutover. Consult the active
release plan before treating a backend or payload package as accepted.

## Lifecycle at a glance

```text
private draft
  -> validate and preview
  -> publish immutable version
  -> select exact version for assignment
  -> create or resume a course-owned Assignment Attempt
  -> issue one server-owned Question Attempt
  -> render the active Question Attempt and optionally reserve the next
  -> accept one Submitted Response exactly once
  -> grade only on the server
  -> project permitted feedback and summary
  -> continue a new varied practice Assignment Attempt when policy permits
  -> retain, archive, then delete student records
                         \
                          -> preserve identity-free statistics
```

The arrow is an ownership change, not merely a screen change. A browser can
read an answer-free projection and propose a response; it cannot choose a
installation, published version, seed, deadline, grading backend, score, or deletion
scope. The server derives those facts from authenticated course-owned records.

## Ownership and identities

PLE keeps four related but different things separate:

| Thing               | Owner and lifetime                        | Important identity                                       |
| ------------------- | ----------------------------------------- | -------------------------------------------------------- |
| Draft               | Instructor workspace; private and mutable | `WorkspaceId`                                            |
| Published question  | Shared immutable Question Library content | `QuestionId` and `QuestionRevisionNumber`                |
| Assignment activity | One course's teaching configuration       | Course and assignment IDs                                |
| Student activity    | Course-owned educational record           | Enrollment, Assignment Attempt, and Question Attempt IDs |

Publication is the boundary between the first two rows. Every content change
publishes a new immutable question with a fresh Question ID and fresh hidden
`(QuestionId, QuestionRevisionNumber)` pair; optional one-way provenance may identify its
source. An Assignment, Assignment Attempt, or Question Attempt retains its exact
pinned pair and does not copy prompt, assets, source, or answer material into the
course. An Assignment Attempt is one pass through an Assignment, and a Question
Attempt is one issued instance of one assignment position. Repeated use of the
same exact published question does not merge distinct assignment positions or
Question Attempts.

The type-level identity and browser-safety rules are defined in
[QUESTION_MODEL.md](QUESTION_MODEL.md). The enrollment, Assignment Attempt, Question Attempt, and
summary records are defined in [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).

## Author, validate, publish

### 1. Author a private draft

An instructor or authorized collaborator edits an unversioned workspace draft.
The draft can contain answer-bearing source, author feedback, and provider
details, so it remains private. Browser preview is useful but is never
publication authority: the server reruns the same validation against the stored
draft and trusted adapter capability declaration.

The draft editor uses a strong revision precondition. A stale save, review,
conversion, deletion, or publication request conflicts instead of overwriting
newer author work. A failure leaves the prior draft intact and creates neither
a public Question identity nor a partial publication.

### 2. Validate delivery capabilities

Before publication, PLE checks the selected question's declared capabilities
against the assignment delivery requirements. This checks the full requested
set, rather than stopping at the first gap, so the instructor can correct the
whole configuration. Browser/Wasm validation is early feedback; the server
repeats it before writing a durable transition.

### 3. Commit an immutable publication

The server resolves the workspace-owned draft, validates it, mints a fresh Question
ID and hidden `(QuestionId, QuestionRevisionNumber)` pair only after success, and commits
immutable metadata, public payload, private grader material or source binding,
visibility grant, and draft removal as one transaction. A publication never
mutates an existing published question. Every content change publishes a new
question; optional one-way provenance may identify its source. A deliberate,
revision-checked assignment replacement changes future Assignment Attempts only, while issued
Assignment Attempts and Question Attempts retain their original exact evidence.

Object storage follows the same boundary. The database records intended object
existence and typed object identities; it does not give a browser a bucket key
or source URL. Immutable writes reject overwrite and reads verify their
checksum. The publication authorization and object rules are in
[SECURITY_MODEL.md](SECURITY_MODEL.md) and [OBJECT_STORAGE.md](OBJECT_STORAGE.md).

## Select and start activity

### 4. Build an assignment from versions

An Assignment stores ordered references to Published Question Revisions,
completion, grade, continued-practice, Question Variation Rule, disclosure, and
its exact authored content and policies. New Assignments are Unreleased. The
Instructor explicitly releases an Assignment after setting instructions,
availability/due/close, whole-Assignment-Attempt and Question Attempt limits,
late behavior, and deadline behavior. Course-local input is resolved by the
server through the stored IANA zone; only the resulting absolute Base Assignment
Policy is durable. These policies are intentionally independent in the domain
model.
The instructor UI can present teaching-oriented assignment types while storing
their explicit policy values.

The assignment belongs to one course. Enrolling a student creates a
course-owned educational relationship, not a copy of shared question content.
The authenticated session supplies the user identity; the server verifies that
the Student Record belongs to that Account and Course Instance rather than assuming `AccountId` and `StudentRecordId` are
interchangeable.

### 5. Create or resume an Assignment Attempt

The server starts the initial Assignment Attempt or resumes the one active Assignment Attempt that belongs to
the enrollment only after current S5 entitlement and S3 resolution. Stored
Released Assignment Status is the sole Student-accessible G1 state; Unreleased,
Closed, and Archived Assignments do not start Student work. It assigns server
timestamps, one-based Assignment Attempt number, and
the Question Variation Rule actually used. Completion is derived from attempt states;
it is not a mutable Boolean that can disagree with the attempt history. Attempt
limits count completed Assignment Attempts, so the final allowed active Assignment Attempt remains resumable
instead of denying itself.

Completion is a milestone, not a lockout. When the policy permits continued
practice, the student can start another Assignment Attempt while retaining its
Questions with fresh Question Seeds. For a typical mastery assignment this means
all-correct completion, highest-score selection, unlimited later Assignment Attempts,
the retain-Questions-with-fresh-Seeds rule, and five Assignment disclosure fields set to
`AfterSubmit`. The exact composition remains an assignment decision, described in
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).

## Issue and present

### 6. Issue exactly one active Question Attempt

The Assignment Attempt service issues at most one unresolved Question Attempt at a time. The Question Attempt
binds the authenticated Student and Course through its enrollment and Assignment Attempt, the
assignment position, immutable version, seed, policy, timing state, grader
backend, and Question Attempt Reproduction Details. Resume returns the stored attempt and stored seed; it
does not generate a different problem mid-attempt.

Question Attempt issuance is a transactional storage operation. PostgreSQL locks the Assignment Attempt
and its equivalent Store contract enforces the same invariant, so concurrent
requests cannot create two active timers. Server timestamps decide issue time,
deadline, arrival time, completion, and timer verdict. The browser timer is a
display and submission aid, never the timing authority.

### 7. Render an answer-free screen

The student receives a public render envelope and the smallest state needed to
use it. Rich render data includes prompt blocks, sanitized markup, accessible
asset references, Question Response Format, item order, and public constraints. It may
also include seed and version to identify the public render. It excludes correct
answers, expected values, private rubrics, raw sources, provider credentials,
upstream fields, storage locations, and grader state.

An attempt-specific presentation binding protects against a valid but wrong
render being submitted for the wrong attempt. Each selectable object has a
small rendered-item ID; the full public descriptor has a presentation checksum.
The Question Presentation Checksum and its public Question Presentation Token are consistency checks, not authentication mechanisms or transport
checksum. The exact wire contract, CRC16 collision rule, readiness requirement,
and mismatch recovery are in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

### 8. Reserve one next question safely

When policy allows it, PLE may prepare one next question while the student is
working. A prefetch reservation is Course-, Student-, Assignment Attempt-, predecessor-, and
position-bound. It has no Question Attempt ID, response, grade, or started timer.

At the secure-payload target boundary, an untimed-practice browser may hold the
answer-free envelope in memory and warm a bounded set of same-origin assets.
For timed or exam work, PLE may prepare privately but withholds the next
envelope until the predecessor commits. The current bodyless prefetch route has
not yet enforced this timing-policy distinction. Only an exact committed receipt
promotes a reservation to an issued attempt. A reload, mismatch, cancellation,
or route exit discards the browser cache; it does not invent another seed or
advance an Assignment Attempt.

## Submit, grade, and project

### 9. Submit the minimal response

At the secure-payload target boundary, the route identifies the attempt once.
The request supplies only the presentation checksum and a Question-Type-minimal answer;
a bounded idempotency key is in the request header. The server loads the
authoritative attempt and therefore derives response shape, question revision,
seed, assignment, backend, deadline, and student ownership rather than
accepting browser copies.

The server rejects a digest mismatch before grading and keeps the attempt
unchanged. The browser reloads the same attempt, retains compatible unsent work
in memory, and asks the student to review it. Repeating the same idempotency
key and same response returns the first committed receipt. Reusing a key or
attempt with a changed response conflicts before a second grade or state
transition occurs.

### 10. Grade under server authority

The server first validates response structure against the issued public schema,
then invokes the selected trusted backend. All answer normalization,
correctness, component credit, partial-credit computation, and score selection
stay server-side. The browser never submits a score, component weight, answer
key, or correctness assertion for ordinary grading. The current tagged
`StudentResponse` route accepts `kind`, but derives and validates the
expected Question Type from the issued attempt; it is not submission authority.

Acceptance first commits the validated response, immutable issued-work witness,
pending evaluation, execution record, and ready grading job as one transaction.
The student receives `202 Accepted` and may read the route-bound submission
status while the sealed worker owns grading. A successful worker transaction
then commits the grading result, score event, Question Attempt transition, Assignment Attempt completion,
enrollment pointers, summary projection, successor receipt, and immutable
idempotency receipt together. The completed receipt copies the issued,
answer-free `QuestionPresentation` and exact public Presented Question Asset snapshot.
Replay and status reads therefore use durable accepted or completed evidence,
never a newer published Question/backend render, and never re-grade an answer.

### 11. Return a policy-projected receipt

While grading is pending, the student receives only accepted status and the
committed attempt identity, with an accessible action to check grading status.
After completion, the student receives policy-permitted correctness and points,
sanitized feedback, and either an immutable `nextIssued` descriptor or
`nextPending`. The Store evaluates student
disclosure at projection time only after S5 entitlement, from the current
S3-resolved effective-policy verdict, assignment-owned policy, authoritative
time, and the submitted fact; the request cannot choose it. The historical S3
receipt remains immutable attempt evidence, not a disclosure input.
`nextPending` means the grade receipt succeeded but successor
delivery has not; recovery may finish that single pending delivery, while a
replay never resubmits or consults changed published Question/backend state. Withheld
feedback remains withheld even though the result is persisted. An instructor or
gradebook view reads the summary projection and lazily paged history rather
than recomputing a grade by scanning all attempts. A separate scoring freshness
state can mark the maintained summary Recalculating or Failed; student routes
then omit aggregate and Assignment Attempt scores, Grading Results, and disclosed point values
until it is Current, without changing the student's semantic
activity/disclosure state.

The attempt state machine, feedback policy, timer rule, and summary projection
are detailed in [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md). The narrow current and
target request/receipt shapes are detailed in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Question Backend authority

The common lifecycle deliberately ends at a backend boundary. Adapters can
share public attempt behavior without sharing private grading data or assuming
the same source format.

| Question Backend | Publication authority                                                            | Render authority                                                | Grade authority                                                | Important recovery rule                                                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PLE              | PLE compiles PLE Question JSON source into public definition and server-only key | PLE public renderer                                             | PLE Question Grader                                            | First grade uses the issued checksummed snapshot, private envelope, and PLE Question JSON grading contract; it never reloads a current published Question/grader view |
| QTI              | PLE stages, reports, reviews, and promotes a supported profile atomically        | PLE's opted-in published runtime or converted PLE definition    | Server-only `PostgresGraderStore` when enabled                 | Reparse the checksum-pinned archive; refuse unsupported profile features                                                                                              |
| WeBWorK          | PLE copies licensed PG/PGML source and provenance into immutable storage         | Private external `/render-api`, then PLE sanitizes and projects | Private external renderer through PLE                          | First grade loads the issued presentation, mapping, WebWork grading contract, and immutable source provenance; submitted reads never rerender                         |
| iMathAS          | PLE publishes an answer-free launch control plus trusted backend configuration   | iMathAS Question Backend Session is server-mediated             | iMathAS protocol validation through an iMathAS Result Exchange | Generic attempt records carry no backend token, raw answer, or backend score                                                                                          |

PLE Question JSON Questions use PLE's public `QuestionRevision` plus separate
grader-only material. The exact PLE Question JSON authoring format is
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md), not a second generic
runtime model.

QTI import refuses unsupported input rather than silently dropping semantics.
Published QTI runtime is feature-gated and separates its PostgreSQL grader
access from ordinary public projections. Its profile and promotion contract is
registered in [CONTRACTS.md](CONTRACTS.md).

WeBWorK is a private service integration. PLE sends the trusted source, fixed
seed, and renderer credentials only from the server, turns the approved radio
control into PLE opaque choices, and keeps upstream names, values, source,
cookies, and raw response bodies out of attempts and browsers. The exact
bounded RC3 contract and its release scope are in
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

The iMathAS Question Backend remains deliberately sparse in the generic model. The
backend is not allowed to widen an ordinary student response into a token or
raw payload. iMathAS Question Backend Sessions and transcripts are course-owned student records
with their own authorization and retention handling.

## Failure and recovery semantics

| Boundary                                     | Safe outcome                                                                                                                        |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Draft validation or publication fails        | Keep the private draft; do not mint public identity or create a partial immutable version.                                          |
| Capability check fails                       | Return the complete missing-capability report before publication or assignment persistence.                                         |
| Concurrent issue/resume                      | Lock and return the sole unresolved Question Attempt.                                                                               |
| Public render or asset fails                 | Keep the Assignment Attempt resumable; offer retry without changing seed or Question Attempt.                                       |
| Presentation mismatch                        | Return stable conflict, persist bounded diagnostic evidence, reload the same attempt, and never grade stale state.                  |
| Network loss after submit                    | Retry with the same idempotency key and response to receive the committed receipt.                                                  |
| Changed submission replay                    | Conflict before grading or state mutation.                                                                                          |
| Renderer/backend outage                      | Preserve the active attempt; expose a bounded degraded state only for the affected question.                                        |
| Commit interruption after prefetch promotion | Heal only the sole owned, committed-but-unlinked successor; never derive a different successor from later Assignment Attempt state. |
| Retention object failure                     | Keep the course archived and retry the frozen typed-object manifest; never report deletion early.                                   |

These rules make failures visible and recoverable without turning a browser
cache, a renderer response, or a retry into new authority. The more detailed
route, storage, and RLS guarantees are in [SECURITY_MODEL.md](SECURITY_MODEL.md)
and [CONTRACTS.md](CONTRACTS.md).

## Retain records, keep learning

Student records are course-owned and privacy-sensitive. Course policy first
notifies, then archives and fences student access, then permanently deletes the
complete student graph and typed student-record objects. The deletion path uses
a frozen manifest, idempotent object deletion, lease and generation fencing,
and one verified relational purge transaction. It never follows an assignment
reference into shared published content.

Published Questions, immutable Question Revisions, Instructor Drafts, and anonymous
question statistics have different retention rules. A first completed
assignment can contribute an identity-free aggregate exactly once. That
aggregate supports future library improvement but is not a course-local
gradebook or a route back to a student record. The retention defaults, backup
boundary, and permanent-versus-one-time verification policy are in
[RETENTION_POLICY.md](RETENTION_POLICY.md).

## Contract map

Use this lifecycle document to find the right detailed contract:

- [QUESTION_MODEL.md](QUESTION_MODEL.md): public model, durable identities,
  response shapes, generation, and browser-safe type boundary.
- [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md): policy composition, attempt states,
  timing, idempotency, completion, and summary projection.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md): student render,
  rendered IDs, presentation checksum, minimal response, receipt, and prefetch.
- [SECURITY_MODEL.md](SECURITY_MODEL.md): authorization, grading secrecy,
  publication, Assignment Attempt, asset, and retention security boundaries.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md): typed Object Addresses, bucket roles,
  checksums, delivery, and reconciliation.
- [CONTRACTS.md](CONTRACTS.md): module ownership, frozen contracts, and change
  rules.
- [RETENTION_POLICY.md](RETENTION_POLICY.md): privacy lifecycle and anonymous
  aggregate preservation.
- [active_plans/implementation_plan.md](active_plans/implementation_plan.md):
  milestone dependency order and acceptance criteria.
