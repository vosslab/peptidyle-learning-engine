# Instructor guide

## Current local Live Demo

The current local Live Demo lets a reader select the seeded Elena Instructor
persona on the visible account page. The server resolves the configured Account
and creates the ordinary Authenticated Session. The selector supplies neither
course authority nor a browser role claim.

The current Browser Surface ends at the account/session entry. Course
creation, Blueprint Course work, Course Instance management, roster and
invitation work, assignment workspace controls, Student view, grading
operations, and Gradebook screens are retained product workflows rather than
current Live Demo routes. Start the local stack through [USAGE.md](USAGE.md) to
inspect the available entry boundary.

## Future Instructor teaching contract

PLE's retained product design separates reusable **Blueprint Course** content
from a term-specific **Course Instance**. A Blueprint Course contains no
Students, deadlines, releases, accommodations, grades, or delivery settings. A
Course Instance owns those teaching records and derives access from its exact
Teaching Team Members and Student memberships.

The intended Instructor workflow is:

1. Create or select a Blueprint Course, then create a Course Instance from one
   exact Blueprint revision.
2. Author and release Course Instance assignments using published Questions
   and Course-owned delivery rules.
3. Invite or otherwise establish exact Student Course Memberships.
4. Inspect answer-free Student delivery and authorized Gradebook evidence.
5. Use bounded, receipt-backed recovery for an eligible grading operation.

These are future Store-backed workflows. They preserve server-held Answer Keys,
exact relationship-derived authorization, immutable issued evidence, and
separate Student ownership. A public route reference locates an intended
resource; it never grants authority.

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

The intended Instructor interface uses visible controls and the keyboard model
in [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). A
restored browser acceptance owner must verify the available workflow before this
guide presents it as an executable demonstration.
