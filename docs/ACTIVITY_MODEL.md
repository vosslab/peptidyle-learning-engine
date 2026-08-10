# Activity model

Peptidyle treats completion as a milestone, not the end of activity. A student
may complete an assignment and keep starting new runs to learn from algorithmic
variation. The model therefore separates enrollment, run, and question attempt.

This is the durable record and policy contract. It complements the end-to-end
ownership map in [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), the
teaching rationale and future instructor experience in
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md), and the
server-only learner boundary in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The active
release plan remains the source of truth for package status and acceptance
evidence.

## The three levels

| Record                 | Meaning                                        | Cardinality                    |
| ---------------------- | ---------------------------------------------- | ------------------------------ |
| `AssignmentEnrollment` | One student's relationship with one assignment | One per student and assignment |
| `AssignmentRun`        | One pass through the assignment                | Many per enrollment            |
| `QuestionAttempt`      | One issued question and submitted response     | Many per run                   |

The owner has observed students voluntarily run a completed assignment 30 or
more times. The dedicated WP-C3 acceptance test therefore completes 31 runs and
checks the compact summary rather than treating the first completion as terminal.

## Tenant ownership

All three activity records and `StudentAssignmentSummary` are educational
records. Each row carries `TenantId` directly. The PostgreSQL implementation in
MOD-SCHEMA can therefore apply forced row-level security to every table without
depending on a parent-table join to discover the tenant.

The browser never chooses this value. The server supplies tenant context from
the authenticated session, and storage later verifies that context under its
conformance suite.

Courses and assignments sit immediately above this activity hierarchy. A
course is tenant-owned and grants explicit course-local membership to
authenticated `UserId` values. An assignment belongs to exactly one course and
stores ordered `(ProblemId, VersionId)` references to shared immutable content;
it never owns or copies the question payload. Enrollment then links a student
record to that tenant-owned assignment.

## Enrollment

`AssignmentEnrollment` owns cross-run state:

- `user` is the authenticated person authorized to act on the enrollment;
- `student` is the institution's pedagogical record identity and is not
  inferred to be the same identifier as `user`;
- `first_completed_at` records the first server time a run met completion.
- `current_grade_run` points to the run selected by grade policy.
- `best_grade_run` points to the highest-scoring completed run.

`EnrollmentStatus` is derived from `first_completed_at`. It is not stored as a
second value that can disagree with the first-completion record.

## Run

`AssignmentRun` records its one-based run number, server timestamps, score,
mode, and the variation policy that was actually applied. `RunMode` distinguishes
initial assigned work from post-completion practice.

There is no stored within-run `complete` boolean. MOD-RUN derives completion
from the current state of every required question. Once the policy is satisfied,
the server records the completion timestamp and score as a transition.

## Question attempt

`QuestionAttempt` belongs to one run and records:

- its tenant and run IDs;
- its zero-based assignment position, which distinguishes repeated references
  to the same published problem version and groups retries correctly;
- the immutable published `ProblemId` and `VersionId`;
- the generation seed and parameter hash;
- the student response and server-side result when available;
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
duplicating the seed, parameter hash, problem ID, or version ID already carried
directly by `QuestionAttempt`. Parameters themselves are regenerated from seed
and generator version; the hash detects a mismatch without storing the same
data on hundreds of millions of rows.

The logical attempt lifecycle and the durable record status are related but not
interchangeable. `domain::attempt::AttemptState` is a pure state machine for a
logical assignment position: it decides whether the next outcome is correct,
retryable, exhausted, timed out, or abandoned. `QuestionAttempt.status` records
the operational state of one issued evidence record. The latter also represents
server workflows that are not a learner retry decision, including
`NeedsManualGrading`, `Cleared`, and `Exempt`. Neither representation gives the
browser authority to change a score, bypass a timer, or erase earlier evidence.

