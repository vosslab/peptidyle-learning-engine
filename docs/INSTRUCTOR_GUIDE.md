# Instructor guide

## Current local Live Demo

The current local Live Demo lets a reader select the seeded Elena Rivera Instructor
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

## Assignment changes and Unrelease

An Assignment is one current teaching aggregate. Each save supplies its current
Assignment Edit Number and receives the authoritative Assignment and its next
Edit Number. Its fixed Questions and pool items pin exact Question Revisions,
so later Question publication never changes an Assignment silently.

Release validates the current Assignment. A Released Assignment may be edited
when the resulting current configuration remains release-valid; an accepted
edit governs later Attempts. Existing Attempts continue to use their retained
Assignment and Issued Question evidence, including their exact Question
Revision and issued seed.

**Unrelease** is the deliberate destructive recovery operation for a Released
Assignment. The Instructor confirms the exact current title and Edit Number.
The system presents only aggregate impact counts, returns the Assignment to
Unreleased, and atomically removes the Assignment's Student Work. It retains
the current Assignment, Course relationships, and shared published Questions.

## Accessibility contract

The Instructor interface uses visible controls and the keyboard model in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). The
current connected browser owner verifies its supported workflow; role-owned
screenshots remain one-time rendered evidence under
[SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md).
