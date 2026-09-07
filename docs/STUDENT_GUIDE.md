# Student guide

## Current local Live Demo

The current Live Demo lets a reader enter a seeded Student Account through the
visible account selector. That selector creates the ordinary server-owned
Authenticated Session for the configured Account. It does not supply a course
membership, Student record, or authorization claim.

After entry, a seeded Student can claim their own Course Invitation, open an
authorized Course Instance and released Assignment, start or resume an
Assignment Attempt, use the issued Question Response Control, and submit a
Student Response. The server evaluates Course Membership, Assignment Access,
timing, and Student ownership at every protected boundary. Start the local
stack using [USAGE.md](USAGE.md); [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and
[API_CONTRACTS.md](API_CONTRACTS.md) define the current route boundary.

## Student delivery boundary

The retained PLE product contract gives a Student access only through an exact
active Course Membership, Student ownership, an allowed Assignment Access
decision, and the exact Course and Assignment. The Student then receives only
the Questions issued for that Assignment Attempt.

The current workflow is:

1. Open an authorized Course Instance and Assignment.
2. Start or resume an Assignment Attempt.
3. Submit a response through visible Question Response Controls.
4. Read the answer-free submission acknowledgement and permitted Student
   Feedback or grading state.
5. Continue or begin another Assignment Attempt only when Assignment rules
   allow it.

The server keeps Answer Keys, Question Graders, private Question Source data,
and Question Attempt Reproduction Details outside the Student browser boundary.
Student Feedback is a separate policy-evaluated projection; a submission-status
response reports only the nonce-bound grading state. The server also determines
timing, late-work treatment, and authorization; the browser does not infer them
from its own clock or from an identifier.

## Accessibility contract

The Student interface uses visible controls and the keyboard model in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). The
current connected browser owner validates this journey; role-owned screenshots
remain one-time rendered evidence under [SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md).
