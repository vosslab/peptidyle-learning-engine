# Production release plan, plain-language revision

Written 2026-09-13. Replaces the wording of
`docs/active_plans/active/production_release_readiness_phase_plan.md`. Same four phases, same
milestone numbers, plain words.

> **Compliance correction, 2026-09-14.** The generic object is Assessment,
> responses save while an Attempt is open, and the whole Assessment Attempt is
> the submission target. Old Assignment/Question
> Attempt wording below is implementation evidence only. Blueprint lifecycle,
> roles, retention, Question IDs, and Ribbon behavior follow Human Guidance.

Who wins when documents disagree:

1. `docs/HUMAN_GUIDANCE.md` wins over everything here.
2. Durable current contracts govern accepted implementation boundaries when they agree with Human
   Guidance.
3. This plan and its detailed milestone plans coordinate only compatible implementation work. An
   omitted or speculative technical requirement does not become product intent by appearing in a
   plan.

Where it stands today:

- Phase 1 is finished (2026-09-12).
- Phase 2 milestones 2.1, 2.2, and 2.3 are finished (2026-09-13).
- Phase 2 milestone 2.4 (the Genetics course) is the current work.
- Phases 3 and 4 have not started.

## Context

On September 12 an audit of the app listed ten problems that could mislead a teacher or a
student, hide a page that actually works, or turn a small back-end hiccup into something a user
sees as broken. The old plan grouped those ten problems into four phases. That grouping was fine.
The writing was not: it leaned on jargon ("projection", "lease fencing", "provider-neutral
operational boundary", "WW-C2") until a reader could not tell what would actually change on
screen.

One thing in the old plan was flat wrong, not just badly worded. Audit item 6 suggested giving
the teacher a "retry grading" button. That misread the platform. In PLE:

The actual invariant:

> Submission of an Assessment Attempt produces its grading outcome automatically. Instructors,
> Students, and Sysadmins do not manually grade, regrade, or retry grading.

What follows from it:

- Grading is part of submission. A student who submits sees their grade. There is no
  user-visible "queued", "grading", or "needs instructor attention".
- Once a grade is recorded, it is final.
- "Recovery" has exactly one meaning: when a timed Attempt runs out, the server submits the whole
  Attempt and finalizes whatever the Student had saved. Questions without a saved response remain
  visibly unanswered, receive zero credit, and count as incorrect without backend evaluation. That
  auto-submission produces a grade like any other submission.
- If grading cannot complete, submission cannot complete. The system is broken, and that is an
  operations problem for the sysadmin (renderer down, database down). It is not a product state
  and not something a student or teacher is asked to interpret or act on.

Consequence for the code as it stands on 2026-09-13: milestones 2.2 and 2.3 were built under the
old reading. They added a background grading pipeline with public states `queued`, `grading`,
`graded`, `needsInstructorAttention`, student-page polling for those states, and gradebook counts
of them. That work is retained as internal machinery only if the detailed plan can show it is
needed to make submission-produces-grade reliable; its user-facing surface is removed. The
correction is milestone 2.5 below and must land before Phase 2 closes.

The earlier "Retry reset" (2026-09-13) removed the manual retry behavior. Milestone 2.5 completes
the grading-lifecycle correction. This plan records both so the mistake does not come back.

## Words used in this plan

Short definitions for terms that come from the code, so the rest reads without a dictionary.

| Word in this plan            | Meaning                                                                                                                                                                                                                      |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Blueprint Course             | Reusable course template. No students, no due dates.                                                                                                                                                                         |
| Course Instance              | A real semester course built from a Blueprint. Has students and due dates.                                                                                                                                                   |
| Assessment Attempt           | One Student occurrence of a Course Instance Assessment. Timed by the server.                                                                                                                                                  |
| Assessment Question Editor   | Instructor page for picking and ordering Questions and Pools.                                                                                                                                                                |
| Assessment Properties Editor | Instructor page for Type, dates, time limit, Attempt count, late work, and disclosure. (Current code calls this "Policies".)                                                                                                |
| top menu bar                 | The teacher navigation bar. (Code calls this the "Ribbon".)                                                                                                                                                                  |
| edit number                  | A counter on a saved record so two people cannot overwrite each other. (Code calls this an "ETag".)                                                                                                                          |
| WeBWorK document             | The HTML that the WeBWorK renderer produces for one question. PLE shows it in a frame and passes the student's form back to WeBWorK for grading. PLE does not read inside it. (Code calls this "opaque" or "backend-owned".) |
| Danger Zone                  | The boxed area on an edit page for actions that destroy something.                                                                                                                                                           |

## The ten audit problems, in plain words

| #   | Problem found on September 12                                                                                                                                    | Where it landed                                           |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| 1   | Teacher could change a due date on screen, press Release, and release the OLD saved due date. Unsaved question-order edits could vanish on navigation.           | Phase 1, done                                             |
| 2   | Assessment Overview and Question Editor pages worked, but the top menu bar hid them.                                                                             | Phase 1, done                                             |
| 3   | WeBWorK questions had never been walked through with real teaching content.                                                                                      | Phase 2, milestone 2.4, in progress                       |
| 4   | "Student view" for teachers said "unavailable". The "delivery check" page showed titles only, no actual questions.                                               | Phase 3                                                   |
| 5   | Students saw no due date, no close date, no time zone before starting.                                                                                           | Phase 2, done                                             |
| 6   | Students could not tell "submitted" from "graded". Grading had a needless 3-second delay per question set. Any renderer hiccup was treated as permanent failure. | Phase 2. Built the wrong way (2.2, 2.3); corrected in 2.5 |
| 7   | Saving a theme made an existing course banner disappear until reload.                                                                                            | Phase 1, done                                             |
| 8   | Grade Settings and Teaching Operations pages existed but called API routes that do not exist.                                                                    | Phase 1, done                                             |
| 9   | Archive/restore for questions and blueprints works on the server but has no buttons.                                                                             | Phase 4                                                   |
| 10  | Expired banner uploads are never cleaned up. No check that a restarted or restored system is healthy.                                                            | Phase 4                                                   |

## Phase 1: Teacher pages tell the truth about what is saved (DONE)

Finished 2026-09-12. Recorded here so the numbering matches the old plan.

- 1.1 Assessment Properties Editor saves simple changes (dates, checkboxes, numbers) as soon as
  they are valid, and shows Saving / Saved / Invalid / Save failed / Conflict. Release is
  disabled until everything on screen is saved. Question-order edits still use an explicit Save
  button and warn before you navigate away with unsaved changes.
- 1.2 Saving a theme no longer hides the banner.
- 1.3 Grade Settings and Teaching Operations were removed. Their URLs now give an ordinary
  not-found page.
- 1.4 Assessment Overview and Assessment Question Editor appear in the top menu bar.

## Phase 2: Students can see deadlines and trust grading (COMPLETE 2026-09-14)

What a student gets when this phase is finished: before starting, they know when the assignment
opens, when it is due, when it closes, and how long the timer is, all in their own time zone.
While working, their answers save automatically. If they lose connection or close the laptop,
they come back to the same attempt with the same timer. When time runs out, whatever they saved
is submitted for them. When they submit, they get their grade.

### 2.1 Deadlines and start decision (DONE 2026-09-13)

- Server sends one record per assignment: open time, due time, close time, time limit, the
  student's time zone, and a yes/no "can start now" with a plain reason if no.
- Student course page and the pre-start page both show that record. The browser formats the
  times; it never decides on its own whether the student may start.
- Students can set their own time zone from the course page. New students inherit the inviting
  teacher's zone once.

### 2.2 and 2.3 Grading (BUILT 2026-09-13 UNDER THE WRONG MODEL)

What was built and is still right:

- A grade, once recorded, is permanent.
- Attempt expiry automatically submits the whole Attempt, finalizes its saved responses, and leaves
  other Questions visibly unanswered. Each receives zero credit and counts as incorrect without
  backend evaluation.
- The 3-second delay is gone.
- No role has a grade, regrade, or retry action.

What was built and is wrong:

- Four public grading states (`queued`, `grading`, `graded`, `needsInstructorAttention`) shown
  to students and, as counts, to teachers.
- Student pages that poll for those states after submitting.
- A gradebook "needs attention" count that asks the teacher to notice a condition they cannot
  act on.

These come from treating grading as a separate background process with its own visible
lifecycle. The product model has none of that. Fixed in 2.5.

### 2.5 Submission produces the grade (COMPLETE 2026-09-14)

What a student gets: press Submit (or the timer runs out), and the result page shows the grade.
No intermediate status, no polling, no "check back later".

What a teacher gets: the gradebook shows grades. Nothing else about grading.

Checkpoint: immutable credit storage, current-point read scoring, direct native/WeBWorK finalization,
and removal of public grading status/detail passed connected database and full registered Live Demo
Attempt acceptance. Session 36718 is historical audit evidence, not the closure receipt: the audit
removed a stale private-table inventory check, restored the Assignment-root expiry-lock authority,
and removed a redundant expiry test/runner whose stale global-empty assertion conflicted with the
preceding race oracle's intentionally retained expired Attempts. The retained security-catalog,
expiry-SQL, race, and no-browser journey evidence remains. Unused native/iMathAS grading-job and
failure machinery was removed without a replacement completion framework. The fresh exact
`source source_me.sh && ./launchers/all_test.sh` passed in session 38324, including 6,004 pytest
tests, database-baseline E2E, installation-data provision/opt-out, Course Appearance
PostgreSQL/MinIO, and complete live acceptance with canonical cleanup empty. Independent audit
reviews approved the cleanup, permissions/identity boundary, documentation, and stale-test
removal. M2.5 is complete.

What done looks like:

- When a Student submits before expiration, submission produces the grade in the same operation.
  Backend failure must not record a partial result or lose saved responses. Human Guidance does
  not resolve the failure message or retry behavior. `expires_at` stays the sole authority for
  whether editing or submission is still available.
- Expiry finalization closes editing at the server-owned deadline. Human Guidance does not resolve
  backend-failure continuation or retry timing for an expired Attempt; the product must not invent
  credit or expose a grading state while that technical design remains open.
- The `queued` / `grading` / `needsInstructorAttention` states, the student polling, and the
  gradebook attention counts are removed from every page and public API.
- WeBWorK grading follows the same rule: submit sends the form to the renderer and records the
  grade it returns, in the same operation.
- One product lifecycle and a direct grading boundary. Whole-Attempt submission asks each backend
  to evaluate its complete saved response, receives immutable credit fractions, stores them, and
  makes the permitted result available. A backend that cannot return the fraction does not create
  a second product lifecycle or authorize polling machinery.

Expiry and the one background job it justifies:

> Attempt expiration is enforced from the server-owned expiration time whenever a Student
> interacts with the Attempt. Because expiry must submit the whole Attempt even when nobody has a
> browser open, a small background process also submits expired Attempts through the ordinary
> submission path. That is the only reason it exists.

- The server already owns `expires_at`. On any **Student** interaction with an Attempt (save,
  reload, submit, starting a new Attempt on the same Assessment), the server first asks "is now
  > = expires_at?" If yes, it submits the Attempt from its saved responses and only then handles
  > the request. This is the correctness guarantee: after expiry, nothing can treat the Attempt
  > as open.
- Reads stay reads. A Gradebook read, a result-page read, or an export never calls a Question
  Backend and never writes. An expired Attempt not yet submitted shows truthfully as "expired,
  submitting" for the short window until the background job reaches it.
- The requirement that justifies background execution, written down: HUMAN_GUIDANCE says an
  expired Assessment Attempt must be submitted automatically, finalizing its saved responses. That
  must hold when nobody has a browser open. A student who abandons an attempt and never returns
  still gets it submitted and graded. Product semantics do not bend to avoid the mechanism.
- The background process has one responsibility: find expired Attempts that still need automatic
  submission and submit them through the ordinary Attempt submission path, the same code a
  Student's Submit uses.
- One submission path for everyone:
  Student Submit or expiry job -> ordinary Attempt submission -> evaluate each complete saved
  response through its Question Backend -> credit fraction -> stored outcome.
- Idempotent: the job and a Student interaction racing on the same Attempt produce one result.
  Backend-failure continuation and retry timing remain unresolved. No per-Attempt timers or public
  grading states are added here.
- Keep what already works. The 09-13 code has a generic worker with an expiry sweep and grading
  workers. 2.5 does not delete background execution and rebuild it. It narrows the existing
  worker to expired-Attempt submission, removes the public grading states, the
  student polling, the gradebook attention counts, and any Instructor-facing grading concept
  that leaked out of it. The mistake in 2.2/2.3 was what the worker represented in the product
  model, not that background execution existed.
- Do not build a generic background-job framework. One narrow process for this one workload.
  Generalize only when a second concrete background workload (Phase 4 banner cleanup is the
  candidate) shows a shared abstraction is worth it.
- Effect on the Gradebook: the "expired, submitting" window is however long the job's pass
  interval is (about a minute), or as long as a backend outage lasts.
- A student's completed-attempt count is checked (and any expired attempt submitted) before a
  new start, so attempt limits stay correct.

Precedent from two LMSes that already do this at scale. Both use server-side background
processing for abandoned timed attempts:

- **Canvas (open source).** `Quizzes::QuizSubmission` stores `end_at` and computes time
  remaining as `end_at - now`. An open submission past `end_at` is `overdue_and_needs_submission`,
  a persisted state exposed in the public API. The answer-backup endpoint refuses to keep saving
  once the submission is overdue, so the database timestamp, not the browser timer, is
  authoritative. `Quizzes::OutstandingQuizSubmissionManager` finds "unsubmitted, started, and
  overdue" attempts and completes and grades them from stored state, and Canvas runs quiz
  submission operations through its Delayed Job background infrastructure. So Canvas has both
  the request-time check and background completion. Source comments record that browser-side
  auto-submit ran late when tabs were inactive, which is why the server time wins.
  Sources: [quiz_submission.rb](https://github.com/instructure/canvas-lms/blob/master/app/models/quizzes/quiz_submission.rb),
  [quiz_submissions_api_controller.rb](https://github.com/instructure/canvas-lms/blob/master/app/controllers/quizzes/quiz_submissions_api_controller.rb),
  [quiz_submission_questions_controller.rb](https://github.com/instructure/canvas-lms/blob/master/app/controllers/quizzes/quiz_submission_questions_controller.rb),
  [Quiz Submissions API](https://canvas.instructure.com/doc/api/quiz_submissions.html).
- **Blackboard Learn (proprietary).** Documentation states the timer runs continuously on the
  server whether or not the student is in the test, that a student who loses connection can
  re-enter with the same remaining time, and that with Auto-Submit on the attempt is saved and
  submitted when the limit is reached, including when the student has left. An Anthology
  support article on an auto-submit defect states the expected behavior is for "the background
  task to force submit the attempt": direct evidence of a server-side background task.
  Blackboard also auto-submits saved in-progress attempts at hard due dates.
  Sources: [Timed assessments](https://help.blackboard.com/node/19846),
  [Test and Survey Options](https://dev-help-docs.blackboard.com/Learn/Instructor/Original/Tests_Pools_Surveys/Test_and_Survey_Options),
  [BU: Testing in Blackboard Learn](https://www.bu.edu/tech/services/teaching/lms/blackboard/instructors/testing-in-blackboard-learn/).

Precedent, corrected: both systems check expiry at request time **and** run background
processing to complete abandoned attempts. PLE does the same, with the background part kept to
expired-Attempt submission only.

- Sysadmin-only: a plain log or health signal when grading fails, so a broken renderer is noticed
  by the person who can fix it.

The whole grading model, four lines:

1. Submit the whole Assessment Attempt. For each complete saved response, the Question Backend
   (native PLE, WeBWorK, iMathAS, or H5P) returns a credit fraction such as 0, 0.67, or 1. PLE
   stores that fraction. It never changes.
2. The Assessment score = sum of (stored credit fraction x that Question's current point value).
3. If a teacher changes a point value, the next read computes the new score from the stored
   fractions. No further interaction with the Question Backend, and no stored score to update.
4. Attempt expiry submits the whole Attempt automatically, finalizing saved responses. Other
   Questions remain visibly unanswered, receive zero credit, and count as incorrect without being
   sent to a backend.
5. When several Attempts are submitted, the highest Assessment Attempt score is used.

> Changing points may change a Student's Assessment score. The stored credit fraction for each
> response is untouched.

Why "regrade" is not a word here: PLE is backend-agnostic. It does not own grading semantics, so
there is no general "grade this again" operation for PLE to have or withhold. The backend graded
the exact response and PLE keeps the outcome. A backend may evaluate a complete saved response
earlier when its interaction requires that work, but the Student sees no grading outcome before
whole-Assessment submission.

Starting design: do not store Assessment scores at all. Store the credit fractions. The Student
result page and Gradebook calculate sum(fraction x current point value) when they read. Pilot grade
export provides the same point-based scores as CSV or TSV for Course-level handling in the home
LMS; it does not add weights, categories, percentages, or synchronization. A point-value edit then
needs no recalculation operation, no queue, and cannot leave a
stale score anywhere; the next read simply computes the new number. The immutable fraction is
exactly the "evidence needed to interpret grading after teaching configuration changes" that
HUMAN_GUIDANCE asks Student Work to retain. Store or recompute derived scores only if a measured
Gradebook read is too slow, and that has to be shown, not assumed.

Today the stored result is a yes/no `correct` flag plus `points_earned` and `points_possible`
frozen at grading time (`schemas/base_schema/grading.sql`). 2.5 changes that: store the credit
fraction as the immutable outcome; derive points from the assignment's current value. WeBWorK
partial credit maps straight onto the fraction.

### 2.4 Genetics Blueprint Course (COMPLETE 2026-09-14)

This is audit item 3, the WeBWorK teaching walkthrough, turned into something durable: build the
Fall Genetics course as one reusable Blueprint Course and prove it works end to end.

What "done" looks like:

- One Blueprint Course with the eleven Genetics topic assignments in order, each mapped to
  question pools that keep the original variation.
- The Blueprint reloads through the ordinary blueprint pages and can create a throwaway Course
  Instance.
- In that Course Instance, a test student can: see a WeBWorK question render legibly (equations,
  sub/superscripts, images where used), answer, save, reload and still see the answer, submit,
  and get a grade that matches what WeBWorK says. The teacher sees the same grade in the
  gradebook.
- With the renderer stopped, Submit fails with a plain message and the attempt stays open. With
  the renderer back, Submit works. No teacher-facing status appears at any point.
- Any topic that cannot be represented is written down with why, not silently dropped.
- No PLE code is added to understand individual WeBWorK control types. WeBWorK owns its own
  questions (HUMAN_GUIDANCE, "Question Backend ownership").
- Real question files are one-time inputs. Copies and probes live in `tests/_temp/` and are
  deleted at the end. Nothing about the Genetics content becomes a permanent test.

The reviewed source path is one opaque WeBWorK path: all six observed BBQ response shapes compile
to faithful static PG questions. WeBWorK renders and grades each generated PG document; PLE never
needs to classify or parse its controls. The r11 corpus preserves the ordered eleven topics, 119
banks, and 20,579 rows, and the private renderer proof passed all 20,579 generated static PG
questions. The fresh canonical runtime at `https://localhost:55390` published `BP-2`, Revision 1,
in session 53170 with those r11 hashes. It is implementation evidence for the
ordinary reusable Blueprint; current product lifecycle calls an adoptable Blueprint Public,
and reusable for the installation lifetime. The proof runtime is intentionally disposable and
makes no production-deployment claim.

The reviewed r5 browser proof passed: it reloaded that exact Blueprint, created throwaway Course
and Assessment records, visibly restored a saved radio selection after reload and a new
authenticated Mary session, navigated directly to the same Attempt, and used the UI to submit
wrong (`0`) then correct (`1`) responses with Student-history/Gradebook agreement. Root inspected
the new Topic 11 screenshot as legible. The r4 outage proof passed with an actual submission
`503`, plain copy, saved-selection restoration before and after renderer recovery, accepted UI
submission, and a healthy renderer. GET receipt R6 establishes public API observations of document
restore and equal saved opaque payload/progress projection, plus reviewed SQL evidence that the
GET writes neither Student Work nor outcomes; it does not claim a private database snapshot. The
registered journey in session 38324 remains the shared no-browser-expiry evidence. Session 78713's
exact aggregate passed before fresh M4 startup (374 Node, 6,004 pytest, corrected PG baseline,
installation provision/opt-out, Course Appearance PostgreSQL/MinIO, complete live acceptance, and
empty owned inventory). Authorized closeout deleted only the temporary
`tests/_temp/genetics_bank_port` corpus/probe tree (208,331 files in 1,490 directories, about
3.59 GiB), with no archive or permanent fixture. The original 11-topic Biology Problems Website
Genetics source, runtime/volumes/`BP-2`, and Git index were untouched. The retained conclusions
above are receipt evidence, not recurring inventory gates.

After this one-time proof closeout, the approved installation scope expanded: the complete free and
open-source Genetics Blueprint is the example course that ships with every installation, including
an installation provisioned with `--without-live-demo`. That durable product content is distinct
from the deleted temporary corpus and from a test fixture, SQL dump, or separate pilot deployment.
The portable content validated. Fresh default and `--without-live-demo` installations expose the
reusable Genetics Blueprint. A repeated provision does not duplicate it. Opt-out omits Live Demo
Accounts, Course, and activity. The full
`source source_me.sh && ./launchers/all_test.sh` gate passed in session 62894.

Ordinary Question Library browsing across pages is covered by server and frontend contract checks.
This receipt does not claim a separate fresh browser proof for paging.

The smallest backend-owned restore correction is implemented and independently approved. The
existing authorized WeBWorK document read carries private saved opaque input and exact Question
Attempt source pins to produce an ephemeral resumed document; it does not replace the immutable
issued HTML. It creates no API/table, expiry rule, GET write, stored outcome, control parser,
queue, submission, or grade action. Existing renderer transport refusal of answer/preview,
process, and source-URL overrides remains in force. Temporary backend-only proof passed real r11
MC, MA, FIB, NUM, and duplicate-MA cases with restored visible controls and no feedback. The r5
real PLE integration proof subsequently completed the new-Mary reload and UI Submit journey.

Order: 2.5 first, then 2.4, so the Genetics walkthrough is run against the corrected grading
model rather than the wrong one. Phase 2 is finished when both are done.

## Phase 3: Teacher "View as Student" (NOT STARTED)

Audit item 4. HUMAN_GUIDANCE asks for "a clearly labeled, answer-free Student view without
changing their identity."

What a teacher gets when this phase is finished: one page that answers two questions.
"Could this student start this assignment right now?" and "What would they actually see?"
The page shows the real questions the way a student sees them before submitting, and creates no
student work, no attempt, and no grade.

### 3.1 Pick whose view

- Teacher chooses a real enrolled student (uses that student's actual attempt history and
  accommodations) or "Simulated preview" (a made-up student with a chosen moment in time and a
  chosen number of prior attempts).
- The two cases are labeled differently on screen so a simulated view is never mistaken for a
  real student's record.
- Only teachers of that course can do this. The server sends no student identity the page does
  not need.

### 3.2 Show the real questions, read-only

What the preview shows, decided here so Phase 3 does not have to guess:

- The preview shows the assignment as a student sees it **before submitting**: instructions,
  the questions, empty answer controls. That state never contains answers or feedback for any
  assessment type, so nothing needs stripping.
- The preview does **not** simulate the after-grading view (own answer, right/wrong, correct
  answer, feedback). Those rules differ by assessment type (Practice shows answers; Quiz and
  Exam wait for the whole class) and belong to a later "preview results as student" feature if
  Neil wants one. Out of scope for this release.

How, without breaking the WeBWorK boundary:

- Native PLE questions: reuse the student display code. It already renders without answers
  before submission.
- WeBWorK questions: ask the renderer for the same pre-submission document it gives a student.
  WeBWorK owns what is in that document. PLE does not read or edit its HTML.
- Do not write a second renderer.
- Submit buttons and the browser form bridge are disabled.

Proof that preview changes nothing:

- The preview server operation runs inside a read-only database transaction. A write attempt
  fails at the database, not by convention. This is the real guarantee.
- A before/after count of attempts, responses, grades, and stored objects is kept as a second,
  cheap check. It catches creation; the read-only transaction catches
  modification.

### 3.3 The page itself

- Scenario picker, selected moment, effective settings summary (due, close, time limit, attempt
  count, late rule, answer disclosure), the start decision in plain words ("Can start" /
  "Cannot start: closes at ..."), the Assessment instructions, and the Questions.
- Opens separately from the editing pages, as Human Guidance requires for Assessment preview.

Phase 3 is finished when a teacher can do all of the above, the preview runs read-only at the
database, and the before/after counts match.

## Phase 4: Archive buttons and housekeeping (NOT STARTED)

Audit items 9 and 10. Two independent pieces that can run side by side.

### 4.1 Archive and restore buttons

The server already supports archiving and restoring published questions and blueprint courses.
Add the buttons.

- "Archive Published Question" and "Archive Blueprint Course" go in the Danger Zone, show the
  shared-availability consequence, and require conspicuous confirmation (HUMAN_GUIDANCE,
  Interface philosophy).
- Restore is an ordinary control on the archived item's page.
- Archiving removes the item from search and from being newly picked in an editor. Nothing
  already pointing at an exact revision breaks:
  - An assignment pinned to an archived question revision still delivers and grades it.
  - A Blueprint Revision that references an archived question revision still creates a Course
    Instance, and that instance still delivers the archived question. Archive hides; it never
    deletes or substitutes.
  - The teacher sees an "archived" marker on such a question in the Question Editor so they
    know why they cannot pick it again.
- No revision editor, no fork UI, no collaboration features. Those stay in TODO.

### 4.2 Cleanup and health checks

- Expired, never-promoted banner uploads get cleaned up automatically in small batches. This
  is the second concrete background workload after expiry submission (2.5); if the two share a
  small abstraction cleanly, share it, otherwise keep them as two narrow passes in the same
  background process. Cleanup deletes only objects the database says are expired and
  unpromoted, and never accepts a file path from a request. Each object ends in one of five
  recorded outcomes: kept (not eligible after all), removed, already gone, try again later
  (temporary store error), or needs repair. "Needs repair" is for a stored checksum that does
  not match the object; cleanup leaves that object alone, records it in a list the sysadmin can
  see, and does not retry it forever. Repair is a sysadmin action (confirm and delete, or
  restore the record), not an automatic one.
- Two processes, each with a health check that says whether it can do its own job. Web server:
  database reachable, schema matches, object store reachable, renderer reachable when WeBWorK
  questions exist. Background process: database reachable, schema matches, object store
  reachable, renderer reachable. Missing configuration shows as "not ready" with a redacted
  reason.
- After a restart, attempts that expired while the system was down are submitted by the
  background process's first pass, exactly once. No restart-specific step exists.
- After restoring a database plus object store from backup, a verification command runs the
  ordinary migration check, type checks, and a few representative reads, and fails closed if the
  database references an object the store does not have.
- Out of scope: AWS, OpenTofu, Podman, container definitions, backup creation. Those are
  deployment choices, not app behavior.

Phase 4 is finished when both pieces pass their checks.

## Release decision

All four phases done, the Genetics walkthrough recorded, and the full gate green:

```bash
source source_me.sh && ./launchers/all_test.sh
```

The full gate is the last regression check, not the proof. This project has already had a green
full gate sitting next to a real defect (the banner-vanishes-on-theme-save bug). Each milestone
above names its own user-facing "done" condition and must show focused evidence for it before
the full gate is run.

Then Neil makes the production call, including the schema freeze (after which schema changes use
forward migrations instead of editing the base schema).

## Rules carried from HUMAN_GUIDANCE that shape the remaining work

Added or clarified since the original plan; listed so no milestone drifts from them.

- Submission of an Assessment Attempt produces its final grade automatically. No manual grading,
  regrading, or retry for any role. No user-visible grading lifecycle.
- Attempts are wall-clock timed by the server. Reconnecting resumes the same attempt. Expiry
  auto-submits saved answers.
- Regular assignments default to unlimited attempts; teachers may restrict.
- Assessment types: Regular Assignment, Practice Question Assignment, Bonus Assignment, Quiz,
  Exam. Assignment is not an object or parent Type. Practice uses the same submission boundary and
  shows the correct answer immediately after submission. Regular and Bonus rarely show the correct
  answer (the Student sees their response and right/wrong). Optional Question Feedback is shown when
  provided and does not use Assessment correct-answer disclosure settings.
  Quiz and Exam show the correct answer only after the whole class has finished. This affects
  what Phase 3's preview must strip and what the Properties Editor offers; it is not itself a
  milestone here.
- Blueprint Courses save as immutable Blueprint Revisions on each explicit Save. Course Instances
  pin one revision. Newer revisions reach instances through quick teacher review, never silently.
  Phase 2.4 builds on this; Phase 4.1 archive must not break pinned revisions.
- WeBWorK, iMathAS, and H5P own their own question rendering and grading. PLE passes the
  document through and records the outcome. No per-control PLE code.
- Native PLE JSON is unversioned; when the shape changes, all stored questions upgrade together.
- Students never upload files. Teachers create content through text boxes.
- Every source file stays under 1000 lines.
- Tests are liabilities too. One-time walkthroughs stay one-time. Permanent tests protect stable
  behavior only.

## Verification

- Each milestone runs its focused Rust, Node, or Python tests, then the offline aggregate
  `launchers/run_fast_checks.sh`, then one `docs/CHANGELOG.md` entry.
- Phase exits run the full gate `source source_me.sh && ./launchers/all_test.sh`, which
  includes the disposable live stack and browser journeys.
- Phase 3 additionally requires the before/after count proof for non-mutation.
- Phase 4.2 additionally requires the restore-verification command to pass on a restored copy.

## Decisions made in this revision (Neil can overrule any of them)

- Grading is part of submission. The 2.2/2.3 background grading lifecycle and its four public
  states are a design mistake; milestone 2.5 removes their user-facing surface before 2.4 runs.
- The Question Backend produces the credit fraction; PLE stores it immutably; scoring uses
  stored fractions x current point values, computed on read. No stored Assessment score, no
  recalculation operation. Store derived scores only if a measured read is too slow.
- Reads are reads. Gradebook, result page, and export never finalize, grade, or write.
- Attempt expiry is enforced on every Student interaction (correctness) and by one small
  background process (completeness, because expiry must submit even with no browser open). That
  process submits expired Attempts through the ordinary path. It is not a grading daemon and owns
  no product states.
- Keep the existing 09-13 worker infrastructure where it already provides that; narrow it
  rather than delete and rebuild. What is removed is what leaked into the product model.
- 2.4 needs no human choice. PG files use the WeBWorK path; flat banks use the supported native
  import types. The manager proceeds.
- Phase 3 preview shows the pre-submission student view only. After-grading disclosure preview is
  a separate future feature.
- Phase 3 non-mutation is guaranteed by a read-only database transaction, with counts as a
  secondary check.
- Archive hides; it never deletes or substitutes. Pinned and Blueprint-referenced revisions keep
  working.
- Cleanup checksum mismatch becomes a visible "needs repair" item for the sysadmin, not a silent
  stop and not an infinite retry.
- Two processes: the web server and one narrow background process. No generic job framework
  until a second workload (Phase 4 banner cleanup) shows a shared abstraction is worth it.
- Precedent corrected: Canvas and Blackboard both check expiry at request time and run
  background processing to complete abandoned attempts.

Nothing in this plan blocks on a human answer.
