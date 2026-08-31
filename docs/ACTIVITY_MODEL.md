# Activity model

Peptidyle treats completion as a milestone, not the end of activity. A student
may complete an assignment and keep starting new Assignment Attempts to learn
from algorithmic variation. The model therefore separates the Student Record,
Assignment Attempt, Issued Question, and Question Attempt. The terminology and hierarchy are
owned by [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

This is the durable record and policy contract. It complements the end-to-end
ownership map in [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), the
teaching rationale and future instructor experience in
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md), and the
server-only student boundary in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The active
release plan remains the source of truth for package status and acceptance
evidence.

## Assignment hierarchy

| Record                 | Meaning                                        | Cardinality                    |
| ---------------------- | ---------------------------------------------- | ------------------------------ |
| Student Record | One Student Account's durable educational record in one Course Instance | One retained record per Student Account and Course Instance |
| Assignment Attempt | One pass through one Assignment | Many per Student Record and Assignment |
| Issued Question | One selected Question Version delivered in one Assignment Attempt | Ordered within one Assignment Attempt |
| Question Attempt | One server-issued try for one Issued Question | Many when retry policy permits |
| Question Submission | One accepted Student Response for one Question Attempt | One immutable accepted event per Question Attempt |

The owner has observed Students voluntarily complete a finished assignment 30 or
more times. The dedicated WP-C3 acceptance test therefore completes 31 Assignment Attempts and
checks the compact summary rather than treating the first completion as terminal.

## Single-installation authorization

PLE is one installation with global accounts. It has no institution selector,
an installation-wide account selector, leading scope key, or client-selected database context. Institution
policy configuration is deployment metadata; it is not an account boundary,
authorization partition, or activity-record owner. Historical pre-SD1 source
still contains legacy installation-scope fields; that source is migration
input, not the target activity contract.

`CourseId` is the exact educational-record boundary. An assignment belongs to
one course and stores ordered `(QuestionId, QuestionVersionNumber)` references to shared
immutable published content; it never owns or copies the question payload. Each
activity record is resolved to one exact course, and student-owned records also
name their exact `StudentRecordId` owner. Child identities must agree with the
Student Record, Assignment, and Course chain; a direct child identifier never widens
that scope.

The server resolves an authenticated session record to its global account and session identity from the authenticated
global account session. A browser field, request path, header, queue payload,
object key, or provider response can identify a candidate record, but cannot
establish Account authority or select a course. The Store and PostgreSQL boundary
re-evaluate the exact relationship in the same transaction as each protected
operation.

| Activity record | Durable ownership scope | Allowed human authority |
| --- | --- | --- |
| Student Record | Exact `CourseId` and Student Account | That Student with a current Student Course Membership, or a current course Instructor |
| Assignment Attempt | Student Record and Assignment | That Student, or a current course Instructor |
| Issued Question | Assignment Attempt and exact Question Version | That Student, or a current course Instructor |
| Question Attempt | Issued Question and its Student owner | That Student, or a current course Instructor |
| Assignment Grade | Student Record and Assignment | That Student projection, or a current course Instructor |

