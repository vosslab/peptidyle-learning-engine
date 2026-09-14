# Assessment lifecycle

This document is the durable map of how one assessment item moves through PLE.
It connects authoring, immutable publication, course activity, grading, and
privacy cleanup without redefining their detailed contracts. The active release
plan remains the source of truth for package status and acceptance evidence.

## Status and scope

The ownership, publication, activity, grading, and retention semantics below
are the durable platform design. The precise minimal student payload described
in "Submit, grade, and project" is the accepted target contract, not a claim
that the current broader student DTO has already been
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
read an answer-free Question Presentation and propose a Student Response; it cannot choose an
installation, published Question Revision, Question Seed, deadline, Question Backend, score, or deletion
scope. The server derives those facts from authenticated course-owned records.

## Ownership and identities

PLE keeps four related but different things separate:

| Thing               | Owner and lifetime                       | Important identity                                       |
| ------------------- | ---------------------------------------- | -------------------------------------------------------- |
| Draft question      | Authoring Workspace; private and mutable | Draft Question UUID and Draft Question Reference         |
| Published question  | Shared Question Library lineage          | Stable `QuestionId`                                      |
| Question revision   | Immutable source and historical evidence | `QuestionId` and `QuestionRevisionNumber`                |
| Assignment activity | One course's teaching configuration      | Course and assignment IDs                                |
| Student activity    | Course-owned educational record          | Enrollment, Assignment Attempt, and Question Attempt IDs |

Publication is the boundary between the Draft Question and Published Question
lineage. A compatible Question Source change creates a new immutable Question
Revision under the stable Question ID. A substantive fork creates a new
Published Question lineage and may retain an optional Question Fork Source.
Updating lineage metadata such as Question Title or Question Description creates
no Question Revision. An Assignment, Assignment Attempt, or Question Attempt retains its exact
pinned pair and does not copy Question Prompt, assets, Question Source, or Answer Key into the
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

### 3. Publish from isolated draft storage

The server resolves the workspace-owned Draft Question and validates its exact Edit Number,
complete Question Source, and required publication metadata. The publication transaction creates a
fresh Question ID for a new lineage or the next Question Revision Number for an accepted same-lineage
change. It copies validated discovery values from the private Draft Question Metadata table into the
separate Published Question Metadata table, writes the complete source to a new immutable Question
Revision-owned object path, creates the Question Revision Source Binding, and records the
Question Library publication event.

Published storage has no Draft Question foreign key, draft object path, or draft metadata row. Draft
expiration and cleanup proceed separately after the configured warning or recovery period. A change
to the complete Question Source creates a Question Revision; a stable-lineage metadata change such as
Question Title or Question Description does not. A deliberate, revision-checked Assignment update
changes future Assignment Attempts only, while issued Assignment Attempts and Question Attempts
retain their original exact evidence.

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
The Authenticated Session identifies an Account. The server verifies that the
Student Record belongs to that Account in that Course Instance rather than
assuming `AccountId` and `StudentRecordId` are interchangeable.

### 5. Create or resume an Assignment Attempt

The server starts the initial Assignment Attempt or resumes the one active Assignment Attempt that belongs to
the enrollment only after current Active Student Course Membership and Assignment Access evaluation. Stored
Released Assignment Status is the sole Student-accessible G1 state; Unreleased,
Closed, and Archived Assignments do not start Student work. It assigns server
timestamps, one-based Assignment Attempt number, and
the Question Variation Rule actually used. A timed Attempt stores one immutable
`expires_at` when it starts: the earlier of the retained close instant and the
retained start-plus-time-limit instant, or no expiry when neither limit applies.
Later Assignment or accommodation edits do not move that instant. Reconnect,
reload, and another authenticated session for the same Student resume the same
Attempt and its saved responses, as required by the Assignment Attempt philosophy
in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

Completion is derived from attempt states;
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

### 6. Issue one retained Question set for the Assignment Attempt

Starting the Assignment Attempt selects and issues its ordered Question set in one server-owned
operation. Each Question Attempt binds the authenticated Student and Course through the Assignment
Attempt, its fixed position, immutable Question Revision, Question Seed, policy, timing evidence,
Question Grader Version, and Question Attempt Reproduction Details. Resume returns that retained set
and its stored Seeds; it never substitutes a later Question Revision.

Assignment Attempt issuance is transactional. PostgreSQL locks the Student Work root and the Store
contract enforces the same invariant, so concurrent start/resume requests cannot create two active
Assignment Attempts or timers. Server timestamps decide start, expiry, response-save eligibility,
and finalization. The browser countdown is a display aid, never timing authority.

### 7. Render an answer-free screen

The Student receives one position's public Question Presentation and the smallest state needed to
use it. Rich render data includes typed prompt blocks, accessible
asset references, Question Response Format, item order, and public constraints. It may
also include Question Seed and Question Revision to identify the public render. It excludes correct
answers, expected values, private rubrics, raw sources, provider credentials,
upstream fields, storage locations, and grader state.

