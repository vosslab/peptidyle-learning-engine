# Assessment payload design

## Status and authority

This document explains PLE's student-facing assessment payload boundary, compares it with the local
LibreTexts ADAPT snapshot, and summarizes the accepted implementation direction. It distinguishes
the current implementation from the target contract so contributors do not mistake planned wire
changes for current behavior.

The canonical browser path is the single production-shaped live-demo application. Payload examples
and one-time wire fixtures in this document describe bounded codec evidence only; they are not a
second browser application or a source of seeded student records.

The exact codec and cutover requirements are owned by this contract and its
focused tests. The single-installation ownership and authorization target is
normative in [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).
The [release_completion_plan.md](active_plans/active/release_completion_plan.md) owns dependency
order. This durable guide explains why those decisions exist and how the boundaries fit together.

## Design summary

PLE should send a rich, answer-free render payload once and accept a much smaller student response.
The browser needs enough public information to draw the prompt, choices, input controls, accessible
labels, and assets. It does not need database provenance, grading rules, answer keys, backend
selection, renderer credentials, or the complete attempt record.

The target contract is:

```text
Browser                                      PLE
   |                                          |
   | GET /api/assignment-attempts/{assignmentAttempt}/screen |
   |----------------------------------------->|
   | minimal shell + attempt + public render  |
   |<-----------------------------------------|
   |                                          |
   | POST .../attempts/{attempt}/submissions  |
   | Idempotency-Key: ...                     |
   | {presentationToken, answer}             |
   |----------------------------------------->|
   | compact policy-permitted receipt         |
   |<-----------------------------------------|
```

The authenticated `QuestionAttemptId` in the route is the primary student-response binding. The
server resolves it to one exact `CourseId`, `StudentRecordId`, `AssignmentAttemptId`, and immutable
`QuestionRevisionReference` plus seed before reading or mutating anything. The submitted Question
Presentation Token is compared with the complete server-held Question Presentation Checksum to check that
the browser answered the same render state PLE issued. Compact
CRC16 rendered-item IDs identify choices, blanks, matching sides, ordered items, Hotspot Surfaces,
and Hotspot Regions within that presentation. Neither the Question Presentation Checksum nor CRC16 authenticates the student or proves
correctness.

### SD1 authorization binding

An issued attempt is an educational record with one closed server-side identity tuple:

```text
(CourseId, StudentRecordId, AssignmentAttemptId, QuestionAttemptId,
 QuestionRevisionReference { question_id, revision_number }, seed)
```

`CourseId` and `StudentRecordId` are checked against the authenticated Account's exact Course Membership
and Student ownership. `AssignmentAttemptId` and `QuestionAttemptId` must resolve through that same relationship;
the immutable `QuestionRevisionReference` and seed must match the Issued Question. A route parameter,
browser field, cache key, provider identifier, or queue payload cannot widen or replace this tuple.
The protected read or write performs the relationship check and data operation in one forced-RLS
transaction.

The attempt also carries a server-owned typed capability such as
`IssuedAttemptCapability::PleQuestionJsonPresentation` or `WebworkPresentation`. Its matching private
grading envelope, presentation snapshot, and (for WeBWorK) replay state are required or explicitly
`NotApplicable`; a missing or mismatched required capability is unavailable. Worker execution uses
the same exact target from a locked typed lease. Provider metadata remains external protocol data
only: renderer identity, provider profile, upstream field/value names, and source-artifact details
may support one server-to-provider exchange but never act as an Account, course, Student, attempt, or
authorization selector.

## Current PLE boundary

### Current render payload

The implemented [QuestionPresentation](../crates/question_model/src/envelope.rs) is deliberately
answer-free. Its nested [QuestionVariation](../crates/question_model/src/envelope.rs) retains the
exact Question Revision, seed, and declared generator recipe in server/cache evidence; its browser
serialization carries only the version-and-seed binding. The presentation contains:

- student-facing `title`;
- ordered prompt blocks for text, math, images, code, and tables;
- public asset IDs, SHA-256 checksums, and accessible descriptions; and
- a tagged `QuestionResponseFormat` that selects the browser widget.

The response `kind` is necessary in this render payload. Without it, the browser cannot know whether
to draw radio buttons, checkboxes, text boxes, ordering controls, matching controls, or a hotspot
surface. Content-block `kind` values are similarly useful render discriminants.

