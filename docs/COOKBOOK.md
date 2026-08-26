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

1. Open **New assignment** and enter a title.
2. Prefer **Reuse questions from an existing assignment**; use a visible `AAA-BBBB` Question ID
   only for an occasional direct lookup.
3. Confirm the selected questions and configure the Mastery run and disclosure policies.
4. Create the assignment, then open **Teaching operations**.
5. Enter learner instructions, publish it, and save availability, due/close times, run limits,
   late behavior, and deadline behavior in the course time zone.
6. Open the learner-facing assignment link and complete a run before reviewing the gradebook.

Assignments are Draft until the teaching-operations save publishes them. Existing assignments pin
exact immutable questions; replace a question only after reviewing the old and new IDs. See
[PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md) for identity rules and [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md)
for the complete recipe.

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
