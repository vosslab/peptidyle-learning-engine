# Instructor guide

This guide follows the supported teaching loop: choose or create a reusable Blueprint Course,
create a Course Instance for one teaching term, invite Students, deliver assignments, and confirm
learning in the Gradebook. Start the local system first with [USAGE.md](USAGE.md).

**Blueprint Course** is shared course-level reusable content and structure. A published Blueprint
Course is visible to every vetted (approved) Instructor. A draft is private to its owning workspace
and authorized collaborators. A Blueprint Course has no Students, live deadlines, releases,
accommodations, grades, or delivery settings.

**Course Instance** is the teaching and delivery aggregate created from exactly one Blueprint Course.
Its parent and applied Blueprint revision are immutable. A Course Instance is private to its current
equal co-Instructors and enrolled Students, and owns enrollment, deadlines, releases,
accommodations, grades, and delivery settings. It never sends those teaching records back upstream.

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

## Create a course instance

1. Sign in through the visible PLE account page with email, an ordinary passkey, or the enabled
   seeded Instructor selector.
2. From **Courses**, choose **Create Course Instance**.
3. Choose an existing published **Blueprint Course**, or choose **Create Blueprint Course** to make
   the minimal new Blueprint parent first. A Course Instance cannot exist without exactly one parent.
4. Enter the teaching title, inclusive **Start date** and **End date**, and exact case-sensitive
   **Time zone (IANA)**, such as `America/Chicago`.
5. Activate **Create Course Instance**, then open the new Course Instance card.

The source Blueprint contributes reusable definitions, policies, theme defaults, and reviewed
relative schedule offsets. The new Course Instance starts with no Students, invitations, runs,
responses, grades, retention state, or issued work copied from another course. Its live dates are
resolved against this instance's term and time zone. The form never guesses a browser time zone. If
a term value is invalid, it preserves all inputs, announces and focuses the field to correct, and
supports an immediate retry.

## Invite a student

1. Open **Students** from the Course Instance navigation.
2. Enter the learner's **Institutional email** and course-scoped **Institutional student ID**.
3. Activate **Create invitation**.
4. In **Share this invitation**, copy the one-time link and share it through the trusted LMS or
   another trusted course channel. Configured SMTP may deliver the same link, but the copy-link
   path remains available.
5. Confirm the roster reports **Invitation pending** until the learner authenticates their PLE
   account and claims the invitation; after claiming, confirm the member reports **Active**.

Students belong only to the Course Instance. They cannot enroll in or browse a Blueprint Course.

## Build and publish a blueprint

Use the reusable-course workspace when the content or structure should be available to other terms
or other vetted Instructors.

1. Open **Blueprint Courses** and choose **New Blueprint Course**, or open a draft you own or are
   authorized to edit.
2. Add ordered modules or weeks and ordered assignments. For each assignment, select already
   published questions by their visible `AAA-BBBB` Question IDs, or use the saved-assignment picker
   and reusable pools. Do not substitute a UUID.
3. Save the complete ordered definition as one Blueprint revision. Relative calendar-day and local
   wall-clock values are reusable defaults, not live deadlines.
4. Review the answer-free projection and choose **Publish Blueprint**. Publishing makes that
   revision discoverable to every vetted Instructor; it does not enroll Students or release work.

An unpublished draft remains private to its workspace. A published Blueprint revision is immutable;
editing it creates a new revision and does not silently change any existing Course Instance.

## Apply a blueprint to teaching

When a Course Instance is created from a Blueprint, the instance receives the reusable definitions
and an applied source revision. Review the resulting assignments in the instance before release.

When a later Blueprint revision adds an assignment, each daughter Course Instance receives that
assignment as **Unreleased**. The current equal co-Instructors review its Questions and Policies,
resolve its schedule against the instance term, and explicitly release it. Propagation never silently
releases an assignment or overwrites instance-owned delivery changes.

The assignment-local navigation keeps delivery work together:

- **Overview** is the assignment home. It reports current state, readiness, instructions, and
  delivery summary.
- **Questions** owns fixed questions, reusable pools, ordering, reuse, and replacement. Its
  **Save questions and order** action commits the complete ordered definition.
- **Policies** owns instance delivery and publishing: learner instructions, lifecycle, schedule,
  limits, run policies, and feedback visibility. Its save action is separate from Questions.
- **Student view** is a stable-identity, answer-free inspection of the current live assignment. It
  keeps the Instructor session and creates no learner run, submission, grade, or other work.

