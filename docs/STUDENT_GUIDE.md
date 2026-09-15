# Student guide

## Current local Live Demo

The local Live Demo lets a reader enter through a seeded Student Account. The
selector creates the ordinary authenticated session; it does not supply Course
membership, a Student record, or authorization by itself.

Some current routes and labels still use `assignment`. The current product term
is Assessment, and Student work is collectively Coursework.

## Student workflow

1. Open an authorized Course Instance and choose an Assessment under
   Coursework. A particular item is labeled with its Assessment Type, such as
   Quiz or Regular Assignment.
2. Start or resume an Assessment Attempt.
3. Work one Question at a time and navigate among all Questions in the
   Assessment.
4. Save complete responses. A saved response remains editable while the
   Attempt is open.
5. Submit the whole Assessment Attempt, or let the server submit its saved
   complete responses automatically at the deadline.
6. Read results and feedback only when the Assessment policy allows them.
7. Start another Attempt only when Assessment policy permits it.

Incomplete responses are not saved as complete and are not graded. The server
owns timing and Course access, and the Question Backend owns rendering,
response interpretation, grading, feedback, and opaque backend state. The
browser never receives private Question source, Answer Keys, credentials, or
undisclosed results.

Removing or deactivating the Student's Course access does not delete the global
Student Account or Student Work. Course retention is a separate notified
process.

Start the local stack with [USAGE.md](USAGE.md). See
[ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) for the Attempt contract and
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) for product intent.

## Accessibility

The Student interface follows
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md), keeps
Question navigation and saved status visible, and preserves stable page
geometry. Screenshots require fresh capture and behavior evidence before they
claim current acceptance.
