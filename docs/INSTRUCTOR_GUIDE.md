# Instructor guide

This guide follows the supported browser teaching loop: create a course, invite a learner through
the course roster, build a timed Mastery assignment from the published problem corpus, and confirm
learning in the gradebook. Start the local system first with [USAGE.md](USAGE.md).

All people and course records shown in these captures are fictional live-demo data. The seeded
personas are ordinary PLE Instructor and Student records in the disposable baseline; regeneration
discards them and recreates the same fictional baseline.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Instructor assignment workspace Policies page showing delivery, lifecycle, and disclosure controls](screenshots/instructor/assignment_workspace/01_assignment_policies.png)

![Instructor roster showing active course members](screenshots/instructor/course_management/01_instructor_active_roster.png)

![Instructor assignment workspace Student view showing the answer-free learner landing](screenshots/instructor/assignment_workspace/02_student_view.png)

![Instructor gradebook showing Mary Okafor at 100 percent on Peptide Bonds Guided Practice](screenshots/instructor/grading/01_instructor_gradebook.png)
<!-- screenshots:end -->

## Before you begin

- Launch the local stack and open its HTTPS URL.
- Sign in through the visible PLE account page. Email is the canonical passwordless path; an
  ordinary passkey is an optional shortcut for an existing account.
- When the deployment enables the seeded persona selector, choose the fictional live-demo Instructor
  persona. It enters the same PLE account/session state and is unavailable when that deployment gate
  is not configured.

## Create a course

1. Sign in through the visible PLE account page with email, an ordinary passkey, or the enabled
   seeded instructor selector.
2. Enter a descriptive title in **Course title**.
3. Enter the inclusive **Start date** and **End date** for the teaching term.
4. Enter the exact case-sensitive **Time zone (IANA)**, such as `America/Chicago`.
5. Activate **Create course**.
6. Open the new course card.

The created course is a real PostgreSQL-backed course. It is not a browser fixture or an API-only
arrangement. The form never guesses a browser time zone. If a term value is invalid, it preserves
all four inputs, announces and focuses the field to correct, and supports an immediate retry.

## Invite a student

1. Open **Students** from the course navigation.
2. Enter the learner's **Institutional email** and course-scoped **Institutional student ID**.
3. Activate **Create invitation**.
4. In **Share this invitation**, copy the one-time link and share it through the trusted LMS or
   another trusted course channel. Configured SMTP may deliver the same link, but the copy-link
   path remains available.
5. Confirm the roster reports **Invitation pending** until the learner authenticates their PLE
   account and claims the invitation; after claiming, confirm the member reports **Active**.

## Build an assignment

1. Return to the course and open **New assignment**.
2. Enter the assignment title.
3. Activate **Create assignment draft**. The system creates a real Draft and opens its **Questions**
   page; the assignment title link in the course list opens the assignment's **Overview** page.
4. On **Questions**, prefer **Reuse questions from an existing assignment**. Choose the source
   assignment, then add
   its entire question set or use the checklist for a subset.
5. For an occasional direct lookup, copy the visible `AAA-BBBB` Question ID from the published
   problem catalog and paste it into **Add by question ID**. Never substitute a UUID.
6. Confirm the selected list contains the intended questions in teaching order. Engine and response
   labels identify the mix without imposing a fixed question count.
7. Use **Replace question** on Questions when an assigned item needs a revision. Review the
   existing and replacement titles and IDs, then save **Questions and order**. The replacement
   applies to future runs; issued learner work retains its original question.
8. Open **Policies** from the assignment-local navigation. Enter the plain-text student
    instructions, choose **Published**, and
   set availability, due, and close times in the displayed course time zone. Set **15** minutes per
   run, the intended attempt limit, late-submission behavior, and deadline behavior, then activate
   its separate save control.
9. In **What students can see**, choose a timing for each independent field: **Score**,
   **Per-item correctness**, **Feedback text**, **Correct answer or solution**, and **Class
   statistics**. Each offers During attempt, After submit, After due, After close, and Never.
   After due and After close remain withheld when the matching boundary is not set. For this guide,
   use After submit for the first four fields and Never for class statistics.

The assignment-local navigation keeps the four tasks together:

- **Overview** is the assignment home. It reports the current state, readiness, instructions, and
  delivery summary.
- **Questions** owns the title, fixed questions, reusable pools, ordering, reuse, and replacement.
  Its **Save questions and order** action commits the complete ordered definition.
- **Policies** owns delivery and publishing: lifecycle, learner instructions, schedule, limits,
  run policies, and feedback visibility. Its save action is separate from Questions.
- **Student view** is a stable-identity, answer-free inspection of the current live assignment.
  It keeps the Instructor session and creates no learner run, submission, grade, or other work.

Use the assignment title link to return to **Overview** at any time. The supported paths are
`/instructor/courses/:courseRef/assignments/:assignmentRef`, with `/questions`, `/policies`, or
`/student-view` appended for the focused pages. Public route references locate the assignment;
they do not grant authority.

