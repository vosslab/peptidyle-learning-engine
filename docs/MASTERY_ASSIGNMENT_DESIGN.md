# Mastery assignment design

PLE treats mastery as continued learning, not a one-time finish line. A student can complete an
assignment, receive a recorded score, and begin another varied run to consolidate the same concepts.
The model supports a student who returns repeatedly because another run is a new learning
opportunity, not a replacement for the earlier educational record.

This document explains the teaching intent, the implemented policy model, and the planned
simplification of the instructor experience. It complements the record-level contract in
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md), the model vocabulary in
[QUESTION_MODEL.md](QUESTION_MODEL.md), and the frozen ownership register in
[CONTRACTS.md](CONTRACTS.md).

## Teaching intent

A mastery assignment asks a human question: "Can the learner now apply this concept reliably?" It
does not ask whether the learner happened to complete one fixed worksheet first. Appropriate learner
behavior is to retry, use feedback, work through fresh instances, and stop only when the idea is
dependable.

The durable product promise is:

- Completing one run records an accomplishment; it does not erase the opportunity to learn more.
- A later practice run keeps the earlier run, responses, scores, and provenance as educational
  history.
- Fresh variation supports transfer to a new instance instead of memorization of a previous answer.
- The learner sees only feedback authorized for the question and assignment context.
- The authenticated server, not the browser, decides timing, correctness, retries, completion, and
  scores.

This is particularly useful in an open-book course: students should read, reason, draw, calculate,
and connect evidence rather than stop at recall of a familiar prompt.

## Current model

### Three durable levels

The implemented activity model has three course-owned records:

| Level            | Role                                         | Important consequence                                    |
| ---------------- | -------------------------------------------- | -------------------------------------------------------- |
| Enrollment       | One learner's relationship to one assignment | Preserves cross-run completion and grade pointers        |
| Run              | One pass through an assignment               | Records score, mode, variation, and server timestamps    |
| Question attempt | One issued instance and response             | Preserves seed, provenance, timing, response, and result |

`AssignmentEnrollment.first_completed_at` derives whether the learner has completed the assignment
at least once. A new run after that point is `Practice`; a run before it is `Assigned`. The system
does not store a competing within-run `complete` flag: completion is derived from current
required-question states, then recorded as one server transition. These are implemented in
[crates/question_model/src/activity.rs](../crates/question_model/src/activity.rs) and
[crates/domain/src/completion.rs](../crates/domain/src/completion.rs).

### One active question

PLE permits at most one unresolved question attempt per run. Starting or reopening an assignment
returns the existing active run when there is one; it does not create a competing run or timer. Once
a response commits, the server advances through unattempted positions before offering an allowed
retry. This makes resume safe and keeps attempt sequencing understandable.

The route begins a run only after authenticating the session, while both in-memory and PostgreSQL
stores enforce the same run policy, timing, run-number, and one-active-run rules. See
[crates/server/src/run/routes.rs](../crates/server/src/run/routes.rs),
[crates/learning-data-access/src/in_memory/runs.rs](../crates/learning-data-access/src/in_memory/runs.rs),
and
[crates/learning-data-access/src/postgres/run_lifecycle.rs](../crates/learning-data-access/src/postgres/run_lifecycle.rs).

### Retries preserve evidence

An incorrect response enters `RetryAvailable` only when policy permits another response. Starting
the retry issues a new `QuestionAttempt`; it never overwrites the prior response, grade, seed, or
provenance. An unlimited question attempt policy is represented by `max_attempts: None`. This is the
implemented mastery retry behavior in
[crates/domain/src/attempt.rs](../crates/domain/src/attempt.rs) and
[crates/question_model/src/run_policy.rs](../crates/question_model/src/run_policy.rs).

### Completion and gradebook score differ

Completion is a condition on one run. Grade selection is a separate condition across completed runs.
For example, requiring all questions correct determines when a mastery run completes; retaining the
highest run score determines what reaches the gradebook. Neither rule implies the other.