The current [QuestionResponseFormat](../crates/question_model/src/response.rs) is broader than the
eventual student schema. For example, it exposes numeric tolerance and short-text match mode even
though those values describe grading rather than rendering. The target projection retains public
input constraints and displayed units while keeping tolerances, normalization rules, answer keys,
weights, and rubrics server-only.

Asset bytes do not belong inline in the JSON envelope. The envelope carries logical asset references
and checksums; the browser fetches image or other binary bytes through independently cacheable asset
routes. This keeps a large image from being retransmitted with every question or response.

### Current attempt projection

The current browser Assignment Attempt screen receives a complete
[QuestionAttempt](../crates/question_model/src/lib.rs). That persistence record contains:

- course, Student Record, Assignment Attempt, immutable Question Revision reference, Assignment Entry, and seed;
- parameter hash, response, status, result, and timer state; and
- Question Backend Version, Question Renderer Version, generator, source-object,
  asset-object, Question Grader Version, and rendered-hash provenance.

Most of those fields are legitimate server evidence but unnecessary browser data. The active UI
needs only the attempt ID, student-visible deadline, presentation binding, and public envelope. It
does not need Student identity, course authorization evidence, parameter hashes, source-object IDs,
Question Backend, Renderer, or Grader Versions, or complete provenance.

The implemented `getAssignmentAttemptScreen` client currently assembles a screen by loading the
Assignment Attempt, Student Record, cursor-paged Question Attempts, Assignment, Course Instance,
appearance, and Issued Question. In a one-time wire
fixture,
that required at least seven JSON responses across four dependent waves. A purpose-built server
projection can perform those relationship checks once and return one bounded student screen.

### Current response payload

PLE currently accepts:

```http
POST /api/courses/{courseId}/assignments/{assignmentId}/attempts/{attemptId}/submissions
Idempotency-Key: <opaque bounded key>
Content-Type: application/json
```

```json
{
  "response": {
    "kind": "multipleChoice",
    "selected": ["amide"]
  }
}
```

The current server authenticates the session, loads the RLS-visible Question Attempt and owning Assignment Attempt, validates
the response against the checksummed issued public snapshot, and atomically records immutable
accepted work plus a ready grading job. The sealed worker translates rendered IDs through the
matching server-only grading envelope, grades under server authority, and commits the completed
aggregate. Neither path reproduces a mutable issued envelope. Therefore the submitted `kind` is
redundant. The attempt already determines the expected Question Type.

Removing `kind` is not merely deleting one JSON property. The v1 handler must first load the attempt
and its issued public snapshot Question Response Format, then select a closed
Question-Response-Format-specific decoder for `answer`.
Unknown fields and shapes must continue to fail closed. Rich tagged Rust and TypeScript draft types
may remain internal even though the public answer wire is type-free.

### Current result payload

The current receipt repeats a full `QuestionAttempt` plus feedback and next-question data. That
returns persistence and provenance fields the active screen does not need. The target receipt returns
only:

- whether the submission was accepted;
- the committed attempt ID;
- policy-permitted correctness or points, if disclosure allows them;
- sanitized student feedback, if disclosure allows it; and
- the minimal next-attempt descriptor, if one was promoted.

The receipt never returns an answer key, expected value, private rubric, component weights, source,
renderer field names, credentials, or raw provider results.

## Attempt authority

### Why the attempt is primary

An issued attempt already binds the facts required to grade safely:

- authenticated student through exact CourseId and Student ownership;
- course and assignment context;
- exact immutable QuestionRevisionReference and assignment position;
- generated seed and immutable provenance;
- expected Question Type and grading backend;
- issue time, effective deadline, and submission state; and
- feedback, retry, grading, and continued-practice policies.

The browser therefore does not need to resend a question ID, course ID, assignment ID, version,
seed, backend, Question Type, points, or grading mode. Treating browser copies of those values as
authority would create disagreement cases without adding information.

### UUID cost

PLE uses a UUID-sized attempt identity once in the submission path. Its 36-character text spelling
is insignificant beside the render payload, HTTP headers, and assets. PLE should not assign UUIDs to
every rendered choice. Selectable objects use four-character presentation-scoped IDs, so a ten-choice
answer does not repeat ten UUIDs.

