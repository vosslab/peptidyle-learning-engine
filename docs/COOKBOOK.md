# Cookbook

Practical recipes for operating the disposable PLE developer stack and walking the supported
Instructor, Student, and Sysadmin workflows. Use [INSTALL.md](INSTALL.md) for installation,
[USAGE.md](USAGE.md) for commands, and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) for the full
browser walkthrough.

A **Blueprint Course** is reusable course-level content and structure. A published Blueprint is
visible to every vetted Instructor; a draft is private to its owning workspace and collaborators.
It has no Students, live deadlines, releases, accommodations, grades, or delivery settings. A
**Course Instance** is created from exactly one Blueprint parent and is private to its current equal
Teaching Team Members and enrolled Students. It owns enrollment, deadlines, releases, accommodations,
grades, and delivery settings.

Instructor and Sysadmin evidence uses the fixed desktop profile at exactly 1280 by 800 CSS pixels.
Student evidence uses the maintained laptop (1280 by 800), tablet (800 by 1280), iPhone Pro (393 by
852), and square (800 by 800) profiles. Review each Student layout at the viewport being demonstrated;
do not treat one profile as proof for every Student size. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md#visual-evidence-profiles)
for the evidence boundary.

## Start a teaching session

Use the fixed owner so the browser, PostgreSQL, storage, worker, gateway, and private renderer share
one production-shaped disposable stack.

```bash
./run_live_demo.sh
```

For a headless session, print the HTTPS origin instead of opening a browser:

```bash
./run_live_demo.sh --headless
```

Choose the seeded Instructor persona when available, then use the visible account and Course Instance
controls. The persona enters ordinary PLE authorization; it is not an alternate application path.
See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the identity and disposable-state boundary.

## Create a course instance

1. Open **Courses** and choose **Create Course Instance**.
2. Choose an existing published Blueprint Course, or choose **Create Blueprint Course** to create the
   minimal reusable parent first.
3. Enter the teaching title, inclusive start and end dates, and an exact IANA time zone such as
   `America/Chicago`.
4. Activate **Create Course Instance**, open the new card, and confirm the applied Blueprint revision.
5. Open **Students**, create an invitation with the Student's course roster email and course-scoped
   student ID, and copy the one-time link from **Share this invitation** into the trusted course
   channel.
6. Confirm **Invitation pending**, then **Active** after the Student claims it.

The Course Instance is real PostgreSQL-backed state. It receives no Students, invitations, Assignment Attempts,
responses, grades, retention state, or issued work from the source. Relative schedule defaults are
resolved against the instance term and time zone. Invalid term values preserve the form for
correction. The invitation remains single-use; a queued email is not proof of mailbox delivery. See
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Create and publish a blueprint

1. Open **Blueprint Courses** and choose **New Blueprint Course**, or open a draft you own or can edit.
2. Add ordered modules or weeks and ordered assignments. Select already published questions by
   visible `AAA-BBBB` Question IDs, reuse a saved assignment, or configure reusable pools.
3. Save the complete ordered definition as one Blueprint revision. Relative calendar-day and local
   wall-clock values are defaults, not live deadlines.
4. Review the complete, answer-free Blueprint revision and choose
   **Publish Blueprint**.

Publishing is explicit. It makes the revision discoverable to every vetted Instructor, but it does
not create enrollment, release an assignment, or create Student work. Editing a published Blueprint
creates a new revision and does not silently mutate existing Course Instances.

## Exercise Sysadmin operations

1. Start the disposable stack and select the seeded **Sysadmin** persona.
2. Use the ordinary course-operation surfaces when the seeded state exposes them.

The fresh email-code and passkey adapters are not mounted in this build. The visible seeded
Sysadmin selector provides the current disposable-demo entry while those ordinary authentication
paths are reconstructed on the single Authenticated Session foundation.

The seeded Sysadmin is Morgan. Direct entry resolves an ordinary server-owned account and role; it
does not grant a browser-controlled role claim. Sysadmin work can change disposable data, and
regeneration restores the seeded baseline. Keep this workflow at the desktop viewport. See
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md#sysadmin-perspective) and [USAGE.md](USAGE.md#sign-in-and-switch-personas).

## Build and release an assignment

1. In a Course Instance, open **Assignments**, choose **New assignment**, enter a title, and select
   **Create Assignment**.
2. In **Questions**, use the saved-assignment picker or question library to add published questions.
   Arrange fixed Questions, add Question Pools with Question IDs and selection counts, and choose **Save questions
   and order**. Pool samples create no Student work.
3. Open **Policies**. Enter Student instructions, availability and due/close times in the Course
   Instance time zone, Assignment Attempt limits, completion and continued-practice rules, late behavior, and
   disclosure settings. Choose **Save assignment policies**.
4. Read **Release requirements** on **Overview**. Resolve every Assignment Release Issue. Choose
   **Released - eligible for Student access** in the lifecycle control and save it. Until that save
   succeeds, the Assignment remains **Unreleased - students cannot access it**.
