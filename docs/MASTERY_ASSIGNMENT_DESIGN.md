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

A mastery assignment asks a human question: "Can the student now apply this concept reliably?" It
does not ask whether the student happened to complete one fixed worksheet first. Appropriate student
behavior is to retry, use feedback, work through fresh instances, and stop only when the idea is
dependable.

The durable product promise is:

- Completing one run records an accomplishment; it does not erase the opportunity to learn more.
- A later practice run keeps the earlier run, responses, scores, and provenance as educational
  history.
- Fresh variation supports transfer to a new instance instead of memorization of a previous answer.
- The student sees only feedback authorized for the question and assignment context.
- The authenticated server, not the browser, decides timing, correctness, retries, completion, and
  scores.

This is particularly useful in an open-book course: students should read, reason, draw, calculate,
and connect evidence rather than stop at recall of a familiar prompt.

## Current model

### Durable Student Work Records

The implemented Student Work Records model has four durable records and one selected course result:

| Level              | Role                                           | Important consequence                                               |
| ------------------ | ---------------------------------------------- | ------------------------------------------------------------------- |
| Student Record     | One Student's protected course record          | Separates course records from global Account identity               |
| Assignment Attempt | One pass through an Assignment                 | Records mode, variation, server timestamps, and completion          |
| Issued Question    | One selected Question in an Assignment Attempt | Preserves version, order, selection evidence, and scoring treatment |
| Question Attempt   | One server-issued try under an Issued Question | Preserves seed, timing, response, and grading result                |

`AssignmentGrade.first_completed_at` derives whether the Student has completed
the Assignment at least once. A new Assignment Attempt after that point is
`Practice`; an earlier Assignment Attempt is `Assigned`. The system does not
store a competing within-attempt `complete` flag: completion is derived from
current required-question states, then recorded as one server transition. These are implemented in
[Question Model Student Work Records](../crates/question_model/src/lib.rs) and
[crates/domain/src/completion.rs](../crates/domain/src/completion.rs).

`AssignmentGrade` is the selected course result for one Student Record and
Assignment. **Assignment Progress** is the separate derived activity view;
`AssignmentProgressRecord` is its internal server representation.

### One active question

PLE permits at most one unresolved Question Attempt per Assignment Attempt. Starting or reopening
an Assignment returns the existing active Assignment Attempt when there is one; it does not create
a competing attempt or timer. Once
a response commits, the server advances through unattempted positions before offering an allowed
retry. This makes resume safe and keeps attempt sequencing understandable.

The route begins an Assignment Attempt only after authenticating the session.
The canonical path validates its policy, timing, sequence, and one-active-attempt
rules before it commits activity. See
[Question Model Student Work Records](../crates/question_model/src/lib.rs),
and [crates/domain/src/effective_assignment_policy.rs](../crates/domain/src/effective_assignment_policy.rs).

### Retries preserve evidence

An incorrect response enters `RetryAvailable` only when policy permits another response. Starting
the retry issues a new `QuestionAttempt`; it never overwrites the prior response, grade, seed, or
Question Attempt Reproduction Details. An unlimited question attempt policy is represented by `max_attempts: None`. This is the
implemented mastery retry behavior in
[Question Model Student Work Records](../crates/question_model/src/lib.rs) and
[the Domain Assignment Activity module](../crates/domain/src/lib.rs).

### Completion and gradebook score differ

Completion is a condition on one Assignment Attempt. Grade selection is a separate condition across
completed Assignment Attempts. For example, requiring all Questions correct determines when a mastery
Assignment Attempt completes; retaining the highest Assignment Attempt score determines what reaches the
gradebook. Neither rule implies the other.

`AllCorrect`, `AnswerAll`, and `ScoreAtLeast` are implemented completion requirements. `First`,
`Latest`, `Highest`, and `InstructorSelected` are implemented grade policies. Highest-score ties
retain the earlier run, which keeps the selected grade pointer stable. The pure rules live in
[crates/domain/src/completion.rs](../crates/domain/src/completion.rs) and
[crates/domain/src/scoring.rs](../crates/domain/src/scoring.rs).

