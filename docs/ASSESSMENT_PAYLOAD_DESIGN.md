# Assessment payload design

## Status and authority

This document describes the current Student Assignment Attempt payload boundary. The controlling
product behavior is in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). The durable lifecycle and security
owners are [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md),
[CONTRACTS.md](CONTRACTS.md), [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md), and
[SECURITY_MODEL.md](SECURITY_MODEL.md).

Submission belongs to the Assignment Attempt. The browser saves working responses by Question
position, but it does not submit or grade individual Questions. Recovery is the server-owned
auto-submission of saved responses when the Assignment Attempt expires.

## Current browser contract

One public `R-n` Assignment Attempt reference selects the current Student-owned Attempt. The
browser uses focused, answer-free routes:

| Purpose                          | Method and route                                                              | Public result                                                                          |
| -------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Display context and server timer | `GET /api/assignment-attempts/{attempt}/context`                              | Course and Assignment labels, Attempt number, display zone, expiry, and remaining time |
| Question navigation              | `GET /api/assignment-attempts/{attempt}/student-progress`                     | Fixed positions and `unanswered`, `saved`, `submitted`, or `closed` state              |
| One presentation                 | `GET /api/assignment-attempts/{attempt}/student-question?position={position}` | Answer-free presentation and the Student's saved working response                      |
| Backend-owned document           | `GET /api/assignment-attempts/{attempt}/questions/{position}/document`        | One authorized immutable backend document                                              |
| Save working response            | `PUT /api/assignment-attempts/{attempt}/responses/{position}`                 | Attempt, position, and `saved` acknowledgement                                         |
| Submit whole Attempt             | `POST /api/assignment-attempts/{attempt}/submission`                          | Attempt and `submitted` acknowledgement                                                |

All responses are `no-store`. The browser validates canonical `R-n` references and positive
positions before sending a same-origin request. Strict decoders reject unknown response fields,
foreign references, position mismatches, and unexpected HTTP status codes.

The browser does not receive Account IDs, Student Record IDs, private database IDs, Answer Keys,
grading inputs, worker lease facts, provider credentials, raw provider results, or Instructor-only
data.

## Assignment Attempt flow

```text
server starts one timed Assignment Attempt
                    |
                    v
server issues a fixed Question set and immutable presentations
                    |
                    v
Student opens positions and saves working responses
                    |
          +---------+---------+
          |                   |
          v                   v
Student reconnects       Attempt expires
to same active Attempt        |
          |                   v
          |             server auto-submits
          |             all saved responses
          |                   |
          +---------+---------+
                    |
                    v
whole Attempt has one immutable Assignment Submission
                    |
                    v
automated grading creates terminal immutable results
```

Reconnect, reload, and another authenticated browser session resume the same active Attempt. They
do not pause, reset, or extend its wall-clock deadline. Autosave preserves working responses; it is
not submission and it is not recovery.

## Presentation payload

The selected Question presentation contains only facts needed to render the Student experience:

- immutable Question Revision reference and Question Seed;
- presentation nonce and public Presentation Token;
- ordered prompt blocks and immutable asset references;
- public input constraints and a response-format discriminant; and
- presentation-scoped references for addressable response items.

The response-format `kind` tells the browser whether to render a choice, text, numeric, matching,
ordering, hotspot, backend-owned, or other supported control. Private tolerances, Answer Keys,
rubrics, weights, source objects, renderer credentials, and grading configuration remain on the
server.

Binary assets are fetched separately through authorized immutable asset routes. They are not
embedded in the JSON presentation. A Question Presentation Checksum binds the complete public
presentation retained by the server.

## Saved response payload

The browser saves the exact response selected for one issued position:

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

The response `kind` is a closed browser wire discriminant. The server reproduces the exact issued
presentation selected by the authenticated Attempt and position, validates the public response
format, translates presentation-scoped item references to durable private references, and then
saves the canonical response.

A later valid save for the same active position replaces the working saved response. Finalization
serializes against saves. After the Attempt is submitted or expires, the working response cannot be
changed.

The save acknowledgement is deliberately small:

```json
{
  "assignmentAttempt": "R-7",
  "position": 2,
  "responseState": "saved"
}
```

There is no public per-Question submission receipt, grading result, successor Question, prefetch
reservation, or grading action on this route.

## Attempt submission and recovery

The Student can select `Finish Assignment` to request whole-Attempt finalization. Explicit
finalization succeeds only when every fixed Question position has a saved response. If a response
is missing, the Attempt remains active; the existing progress read identifies the positions that
still need work.

Expiry has different missing-response behavior. PostgreSQL owns the deadline and atomically:

- submits every response saved before the deadline;
- closes each Question without a saved response as unanswered;
- creates one immutable Assignment Submission with finalization kind `deadline`; and
- obtains immutable Question Backend credit outcomes for accepted saved responses.

This expiry transition is recovery. Student operations enforce the server-owned expiry instant
before allowing a change, while narrow background execution submits expired Attempts that need no
further Student request. Reads remain read-only. Repeating the transition returns the
already-submitted state without creating another Assignment Submission, Question Submission, or result.

## Internal submission evidence

The database uses one internal immutable Question Submission for each saved response accepted when
the whole Attempt finalizes. This is evidence beneath the Assignment Submission; it is not a
Student-visible per-Question submission action.