An Attempt-specific presentation binding protects against saving a valid but wrong render for the
wrong position. Each selectable object has a
small Presentation Response Item Reference; the full public descriptor has a Question Presentation Checksum.
The Question Presentation Checksum and its public Question Presentation Token are consistency checks, not authentication mechanisms or transport
checksum. The exact wire contract, CRC16 collision rule, readiness requirement,
and mismatch refusal are in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

### 8. Save responses and resume the same active Attempt

The Student moves among issued positions and saves each locally valid response while the Assignment
Attempt remains active. A save is bound to the authenticated Student, exact Assignment Attempt, and
fixed position. The server derives the Question Revision, Seed, backend, response format, and timing
from retained evidence rather than accepting those facts from the browser.

Reload, reconnect, or another authenticated browser session resumes the same active Assignment
Attempt and returns its successfully saved responses. Failed browser transport keeps the visible
response available to save again while time remains. None of these operations starts a new Attempt,
pauses or extends its clock, or accepts a grade.

## Submit, grade, and project

### 9. Finalize the whole Assignment Attempt

The Student submits the Assignment Attempt once after saving every intended response. The request
identifies only that Assignment Attempt; the server loads its authoritative Student, Course,
Assignment, issued Questions, saved responses, backend facts, and timing.

A presentation or saved-response mismatch is refused before finalization. The browser reloads the
same active Attempt and asks the Student to review it. Repeating finalization after success returns
the already-submitted state and cannot create another Question Submission or result.

Whole-Assignment submission and deadline finalization use one ordinary submission
path. At or after `expires_at`, every Student operation that could change the
Attempt first applies the expiry rule. Each saved supported response becomes one
immutable Question Submission and receives the Question Backend's immutable
normalized credit outcome when available; each unanswered Question becomes
`closed_at_deadline` and is resolved as zero points. A zero-response Attempt
completes immediately with zero points and creates no Question Submission.
Repeating finalization is a no-op, and a response arriving after expiry is
refused without replacing the previously saved bytes.

The context returns the exact expiry instant in the Student's display time zone
and a remaining duration computed from the same database read. The browser uses
the duration for a monotonic countdown, refreshes authoritative state at zero,
and retries after a lost connection. Background execution finalizes expired
Attempts that no Student revisits. This preserves the
wall-clock, reconnect, saved-work, and automatic-submission commitments in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

### 10. Grade under server authority

The server first validates response structure against the issued public schema,
then invokes the selected trusted backend. All answer normalization,
correctness, component credit, partial-credit computation, and score selection
stay server-side. The browser never submits a score, component weight, answer
key, or correctness assertion for ordinary grading. The current tagged
`StudentResponse` route accepts `kind`, but derives and validates the
expected Question Type from the issued attempt; it is not submission authority.

Whole-Attempt Student finalization or expiry auto-submission commits validated saved responses as
immutable Question Submissions. Each submission remains bound through its Question Attempt to the
immutable Issued Question and private Question Attempt Reproduction Details. A Question Backend
that completes immediately produces its immutable normalized credit result and receipt in that
ordinary submission operation. A backend-specific internal completion path may finish a backend
that requires polling; it exposes no grading lifecycle to Students or Instructors.
Finalization replay and status reads use durable accepted or completed evidence, never a newer
published Question or backend render, and never regrade an answer.
An internal completion commit requires its exact current lease token when a backend requires
polling. A completed immutable outcome is terminal; no user can grade, retry, or replace accepted
work.

### 11. Return policy-projected Attempt status

After submission, separately authorized Student history may expose policy-permitted correctness,
points, and sanitized feedback. Backend-specific completion remains internal where a backend does
not return immediately; it is not a public grading status. The Store
evaluates Student Feedback disclosure only after Active Student Course Membership and Assignment
Access, from the current S3-resolved effective-policy verdict, Assignment-owned policy,
authoritative time, and the submitted fact; the request cannot choose it. The historical S3 receipt
remains immutable Attempt evidence, not a disclosure input. Withheld feedback remains withheld even
though the result is persisted. An Instructor or Gradebook reads the Assignment
Attempt Summary and lazily paged history. Scores are calculated on read from
stored normalized credit fractions and current Assignment Entry point values;
there is no maintained Assignment total or scoring-freshness lifecycle.

The attempt state machine, feedback policy, timer rule, and Assignment Attempt Summary
are detailed in [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md). The narrow current and
target request/receipt shapes are detailed in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Question Backend authority

The common lifecycle crosses each Question Backend through the same operations.
PLE owns Draft Question authoring, publication, Assignment selection, issuance,
submission, evaluation recording, feedback release, and Gradebook effects.
Each adapter owns only validation, presentation, and evaluation behavior for
its complete format-specific Question Source.

