# Assessment payload design

## Status and authority

This document explains PLE's learner-facing assessment payload boundary, compares it with the local
LibreTexts ADAPT snapshot, and summarizes the accepted implementation direction. It distinguishes
the current implementation from the target contract so contributors do not mistake planned wire
changes for released behavior.

The exact codec, migration, cutover, ownership, and acceptance requirements remain normative in the
[secure_question_grading_payload_plan.md](active_plans/decisions/secure_question_grading_payload_plan.md).
The [release_completion_plan.md](active_plans/active/release_completion_plan.md) owns dependency
order. This durable guide explains why those decisions exist and how the boundaries fit together.

## Design summary

PLE should send a rich, answer-free render payload once and accept a much smaller learner response.
The browser needs enough public information to draw the prompt, choices, input controls, accessible
labels, and assets. It does not need database provenance, grading rules, answer keys, backend
selection, renderer credentials, or the complete attempt record.

The target contract is:

```text
Browser                                      PLE
   |                                          |
   | GET /api/runs/{run}/screen               |
   |----------------------------------------->|
   | minimal shell + attempt + public render  |
   |<-----------------------------------------|
   |                                          |
   | POST /api/submissions/{attempt}          |
   | Idempotency-Key: ...                     |
   | {presentationDigest, answer}             |
   |----------------------------------------->|
   | compact policy-permitted receipt         |
   |<-----------------------------------------|
```

The authenticated `QuestionAttemptId` in the route is the primary learner-response binding. A
presentation digest checks that the browser answered the same render state PLE issued. Compact
CRC16 rendered-item IDs identify choices, blanks, matching sides, ordered items, and hotspot
surfaces within that presentation. Neither the digest nor CRC16 authenticates the learner or proves
correctness.

## Current PLE boundary

### Current render payload

The implemented [QuestionEnvelope](../crates/question_model/src/envelope.rs) is deliberately
answer-free. It currently contains:

- immutable question `version`;
- server-issued `seed`;
- learner-facing `title`;
- ordered prompt blocks for text, math, images, code, and tables;
- public asset IDs, SHA-256 checksums, and accessible descriptions; and
- a tagged `ResponseDefinition` that selects the browser widget.

The response `kind` is necessary in this render payload. Without it, the browser cannot know whether
to draw radio buttons, checkboxes, text boxes, ordering controls, matching controls, or a hotspot
surface. Content-block `kind` values are similarly useful render discriminants.

The current [ResponseDefinition](../crates/question_model/src/response.rs) is broader than the
eventual learner schema. For example, it exposes numeric tolerance and short-text match mode even
though those values describe grading rather than rendering. The target projection retains public
input constraints and displayed units while keeping tolerances, normalization rules, answer keys,
weights, and rubrics server-only.

Asset bytes do not belong inline in the JSON envelope. The envelope carries logical asset references
and checksums; the browser fetches image or other binary bytes through independently cacheable asset
routes. This keeps a large image from being retransmitted with every question or response.

### Current attempt projection

The current browser run screen receives a complete
[QuestionAttempt](../crates/question_model/src/activity.rs). That persistence record contains:

- tenant, run, problem, immutable version, assignment position, and seed;
- parameter hash, response, status, result, and timer state; and
- adapter, renderer, generator, source-object, asset-object, grading, and rendered-hash provenance.

Most of those fields are legitimate server evidence but unnecessary browser data. The active UI
needs only the attempt ID, learner-visible deadline, presentation binding, and public envelope. It
does not need tenant IDs, problem IDs, parameter hashes, source-object IDs, implementation versions,
or complete provenance.

The implemented `getRunScreen` client currently assembles a screen by loading the run, enrollment,
cursor-paged attempts, assignment, course, appearance, and issued question. In the recorded fixture,
that required at least seven JSON responses across four dependent waves. A purpose-built server
projection can perform those relationship checks once and return one bounded learner screen.

### Current response payload

PLE currently accepts:

```http
POST /api/submissions/{attemptId}
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

The current server authenticates the session, loads the RLS-visible attempt and owning run, loads the
immutable question, reproduces the exact issued envelope, validates the response shape, selects the
stored backend, grades, and commits once. Therefore the submitted `kind` is redundant. The attempt
already determines the expected response family.

Removing `kind` is not merely deleting one JSON property. The v1 handler must first load the attempt
and its reproduced response schema, then select a closed, family-specific decoder for `answer`.
Unknown fields and shapes must continue to fail closed. Rich tagged Rust and TypeScript draft types
may remain internal even though the public answer wire is type-free.

### Current result payload

The current receipt repeats a full `QuestionAttempt` plus feedback and next-question data. That
returns persistence and provenance fields the active screen does not need. The target receipt returns
only:

- whether the submission was accepted;
- the committed attempt ID;
- policy-permitted correctness or points, if disclosure allows them;
- sanitized learner feedback, if disclosure allows it; and
- the minimal next-attempt descriptor, if one was promoted.

The receipt never returns an answer key, expected value, private rubric, component weights, source,
renderer field names, credentials, or raw provider results.

## Attempt authority

### Why the attempt is primary

An issued attempt already binds the facts required to grade safely:

- authenticated learner and tenant through the owned run and enrollment;
- course and assignment context;
- exact published problem version and assignment position;
- generated seed and immutable provenance;
- expected response family and grading backend;
- issue time, effective deadline, and submission state; and
- feedback, retry, grading, and continued-practice policies.

The browser therefore does not need to resend a question ID, course ID, assignment ID, version,
seed, backend, response family, points, or grading mode. Treating browser copies of those values as
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

### Learner screen

The target `GET /api/runs/{run}/screen` response contains one navigation shell, one active attempt
descriptor, and one public envelope:

```json
{
  "scope": {
    "course": "...",
    "assignment": "...",
    "theme": "grass"
  },
  "run": {
    "number": 4,
    "mode": "practice"
  },
  "attempt": {
    "id": "...",
    "deadline": null,
    "presentationDigest": "pd1_..."
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

The run ID is already in the request path. The response omits complete enrollment, assignment,
course, attempt, and provenance records. The browser receives `version` and `seed` because they help
identify and reproduce the public render, but it does not send either value back when answering.

### Student response

Every ordinary answer uses the same outer request:

```json
{
  "presentationDigest": "pd1_...",
  "answer": {}
}
```

The attempt ID remains in the path and the idempotency key remains in the header. The server chooses
the strict `answer` decoder from the attempt's issued response definition.

| Family            | Minimal `answer` representation                              |
| ----------------- | ------------------------------------------------------------ |
| Single choice     | `{ "selected": "4ef3" }`                                     |
| Multiple answer   | `{ "selected": ["4ef3", "91c2"] }`                           |
| Fill in the blank | `{ "text": "learner text" }`                                 |
| Multiple blanks   | `{ "blanks": [{ "slot": "4ef3", "text": "learner text" }] }` |
| Numerical         | `{ "text": "1.25e-3" }`                                      |
| Matching          | `{ "matches": [{ "prompt": "12a4", "choice": "ef32" }] }`    |
| Ordering          | `{ "order": ["91c2", "bb28", "4ef3"] }`                      |
| Hotspot           | `{ "surface": "4ef3", "points": [{ "x": 512, "y": 233 }] }`  |

Numerical input remains lexical text on the wire. This preserves what the learner typed, permits
strict server parsing, and avoids browser/server disagreement about floating-point serialization or
accepted scientific notation.

Matching sends only rendered IDs for each relationship, not duplicated prompt or choice objects.
Ordering sends the ordered identifiers, not item content. Hotspot coordinates use a documented
normalized integer coordinate space bound to a rendered surface ID; the browser never sends raw
image bytes or grading regions.

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
claims of correctness. Native PLE grading or the private backend computes all component results and
the server projects only what the learner may see.

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
- hotspot surfaces or named regions where the response contract needs one.

A single free-text or numeric field does not need an item ID unless a presentation contains multiple
addressable fields.

### CRC16 contract

`RenderedItemIdV1` is exactly four lowercase hexadecimal characters derived with
CRC-16/CCITT-FALSE:

- polynomial `0x1021`;
- initial value `0xffff`;
- reflected input and output disabled;
- final XOR `0x0000`; and
- check vector `123456789` produces `29b1`.

The checksum input is domain-separated and includes the presentation nonce, immutable version, seed,
item role, ordinal, durable internal item ID, and SHA-256 of the canonical public rendered content.
The normative byte framing is defined in the
[secure_question_grading_payload_plan.md](active_plans/decisions/secure_question_grading_payload_plan.md).
Rust owns this codec; the browser calls the Rust/Wasm implementation and does not reimplement it in
TypeScript.

CRC16 does not replace the durable internal `ChoiceId`, slot ID, or item identity. PLE stores or
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

The learner never receives a presentation containing a duplicate. CRC16 is attractive here because
it is compact and easy to inspect, not because it is collision-resistant or secret.

## Presentation consistency

### Whole-presentation digest

Fine-grained IDs say which rendered objects the learner selected. A separate SHA-256 digest binds the
complete public presentation:

- immutable version and seed;
- presentation nonce;
- public title and prompt blocks;
- response schema and widget constraints;
- rendered item roles, order, IDs, and public content;
- asset IDs and content checksums; and
- normalized hotspot geometry where applicable.

The database stores all 32 digest bytes. The public `pd1_` token carries the first 16 bytes in
base64url form. A 128-bit public prefix is inexpensive and is more suitable than CRC16 for detecting
whole-presentation disagreement. It is still a consistency value, not an authentication token.

### Detection boundary

| Mechanism                         | Detects                                                                                                 | Does not prove                                                 |
| --------------------------------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Rendered-item membership and role | Unknown or stale selection, wrong ordering map, wrong matching side, wrong blank, wrong hotspot surface | Learner identity or correctness                                |
| Presentation digest               | Stale or mixed cached state, changed prompt/schema/order/assets/geometry, wrong version/seed/nonce      | TLS, browser integrity, pixel display, or image decode success |
| Authenticated attempt             | Learner ownership, course/run binding, lifecycle, timing, backend, and immutable version                | That the browser rendered every asset                          |
| Idempotency record                | Exact retry versus changed replay                                                                       | Correctness of the answer                                      |

An ordinary transport checksum is unnecessary because TLS and HTTP already detect transfer
corruption. The digest addresses application-state disagreement: the wrong valid render paired with
the wrong valid attempt.

### Mismatch recovery

A presentation mismatch must not grade or mutate the attempt. The server returns a stable
`409 presentation_mismatch` and records only bounded diagnostic identifiers and reason codes, never
answers, source, credentials, or provider state.

The browser then:

1. disables submission;
2. preserves the learner's editable draft in memory under attempt ID plus digest;
3. reloads the same attempt presentation;
4. restores the draft only when the response schema and rendered IDs remain compatible;
5. asks the learner to review the restored answer; and
6. submits again only after the current presentation validates.

PLE must not silently issue a new seed, grade against the stale state, or discard typed work. A
repeatable mismatch after same-attempt refresh is a server defect or corrupt persisted binding and
must fail closed for operator investigation.

## Native flat grading

For native flat questions, PLE owns both immutable content and grading. The normal path is:

1. Load the authenticated active attempt.
2. Load the immutable published version and server-only grading payload.
3. Verify the stored and submitted presentation digest.
4. Decode the type-free `answer` using the issued public response schema.
5. Map rendered IDs to durable internal IDs.
6. Apply answer normalization, correctness, and partial-credit rules server-side.
7. Atomically persist response, score events, attempt/run transitions, and idempotency result.
8. Return only the policy-permitted receipt.

This keeps rich type information inside Rust where exhaustive enums are useful while keeping the
network request small. Adding future server-only rubrics does not require the browser to submit
scores or grading metadata.

## WeBWorK grading

### Current private flow

The browser-facing WeBWorK envelope is already sanitized and answer-free. PLE resolves immutable PG
source from server-only object storage and caches the safe render by problem, version, and seed. A
cache hit performs no renderer call.

PLE privately calls the shipped `/webwork2/render_rpc` form endpoint with source, file path, seed,
course, user, password, and display controls. Those fields never cross the browser boundary. The
current grading path performs two private calls:

1. rerender the same source and seed to recover and validate the opaque PLE-choice to upstream
   `AnSwEr...` field/value mapping; and
2. call the same endpoint with the selected upstream field/value and `WWsubmit=1`.

The official upstream endpoint is stateless, so PLE must still send source and server credentials on
the private grade call. That repetition is an internal service cost, not learner payload.

### Target private flow

At issue time, PLE should persist a bounded, server-only replay record mapping each rendered-item ID
to its validated upstream field/value. The record contains no source, credential, session key,
correct-answer flag, raw provider result, or browser-visible field name.

Normal grade then:

1. loads and validates the attempt-bound replay record;
2. resolves the learner's rendered-item ID to one upstream field/value;
3. loads immutable source and private renderer credentials server-side;
4. makes one private `render_rpc` grade call; and
5. accepts only the supported result shape and score policy.

If replay state is missing after a recoverable pre-commit failure, PLE may perform one fully validated
same-seed self-heal rerender. Binding disagreement refuses before grading. The browser never receives
or resubmits PG source, upstream field names, radio values, passwords, session keys, renderer URLs,
or provider score objects.

Current RC3 WeBWorK supports one `RadioButtons` group as a single-choice interaction with
all-or-nothing grading. Matching and partial-credit WeBWorK interactions require their own accepted
adapter contracts; the payload design does not pretend they already exist.

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

### Native ADAPT flow

ADAPT's assignment question view returns rich question records containing IDs, technology, revision,
seeded and learner-sanitized QTI JSON, prior response, points, timing/attempt state, media URLs, and
external iframe state. Its QTI formatter appropriately strips correct responses, private feedback,
and solutions when release policy does not allow them.

The native browser submits `assignment_id`, `question_id`, serialized `submission`, and a
client-selected `technology`; current client code also constructs `max_score`. The server loads the
stored question and infers `questionType`, so ADAPT already demonstrates that the learner does not
need to submit the native question family. It computes partial credit server-side.

ADAPT's simple multiple-choice answer can be one choice identifier, but some families are more
verbose than necessary. Matching submits complete mutated `termsToMatch` objects even though grading
needs relationships between identifiers. No attempt ID, presentation digest, version token, or ETag
was found on the inspected native submission boundary.

### ADAPT WeBWorK flow

ADAPT embeds WeBWorK in an iframe with an encrypted/signed `problemJWT`. The renderer sends a signed
`answerJWT` to ADAPT; ADAPT validates and decrypts the context, stores an unconfirmed renderer result,
and the browser later confirms it. This protects external context better than trusting plain browser
IDs, but rich renderer answer and score objects participate in the browser-facing workflow.

### Decisions from comparison

| Concern             | ADAPT observation                                    | PLE decision                                             |
| ------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| Render sanitization | Strips correct responses and feedback by role/policy | Adopt and preserve                                       |
| Determinism         | Stores a per-learner assignment/question seed        | Adopt the deterministic principle; bind it to an attempt |
| Native type         | Server infers `questionType`                         | Remove submission `kind` in PLE v1                       |
| Native identity     | Browser sends assignment and question IDs            | Use one attempt ID that already binds both               |
| Native render scope | Rich assignment/question records                     | Return one minimal active learner screen                 |
| Matching response   | Whole mutated objects                                | Send only rendered-ID relationships                      |
| Backend selector    | Browser sends `technology`                           | Derive backend from the attempt                          |
| Partial credit      | Server computes it                                   | Preserve server-only scoring                             |
| External context    | JWT/JWE protects renderer context                    | Keep private renderer exchange entirely behind PLE       |
| Renderer result     | Rich WebWork data crosses browser-facing flow        | Return only PLE's policy-projected receipt               |
| Presentation check  | No native digest found                               | Bind the answer to a canonical PLE presentation digest   |

PLE should not copy ADAPT merely because ADAPT has more features. It should adopt the mature ideas
that match PLE's goals and intentionally differ where an attempt-bound, server-mediated architecture
is smaller and easier to secure.

## Responsiveness strategy

### Recorded payload evidence

One-time fixture measurements from the accepted decision recorded:

| Current artifact                       |                             Recorded size |
| -------------------------------------- | ----------------------------------------: |
| Answer-free PLE envelope               |                               1,091 bytes |
| Full PLE attempt                       |                                 977 bytes |
| One-attempt page                       |                               1,007 bytes |
| Current MC submit body                 |                                  59 bytes |
| Current submission receipt             |                               1,039 bytes |
| Current run-screen JSON bodies         | 3,757 bytes over at least seven responses |
| Two-choice private WeBWorK render form |                                 310 bytes |
| Two-choice private WeBWorK grade form  |                                 334 bytes |
| Recorded private WeBWorK JSON response |                               1,221 bytes |

These figures describe test fixtures, not production percentiles or permanent limits. Real prompt
HTML and media can be much larger. PLE permits a bounded PG source whose private base64 form can dwarf
the learner's answer JSON.

### Where latency matters

The end-to-end submission path contains:

1. browser-to-PLE network round trip;
2. authentication and bounded JSON parsing;
3. RLS-visible attempt, run, question, and idempotency reads;
4. presentation and response validation;
5. native grading or private PLE-to-WeBWorK round trip and execution;
6. atomic persistence and summary updates; and
7. compact receipt serialization and browser return.

Removing a roughly 20-byte `kind` field improves clarity but is not a meaningful latency
optimization by itself. The higher-value actions are:

- collapse the run-screen request fan-out into one projection;
- cache immutable assets and safe renders;
- avoid inline binary media;
- remove the normal extra WeBWorK rerender;
- keep database access bounded and indexed; and
- instrument stage timing before selecting further optimizations.

WP-P6 records representative payload sizes and p50/p95 stage times. It does not turn the fixture byte
counts or arbitrary latency thresholds into permanent tests.

## Caching and prefetch

### Safe caching

PLE may cache public render data by immutable version, seed, and the presentation binding. Cache
entries may contain only the answer-free envelope, sanitized markup, public asset references, and
renderer identity needed for provenance. They must not contain correct answers, private rubrics,
credentials, session keys, source archives, or raw provider responses.

Assets use immutable logical URLs and content checksums. The browser can fetch and cache them
independently. The presentation becomes submission-ready only after required response controls and
assets report their documented readiness; the digest does not prove an image decoded.

### Safe prefetch

PLE can prepare one next question while the learner works on the current question when policy allows
it. The server, not the browser:

- chooses the next assignment position and fresh seed;
- creates the tenant/learner/run/predecessor-bound reservation;
- resolves source and backend;
- renders and stores the public presentation binding; and
- promotes the reservation atomically only after the predecessor commits.

For untimed mastery and practice work, the browser may receive the next answer-free envelope early
and warm a bounded set of same-origin assets. For timed or exam policy, PLE may pre-render privately
but must not reveal the next envelope until the current attempt commits. Prefetch never grades,
starts the next timer, or lets the browser choose a seed, backend, or source.

## Security properties

The security boundary is:

- TLS and same-origin browser transport;
- authenticated HttpOnly session;
- tenant RLS and owned run/attempt lookup;
- immutable version, seed, timing, and backend binding;
- strict schema-selected answer decoding;
- attempt lifecycle checks;
- idempotency and atomic commit; and
- server-only grading data and provider credentials.

CRC16 and the presentation digest add useful consistency evidence. They do not replace any item in
that list. A malicious authenticated learner can see every public choice and can submit any valid
rendered ID; secrecy of distractor IDs is neither expected nor required. Correctness remains known
only to the server-side grader.

## Implementation packages

The active decision defines six dispatchable packages. This summary makes the durable architecture
easy to navigate without duplicating its exact migration and codec specification.

### WP-P1: Contract codec

- Owner: Rust implementation, independently reviewed for Wasm parity.
- Files: question-model presentation and learner descriptors, response wire types, Wasm exports,
  generated browser types, and fixed vectors.
- Behavior: implement canonical descriptor bytes, SHA-256 digest, CRC16 item IDs, collision retry,
  and all eight minimal answer shapes.
- Success: Rust and Wasm vectors agree byte-for-byte; meaningful descriptor changes alter the digest;
  duplicate rendered IDs trigger bounded nonce retry and fail closed after eight attempts.
- Validation: focused Rust/Wasm/vector tests, formatter, strict Clippy, generated-binding freshness,
  and independent Wasm review.

### WP-P2: Persistent binding

- Owner: PostgreSQL and Store implementation.
- Files: `2026080908_secure_question_grading_payloads.sql`, Store traits, Memory/PostgreSQL stores,
  prefetch promotion, retention, backup/restore, and conformance tests.
- Behavior: persist the presentation version, nonce, digest, request-contract version, prefetch
  binding, and bounded private WeBWorK replay state under forced RLS.
- Success: constraints reject malformed data; foreign tenants cannot read or write it; Memory and
  PostgreSQL agree; retention and restore preserve or remove bindings correctly.
- Validation: fresh/no-op migration, malformed-version tests, forced-RLS tests, Store parity,
  retention, backup/restore, and independent PostgreSQL review.

### WP-P3: Native API cutover

- Owner: server and native grading implementation.
- Files: run-screen and submission projections, native backend validation, route tests, API fixtures,
  and generated client contracts.
- Behavior: serve one minimal learner screen, decode type-free answers after attempt load, verify
  digest and idempotency before grading, and return compact receipts.
- Success: every family accepts its exact shape and rejects extras; exact replay returns the first
  receipt; changed replay conflicts before grading; mismatch does not mutate; no raw attempt or
  provenance crosses the active learner route.
- Validation: focused Axum and security tests, family wire vectors, native regression, and independent
  server review.

### WP-P4: WeBWorK replay

- Owner: WebWork adapter and server backend, independently security reviewed.
- Files: renderer contract, WebWork backend, replay-state Store API, request-count tests, and private
  live test.
- Behavior: persist the safe issued mapping and make normal grading one private RPC; permit one
  validated self-heal rerender only when replay state is missing.
- Success: normal, retry, recovery, and mismatch traces prove exact call behavior and prove no source,
  credential, session key, upstream mapping, or raw result reaches browser-visible state.
- Validation: recorded upstream contract tests, private-container trace, state scans, and security
  review.

### WP-P5: Browser recovery

- Owner: SolidJS browser implementation with HCI review.
- Files: API decoders/query owner, run page, attempt state, response widgets, Wasm bridge, and
  Playwright scenarios.
- Behavior: consume the single learner screen, compute/verify through Wasm, gate required asset
  readiness, submit the compact body, and recover compatible drafts after same-attempt refresh.
- Success: keyboard-only paths for all accepted families pass; a network trace contains no submission
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
- rendered-ID membership, role, collision retry, and fail-closed issuance;
- presentation mismatch causes no grade or mutation;
- exact idempotent replay and changed-replay conflict;
- learner-screen and receipt allowlists;
- prefetch promotion and timed-content withholding;
- normal one-call WeBWorK grading; and
- browser traces exclude private material.

One-time implementation evidence records:

- fixture byte sizes;
- request-wave counts during the old-to-new comparison;
- p50/p95 latency observations;
- temporary database query plans used to justify an index; and
- migration rehearsal diagnostics already enforced by the final behavior.

Do not preserve exact fixture sizes, arbitrary response counts, incidental function names, temporary
probes, or latency thresholds as permanent tests. The permanent-test decision follows
[PYTEST_STYLE.md](PYTEST_STYLE.md): retain deterministic public behavior and security invariants;
remove rebuild-only evidence once the maintained gate proves the final contract.

## Rollout decision

The public contract changes atomically before WP-RC5 adds new flat families. PLE must not maintain a
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
is never deleted or recreated as a shortcut. File upload and external-tool submission contracts stay
out of this v1 cutover because they require separate object-transfer and broker designs.

## Final decisions

- Keep `kind` in the render schema; remove it from the v1 learner answer.
- Use one authenticated attempt ID as submission identity; do not resend question context.
- Give every addressable rendered object a four-hex CRC16 ID unique within its presentation.
- Keep durable internal IDs independent from browser-facing rendered IDs.
- Use SHA-256 for whole-presentation consistency and CRC16 for compact item correspondence.
- Keep all correctness, component scoring, and partial credit server-owned.
- Return one minimal learner screen and one compact, policy-projected receipt.
- Cache only answer-free public renders and immutable assets.
- Prefetch only through server-owned reservations and withhold timed content.
- Keep the official WeBWorK exchange private and reduce normal grading from two RPCs to one through
  bounded server-only replay state.
- Optimize request fan-out, assets, database work, and renderer execution before shaving already tiny
  answer JSON.