`AllCorrect`, `AnswerAll`, and `ScoreAtLeast` are implemented completion requirements. `First`,
`Latest`, `Highest`, and `InstructorSelected` are implemented grade policies. Highest-score ties
retain the earlier run, which keeps the selected grade pointer stable. The pure rules live in
[crates/domain/src/completion.rs](../crates/domain/src/completion.rs) and
[crates/domain/src/scoring.rs](../crates/domain/src/scoring.rs).

### Continued practice and variation differ

Continued practice answers whether another run may begin after first completion:

- `Unlimited` allows any number of additional runs.
- `Capped` allows a stated number after the first completed run.
- `Closed` rejects another run after completion.

Variation answers what changes in a new run:

- `NewSeeds` keeps the assignment's selected questions but changes generated values.
- `SelectedProblemVariants` uses instructor-selected variants.
- `FullRegeneration` redraws questions as well as reseeding.

The continued-practice check does not choose a grade, and the variation policy does not permit a
run. This separation is deliberate and implemented in
[crates/question_model/src/run_policy.rs](../crates/question_model/src/run_policy.rs) and
[crates/domain/src/run.rs](../crates/domain/src/run.rs). A resumed existing attempt keeps its stored
seed; only a newly issued instance receives a fresh one.

## Recommended mastery bundle

PLE currently stores the independent policies below. The recommended mastery configuration is an
explicit composition of those existing values, not a hidden special case:

| Concern            | Recommended mastery value     | Learner meaning                                                                                             |
| ------------------ | ----------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Completion         | `AllCorrect`                  | Keep working until every required question is correct                                                       |
| Grade              | `Highest`                     | Further practice cannot lower the recorded best score                                                       |
| Continued practice | `Unlimited`                   | Start another run after completion whenever useful                                                          |
| Variation          | `NewSeeds`                    | See the same concepts with fresh generated values                                                           |
| Question attempts  | `max_attempts: None`          | Retry a question until correct                                                                              |
| Learner disclosure | All five fields `AfterSubmit` | See the selected score, correctness, teaching feedback, solution, and permitted statistics after submitting |
| Timing             | `Untimed`                     | Work at a learning pace rather than against a clock                                                         |

The first four fields are assignment `RunPolicies`. The assignment also owns the five independent
learner-disclosure timings. Attempt count and question timer are immutable properties of the selected
published question version. The Questions and Policies workspace pages expose these assignment controls
separately; they do not override question policies. See
[src/pages/assignment_workspace/assignment_workspace_questions_page.tsx](../src/pages/assignment_workspace/assignment_workspace_questions_page.tsx),
[src/pages/assignment_workspace/assignment_workspace_policies_page.tsx](../src/pages/assignment_workspace/assignment_workspace_policies_page.tsx), and
[crates/question_model/src/definition.rs](../crates/question_model/src/definition.rs).

An instructor may intentionally choose a different composition. For example, a mastery threshold can
use `ScoreAtLeast` when partial-credit questions are pedagogically meaningful, and per-item
correctness or feedback text can use a different assignment timing when an answer-oriented solution
would spoil a later self-explanation. Such choices remain explicit policy decisions, not accidental
side effects of a label.

## Feedback and timing

### Feedback is server-projected

Each assignment independently schedules five learner-visible fields: score,
per-item correctness, feedback text, solution, and class statistics. Each uses
one timing: `DuringAttempt`, `AfterSubmit`, `AfterDue`, `AfterClose`, or
`Never`. The server first requires S5 entitlement, then uses the current
S3-resolved effective-policy verdict and authoritative time to evaluate the
current assignment policy. A field scheduled `AfterDue` or `AfterClose` remains withheld when its
corresponding boundary is absent; a withheld field is omitted rather than sent
as a hidden null. `feedback_release` is immutable audit evidence of an
instructor action, never a learner-result unlock.