For each supported accepted response, the same transaction creates:

- one Question Submission rooted at the exact Question Attempt;
- the immutable normalized credit result and receipt when the Question Backend completes immediately.

A Question closed unanswered at expiry has no Question Submission or result and contributes
zero points. The Attempt-level status reader still represents that position as terminal.

## Outcome completion

The Question Backend evaluates an immutable accepted response once and PLE stores its immutable
normalized credit fraction and receipt. A backend that requires polling uses an internal
lease-fenced completion path. This mechanism is not a Student or Instructor lifecycle. No user can
grade, retry, reopen, overwrite, or replace accepted work or a Grading Result in PLE.

## Authorization binding

An issued Question is resolved through one closed server-side relationship:

```text
(authenticated Account, Course Membership, Student Record,
 Assignment Attempt, issued position, Question Attempt,
 immutable Question Revision, Question Seed, presentation binding)
```

The public Attempt reference and position are selectors only. They do not grant authority. The
server resolves Student ownership and performs each protected read or write through forced RLS in
the same transaction. Another Account, Course, Student Record, Attempt, position, presentation,
provider identifier, or cache entry cannot widen that relationship.

No Instructor grading-detail route exists. Course-authorized Gradebook reads project only
answer-free completed results; they do not grant grading, regrading, retry, or background-work
control.

## Presentation consistency

Presentation Response Item References are four lowercase hexadecimal characters scoped to one
Question presentation. They identify choices, blank slots, matching sides, ordering items, and
hotspot regions without exposing durable authored references.

Rust derives these references with CRC-16/CCITT-FALSE from domain-separated presentation facts.
Issuance rejects a collision across the complete presentation and retries with a new nonce up to
the bounded issuance limit. CRC16 is a compact correspondence check, not authentication or secrecy.

The server also stores a SHA-256 Question Presentation Checksum over the complete normalized public
presentation. The public Presentation Token carries its bounded prefix. The checksum detects stale
or mixed presentation state; it does not replace TLS, session authentication, RLS, lifecycle checks,
or server-only grading.

## Backend ownership

Native PLE responses use typed response controls and are translated through the retained issued
presentation. A WeBWorK Question keeps its interaction document and submitted form payload opaque
to PLE. PLE stores and forwards the exact backend-owned response, while the WeBWorK adapter alone
understands renderer fields and grading semantics.

Backend ownership does not change Attempt behavior. The Student saves responses during the active
Attempt, and explicit finalization or expiry auto-submission accepts the whole Attempt. Provider
failure can delay internal completion but cannot create a browser or Instructor grading action.

[QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) owns the detailed backend contracts.

## Caching boundary

PLE may cache an answer-free native presentation by immutable Question Revision, Question Seed, and
presentation binding. Immutable assets may use content-addressed browser caching. Cache entries
never contain Answer Keys, private grading inputs, credentials, session material, or raw provider
results.

The browser does not prefetch, reserve, promote, or receive a successor Question. The fixed issued
Question set already belongs to the Assignment Attempt, and the Student selects one position at a
time. [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md) records the current no-prefetch boundary.

## Failure behavior

| Failure                                     | Required behavior                                                                                                   |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Save loses connectivity                     | Keep the edit in the control, show that saving did not finish, and allow the same active response to be saved again |
| Browser reloads before expiry               | Restore the server-saved response and continue the same Attempt with the original deadline                          |
| Browser remains disconnected through expiry | Server auto-submits the saved responses and closes unanswered Questions                                             |
| Explicit finalization has missing responses | Keep the Attempt active; use the existing progress read to identify missing positions                               |
| Finalization response is lost               | A repeated whole-Attempt request converges on the existing Assignment Submission                                    |
| Internal completion pauses before a result  | Preserve accepted evidence; the lease-fenced internal operation may resume                                          |
| Backend completion fails                    | Preserve accepted evidence and expose bounded diagnostics only to the Sysadmin who can repair the dependency        |
| Grading Result exists                       | Return authorized feedback projections; never contact the backend again                                             |

[FAILURE_RECOVERY.md](FAILURE_RECOVERY.md) owns the generic request and infrastructure failure
taxonomy. Its Assignment Attempt section uses the same narrower definition: recovery is expiry
auto-submission.

## Evidence

Permanent tests protect:

- strict browser decoders and same-origin route construction;
- saved-response validation and exact position binding;
- reconnect restoration without an unintended save;
- failed autosave preservation and explicit save acknowledgement;
- explicit whole-Attempt finalization and missing-response refusal;
- bounded expiry sweeps and atomic deadline auto-submission;
- one immutable submission/result lineage under concurrent claims and stale leases;
- authorized feedback projections; and
- absence of public grading states, polling, mutations, and retired per-Question submission routes.

Connected PostgreSQL and live-browser evidence remain distinct from fast deterministic offline
tests under [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Retired designs

The following concepts are not part of the current contract:

- a Student per-Question submit action or public per-Question submission-status route;
- browser `nextIssued`, `nextPending`, prefetch reservation, or promotion state;
- a separate browser recovery state machine;
- browser-visible connectivity recovery after grading;
- an Instructor grading, Retry, or regrading operation;
- a grading-operation generation used to create a newer result; and
- mutable or replacement Grading Results.

Historical plans or reports may retain those names as superseded evidence. They do not authorize
implementation.