The attempt ID is durable database identity. Rendered-item IDs are temporary wire identity. Keeping
those roles separate avoids compressing durable IDs prematurely or making a 16-bit value globally
authoritative.

## Target network contract

### Student screen

The target `GET /api/assignment-attempts/{assignmentAttempt}/screen` response contains one navigation shell, one active Assignment Attempt
descriptor, and one public envelope:

```json
{
  "scope": {
    "course": "...",
    "assignment": "...",
    "theme": "grass"
  },
  "assignmentAttempt": {
    "attemptNumber": 4
  },
  "attempt": {
    "id": "...",
    "deadline": null,
    "presentationToken": "pd1_..."
  },
  "envelope": {
    "version": "...",
    "seed": 90210,
    "presentationNonce": "...",
    "title": "Peptide bonds",
    "prompt": [],
    "response": {
      "kind": "multipleChoice",
      "choices": []
    }
  }
}
```

The Assignment Attempt reference is already in the request path. The response omits complete Student Record, assignment,
course, Student, Question Attempt, and Question Attempt Reproduction Details. The authenticated server resolves Student ownership
from the exact Course Membership; the browser does not choose or receive a Student identifier. The
browser receives `version` and `seed` because they help identify and reproduce the public render, but
it does not send either value back when answering.

### Student response

Every ordinary answer uses the same outer request:

```json
{
  "presentationToken": "pd1_...",
  "answer": {}
}
```

The attempt ID remains in the path and the idempotency key remains in the header. The server chooses
the strict `answer` decoder from the attempt's issued Question Response Format.

| Question Type     | Minimal `answer` representation                              |
| ----------------- | ------------------------------------------------------------ |
| Single choice     | `{ "selected": "4ef3" }`                                     |
| Multiple answer   | `{ "selected": ["4ef3", "91c2"] }`                           |
| Fill in the blank | `{ "text": "student text" }`                                 |
| Multiple blanks   | `{ "blanks": [{ "slot": "4ef3", "text": "student text" }] }` |
| Numerical         | `{ "text": "1.25e-3" }`                                      |
| Matching          | `{ "matches": [{ "prompt": "12a4", "choice": "ef32" }] }`    |
| Ordering          | `{ "order": ["91c2", "bb28", "4ef3"] }`                      |
| Hotspot           | `{ "selections": [{ "region": "4ef3" }] }`                   |

Numerical input remains lexical text on the wire. This preserves what the student typed, permits
strict server parsing, and avoids browser/server disagreement about floating-point serialization or
accepted scientific notation.

Matching sends only rendered IDs for each relationship, not duplicated prompt or choice objects.
Ordering sends the ordered identifiers, not item content. Hotspot sends rendered Hotspot Region
identifiers; the server resolves each one against the exact issued presentation. The browser never
sends raw image bytes, grading regions, or authoring geometry.

### Grading result

The target result is a compact, policy-projected receipt, conceptually:

```json
{
  "accepted": true,
  "attempt": "...",
  "outcome": {
    "correct": true,
    "pointsEarned": 1.0,
    "pointsPossible": 1.0
  },
  "feedback": null,
  "next": null
}
```

`outcome` or parts of it may be absent when the feedback policy withholds correctness or score. The
browser never calculates partial credit and never sends component scores, weights, maximum score, or
claims of correctness. PLE Question Backend grading computes all component results and
the server projects only what the student may see.

## Rendered-item IDs

### Purpose

A visible label such as `B` is positional. If a stale browser associates a response with a different
choice order, `B` can still look syntactically valid. PLE instead gives each selectable rendered
object an attempt-presentation-specific ID such as `4ef3`. The browser may display ordinary labels
while submitting the rendered ID.

Rendered IDs apply to:

- single-choice and multiple-answer choices;
- multi-blank slots;
- both sides of matching;
- ordering items; and
- hotspot surfaces and named regions.

A single free-text or numeric field does not need an item ID unless a presentation contains multiple
addressable fields.

### CRC16 contract

`PresentationResponseItemReference` is exactly four lowercase hexadecimal characters derived with
CRC-16/CCITT-FALSE:

- polynomial `0x1021`;
- initial value `0xffff`;
- reflected input and output disabled;
- final XOR `0x0000`; and
- check vector `123456789` produces `29b1`.