Student access requires current active Student membership for the exact course
and ownership of the exact `StudentRecordId`; another Student, another course, a
revoked membership, and an inactive retention state fail closed. Instructor
access requires current approved-Instructor status and current direct Instructor
membership for that exact course. All current Teaching Team Members receive the same
teaching and FERPA-read decisions for equivalent state. Sysadmin status alone is
not FERPA authority; support and coarse retention lifecycle operations are
narrow, audited exceptions defined by [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

Membership revocation, approval withdrawal, and retention fencing serialize with
activity reads and writes. A stale browser identifier therefore cannot read,
mutate, disclose, or delete a record after its relationship has ended. These
authorization checks do not rewrite historical activity evidence.

## Student Record and Assignment Grade

`StudentRecord` binds one Student Account's exact Course Membership to the
durable course educational record. The authenticated Account and session are
server-derived; a browser request cannot select a different Student Record.

`AssignmentGrade` is the selected course-record result for one exact Student
Record and Assignment. It records the policy-selected result without becoming
the owner of activity. Activity summaries are derived views: they report recent
work and counts but never replace the Assignment Attempt and Issued Question
evidence that establishes them.

## Assignment Attempt

An Assignment Attempt records its one-based attempt number, server timestamps,
score, mode, and the variation policy that was actually applied. It distinguishes
initial assigned work from post-completion practice.

There is no stored within-Assignment-Attempt `complete` boolean. The assignment-activity model derives completion
from the current state of every required question. Once the policy is satisfied,
the server records the completion timestamp and score as a transition.

## Issued Question and Question Attempt

An Issued Question freezes the selected Question Version, Assignment Entry,
delivery order, applied point value, scoring rule, and Question Pool selection
evidence for one Assignment Attempt. It is the immutable bridge between a live
Assignment definition and a Student's individual tries.

`QuestionAttempt` belongs directly to one Issued Question and records:

- its exact Issued Question and therefore its Student Record, Assignment, and
  immutable Question Version scope;
- one server-owned try sequence within that Issued Question;
- the generation seed and parameter hash;
- the accepted Student Response and server-side Grading Result when available;
- server-issued timing data;
- the adapter and grading implementation versions;
- the typed generator ID and version, plus the renderer version, when they apply;
- the source object ID and checksum when one exists;
- referenced asset object IDs; and
- the rendered-question checksum.

An attempt result contains correctness and points, not an answer key. Correct
answers and grading implementations remain in `crates/grading`, outside the
WebAssembly dependency closure. Feedback disclosure controls whether and when a
result reaches a student response.

`AttemptProvenance` groups the implementation and object details without
duplicating the seed, parameter hash, problem ID, or version ID carried by the
owning Issued Question. Parameters themselves are regenerated from seed and
generator version; the hash detects a mismatch without storing the same data
on hundreds of millions of rows.

Issued Question Progress and Question Attempt state remain separate. The
server derives Issued Question Progress from its retained Question Attempts,
Question Submissions, and Grading Results. `QuestionAttempt.status` records the
operational state of one issued evidence record, including server-owned
`AutoSubmitted`, `Cleared`, and `Exempt` transitions. The separate
`SubmissionEvaluationStatus` records `automated_pending`, `automated_exception`,
`graded`, or `exempt`. Neither representation gives the browser authority to
change a score, bypass a timer, or erase earlier evidence.

Seed replay is secondary to fresh practice. The server gives every newly issued
parameterized Generated Question a fresh seed. Resuming or re-rendering that
same `QuestionAttempt` uses its stored seed so the question does not change
mid-attempt. Seeds minted for the JSON API come directly from the operating
system random source and are masked to 53 bits, the exact nonnegative integer
range shared by Rust and JavaScript. The internal generator contract remains
`u64`, so committed vectors and non-browser callers retain its full domain.

The Assignment Attempt service issues at most one unresolved `QuestionAttempt` at a time. A
resume returns that same record and seed. After its response commits, the
service advances to the first never-attempted assignment position; only after
every position has a response may it issue an allowed retry. The store locks
the Assignment Attempt and enforces the same invariant so concurrent requests cannot start two
question timers.

## Assignment activity persistence

The activity API starts or resumes the Student Record owner's active Assignment
Attempt, lists attempt history with bounded cursors, records submissions, and reads the
transactionally maintained summary. PostgreSQL supplies Assignment Attempt numbers, issue and
submission timestamps, deadlines, and completion timestamps. The browser does
not submit any of those values.

Every submission carries a bounded idempotency key. Repeating the exact key and
response returns the same accepted or completed projection without grading twice. Reusing
either the attempt with a different key or the key with a different response
is a conflict. The first transaction atomically records the accepted response,
issued-work witness, pending evaluation, execution, and ready job. The response
is immutable server-private acceptance evidence; its metadata parent is
answer-free and its canonical UTF-8 response is held by the private execution
capability. The sealed worker's successful transaction atomically records the
grade event, Question Attempt and Assignment Attempt transitions, Assignment Grade
selection pointers, summary
projection, and completed receipt.

An acknowledged student response is recoverable without another answer POST.
The submission route returns `accepted_pending` when the exact synchronous
claim does not complete, clears the browser response buffer, and exposes
**Check grading status**. The route-bound status GET returns the same
answer-free union. A deterministic execution failure projects
`instructor_attention`; an Instructor retry creates a new execution generation
for the same immutable submission. The ordinary worker then uses the shared
handler, and the assignment scoring path publishes the current Gradebook total.

The server repeats key-free response-format validation before invoking a
trusted grading backend. Storage independently rejects malformed point values.
Student routes return the response and only policy-permitted correctness and
points; answer keys and checker state never enter the activity model.

## Question activity records

The server records each fact at its owning level: an Issued Question owns the
selected content, each Question Attempt owns one server-issued try and its
operational state, each accepted Question Submission owns one Student Response,
and each Grading Result owns one authoritative evaluation. **Issued Question
Progress** is derived from those retained records. This keeps retries,
timeouts, submission, grading, and Instructor exclusions independently
auditable instead of compressing them into one mutable lifecycle state.

The server supplies every event. Grading cannot skip `Submitted`, policy must
turn `Incorrect` into either `RetryAvailable` or `Exhausted`, and terminal
states accept no later event. Starting a retry means issuing a new
`QuestionAttempt` with a fresh server-owned seed. It never clears the response,
result, seed, or provenance of the earlier attempt.

## Timer verdicts

`domain::timing::timer_verdict` is the one authoritative timer evaluation. It
receives a `TimerEvaluation` containing `TimingPolicy`, `QuestionAttemptTiming`, a
server evaluation timestamp, and the cumulative authorized pause extension.
It never reads a clock.

The stored deadline is the base server-issued deadline. A server reconstructs
the pause extension from its audit events and passes that total into the pure
function. The effective deadline is the base deadline plus that extension;
grace begins after the effective deadline. Both the deadline and grace boundary
are inclusive.

An unsubmitted timer is `Open` through its effective deadline, then
`GracePeriod` while the server waits for an in-flight response, then
`TimedOut`. Grace is network tolerance, not extra student working time. A
submitted response is `SubmittedOnTime`, `SubmittedWithinGrace`, or `TimedOut`
according to its server-recorded arrival timestamp. An untimed policy has no
deadline or pause extension.

The browser may project these values for display and submits at its displayed
expiry. Its local clock never becomes an input to the authoritative verdict.
**Current pre-WN1 behavior:** the API fallback and WebAssembly export use the same lower-camel JSON
contract, including `pauseExtensionMillis` and `submittedWithinGrace`. The matrix-selected
`WP-INST-WN1-WA` closure changes PLE bridge JSON to direct `pause_extension_millis` and
`submitted_within_grace`; raw wasm-bindgen exports remain protocol-owned.

## Independent policies

The four Assignment Activity policy dimensions compose freely rather than
forming a fixed menu of assignment modes.

| Policy                 | Options                                                    |
| ---------------------- | ---------------------------------------------------------- |
| Completion requirement | Answer all, all correct, or score threshold                |
| Grade policy           | First, latest, highest, or instructor-selected Assignment Attempt |
| Continued practice     | Unlimited, capped, or closed after completion              |
| Variation policy       | New seeds, selected Question Variants, or full regeneration |

For example, an instructor can require mastery, keep the highest score, allow
unlimited practice, and issue new seeds on every Assignment Attempt. Continued practice does
not decide which score counts; grade policy remains independent.

Question-level policies remain separate from Assignment Activity policy. Every immutable
published question version owns an `AttemptPolicy` retry bound and a
`TimingPolicy`; an assignment cannot silently rewrite either one. Attempt
policy does not disclose results, feedback, or answers. That lets the same Assignment Attempt
model work for native, QTI, WeBWorK, and future Question Backends while keeping
response and grading authority server-side.

### Student disclosure

Each Assignment owns one `StudentDisclosurePolicy`. Its five independent
fields are `score`, `per_item_correctness`, `feedback_text`, `solution`, and
`class_statistics`. Each field uses one timing: `DuringAttempt`, `AfterSubmit`,
`AfterDue`, `AfterClose`, or `Never`.

The server first requires the S5 student entitlement, then uses S3's current
effective policy and an authoritative server timestamp to evaluate every field.
`AfterSubmit` requires that student's submission; due and close timings use the
current resolved boundary. A missing due or close boundary does not release its
field. The browser receives no policy, clock, entitlement, or identifiers from
which it could infer a withheld result.

The server omits withheld fields rather than sending placeholders or answer
material. Private feedback generation, grading keys, correct answers, and
grading implementations remain server-only. `feedback_release` is immutable,
retention-fenced audit evidence of an instructor action. It never unlocks or
changes the student projection. See
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) for the teaching
rationale for independent disclosure choices.