Above those controls, verify the current server status. It states the stored
intent and the current clock result separately, for example **Published, open
now** or **Published, closed since 2026-09-01 23:59 America/Chicago**. This
status comes from the server's authoritative time; changing the computer clock
does not change it.

The Policies save is independent of the Questions save but shares the assignment revision. A
conflict offers the current server values without discarding the local Questions or Policies draft.
Invalid or ambiguous local times, a timestamp outside the course term, invalid ordering, or an
illegal lifecycle transition preserve the draft, announce the exact field, and move focus there.
Closing removes learner start access; archiving is terminal and cannot be reopened.

Only corpus publication is arranged outside the browser. Course creation, roster activation,
assignment reuse, Question ID lookup, timing, and assignment construction use visible instructor
controls. If an ID is malformed, unavailable, unauthorized, or already selected, Questions keeps
the pasted text and selected questions unchanged so it can be corrected and tried again.

## Inspect and run as a student

From the assignment workspace, open **Student view** to inspect the current live learner landing.
The view has a stable assignment identity, contains no answer material, and leaves the Instructor
session in place. It is an inspection surface only: it does not start a run or create graded work.

For graded work, sign out, choose a seeded Student and authorized course, open the assignment through
the ordinary Student course page, and choose **Start assignment**. Submit through the visible response
controls. If the response is accepted for grading, use **Check grading status** until feedback or an
instructor-attention state appears. When attention appears, sign back in as the Instructor, open
**Grading operations**, review the metadata-only recovery row, and choose its currently enabled named action
when the operation is eligible. Follow the operation's current state and available action, then open **Gradebook**
and confirm the current score. This
ordinary Student entry creates the real learner run, submission, receipt, grade, and
instructor-visible gradebook history. Instructor **Student view** remains the answer-free inspection
surface.

## Configure course grades

The current WP-PROF-S6 slice supports two course-grade modes:

- **Total points** adds included assignment scores over included points possible.
- **Weighted categories** assigns ordered categories and weights, with optional drop-lowest rules
  inside each category.

Completion-based grading is deferred to a later package and is not available in the current selector.
Open **Grade settings** from the course navigation to read the current scheme. Assignment titles in
this view come from the server and are read-only; the settings payload changes inclusion, category,
and position, not a title. Save replaces the whole scheme and requires the current strong revision. If
another instructor saved first, reload the settings and retry the preserved changes after the revision
conflict.

The gradebook totals view is a compact server projection. It uses one scheme snapshot and maintained
assignment summaries, and reports a score or an explicit unavailable state such as recalculating,
failed, empty after drop, or zero possible points. The browser does not recompute totals. The course
**Export grades CSV** action is synchronous and bounded to 500 active-student rows. Email and display
name are used only in the ephemeral instructor download; the durable export audit stores no learner
PII, only course, actor, revision, mode, rounding, row count, and timestamp metadata.

This is an accepted capability. Connected evidence runs under the fixed
`ple-live-demo-browser` owner: one canonical production-browser invocation is
followed serially by the distinct WebWork renderer and two-API/one-PostgreSQL
replica service oracles.

## Adopt reusable curriculum

Open **Curriculum adoption** from an Instructor course. This is the live teaching workflow for
turning reusable meaning into ordinary course state. Choose a Blueprint assignment, an Alpha course,
course rollover, term shift, or import inspection. Select the source and target term when the
operation requires them, then choose **Prepare proposal**. To make an independently editable Alpha,
open its public curriculum detail and choose **Create independent copy**.

Review the server-owned, answer-free proposal before applying it. The proposal identifies its source
revision, destination, schedule resolution, and any exact Question ID that needs replacement. If a
source pin is no longer available or a local time falls in a DST gap or ambiguity, the page preserves
the draft choices and names the correction; choose a replacement or corrected time and prepare a new
proposal.

- **Blueprint or Alpha instantiation:** create an ordinary draft assignment or a new ordinary
  teaching course with its explicit target term, title, and source revision.
- **Fork Alpha:** from the public Alpha detail, create an independently editable Alpha with
  immutable source-lineage evidence.
- **Rollover:** create the next teaching course while leaving rosters, learner records, attempts,
  grades, retention, and issued work behind. The destination starts empty of learner activity.
- **Term shift:** move all unissued assignment schedules together. Relative calendar-day and local
  wall-clock values resolve in the target course IANA zone. Any issued run makes in-place shifting
  ineligible; use rollover instead.
- **Import maintenance:** inspect the current import, fast-forward only an eligible untouched
  assignment, or create a new source-derived draft when the destination diverged. The divergent
  assignment remains unchanged.

Activate **Apply proposal** only after reviewing the destination and correction state. The server
accepts the exact eligible preview, commits atomically, and returns an immutable receipt. Use
**Check receipt evidence** to reconcile B2-owned derived rows; incomplete immutable evidence refuses
reconciliation and requires operator recovery. The workflow exposes the destination and next action
after success.

## Review learning

After the student completes and repeats the assignment, open **Gradebook** and expand **View run
history** for the assignment row. The captured pilot shows Best and Latest at 100 percent, Completed
at 2, and two completed run-history entries.

The companion [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the learner path. The platform keyboard
contract is documented in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