The checksum input is domain-separated and includes the presentation nonce, immutable version, seed,
item role, ordinal, durable internal item ID, and SHA-256 of the canonical public rendered content.
This contract defines the normative byte framing.
Rust owns this codec; the browser calls the Rust/Wasm implementation and does not reimplement it in
TypeScript.

CRC16 does not replace the durable Response Item Reference, Text Entry Slot Reference, or other
Question Response Format identity. PLE stores or
reproduces an authoritative mapping from the presentation ID to the internal object for the attempt.

### Collision handling

For ten uniformly distributed identifiers there are 45 pairs, so the probability of at least one
pairwise collision is approximately `45 / 65,536`, or about one in 1,456 presentations before
collision rejection. This is acceptable because issuance enforces uniqueness across every rendered
item in the whole presentation:

1. Generate a random 16-byte presentation nonce.
2. Derive every rendered-item ID.
3. Reject the set if any ID is duplicated, including across different item roles.
4. Retry with a fresh nonce, at most eight times.
5. Fail closed rather than issue an ambiguous presentation.

The student never receives a presentation containing a duplicate. CRC16 is attractive here because
it is compact and easy to inspect, not because it is collision-resistant or secret.

## Presentation consistency

### Question Presentation Checksum

Fine-grained IDs say which rendered objects the student selected. A separate Question Presentation Checksum binds the
complete public presentation:

- immutable version and seed;
- presentation nonce;
- public title and prompt blocks;
- Question Response Format and widget constraints;
- rendered item roles, order, IDs, and public content;
- asset IDs and content checksums; and
- normalized hotspot geometry where applicable.

The database stores all 32 Question Presentation Checksum bytes. The public Question Presentation Token carries the first 16 bytes in
base64url form. A 128-bit public prefix is inexpensive and is more suitable than CRC16 for detecting
whole-presentation disagreement. It is still a consistency value, not an authentication token.

### Detection boundary

| Mechanism                         | Detects                                                                                                   | Does not prove                                                 |
| --------------------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Rendered-item membership and role | Unknown or stale selection, wrong ordering map, wrong matching side, wrong blank, or wrong Hotspot Region | Student identity or correctness                                |
| Question Presentation Checksum    | Stale or mixed cached state, changed prompt/schema/order/assets/geometry, wrong version/seed/nonce        | TLS, browser integrity, pixel display, or image decode success |
| Authenticated Question Attempt    | Student ownership, Course/Assignment Attempt binding, lifecycle, timing, backend, and immutable version   | That the browser rendered every asset                          |
| Idempotency record                | Exact retry versus changed replay                                                                         | Correctness of the answer                                      |

An ordinary transport checksum is unnecessary because TLS and HTTP already detect transfer
corruption. The digest addresses application-state disagreement: the wrong valid render paired with
the wrong valid attempt.

### Mismatch recovery

A presentation mismatch must not grade or mutate the attempt. The server returns a stable
`409 presentation_mismatch` and records only bounded diagnostic identifiers and reason codes, never
answers, source, credentials, or provider state.

The browser then:

1. disables submission;
2. preserves the student's editable draft in memory under attempt ID plus digest;
3. reloads the same attempt presentation;
4. restores the draft only when the Question Response Format and rendered IDs remain compatible;
5. asks the student to review the restored answer; and
6. submits again only after the current presentation validates.

PLE must not silently issue a new seed, grade against the stale state, or discard typed work. A
repeatable mismatch after same-attempt refresh is a server defect or corrupt persisted binding and
must fail closed for operator investigation.

## PLE Question JSON grading

For PLE Question JSON Questions, PLE owns both immutable content and grading. The normal path is:

1. Load the authenticated active attempt.
2. Load its checksummed issued PLE Question JSON grading contract, not a current published Question or grader view.
3. Verify the submitted Question Presentation Token against the stored complete
   Question Presentation Checksum.
4. Decode the type-free `answer` using the issued public Question Response Format.
5. Map rendered IDs to durable internal IDs.
6. Apply answer normalization, correctness, and partial-credit rules server-side.
7. Atomically persist response, score events, Question Attempt/Assignment Attempt transitions, and idempotency result.
8. Return only the policy-permitted receipt.

This keeps rich type information inside Rust where exhaustive enums are useful while keeping the
network request small. Adding future server-only rubrics does not require the browser to submit
scores or grading metadata.

