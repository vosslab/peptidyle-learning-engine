# Instructor guide

## Current local Live Demo

The current local Live Demo lets a reader select the seeded Elena Instructor
persona on the visible account page. The server resolves the configured Account
and creates the ordinary Authenticated Session. The selector supplies neither
course authority nor a browser role claim.

After entry, the Instructor can use the Question Library and private authoring,
create Blueprint Courses and Course Instances, import a roster, create and
release Assignments, inspect answer-free Gradebook evidence, and download the
protected Course Invitation export. Each action requires the stored Instructor
role and exact Course relationship where applicable. Start the local stack
through [USAGE.md](USAGE.md); [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and
[API_CONTRACTS.md](API_CONTRACTS.md) define the current route boundary.

## Instructor teaching boundary

PLE's retained product design separates reusable **Blueprint Course** content
from a term-specific **Course Instance**. A Blueprint Course contains no
Students, deadlines, releases, accommodations, grades, or delivery settings. A
Course Instance owns those teaching records and derives access from its exact
Teaching Team Members and Student memberships.

The current Instructor workflow is:

1. Create or select a Blueprint Course, then create a Course Instance from one
   exact Blueprint revision.
2. Author and release Course Instance assignments using published Questions
   and Course-owned delivery rules.
3. Invite or otherwise establish exact Student Course Memberships.
4. Inspect answer-free Student delivery and authorized Gradebook evidence.
5. Use bounded, receipt-backed recovery for an eligible grading operation.

These Store-backed workflows preserve server-held Answer Keys, exact
relationship-derived authorization, immutable issued evidence, and separate
Student ownership. A public route reference locates an intended resource; it
never grants authority. The browser downloads invitation-export data but does
not send mail.

## Successor Assignment Revision

When a structural Assignment edit conflicts with issued Student activity, the
retained model/generated/browser recovery contract is
`SuccessorAssignmentRevisionRequired`. It carries the immutable base revision
that existing Student work pins. Visible guidance calls this outcome a
**Successor Assignment Revision**.

The server-owned command that creates the successor and its Server Route are
future work. Until both exist, this guide does not instruct an Instructor to
perform a structural successor operation.

## Accessibility contract

The Instructor interface uses visible controls and the keyboard model in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). The
current connected browser owner verifies its supported workflow; role-owned
screenshots remain one-time rendered evidence under
[SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md).