For the recommended mastery bundle, set all five fields to `AfterSubmit`.
An assessment can instead schedule each field independently without changing
the selected question or its retry bound.

Private feedback is intentionally not serializable or debug-printable. The public
`DisclosedFeedback` DTO omits locked fields rather than sending hidden nulls. The implementation is
in [crates/question_model/src/feedback.rs](../crates/question_model/src/feedback.rs),
[crates/domain/src/disclosure_policy.rs](../crates/domain/src/disclosure_policy.rs), and
[crates/learning-data-access/src/feedback.rs](../crates/learning-data-access/src/feedback.rs).

### Time is server-owned

`TimingPolicy` supports untimed, per-question, and per-attempt limits with an explicit grace period.
The browser displays remaining time, but only server-issued timestamps and the server timing verdict
can accept, auto-submit, or reject work. Assignment access policy separately controls visibility,
availability, due date, closing date, late treatment, whole-run limits, and run caps. This is why a
mastery bundle can be untimed while an institution still gives an assignment an availability window.

The policy types are in
[crates/question_model/src/run_policy.rs](../crates/question_model/src/run_policy.rs) and
[crates/question_model/src/assignment.rs](../crates/question_model/src/assignment.rs). Run creation
checks resolved assignment timing in both stores before it creates a run.

## Human-facing activity types

### Current interface

The current instructor editor is an advanced policy editor. It exposes completion, grade, continued
practice, and variation independently. The student-facing pages use mastery-oriented language such
as "Start or resume practice", "Keep practicing with a fresh variation", and "Start fresh practice".
Those are current user-interface facts, not evidence that a formal assignment-type chooser exists.

### Planned simplification

The following is a planned instructor-facing layer over the existing orthogonal model. It should be
implemented as recognizable activity types with safe defaults, not as a new persisted combined enum.
The stored assignment remains `RunPolicies`, five independent assignment disclosure timings, selected
published question versions, and access policy. The examples below are proposed defaults, not a
claim that an older coarse feedback bundle is directly representable.

| Activity type              | Default intent                                              | Proposed policy bundle                                                                                                                                       | Status                                                                                                  |
| -------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Mastery                    | Repeated practice until dependable                          | `AllCorrect`, `Highest`, `Unlimited`, `NewSeeds`; untimed; score, correctness, feedback text, solution, and class statistics each `AfterSubmit`              | Fully representable today; chooser is planned                                                           |
| Standard graded assignment | Complete the assigned work once under ordinary course rules | `AnswerAll`, `Latest`, `Closed`; each of the five assignment disclosure fields set deliberately, for example all `AfterSubmit`                               | Fully representable today; chooser is planned                                                           |
| Exam                       | Controlled one-run assessment                               | `AnswerAll`, `Latest`, `Closed`; restricted attempts and server timing; each disclosure field explicitly `AfterDue`, `AfterClose`, or `Never` as appropriate | Fully representable today; chooser is planned                                                           |
| Practice                   | Low-stakes repeated work                                    | `AnswerAll`, `Unlimited`, `NewSeeds`; each of the five assignment disclosure fields explicitly selected for learning                                         | Continued runs are representable; an explicit no-grade / gradebook-visibility policy is not yet modeled |

The Standard and Exam defaults use `Latest` because `Closed` permits one completed run, making it
the only score candidate. That is a clear expression of current semantics, not a claim that every
course should use the same grading rule. An instructor-facing type can expose the small number of
decisions that commonly differ, such as due date, points, and an intentional advanced override.

The planned Practice type needs one additional product decision before it can honestly promise "does
not affect the gradebook": current `GradePolicy` always chooses a completed run or awaits an
instructor choice. Until a grade visibility or weighting policy exists, the UI must not claim that
practice is ungraded merely because it is labeled Practice.