Seed replay is secondary to fresh practice. The server gives every newly issued
parameterized question instance a fresh seed. Resuming or re-rendering that
same `QuestionAttempt` uses its stored seed so the question does not change
mid-attempt. Seeds minted for the JSON API come directly from the operating
system random source and are masked to 53 bits, the exact nonnegative integer
range shared by Rust and JavaScript. The internal generator contract remains
`u64`, so committed vectors and non-browser callers retain its full domain.

The run service issues at most one unresolved `QuestionAttempt` at a time. A
resume returns that same record and seed. After its response commits, the
service advances to the first never-attempted assignment position; only after
every position has a response may it issue an allowed retry. The store locks
the run and enforces the same invariant so concurrent requests cannot start two
question timers.

## Run API persistence

MOD-API-RUN starts or resumes the enrollment owner's active run, lists run and
attempt history with bounded cursors, records submissions, and reads the
transactionally maintained summary. PostgreSQL supplies run numbers, issue and
submission timestamps, deadlines, and completion timestamps. The browser does
not submit any of those values.

Every submission carries a bounded idempotency key. Repeating the exact key and
response returns the first committed receipt without grading twice. Reusing
either the attempt with a different key or the key with a different response
is a conflict. Response, grade event, run completion, enrollment pointers, and
summary projection commit in one transaction.

The server repeats key-free response-format validation before invoking a
trusted grading backend. Storage independently rejects malformed point values.
Student routes return the response and only policy-permitted correctness and
points; answer keys and checker state never enter the activity model.

## Attempt lifecycle

`domain::attempt::apply` is the one pure transition function for a logical
question inside a run:

```text
not_started -> active -> submitted -> correct
                    |            `-> incorrect -> retry_available -> active
                    |                         `-> exhausted
                    +-> timed_out
                    `-> abandoned