When the independent `class_statistics` timing permits it, the student receives
one server-derived anonymous union: `insufficientEvidence`, with no cohort or
metric fields, or `available`, with only `completed_student_cohort_size` and a
normalized `assignment_average_score`. The server reads the current course-local
analysis only after S5 and S3/time evaluation. Its completed-student cohort is
the latest completed Assignment Attempt per Student Record. The default privacy floor is five;
the server returns `insufficientEvidence` for a smaller cohort, incomplete
automated scoring, recent rescoring, or a missing or invalid average. The browser
renders that result and never derives it from policy, timing, a clock, or
aggregate evidence.

## Workspace surface ownership

The assignment workspace is the local navigation owner. It loads one exact
course/assignment aggregate and connects Overview, Questions, Policies, and
Student view; it does not introduce another activity record or a second policy
vocabulary. The assignment revision is shared across the focused writes so
each page can update its own slice without replacing a sibling page's changes.

- Questions edits assignment content: title, ordered fixed questions, pools,
  reuse, and selection order. Its content save changes no delivery or
  lifecycle policy.
- Policies edits delivery and lifecycle policy: audience, disclosure, Assignment Activity
  policies, student instructions, schedule, limits, late behavior, and
  lifecycle. Its policy save changes no question content.
- Teaching operations owns live operational actions around the assignment,
  including group and access operations, policy previews, delivery checks,
  and teaching-authority workflows. Those actions may resolve effective
  delivery or entitlement, but they do not make the workspace's Questions or
  Policies pages interchangeable.