## WeBWorK grading

### Current private flow

The browser-facing WeBWorK envelope is already sanitized and answer-free. PLE resolves immutable PG
source from server-only object storage and caches the safe render by problem, version, and seed. An
**issue** cache hit reuses that public render but still makes one private same-seed renderer call to
recover and verify the replay mapping that the safe cache deliberately excludes. In contrast,
reproducing an already-issued attempt reads only the safe cache and makes no renderer call; its
attempt-bound replay mapping is loaded separately from the protected record for the exact
`CourseId`, `StudentRecordId`, `AssignmentAttemptId`, and `QuestionAttemptId`.

PLE privately calls the external standalone `/render-api` form endpoint with source, file path,
seed, fixed display controls, and signed renderer state. Those fields never cross the browser
boundary. The historical RC3 compatibility grading path originally performed two private calls:

1. rerender the same source and seed to recover and validate the opaque PLE-choice to upstream
   `AnSwEr...` field/value mapping; and
2. call the same endpoint with the selected upstream field/value and `WWsubmit=1`.

The receipt-era persistence slice stores the validated mapping, exact public snapshot, matching
server-only grading envelope, and frozen WeBWorK definition under the issued attempt. Normal grade
validates those artifacts and performs only the private grade call; it never resolves a current
published Question Revision or rerenders to recover state. The official upstream endpoint is stateless, so
PLE still sends immutable source provenance and signed server state on that private grade call. That
repetition is an internal service cost, not student payload.

### Implemented private replay slice and remaining target

At issue time, PLE now persists a bounded, server-only replay record mapping each rendered-item ID
to its validated upstream field/value. The record contains no source, credential, session key,
correct-answer flag, raw provider result, or browser-visible field name.

Normal grade then:

1. loads and validates the attempt-bound replay record;
2. validates the public response against the issued snapshot and resolves its rendered-item ID to
   one protected upstream field/value;
3. loads immutable source and private renderer credentials server-side;
4. makes one private `/render-api` grade call; and
5. accepts only the supported result shape and score policy.

Binding disagreement refuses before grading. Successful submission and terminal instructor action
delete replay state atomically. Missing or malformed state is an intentional unavailable failure:
pre-production receipt-era data has no rerender, self-heal, or compatibility reader. The browser
never receives or resubmits PG source, upstream field names, radio values, passwords, session keys,
renderer URLs, or provider score objects.

The reviewed Chapter 1 WeBWorK profile supports its two `RadioButtons` sources and two matching
sources. Matching partial credit is admitted only when both the source path and immutable source
digest match the accepted evidence profile. Other WeBWorK interactions still require their own
adapter contract and live evidence.

## ADAPT comparison

This comparison uses the local `OTHER_REPOS/adapt` snapshot as implementation evidence. It is not a
compatibility requirement and may differ from a later upstream release.

The inspected evidence is in `routes/api.php`,
`app/Http/Controllers/AssignmentSyncQuestionController.php`, `app/Question.php`,
`app/Submission.php`, `app/Http/Requests/StoreSubmission.php`,
`app/Http/Controllers/JWTController.php`,
`app/Http/Controllers/UnconfirmedSubmissionController.php`,
`resources/js/components/QtiJsonQuestionViewer.vue`, and
`resources/js/helpers/HandleTechnologyResponse.js` under that snapshot.

### ADAPT direct-question flow

ADAPT's assignment question view returns rich question records containing IDs, technology, revision,
seeded and student-sanitized QTI JSON, prior response, points, timing/attempt state, media URLs, and
external iframe state. Its QTI formatter appropriately strips correct responses, private feedback,
and solutions when release policy does not allow them.

The ADAPT browser submits `assignment_id`, `question_id`, serialized `submission`, and a
client-selected `technology`; current client code also constructs `max_score`. The server loads the
stored question and infers `questionType`, so ADAPT already demonstrates that the student does not
need to submit the PLE Question Type. It computes partial credit server-side.

ADAPT's simple multiple-choice answer can be one choice identifier, but some Question Types are more
verbose than necessary. Matching submits complete mutated `termsToMatch` objects even though grading
needs relationships between identifiers. No attempt ID, Question Presentation Token, version token, or ETag
was found on the inspected ADAPT submission boundary.