Use the assignment title link to return to **Overview**. The supported paths are
`/instructor/courses/:courseRef/assignments/:assignmentRef`, with `/questions`, `/policies`, or
`/student-view` appended for the focused pages. Public route references locate the assignment; they
do not grant authority.

Above those controls, verify the server status. It states stored intent and current clock result
separately, for example **Published, open now** or **Published, closed since 2026-09-01 23:59
America/Chicago**. Changing the computer clock does not change it.

The Questions and Policies saves share the assignment revision. A conflict offers current server
values without discarding local work. Invalid or ambiguous local times, a timestamp outside the
Course Instance term, invalid ordering, or an illegal lifecycle transition preserve the draft,
announce the exact field, and move focus there. Closing removes learner start access; archiving is
terminal and cannot be reopened.

## Manage blueprint changes

Use one explicit path for each kind of change:

- **Fork Blueprint:** from a published Blueprint detail, choose **Fork Blueprint**. The result is an
  independently editable Blueprint with immutable source-lineage evidence and no live tether.
- **Publish Blueprint:** from a private draft, choose **Publish Blueprint** after reviewing the
  complete answer-free projection. This is the boundary that makes the revision reusable by all
  vetted Instructors.
- **Propose Blueprint update:** revise the source, then in each affected Course Instance choose
  **Prepare update proposal**. Review the source revision, imported assignment manifest, question
  replacements, and resolved schedule before **Apply proposal**. New assignments remain unreleased.
  Divergent instance work is preserved; use the explicit selected-copy or new-assignment action
  instead of an implicit merge.
- **Rollover Course Instance:** choose **Rollover course**, select the target term, and review the
  manifest. The new Course Instance receives reusable definitions but no Student memberships,
  invitations, attempts, responses, grades, retention state, or issued evidence.
- **Shift Course Instance term:** choose **Shift course term** only for an existing instance with
  no issued learner work. Preview every resolved date in the target IANA time zone, correct any DST
  gap or ambiguity, and apply the witnessed proposal atomically. If work has been issued, use
  rollover instead; issued evidence keeps its original term context.

Use **Check receipt evidence** after applying a proposal. An incomplete receipt refuses reconciliation
and requires operator recovery. Course Instance deadlines, releases, accommodations, grades, and
other delivery settings remain instance-owned after every operation.

## Inspect and run as a student

From the assignment workspace, open **Student view** to inspect the current live learner landing.
The view has a stable assignment identity, contains no answer material, and leaves the Instructor
session in place. It is an inspection surface only: it does not start a run or create graded work.

For graded work, sign out, choose a seeded Student and authorized Course Instance, open the assignment
through the ordinary Student course page, and choose **Start assignment**. Submit through the visible
response controls. If the response is accepted for grading, use **Check grading status** until
feedback or an instructor-attention state appears. When attention appears, sign back in as the
Instructor, open **Grading operations**, review the metadata-only recovery row, and choose its
currently enabled named action when the operation is eligible. Follow the operation's current state
and available action, then open **Gradebook** and confirm the current score. Ordinary Student entry
creates the real learner run, submission, receipt, grade, and Instructor-visible Gradebook history.

## Configure course grades

The current supported Course Instance grade modes are:

- **Total points** adds included assignment scores over included points possible.
- **Weighted categories** assigns ordered categories and weights, with optional drop-lowest rules
  inside each category.

Completion-based grading is deferred to a later package and is not available in the current selector.
Open **Grade settings** from the Course Instance navigation to read the current scheme. Assignment
titles come from the server and are read-only; settings change inclusion, category, and position, not
a title. Save replaces the whole scheme and requires the current strong revision. If another
Instructor saved first, reload the settings and retry preserved changes after the revision conflict.

The Gradebook totals view is a compact server projection. It reports a score or an explicit unavailable
state such as recalculating, failed, empty after drop, or zero possible points. The browser does not
recompute totals. **Export grades CSV** is synchronous and bounded to 500 active-Student rows. The
protected display name appears in Instructor views; roster ID and email remain export-only. The
durable export audit stores no Student PII, only Course Instance, actor, revision, mode, rounding,
row count, and timestamp metadata.

## Review learning

After a Student completes and repeats an assignment, open **Gradebook** and expand **View run
history** for the assignment row. Confirm Best and Latest scores, Completed count, and the authorized
run-history entries after a fresh read.

The companion [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the learner path. The platform keyboard
contract is documented in [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