- Grading operations owns assignment-local automatic-grading recovery. It
  groups safe operation metadata by question or student, includes assignment-
  wide recalculation rows, and exposes guarded retry and recalculation commands;
  it does not own responses, evaluation
  payloads, or score mutation.
- Student view is an Instructor-authorized, answer-free inspection of the
  current assignment. It is a no-store read and creates no Student Record, Assignment Attempt,
  attempt, submission, receipt, score, gradebook row, or preview record.
- Ordinary Student delivery is the real graded path. An enrolled Student's
  start or resume and submission actions create the durable Assignment Attempt,
  attempt, receipt, score, and gradebook evidence described below.

These are presentation and command ownership boundaries over the same
assignment aggregate. They do not alter the historical activity invariants:
completion remains a milestone, post-completion Assignment Attempts remain possible when
policy allows, and only server-owned Student delivery creates student activity.

The fifth workspace page is **Grading operations**. It completes the visible
recovery path from Student status to Instructor action while preserving the
same assignment revision and server-owned activity records.

## Instructor activity types

The implemented stored model is the independent policy vocabulary above. It
does not contain a persisted combined `Mastery`, `Exam`, `Practice`, or
`Standard` enum, and it does not yet contain a separate gradebook-visibility
policy. That is intentional: a label must not conceal a different durable
record contract.

The current assignment workspace composes explicit Policies with immutable question
versions selected on Questions. A teaching-oriented
activity-type chooser is planned as a UI layer that writes those same explicit
values. It is not evidence that the four labels below are current API values:

The Policies surface saves one revisioned assignment teaching-settings
aggregate: Draft/Published/Closed/Archived lifecycle, plain-text student
instructions, availability/due/close schedule, whole-Assignment-Attempt and Question Attempt limits,
late behavior, and deadline behavior. Only Published opens lifecycle gate G1.
Course-local wall-clock input is converted by the server through the course
IANA zone; the browser never derives an authoritative instant. An active Assignment Attempt
does not consume its own attempt-limit slot: completed Assignment Attempts determine whether
another Assignment Attempt may start, while the current active Assignment Attempt remains resumable.

The separate Teaching operations surface performs live operational work such
as access-modifier changes, group management, effective-policy previews, and
teaching-authority actions. It is not a replacement for the Policies editor
and does not change the ownership of the durable activity records below.

| Teaching activity          | Current durable representation                                                                                    | Instructor experience status                                                                                          |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Mastery                    | `AllCorrect`, `Highest`, `Unlimited`, `NewSeeds`, question retry/timing, and assignment disclosure choices        | Fully representable; named chooser planned                                                                            |
| Standard graded assignment | `AnswerAll`, a chosen grade policy, `Closed`, question retry/timing, and assignment disclosure choices            | Fully representable; named chooser planned                                                                            |
| Exam                       | `AnswerAll`, a chosen grade policy, `Closed`, restricted question retry/timing, and assignment disclosure choices | Fully representable; named chooser planned                                                                            |
| Practice                   | Continued Assignment Attempts and learning feedback are representable                                             | A promise that it is absent from the gradebook is planned, because no separate gradebook-visibility policy exists yet |

The recommended mastery bundle is a teaching default, not a special storage
branch: all-correct completion, highest-score selection, unlimited continued
practice, fresh seeds, unlimited question retries where appropriate, an
assignment disclosure policy that supports educational feedback, and normally
untimed work. A course may deliberately use another combination.
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) owns the detailed
bundle, student wording, and planned UI simplification.

## Completion derivation

`domain::completion::derive_within_assignment_attempt_completion` accepts current
required-question states and a `CompletionRequirement`. It returns `InProgress`
or `Complete` without reading storage or a clock.

The derivation follows these rules:

- an empty Assignment Attempt remains in progress;
- `AnswerAll` requires a response for every required question;
- `AllCorrect` requires every required question to be answered correctly; and
- `ScoreAtLeast` requires every question to be answered and the score threshold
  to be met.

Invalid score fractions and point values are explicit errors.

## Summary projection