### Continued practice and Question Variation differ

Continued practice answers whether another Assignment Attempt may begin after first completion:

- `Unlimited` allows any number of additional Assignment Attempts.
- `Capped` allows a stated number after the first completed Assignment Attempt.
- `Closed` rejects another Assignment Attempt after completion.

Question Variation Rule answers what changes in a new Assignment Attempt:

- `ReuseQuestionsWithNewSeeds` retains the Assignment's selected Questions and issues fresh Question Seeds.
- `SelectedQuestionVariants` uses Instructor-selected Question Variants.
- `RedrawQuestionPools` redraws Question Pools and issues fresh Question Seeds.

The continued-practice rule does not choose a grade, and the Question Variation Rule does not permit an
Assignment Attempt. This separation is deliberate and implemented in
`crates/question_model/src/assignment_activity_rules.rs` and
[the Domain Assignment Activity module](../crates/domain/src/lib.rs). A resumed existing attempt keeps its stored
seed; only a newly issued instance receives a fresh one.

## Recommended mastery bundle

PLE currently stores the independent policies below. The recommended mastery configuration is an
explicit composition of those existing values, not a hidden special case:

| Concern                 | Recommended mastery value     | Student meaning                                                                                             |
| ----------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Completion              | `AllCorrect`                  | Keep working until every required question is correct                                                       |
| Grade                   | `Highest`                     | Further practice cannot lower the recorded best score                                                       |
| Continued practice      | `Unlimited`                   | Start another run after completion whenever useful                                                          |
| Question Variation Rule | `ReuseQuestionsWithNewSeeds`  | Keep the selected Questions while using fresh generated values                                              |
| Question attempts       | `max_attempts: None`          | Retry a question until correct                                                                              |
| Student disclosure      | All five fields `AfterSubmit` | See the selected score, correctness, teaching feedback, solution, and permitted statistics after submitting |
| Timing                  | `Untimed`                     | Work at a learning pace rather than against a clock                                                         |

The first four fields are assignment `AssignmentActivityRules`. The Assignment also owns the five independent
Student Feedback Release timings. Attempt count and Question timer are immutable properties of the selected
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

Each assignment independently schedules five student-visible fields: score,
per-item correctness, feedback text, solution, and class statistics. Each uses
one timing: `DuringAttempt`, `AfterSubmit`, `AfterDue`, `AfterClose`, or
`Never`. The server first requires S5 entitlement, then uses the current
S3-resolved effective-policy verdict and authoritative time to evaluate the
current assignment policy. A field scheduled `AfterDue` or `AfterClose` remains withheld when its
corresponding boundary is absent; a withheld field is omitted rather than sent
as a hidden null. `feedback_release` is immutable audit evidence of an
instructor action, never a student-result unlock.

For the recommended mastery bundle, set all five fields to `AfterSubmit`.
An assessment can instead schedule each field independently without changing
the selected question or its retry bound.

Question Feedback is intentionally not serializable or debug-printable. It has
separate selected-choice, correct-outcome, and incorrect-outcome feedback;
automatic grading selects only the applicable authored content. A Question Hint
is separate pre-response instructional support and never belongs to Question
Feedback or the post-grade `StudentFeedback` DTO. The Native adapter verifies
the exact issued Question through its separate `hint_for_issued_question` path
before it provides a Question Hint. The public `StudentFeedback`
DTO is the automatic, policy-released result for one Student; it omits locked
fields rather than sending hidden nulls. The implementation is
in [crates/question_model/src/feedback.rs](../crates/question_model/src/feedback.rs) and
`crates/domain/src/student_feedback_release.rs`; Store
integration returns with the fresh course-delivery reconstruction.

### Time is server-owned

`QuestionAttemptTimeLimit` supports untimed, per-question, and per-attempt limits with an explicit grace period.
The browser displays remaining time, but only server-issued timestamps and the server timing verdict
can accept, auto-submit, or reject work. Assignment access policy separately controls visibility,
availability, due date, closing date, late treatment, whole-run limits, and run caps. This is why a
mastery bundle can be untimed while an institution still gives an assignment an availability window.