### ADAPT WeBWorK flow

ADAPT embeds WeBWorK in an iframe with an encrypted/signed `problemJWT`. The renderer sends a signed
`answerJWT` to ADAPT; ADAPT validates and decrypts the context, stores an unconfirmed renderer result,
and the browser later confirms it. This protects external context better than trusting plain browser
IDs, but rich renderer answer and score objects participate in the browser-facing workflow.

### Decisions from comparison

| Concern              | ADAPT observation                                    | PLE decision                                                                                                  |
| -------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Render sanitization  | Strips correct responses and feedback by role/policy | Adopt and preserve                                                                                            |
| Determinism          | Stores a per-student assignment/question seed        | Adopt the deterministic principle; bind it to an attempt                                                      |
| Server-inferred type | Server infers `questionType`                         | Remove submission `kind` in PLE v1                                                                            |
| Server-held identity | Browser sends assignment and question IDs            | Use one attempt ID that already binds both                                                                    |
| ADAPT render scope   | Rich assignment/question records                     | Return one minimal active student screen                                                                      |
| Matching response    | Whole mutated objects                                | Send only rendered-ID relationships                                                                           |
| Backend selector     | Browser sends `technology`                           | Derive backend from the attempt                                                                               |
| Partial credit       | Server computes it                                   | Preserve server-only scoring                                                                                  |
| External context     | JWT/JWE protects renderer context                    | Keep private renderer exchange entirely behind PLE                                                            |
| Renderer result      | Rich WebWork data crosses browser-facing flow        | Return only PLE's policy-projected receipt                                                                    |
| Presentation check   | No ADAPT digest found                                | Bind the answer to a Question Presentation Token verified against its complete Question Presentation Checksum |

PLE should not copy ADAPT merely because ADAPT has more features. It should adopt the mature ideas
that match PLE's goals and intentionally differ where an attempt-bound, server-mediated architecture
is smaller and easier to secure.

## Responsiveness strategy

### Recorded payload evidence

One-time synthetic wire-fixture measurements from the accepted decision recorded:

| Current artifact                              |                             Recorded size |
| --------------------------------------------- | ----------------------------------------: |
| Answer-free PLE envelope                      |                               1,091 bytes |
| Full PLE attempt                              |                                 977 bytes |
| One-attempt page                              |                               1,007 bytes |
| Current MC submit body                        |                                  59 bytes |
| Current submission receipt                    |                               1,039 bytes |
| Current Assignment Attempt screen JSON bodies | 3,757 bytes over at least seven responses |
| Two-choice private WeBWorK render form        |                                 310 bytes |
| Two-choice private WeBWorK grade form         |                                 334 bytes |
| Recorded private WeBWorK JSON response        |                               1,221 bytes |

These figures describe synthetic payload fixtures, not fictional live-demo records, production
percentiles, or permanent limits. Real prompt HTML and media can be much larger. PLE permits a bounded
PG source whose private base64 form can dwarf
the student's answer JSON.

### Where latency matters

The end-to-end submission path contains:

1. browser-to-PLE network round trip;
2. authentication and bounded JSON parsing;
3. RLS-visible Question Attempt, Assignment Attempt, question, and idempotency reads;
4. presentation and response validation;
5. PLE grading or private PLE-to-WeBWorK round trip and execution;
6. atomic persistence and summary updates; and
7. compact receipt serialization and browser return.

Removing a roughly 20-byte `kind` field improves clarity but is not a meaningful latency
optimization by itself. The higher-value actions are:

- collapse the Assignment Attempt screen request fan-out into one projection;
- cache immutable assets and safe renders;
- avoid inline binary media;
- remove the normal extra WeBWorK rerender;
- keep database access bounded and indexed; and
- instrument stage timing before selecting further optimizations.

WP-P6 records representative payload sizes and p50/p95 stage times. It does not turn the fixture byte
counts or arbitrary latency thresholds into permanent tests.

## Caching and prefetch

### Safe caching

PLE may cache public render data by immutable `QuestionRevisionReference`, seed, and the presentation binding.
Cache entries may contain only the answer-free envelope, sanitized markup, public asset references, and
renderer identity needed for provenance. They must not contain correct answers, private rubrics,
credentials, session keys, source archives, or raw provider responses.

