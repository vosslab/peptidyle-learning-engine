# Student guide

## Current local Live Demo

The current Live Demo lets a reader enter a seeded Student Account through the
visible account selector. That selector creates the ordinary server-owned
Authenticated Session for the configured Account. It does not supply a course
membership, Student record, or authorization claim.

The current browser surface ends at this account/session entry. Course lists,
assignment pages, Question Response Controls, submissions, Student Feedback,
grades, and Assignment Attempt history are not mounted current routes. Start
the local stack using [USAGE.md](USAGE.md) and use the visible account page to
inspect the mounted session entry.

## Future Student delivery contract

The retained PLE product contract gives a Student access only through an exact
active Course Membership, Student ownership, an allowed Assignment Access
decision, and the exact Course and Assignment. The Student then receives only
the Questions issued for that Assignment Attempt.

The intended future workflow is:

1. Open an authorized Course Instance and Assignment.
2. Start or resume an Assignment Attempt.
3. Submit a response through visible Question Response Controls.
4. Read the answer-free submission acknowledgement and permitted Student
   Feedback or grading state.
5. Continue or begin another Assignment Attempt only when Assignment rules
   allow it.

This is an illustrative contract, not a current Live Demo walkthrough. The
server keeps Answer Keys, Question Graders, private Question Source data, and
Question Attempt Reproduction Details outside the Student browser boundary. It
also determines timing, late-work treatment, and authorization; the browser
does not infer them from its own clock or from an identifier.

## Accessibility contract

The future Student interface uses visible controls and the keyboard model in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). A
restored browser acceptance owner must validate the actual mounted journey
before screenshots or this guide describe the delivery interface as current.