The policy types are in
`crates/question_model/src/assignment_activity_rules.rs` and
[crates/question_model/src/assignment.rs](../crates/question_model/src/assignment.rs). Run creation
checks resolved assignment timing in both stores before it creates a run.

## Human-facing activity types

### Current interface

The current instructor editor is an advanced policy editor. It exposes completion, grade, continued
practice, and Question Variation independently. The student-facing pages use mastery-oriented language such
as "Start or resume practice", "Keep practicing with fresh Question Seeds", and "Start fresh practice".
Those are current user-interface facts, not evidence that a formal assignment-type chooser exists.

### Planned simplification

The following is a planned instructor-facing layer over the existing orthogonal model. It should be
implemented as recognizable activity types with safe defaults, not as a new persisted combined enum.
The stored Assignment remains `AssignmentActivityRules`, five independent Student Feedback Release timings, selected
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
not affect the gradebook": current `AssignmentAttemptGradeRule` always chooses a completed Assignment Attempt or awaits an
instructor choice. Until a grade visibility or weighting policy exists, the UI must not claim that
practice is ungraded merely because it is labeled Practice.

This layering preserves the important internal distinction: PLE's domain model represents all valid
combinations, while the normal instructor path represents familiar teaching activities. The activity
type sets defaults; it does not conceal the resulting behavior or turn a later advanced choice into
an unexplained exception.

## Student language

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
Policies owns audience, Student Feedback Release Rules, Assignment activity rules, instructions, schedule, limits, late behavior,
and lifecycle. Each focused save uses the assignment's shared revision and returns the complete
authoritative projection, so a Policies save cannot silently replace Questions content.

An empty persisted Draft is valid while the Instructor builds the assignment across pages. Derived
Assignment Publication Readiness for the exact Draft Assignment Revision blocks Published until an active deliverable position and valid policy state
exist. Once student work is issued, a structural Questions change can return the typed
issued-student-work conflict; the page preserves its draft for recovery. Student view is an
answer-free, non-mutating presentation of the current assignment and does not create a practice run.

Only an ordinary enrolled Student starts or resumes a mastery run and produces submissions, scores,
receipts, and gradebook evidence. The Instructor Student view retains the Instructor identity and
links to explicit Student entry instead of fabricating a student account.

See [API_CONTRACTS.md](API_CONTRACTS.md#instructor-assignment-workspace),
[FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md#client-contract), and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md#assignment-work-is-one-aggregate).

## Authority and records

The browser presents a learning experience; it does not administer the assignment. The server:

- derives course and student authority from the authenticated session;
- chooses or resumes the Assignment Attempt and assigns its attempt number;
- issues attempt identifiers, seeds, deadlines, and immutable Question Attempt Reproduction Details;
- validates response format again before calling a trusted grading backend;
- computes correctness, points, retry availability, completion, and grade summary;
- commits response, feedback record, summary projection, and completion transition atomically; and
- applies the current assignment-owned student-disclosure decision before returning a student-facing result.

The browser can perform key-free format validation for prompt feedback, show a server-projected
timer, and request a new practice Assignment Attempt. It never receives an answer key or gains authority by
constructing a policy, timestamp, Assignment Activity configuration, or score. See [SECURITY_MODEL.md](SECURITY_MODEL.md),
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Change checklist

When changing mastery behavior, update the appropriate contract in the same patch:

- Policy vocabulary or serialization: `crates/question_model/src/assignment_activity_rules.rs` and
  [QUESTION_MODEL.md](QUESTION_MODEL.md).
- Enrollment, run, attempt, or summary behavior: `crates/question_model/src/student_work.rs`,
  `crates/domain/`, and [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).
- Server authority, disclosure, or student response boundary: [SECURITY_MODEL.md](SECURITY_MODEL.md)
  and [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
- Instructor activity-type chooser or student copy: this document, the relevant Solid route, and the
  browser accessibility contract.

Use behavior-focused tests for changed policy outcomes: a mastery run completing only after all
required questions are correct, an allowed post-completion run receiving fresh variation, a closed
assignment rejecting another run, disclosure obeying policy, and a browser action remaining
subordinate to the server result.