Assets use immutable logical URLs and content checksums. The browser can fetch and cache them
independently. The presentation becomes submission-ready only after required response controls and
assets report their documented readiness; the digest does not prove an image decoded.

### Safe prefetch

PLE can prepare one next question while the student works on the current question when policy allows
it. The server, not the browser:

- chooses the next assignment position and fresh seed;
- creates the CourseId/StudentRecordId/Assignment Attempt/predecessor-attempt-bound reservation;
- resolves source and backend;
- renders and stores the public presentation binding; and
- promotes the reservation atomically only after the predecessor commits.

The following is target behavior, not a claim about the current prefetch route. For untimed mastery
and practice work, the browser may receive the next answer-free envelope early and warm a bounded
set of same-origin assets. For timed or exam policy, PLE may pre-render privately but must not reveal
the next envelope until the current attempt commits. The current bodyless reservation route does not
yet enforce that timing-policy distinction. Prefetch never grades, starts the next timer, or lets the
browser choose a seed, backend, or source.

## Security properties

The security boundary is:

- TLS and same-origin browser transport;
- authenticated HttpOnly session;
- exact CourseId and StudentRecordId ownership through forced RLS and Assignment Attempt lookup;
- immutable version, seed, timing, and backend binding;
- strict schema-selected answer decoding;
- attempt lifecycle checks;
- idempotency and atomic commit; and
- server-only grading data and provider credentials.

CRC16 and the Question Presentation Token/Checksum pair add useful consistency evidence. They do not replace any item in
that list. A malicious authenticated student can see every public choice and can submit any valid
rendered ID; secrecy of distractor IDs is neither expected nor required. Correctness remains known
only to the server-side grader.

## Implementation packages

The active decision defines six dispatchable packages. This summary makes the durable architecture
easy to navigate without duplicating its exact migration and codec specification.

### WP-P1: Contract codec

- Owner: Rust implementation, independently reviewed for Wasm parity.
- Files: question-model presentation and student descriptors, response wire types, Wasm exports,
  generated browser types, and fixed vectors.
- Behavior: implement canonical descriptor bytes, Question Presentation Checksum, CRC16 item IDs, collision retry,
  and all eight minimal answer shapes.
- Success: Rust and Wasm vectors agree byte-for-byte; meaningful descriptor changes alter the checksum;
  duplicate rendered IDs trigger bounded nonce retry and fail closed after eight attempts.
- Validation: focused Rust/Wasm/vector tests, formatter, strict Clippy, generated-binding freshness,
  and independent Wasm review.

### WP-P2: Persistent binding

- Owner: PostgreSQL and Store implementation.
- Files: `2026080908_secure_question_grading_payloads.sql`, Store traits, Memory/PostgreSQL stores,
  prefetch promotion, retention, backup/restore, and conformance tests.
- Behavior: persist the presentation version, nonce, Question Presentation Checksum, request-contract version, prefetch
  binding, and bounded private WeBWorK replay state under forced RLS.
- Success: constraints reject malformed data; another AccountId, another course, and a foreign attempt
  cannot read or write it; Memory and PostgreSQL agree; retention and restore preserve or remove
  bindings correctly.
- Validation: fresh/no-op migration, malformed-version tests, forced-RLS tests, Store parity,
  retention, backup/restore, and independent PostgreSQL review.

### WP-P3: PLE API cutover

- Owner: server and PLE grading implementation.
- Files: Assignment Attempt screen and submission projections, PLE Question Backend validation, route tests, API fixtures,
  and generated client contracts.
- Behavior: serve one minimal student screen, decode type-free answers after attempt load, verify
  digest and idempotency before grading, and return compact receipts.
- Success: each Question Type accepts its exact shape and rejects extras; exact replay returns the first
  receipt; changed replay conflicts before grading; mismatch does not mutate; no raw attempt or
  provenance crosses the active student route.
- Validation: focused Axum and security tests, Question Type wire vectors, PLE regression, and independent
  server review.

### WP-P4: WeBWorK replay

- Owner: WebWork adapter and server backend, independently security reviewed.
- Files: renderer contract, WebWork backend, replay-state Store API, request-count tests, and private
  live test.
- Behavior: persist the safe issued mapping and make normal grading one private RPC; permit one
  receipt-era missing or mismatched replay state fails closed without rerendering.
