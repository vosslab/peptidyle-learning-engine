# Assessment payload design

## Status and authority

This document defines the target Student Assessment Attempt boundary under
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Existing `/assignment-attempts/` route
segments and `assignmentAttempt` fields are implementation names awaiting a
separate code migration; they do not make Assignment the generic product term.

Submission belongs to the whole Assessment Attempt. The browser saves working
responses by Question position; the whole-Attempt action finalizes them as
Student Work.

## Browser exchange

An opaque public Attempt reference selects the authorized Student's Attempt.
The browser needs focused, answer-free operations:

| Purpose | Product result |
| --- | --- |
| Display context and timer | Course and Assessment labels, Attempt number, authoritative expiry, and remaining time |
| Navigate Questions | Fixed positions plus saved or unanswered status |
| Display one Question | Answer-free presentation and the Student's current saved response |
| Save a response | Attempt, position, and a saved acknowledgement |
| Submit Assessment | Whole-Attempt submitted acknowledgement |
| Read a disclosed result | Only result and feedback currently allowed by policy |

Responses containing Student Work are `no-store`. The browser does not receive
Account IDs, private Student-record IDs, Answer Keys, private grading inputs,
backend credentials, worker state, raw provider results, or Instructor-only
data.

## Attempt flow

```text
server starts one Assessment Attempt and fixes its Question selections
                         |
                         v
Student opens one Question at a time and saves complete responses
                         |
              +----------+----------+
              |                     |
              v                     v
       Student reconnects     deadline is reached
       to the same Attempt           |
              |                     v
              |             server submits Attempt
              +----------+----------+
                         |
                         v
the whole Attempt is submitted once; policy controls disclosure
```

Reloading or reconnecting resumes the same open Attempt with its fixed deadline
and successfully saved responses. It does not extend time or create another
Attempt.

## Presentation payload

The selected Question presentation contains only what the Student interface
needs:

- the exact published Question or Pool Revision evidence;
- opaque Question Backend state needed to preserve the interaction;
- ordered prompt content and authorized asset references;
- public input constraints; and
- the Student's current saved response, when one exists.

Private tolerances, Answer Keys, rubrics, private source, credentials, and
grading configuration remain on the server. Binary assets use authorized asset
routes rather than browser-supplied object-store paths.

PLE-native Questions use one of the strictly supported native controls. A
backend-owned Question returns an opaque document plus opaque state. PLE must
not infer educational Question Type from HTML controls or parse a backend's
form fields to reproduce its grading semantics.

## Saved response payload

A response save identifies the Attempt and Question position and sends only the
response shape required by that Question Backend. For example, a current native
implementation may use:

```http
PUT /api/assignment-attempts/R-7/responses/2
Content-Type: application/json
```

```json
{
  "response": {
    "kind": "multipleChoice",
    "selected": ["4ef3"]
  }
}
```

The path and field names above document current implementation evidence, not
preferred product terminology. The server resolves the exact Student, Course,
Assessment, Question, Revision, backend, and backend state. It never accepts
those authorities merely because the browser names them.

A complete valid response replaces the working saved response while the Attempt
is open. An incomplete response is not saved as complete and is not graded.
After whole-Assessment submission or automatic deadline submission, the
response cannot be changed.

The acknowledgement is deliberately small. It contains no separate
response-finalization receipt, Student-visible grading result, next-Question
reservation, or grading action.

## Whole-Assessment submission

Selecting the product action to submit the Assessment submits the entire open
Attempt. Every saved complete response is finalized as Student Work in that one
transition. Positions without a complete saved response remain visibly
unanswered, receive zero credit, and count as incorrect without being sent to a
backend.

At the deadline, the server performs the same whole-Attempt transition using
the responses saved before expiry. Repeating an already-completed transition
returns the existing submitted state without duplicating results.

Human Guidance does not require a public Assessment Submission receipt object
or a separate response-finalization product model. Implementations may retain
normalized internal evidence, but documentation and interfaces must not turn
that evidence into another Student action or lifecycle family.

## Evaluation and outcome

The Question Backend may evaluate a complete saved response before the whole
Attempt is submitted. PLE stores the immutable credit fraction returned by the
backend. It does not expose a Student-visible grading outcome until submission
and the applicable disclosure point.

Scores use the current point value for each Assessment Question and the stored
credit fraction. A point-value edit can change the calculated score but does
not regrade the response or change the fraction.

No browser or Instructor grading, Retry, regrading, result-replacement, or
grading-job action is part of this contract. If a real backend needs deferred
technical completion, it remains an internal adapter concern and cannot create
a second product lifecycle.

## Authorization binding

The server closes every operation over one relationship:

```text
authenticated Account
  -> active Student Course relationship
  -> Student record in that Course
  -> Assessment Attempt
  -> Question position and exact backend state
```

Public references are selectors, not access grants. Each protected read or
write verifies the relationship again at the trusted boundary.

## Backend-owned responses

A WeBWorK or other backend-owned response may be bounded opaque form data. PLE
stores and forwards it without adding a control parser. The 64 KiB shared
response bound applies to the canonical opaque browser payload; backend source,
document, envelope, and asset limits are separate contracts.

Backend ownership does not change Attempt behavior: the Student saves during an
open Attempt, and whole-Assessment submission finalizes those saved responses
together.

## Failure behavior

| Failure | Required behavior |
| --- | --- |
| Save loses connectivity | Keep the response visible and allow another save while the Attempt remains open. |
| Browser reloads | Restore the successfully saved response and original deadline. |
| Deadline passes offline | Submit the Attempt using the complete responses saved before expiry. |
| Submission response is lost | Repeating the whole-Attempt request returns the existing submitted state. |
| Backend fails | Preserve Student work, disclose no invented result, and show a bounded error. |

This contract has no separate browser recovery state machine. Detailed backend
ownership is in [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md);
the lifecycle is in [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md).