retry_available ---------------------------------------> abandoned
```

The server supplies every event. Grading cannot skip `Submitted`, policy must
turn `Incorrect` into either `RetryAvailable` or `Exhausted`, and terminal
states accept no later event. Starting a retry means issuing a new
`QuestionAttempt` with a fresh server-owned seed. It never clears the response,
result, seed, or provenance of the earlier attempt.

## Timer verdicts

`domain::timing::timer_verdict` is the one authoritative timer evaluation. It
receives a `TimerEvaluation` containing `TimingPolicy`, `AttemptTimerRecord`, a
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
The API fallback and WebAssembly export use the same lower-camel JSON contract,
including `pauseExtensionMillis` and `submittedWithinGrace`.

## Independent policies

The four run policies are separate enums in
`crates/question_model/src/run_policy.rs`. They compose freely rather than
forming a fixed menu of assignment modes.

| Policy                 | Options                                                    |
| ---------------------- | ---------------------------------------------------------- |
| Completion requirement | Answer all, all correct, or score threshold                |
| Grade policy           | First, latest, highest, or instructor-selected run         |
| Continued practice     | Unlimited, capped, or closed after completion              |
| Variation policy       | New seeds, selected problem variants, or full regeneration |

For example, an instructor can require mastery, keep the highest score, allow
unlimited practice, and issue new seeds on every run. Continued practice does
not decide which score counts; grade policy remains independent.

Question-level policies remain separate from `RunPolicies`. Every immutable
published question version owns an `AttemptPolicy` (retry bound and feedback
disclosure) and a `TimingPolicy`; an assignment cannot silently rewrite either
one. That lets the same run model work for native, QTI, WeBWorK, and future
question families while keeping response and grading authority server-side.

`FeedbackDisclosure` has four choices:

- `ImmediateFull` shows the response, correct answer, and explanation.
- `ImmediateCorrectness` shows correctness and a hint without the answer.
- `Deferred` waits until the run is submitted.
- `OnRelease` waits for an instructor release.

The server stores trusted feedback but projects it only when this question
policy permits it. A deferred response does not become visible merely because
the browser asks again; an on-release response remains hidden until an
authorized instructor transition. See
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) for the
learner-facing meaning of each disclosure choice.

## Instructor activity types

The implemented stored model is the independent policy vocabulary above. It
does not contain a persisted combined `Mastery`, `Exam`, `Practice`, or
`Standard` enum, and it does not yet contain a separate gradebook-visibility
policy. That is intentional: a label must not conceal a different durable
record contract.

The current assignment editor works with the explicit run policies and the
immutable question versions selected for the assignment. A teaching-oriented
activity-type chooser is planned as a UI layer that writes those same explicit
values. It is not evidence that the four labels below are current API values:

| Teaching activity | Current durable representation | Instructor experience status |
| ----------------- | ------------------------------ | ---------------------------- |
| Mastery | `AllCorrect`, `Highest`, `Unlimited`, `NewSeeds`, plus question-level retry, feedback, and timing choices | Fully representable; named chooser planned |
| Standard graded assignment | `AnswerAll`, a chosen grade policy, `Closed`, plus question-level policies | Fully representable; named chooser planned |
| Exam | `AnswerAll`, a chosen grade policy, `Closed`, restricted question policies, and server timing where needed | Fully representable; named chooser planned |
| Practice | Continued runs and learning feedback are representable | A promise that it is absent from the gradebook is planned, because no separate gradebook-visibility policy exists yet |

The recommended mastery bundle is a teaching default, not a special storage
branch: all-correct completion, highest-score selection, unlimited continued
practice, fresh seeds, unlimited question retries where appropriate, immediate
educational feedback, and normally untimed work. A course may deliberately use
another combination. [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md)
owns the detailed bundle, learner wording, and planned UI simplification.

## Completion derivation

`domain::completion::derive_within_run_completion` accepts current
required-question states and a `CompletionRequirement`. It returns `InProgress`
or `Complete` without reading storage or a clock. The compatibility re-export
from `domain::run` remains available to existing WP-C3 consumers.

The derivation follows these rules:

- an empty run remains in progress;
- `AnswerAll` requires a response for every required question;
- `AllCorrect` requires every required question to be answered correctly; and
- `ScoreAtLeast` requires every question to be answered and the score threshold
  to be met.

Invalid score fractions and point values are explicit errors.

## Summary projection

`StudentAssignmentSummary` is the compact gradebook and course-page projection.
It holds current, best, and latest scores, completed-run count, total question
attempts, and last activity time. Historical runs remain separate for analysis.

`domain::scoring::project_summary` is a pure function:

```rust
project_summary(previous, transition, grade_policy) -> Result<next, error>
```

The function reads no database and no clock. A store can write the run
transition and returned summary in one transaction, so a page never computes a
grade by scanning attempt history. Activity time never moves backward when an
older event is replayed.

`domain::scoring::score` is the batch reconciliation contract over completed
run IDs, one-based run numbers, and score fractions. First and latest use run
number rather than input order. Highest keeps the earlier run when scores tie,
so the selected pointer is stable. Instructor-selected grading remains empty
until an instructor names a completed run. The incremental summary and batch
selection are checked against the same hand-computed fixture. The compatibility
re-export from `domain::run` remains available to WP-C3 consumers.

The gradebook reads this compact projection together with tenant-owned course
and assignment records. It does not scan every historical run or attempt when
a student has returned for continued practice many times. Historical records
remain available to authorized history and analysis paths until retention
removes the tenant-owned learner graph.

## Retention boundary

Enrollment, run, attempt, summary, feedback, and associated learner-owned
artifacts are student records. Course retention archives their ordinary
learner-facing access before permanent deletion, then removes the course-owned
record graph and its typed artifacts while preserving immutable shared
published content and identity-free question statistics. The default lifecycle
is notification after 30 days, archive after 100 days, and deletion after 365
days; an institution can configure another ordered policy.

The lifecycle is server- and scheduler-owned. A browser cannot choose the
tenant, deletion scope, work-set identity, lease, or generation, and a passed
deadline alone does not claim that cleanup succeeded. The detailed contract is
[RETENTION_POLICY.md](RETENTION_POLICY.md).

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

The 31-run scenario is in `crates/domain/tests/run_31.rs`. Its expected summary
is written out as a hand-computed value, making repeated post-completion practice
a permanent behavior contract.