5. Open **Student view** to inspect the current answer-free Student landing. Student view retains the
   Instructor session and creates no Assignment Attempt or grade.
6. For graded validation, sign out, select the seeded **Student** persona, open the authorized Course
   Instance and released assignment, choose **Start assignment**, answer, and submit.
7. If **Response received** appears, use **Check grading status** until feedback and **View completed
   Assignment Attempt** appear. If attention is required, use the currently enabled action in **Grading operations**.
8. Return to the Instructor session and confirm the score and authorized evidence in **Gradebook**.

Assignments pin exact immutable questions for issued Student work. A Questions replacement applies to
future Assignment Attempts; existing Assignment Attempts keep their original question. Prefer saved-assignment reuse for a group of
questions. See [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) and
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md).

## Apply a blueprint update

1. Revise the Blueprint Course and publish the new revision explicitly.
2. In an affected Course Instance, open **Blueprint operations** and choose **Prepare update proposal**.
3. Review the source revision, assignment manifest, exact question replacements, and every resolved
   schedule. Correct any DST gap or ambiguity before preparing again.
4. Choose **Apply proposal**. A newly added Blueprint assignment arrives **Unreleased**; current
   equal Teaching Team Members must review and release it in the instance.
5. Choose **Check Assignment Import Receipt Evidence** after applying. An incomplete receipt requires
   operator recovery; an authorized **Assignment Import Repair** handles only its derived Assignment
   import state.

If instance work diverged, choose the explicit selected-copy or new-assignment action. PLE does not
perform an implicit merge, and instance deadlines, releases, accommodations, grades, and other
delivery settings remain instance-owned.

## Fork a Blueprint, copy a Course for a new term, or shift Course Dates

- **Fork Blueprint:** from a published Blueprint detail, choose **Fork Blueprint**. The copy is
  independently editable, retains immutable source-lineage evidence, and has no live tether.
- **Copy Course for New Term:** choose **Copy course for new term**, select a target term, and review the
  manifest. The destination starts without Students, invitations, attempts, responses, grades,
  retention state, or issued evidence.
- **Shift Course Dates:** choose **Shift course dates** only when no Student work has been
  issued. Preview every resolved date in the target IANA time zone and apply the witnessed proposal
  atomically. If work has been issued, use Copy Course for New Term instead.

## Review learning and export grades

After a Student submits, let the visible status determine the next operation:

1. In the Student session, choose **Check grading status** after **Response received**.
2. If the status says **Your response needs instructor attention**, sign in as the Instructor and
   open **Grading operations**. Review the answer-free operation row and choose its enabled action
   when eligible.
3. Return to the Student session and choose **Check grading status** until **Your completed Assignment Attempt is
   recorded.** appears. Then open **Gradebook** and choose **Inspect submitted work**.
4. Confirm the Student's score, latest Assignment Attempt, completed Assignment Attempt count, and authorized submission evidence
   after a fresh read.

Configure **Grade settings** before relying on totals. The supported modes are total points and
weighted categories with drop-lowest. Use **Export grades CSV** for the bounded Instructor download;
the durable export audit excludes Student PII. The server owns totals and authoritative time; the
browser does not recompute either. See [API_CONTRACTS.md](API_CONTRACTS.md) and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md).

## Match canonical browser journeys

The production browser suite covers role entry and authorization; Blueprint authoring and publication;
Course Instance creation and Apply Blueprint Update; assignment authoring, preview, question replacement,
pools, and grade settings; Student delivery, gateway recovery, and automated-grading recovery;
WebWork delivery; Question Library discovery and question curation; rollover and term scheduling; and QTI
profile import. These are live workflows, not mock or fixture walkthroughs. Use [E2E_TESTS.md](E2E_TESTS.md)
for scenario selectors and [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the visible route and policy
boundary. Screenshots are one-time visual evidence, not a substitute for the live browser journey.

## Diagnose or stop the stack

Run read-only checks before changing state:

```bash
source source_me.sh && .venv/bin/python local_stack.py validate
source source_me.sh && .venv/bin/python local_stack.py status
source source_me.sh && .venv/bin/python local_stack.py logs --tail 100
```

When a service needs a bounded restart, name that service explicitly. Preserve the private owner
receipt and follow [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for readiness, renderer, database, and
invitation failures. Stop the session through its authenticated owner:

```bash
./run_live_demo.sh stop
```

Recover the session with the fixed owner, scoped project boundary, and commands in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Run the complete acceptance lane

Use the connected acceptance command when validating the live system. It runs the canonical
production browser lane followed by browser-free renderer and service oracles:

```bash
source source_me.sh && .venv/bin/python local_stack.py acceptance
```

For the complete final-material Validation suite, use:

```bash
source source_me.sh && ./all_test.sh
```

See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the evidence boundary and
[DEVELOPMENT.md](DEVELOPMENT.md) for focused gates.

## Known gaps

- Workflows outside the reviewed four-question Chapter 1 Pilot Question Set need their own source and live
  evidence; see [USAGE.md](USAGE.md#known-gaps).