This layering preserves the important internal distinction: PLE's domain model represents all valid
combinations, while the normal instructor path represents familiar teaching activities. The activity
type sets defaults; it does not conceal the resulting behavior or turn a later advanced choice into
an unexplained exception.

## Learner language

Students should see behavior and purpose, not Rust enum names or implementation settings. For a
mastery assignment, the preferred copy is:

> Keep working until every question is correct. You can start another version afterwards to practice
> the same ideas with fresh values. Your highest completed score is kept.

The screen should also say when a setting changes that experience:

- "You can try this question again" when a retry remains.
- "Feedback is available for this response" when the assignment's applicable timing and
  boundary permit the server to include it.
- "Feedback is not available at this time" when the server withholds it; do not promise that it
  will become available later.
- "This assignment is recorded" when continued practice is closed.
- "Start another practice run" only when the server says another run is allowed.

The last condition is essential. `practice_allowed` in a run summary is advisory presentation state;
`start_or_resume_run` remains the authoritative server transition. The existing run and summary
pages already hide the continuation action unless that state is true, and must continue to handle a
server rejection gracefully.

## Assignment workspace boundary

The Instructor assignment workspace keeps mastery configuration in the same assignment aggregate
while separating the teaching tasks. Questions owns the title and ordered fixed-or-pool content;
Policies owns audience, disclosure, run policies, instructions, schedule, limits, late behavior,
and lifecycle. Each focused save uses the assignment's shared revision and returns the complete
authoritative projection, so a Policies save cannot silently replace Questions content.

An empty persisted Draft is valid while the Instructor builds the assignment across pages. Derived
publication readiness blocks Published until an active deliverable position and valid policy state
exist. Once learner work is issued, a structural Questions change can return the typed
issued-learner-work conflict; the page preserves its draft for recovery. Student view is an
answer-free, non-mutating presentation of the current assignment and does not create a practice run.

Only an ordinary enrolled Student starts or resumes a mastery run and produces submissions, scores,
receipts, and gradebook evidence. The Instructor Student view retains the Instructor identity and
links to explicit Student entry instead of fabricating a learner account.

See [API_CONTRACTS.md](API_CONTRACTS.md#instructor-assignment-workspace),
[FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md#client-contract), and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md#assignment-work-is-one-aggregate).

## Authority and records

The browser presents a learning experience; it does not administer the assignment. The server:

- derives course and learner authority from the authenticated session;
- chooses or resumes the run and assigns the run number;
- issues attempt identifiers, seeds, deadlines, and immutable question provenance;
- validates response format again before calling a trusted grading backend;
- computes correctness, points, retry availability, completion, and grade summary;
- commits response, feedback record, summary projection, and completion transition atomically; and
- applies the current assignment-owned learner-disclosure decision before returning a learner-facing result.

The browser can perform key-free format validation for prompt feedback, show a server-projected
timer, and request a new practice run. It never receives an answer key or gains authority by
constructing a policy, timestamp, run mode, or score. See [SECURITY_MODEL.md](SECURITY_MODEL.md),
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Change checklist

When changing mastery behavior, update the appropriate contract in the same patch:

- Policy vocabulary or serialization: `crates/question_model/src/run_policy.rs` and
  [QUESTION_MODEL.md](QUESTION_MODEL.md).
- Enrollment, run, attempt, or summary behavior: `crates/question_model/src/activity.rs`,
  `crates/domain/`, and [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).
- Server authority, disclosure, or learner response boundary: [SECURITY_MODEL.md](SECURITY_MODEL.md)
  and [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
- Instructor activity-type chooser or learner copy: this document, the relevant Solid route, and the
  browser accessibility contract.

Use behavior-focused tests for changed policy outcomes: a mastery run completing only after all
required questions are correct, an allowed post-completion run receiving fresh variation, a closed
assignment rejecting another run, disclosure obeying policy, and a browser action remaining
subordinate to the server result.
