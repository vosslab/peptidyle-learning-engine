# Cookbook

Practical recipes for operating the disposable PLE developer stack and walking the supported
instructor teaching loop. Use [INSTALL.md](INSTALL.md) for installation, [USAGE.md](USAGE.md) for the
complete command reference, and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) for the full browser
walkthrough.

## Start a teaching session

Use the fixed owner so the browser, PostgreSQL, storage, worker, gateway, and private renderer
share one production-shaped disposable stack.

```bash
./run_live_demo.sh
```

For a headless session, print the HTTPS origin instead of opening a browser:

```bash
./run_live_demo.sh --headless
```

Choose the seeded Instructor persona when available, then use the visible account and course
controls. The persona enters ordinary PLE authorization; it is not an alternate application path.
See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the identity and disposable-state boundary.

## Create a small course

1. Open **Courses**, create a title, inclusive start and end dates, and an exact IANA time zone
   such as `America/Chicago`.
2. Open the new course and choose **Students**.
3. Create an invitation with the learner's institutional email and course-scoped student ID.
4. Copy the one-time invitation link from **Share this invitation** into the trusted course
   channel.
5. Confirm **Invitation pending**, then **Active** after the learner claims it.

The course and roster are real PostgreSQL-backed state. Invalid term values preserve the form for
correction. The invitation remains single-use; a queued email is not proof of mailbox delivery.
See [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Build and publish an assignment

1. In an Instructor course, open **Assignments**, choose **New assignment**, enter a title, and
   select **Create assignment draft**. This creates the persisted Draft that anchors the rest of the
   workspace.
2. In the Draft's **Questions** page, use the saved-assignment picker or the question library to
   add published questions. Arrange the fixed questions in order, add pools when a position should
   draw from candidates, and set each pool's candidate IDs, draw count, and order. Select a fixed
   question's replacement control to review a new Question ID before replacing it. Choose **Save
   questions and order** when the definition is ready. Pool samples are server previews and create no
   learner work.
3. Open **Policies**. Enter learner instructions, audience, availability and due/close times in the
   course time zone, run limits, completion and continued-practice rules, late behavior, and
   disclosure settings. Choose **Save assignment policies** after reviewing the delivery summary.
   The page reports the current lifecycle and any question or settings issue that blocks publication.
4. Return to **Overview** and read **Publication readiness**. Resolve every blocking action through
   Questions or Policies. When the definition is ready, use Policies' lifecycle control to choose
   **Published - eligible for learner access**, then save the policies. This is the publication step;
   the assignment remains **Draft - students cannot access it** until that save succeeds.
5. Open **Student view** from the workspace navigation to inspect the current live, answer-free
   learner landing. Student view retains the Instructor session and creates no learner run or grade.
   Use it to check title, instructions, timing, and the learner entry affordance before asking a
   learner to work.
6. Validate delivery through the ordinary live workflow: sign out, select the seeded **Student**
   persona, open the authorized course and published assignment, choose **Start assignment**, answer
   the visible questions, and submit. If **Response received** appears, use **Check grading status**
   until feedback or an instructor-attention message appears. Sign back in as the Instructor, open
   **Gradebook**, and expand the assignment's **View run history**. Confirm the Student's score,
   latest run, completed-run count, and authorized submission evidence after a fresh read.

Assignments pin exact immutable questions for issued learner work. A later Questions replacement is
for future runs; existing runs keep their original question. Use a visible `AAA-BBBB` Question ID
for an occasional direct lookup, and prefer saved-assignment reuse for a group of questions. See
[PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md) for identity rules and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md)
for the complete recipe. This walkthrough uses the disposable live stack and ordinary UI records;
it does not use fixtures or a parallel mock assignment.

## Review learning and export grades

After a learner submits and optionally repeats the assignment, open **Gradebook** and expand
**View run history** for the assignment row. Configure **Grade settings** before relying on totals:
the current supported modes are total points and weighted categories with drop-lowest. Use
**Export grades CSV** for the bounded instructor download; the durable export audit excludes learner
PII. The server owns totals and authoritative time; the browser does not recompute either. See
[API_CONTRACTS.md](API_CONTRACTS.md) and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md).

## Adopt reusable curriculum

From an Instructor course, open **Curriculum adoption**, choose a Blueprint, Alpha, rollover, term
shift, or import inspection, and select **Prepare proposal**. Review the answer-free proposal and
its source revision before **Apply proposal**. Use **Check receipt evidence** after applying it.
Rollover leaves the original course's activity behind; term shift is for unissued schedules and
refuses an assignment with issued runs. See the adoption section of [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md).

## Diagnose or stop the stack

Run read-only checks before changing state:

```bash
source source_me.sh && python3 local_stack.py validate
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs --tail 100
```

When a service needs a bounded restart, name that service explicitly. Preserve the private owner
receipt and follow [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for readiness, renderer, database, and
invitation failures. Stop the session through its authenticated owner:

```bash
./run_live_demo.sh stop
```

Recover the session with the fixed owner, scoped project boundary, and commands in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md); this keeps cleanup limited to the disposable
PLE resources.

## Run the complete acceptance lane

Use the connected acceptance command when validating the live system. It runs the canonical production
browser lane followed by the browser-free renderer and service oracles, with no required skips:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

For the complete final-material Validation suite, use:

```bash
source source_me.sh && ./all_test.sh
```

See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the evidence boundary and [DEVELOPMENT.md](DEVELOPMENT.md)
for focused gates.

## Known gaps

- Workflows outside the reviewed four-question Chapter 1 corpus need their own source and live
  evidence; see [USAGE.md](USAGE.md#known-gaps).