| Source or import pathway | Format-specific authority                                                                              | Presentation and evaluation authority                                  | Important retained-evidence rule                                                                         |
| ------------------------ | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| PLE Question JSON        | Validate one complete static PLE Question JSON source                                                  | PLE Question Backend                                                   | Use the immutable issued Question Source and exact Question Attempt Reproduction Details                 |
| QTI Import               | Validate the archive and map each supported flat item into a complete PLE Question JSON Draft Question | PLE Question Backend after conversion                                  | Retain the checksum-pinned archive and mapping as Workspace Import evidence                              |
| WeBWorK PG               | Validate and retain one complete licensed PG/PGML Question Source                                      | Private `/render-api`, followed by PLE sanitization and shared outputs | Use the immutable issued source, presentation binding, and WeBWorK Question Attempt Reproduction Details |
| iMathAS                  | Validate the exact deployment, item, profile, and session bindings                                     | Server-mediated iMathAS Question Backend Session                       | Keep backend tokens and raw results inside the exact iMathAS evidence boundary                           |
| H5P Package              | Validate and retain the supported package and archive evidence                                         | H5P adapter through the shared Question contracts                      | Preserve its current ungraded-practice behavior and exact package evidence                               |

PLE Question JSON Questions use PLE's public `QuestionRevision` plus separate
PLE Question JSON Private Grading. The exact PLE Question JSON authoring format is
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md), not a second generic
runtime model.

QTI Import reports unsupported input and maps accepted flat items into PLE
Question JSON. The resulting Draft Question then uses the same authoring,
publication, issuance, submission, and evaluation operations as every other
Question. Its profile and conversion contract is registered in
[CONTRACTS.md](CONTRACTS.md).

WeBWorK is a private service integration. PLE sends the trusted source, fixed
Question Seed, and renderer credentials only from the server, turns the approved radio
control into PLE opaque choices, and keeps upstream names, values, source,
cookies, and raw response bodies out of attempts and browsers. The exact
bounded RC3 contract and its release scope are in
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

The iMathAS Question Backend remains deliberately sparse in the generic model. The
backend is not allowed to widen an ordinary student response into a token or
raw payload. iMathAS Question Backend Sessions and transcripts are course-owned student records
with their own authorization and retention handling.

## Failure semantics

| Boundary                              | Safe outcome                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Draft validation or publication fails | Keep the private draft; do not mint public identity or create a partial immutable version.                         |
| Capability check fails                | Return the complete missing-capability report before publication or assignment persistence.                        |
| Concurrent issue/resume               | Lock and return the sole unresolved Question Attempt.                                                              |
| Public render or asset fails          | Keep the Assignment Attempt resumable; offer retry without changing Question Seed or Question Attempt.             |
| Presentation mismatch                 | Return stable conflict, persist bounded diagnostic evidence, reload the same attempt, and never grade stale state. |
| Network loss while saving             | Keep the response visible; while the Attempt remains active, repeat the same position save after reconnecting.     |
| Assignment Attempt expires            | Auto-submit successfully saved responses and close unsaved Questions unanswered.                                   |
| Finalization replay                   | Return the existing submitted state without creating another Question Submission or result.                        |
| Renderer/backend outage               | Preserve the active attempt; expose a bounded degraded state only for the affected question.                       |
| Retention object failure              | Keep the course archived and retry the frozen typed-object manifest; never report deletion early.                  |

These rules keep failures visible without turning a browser cache, renderer response, or repeated
request into new authority. Assignment Attempt recovery is expiry auto-submission. The more detailed
route, storage, and RLS guarantees are in [SECURITY_MODEL.md](SECURITY_MODEL.md)
and [CONTRACTS.md](CONTRACTS.md).

## Retain records, keep learning

Student records are course-owned and privacy-sensitive. Course policy first
notifies, then archives and fences student access, then permanently deletes the
complete student graph and typed student-record objects. The deletion path uses
a frozen manifest, already-complete object deletion, lease and generation fencing,
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
  timing, Question Submission and Receipt outcomes, completion, and Assignment Attempt Summary.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md): Student render, Presentation
  Response Item References, Question Presentation Checksum, and minimal response payloads.
- [SECURITY_MODEL.md](SECURITY_MODEL.md): authorization, grading secrecy,
  publication, Assignment Attempt, asset, and retention security boundaries.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md): typed Object Addresses, bucket roles,
  checksums, Object Delivery, Object Storage Check, and Object Storage Repair.
- [CONTRACTS.md](CONTRACTS.md): module ownership, frozen contracts, and change
  rules.
- [RETENTION_POLICY.md](RETENTION_POLICY.md): privacy lifecycle and anonymous
  aggregate preservation.
- [ROADMAP.md](ROADMAP.md): durable release direction and acceptance criteria.