- Success: normal, retry, recovery, and mismatch traces prove exact call behavior and prove no source,
  credential, session key, upstream mapping, or raw result reaches browser-visible state.
- Validation: recorded upstream contract tests, private-container trace, state scans, and security
  review.

### WP-P5: Browser recovery

- Owner: SolidJS browser implementation with HCI review.
- Files: API decoders/query owner, Assignment Attempt page, Question Attempt state, question response controls, Wasm bridge, and
  Playwright scenarios.
- Behavior: consume the single student screen, compute/verify through Wasm, gate required asset
  readiness, submit the compact body, and recover compatible drafts after same-attempt refresh.
- Success: keyboard-only paths for all accepted Question Types pass; a network trace contains no submission
  `kind`, private field, full attempt, or provider data.
- Validation: built-browser Playwright, no-mouse contract, mismatch/offline retry, and accessibility
  review.

### WP-P6: Evidence closure

- Owner: integrator and independent reviewers.
- Files: measurement tooling, E2E traces, payload evidence report, architecture/contracts/file map,
  status, usage, and changelog.
- Behavior: record redacted bytes and stage timings without exposing student answers or secrets.
- Success: evidence identifies meaningful latency stages and makes no invented SLO claim before pilot
  measurements.
- Validation: reproducible local-stack collection, full repository gate, and independent Rust,
  security, PostgreSQL, HCI, and documentation review.

## Test classification

Permanent tests protect stable behavior:

- attempt-selected strict answer decoding;
- response `kind` absent from the public submission wire;
- a missing authenticated session, another AccountId, another course, and a foreign attempt are concealed before protected
  payload access;
- every issued and replayed record matches its exact CourseId, StudentRecordId, AssignmentAttemptId, QuestionAttemptId,
  QuestionRevisionReference, and seed;
- rendered-ID membership, role, collision retry, and fail-closed issuance;
- presentation mismatch causes no grade or mutation;
- exact idempotent replay and changed-replay conflict;
- student-screen and receipt allowlists;
- target prefetch promotion and timed-content withholding;
- normal one-call WeBWorK grading; and
- browser traces exclude private material.

One-time implementation evidence records:

- fixture byte sizes;
- request-wave counts during the old-to-new comparison;
- p50/p95 latency observations;
- temporary database query plans used to justify an index; and
- migration validation diagnostics already enforced by the final behavior.

Do not preserve exact fixture sizes, arbitrary response counts, incidental function names, temporary
probes, or latency thresholds as permanent tests. The permanent-test decision follows
[PYTEST_STYLE.md](PYTEST_STYLE.md): retain deterministic public behavior and security invariants;
remove rebuild-only evidence once the maintained gate proves the final contract.

## Rollout decision

The public contract changes atomically before WP-RC5 adds new PLE Question JSON Question Types. PLE must not maintain a
long-lived mixed endpoint where some active attempts use tagged responses and others use type-free
answers without an explicit contract version.

The pre-production cutover:

1. stops new attempt issuance;
2. lets old-contract active attempts finish on the old release;
3. requires no active attempts or prefetch reservations;
4. applies the forward-only payload migration;
5. deploys server, Wasm, generated bindings, and browser together; and
6. rejects retired route/body usage with stable `410 contract_retired`.

Historical records remain available through bounded history and summary projections. Production data
is never deleted or recreated as a shortcut. iMathAS Question Backend submission contracts stay out of this v1
cutover because they require their own backend-session design.

## Final decisions

- Keep `kind` in the render schema; remove it from the v1 Student Response.
- Use one authenticated attempt ID as submission identity; do not resend question context.
- Give every addressable rendered object a four-hex CRC16 ID unique within its presentation.
- Keep durable internal IDs independent from browser-facing rendered IDs.
- Use SHA-256 for whole-presentation consistency and CRC16 for compact item correspondence.
- Keep all correctness, component scoring, and partial credit server-owned.
- Return one minimal student screen and one compact, policy-projected receipt.
- Cache only answer-free public renders and immutable assets.
- Use server-owned reservations for prefetch; add timed-content withholding at the target cutover.
- Keep the official WeBWorK exchange private and reduce normal grading from two RPCs to one through
  bounded server-only replay state.
- Optimize request fan-out, assets, database work, and renderer execution before shaving already tiny
  answer JSON.