**Assignment Progress** is the compact derived view for one exact Student
Record and Assignment. The server's `AssignmentProgressRecord` carries the
internal ownership references; `AssignmentGrade` separately owns the selected
course-record result. Assignment Progress holds current, best, and latest
scores, completed-attempt count, total Question Attempts, and last activity time.
Historical Assignment Attempts remain separate for analysis. The Store updates
the view only for the same exact Student Record and Assignment represented by
the transition.

Student routes instead receive the key-free `AssignmentProgress`
projection. `score_state` is `NoActivity`, `Withheld`, or `Available`. Scores
are present only for `Available`; `NoActivity` means no submitted response and takes precedence over
disclosure. Starting an Assignment Attempt may set `last_activity_at` without changing that score state.
The student projection omits internal course, Student Record, and Assignment
identifiers.
Its independent `scoring_status` is `Current`, `Recalculating`, or `Failed`.
Recalculating and Failed omit aggregate scores, Assignment Attempt scores, Question Attempt results,
and disclosed point values even when disclosure would otherwise permit them,
while keeping the underlying activity/disclosure state so a maintenance
condition is never mistaken for a zero or a new attempt.

`domain::scoring::project_summary` is a pure function:

```rust
project_summary(previous, transition, grade_policy) -> Result<next, error>
```

The function reads no database and no clock. A store can write the Assignment
Attempt transition and returned activity view in one transaction, so a page
never computes a grade by scanning activity history. Activity time never moves
backward when an older event is replayed.

`domain::scoring::select_assignment_attempt_grade` is the batch Recalculation contract over completed
Assignment Attempt IDs, one-based attempt numbers, and score fractions. First and latest use
attempt number rather than input order. Highest keeps the earlier Assignment Attempt when scores tie,
so the selected pointer is stable. Instructor-selected grading remains empty
until an instructor names a completed Assignment Attempt. The incremental summary and batch
selection are checked against the same hand-computed fixture.

The gradebook reads this compact projection together with the exact course and
assignment records. It does not scan every historical Assignment Attempt or Question Attempt when a
student has returned for continued practice many times. Historical records
remain available to authorized history and analysis paths until course
retention removes the course-owned Student graph.

## Retention boundary

Student Record, Assignment Attempt, Question Attempt, summary, feedback, and associated student-owned
artifacts are course-scoped Student records. Course retention archives their
ordinary student-facing access before permanent deletion, then removes the
course-owned record graph and its typed artifacts while preserving immutable
shared published content, private authoring workspaces, and identity-free
question statistics.

The deployment retention policy is trusted policy metadata. It supplies ordered
notification, archive, and deletion windows (the defaults are 30, 100, and 365
days), but it is not an account property, an institution partition, a request
field, or an authorization grant. The Store resolves this metadata when a course
ends and records an immutable schedule snapshot for that exact `CourseId`.

Every scheduled stage has a typed identity:
`(CourseId, RetentionStage, generation)`. `generation` is a positive stale-work
fence. A private worker payload adds only the exact job and active lease; it does
not carry Student IDs, object prefixes, record payloads, or browser authority.
The Store resolves a `RetentionCleanupManifest` for that course, stage, and
generation. The manifest contains the exact typed `StudentRecord` object
metadata to revoke and delete, never a bucket prefix or caller-provided list.
The worker validates the manifest against its lease before each object effect,
and the Store commits the lifecycle transition only when the same generation and
lease remain current.

Archive first revokes ordinary Student-facing access. Permanent deletion then
removes only the course-owned Student rows and exact artifacts after residual
checks; an absent object is idempotent success. A passed deadline makes a stage
eligible but never claims that its effects completed. The detailed lifecycle and
backup boundary are in [RETENTION_POLICY.md](RETENTION_POLICY.md).

Retention and grading share the same evidence rule: while records are retained,
deletion or rescoring cannot rewrite an immutable accepted response, attempt
provenance, receipt, or prior grade event. Retention deletion is a separate,
generation-fenced terminal operation. Current summaries and Gradebook totals may
be recalculated only by the server's deterministic grading contract and the
active scoring generation.

## Behavior evidence

The focused historical acceptance evidence for this model includes:

```bash
cargo test -p question_model
cargo test -p domain
cargo tools tsgen
npx tsc --noEmit
npx eslint src generated/api --max-warnings 0
npx prettier --check generated/api
```

The repository-wide gates for a current change are defined by the active work
package. The following commands remain useful when a change touches the model
and its generated browser contract:

```bash
./check_codebase.sh
pytest tests/
```

The 31-Assignment-Attempt scenario has a hand-computed expected summary, making
repeated post-completion practice a permanent behavior contract.
