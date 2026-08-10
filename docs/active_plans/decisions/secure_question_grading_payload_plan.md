# Plan: Secure question grading payloads

## Context

PLE keeps answer keys, grading logic, source archives, and renderer credentials on the server. The
current learner boundary is nevertheless broader than needed: learner routes expose a full
`QuestionAttempt` projection while submission contains a tagged `StudentResponse`. The attempt is
already the authoritative record for learner, run, immutable version, seed, timing, backend, and
feedback policy. This plan makes that authority explicit before WP-RC5 expands flat-question
families. It is a prerequisite for WP-RC5, not a dependency of WP-RC3 live acceptance.

Current local evidence was rechecked on 2026-08-10 from
`crates/question_model/src/{envelope,response,activity}.rs`, the extracted server run modules,
`crates/adapters/webwork`, and the browser API and run-page modules. A one-time measurement of the
current generated peptide-bond fixture found a 1,091-byte envelope, a 977-byte full attempt, a
1,007-byte one-attempt page, a 59-byte MC submit body, and a 1,039-byte submission receipt. The
current run-screen loader transfers at least seven JSON responses totaling 3,757 body bytes over
four dependent request waves before asset bodies or HTTP headers. These are diagnostic fixture
measurements, not permanent exact-size tests or production percentiles.

### Current PLE payload inventory

Current issuance separates the answer-free `QuestionEnvelope` from grading data, but the learner run
route also returns the persistence-oriented `QuestionAttempt`. The envelope's immutable `version`,
server-issued `seed`, title, prompt blocks, public asset IDs and checksums, presented choices, and
public response schema are legitimate render or consistency inputs. The response `kind` remains
necessary there because it selects the widget. Durable choice IDs, numeric tolerance, and short-text
matching mode are not render inputs; the v1 public schema replaces them with rendered IDs and only
widget constraints. Tolerances, normalization, answer keys, weights, and partial-credit rubrics stay
server-only.

The active browser does not need the attempt's tenant, problem, parameter hash, prior response,
adapter/renderer/generator identities, source-artifact object identity, asset object identities,
grading implementation, or full provenance. It needs the attempt ID, learner-visible deadline, the
presentation digest, and the answer-free envelope. Course and assignment shell identity and run
number are needed once for navigation and context, but their full resource records and policies are
not.

Today `getRunScreen` obtains the run; obtains enrollment and cursor-pages full attempts; obtains the
full assignment; then obtains course, appearance, and issued envelope. The normal fixture path is
seven GETs in four sequential waves, and cursor pagination or crash recovery can add more. This
browser-side resource assembly duplicates relationship checks the server already owns and delays
first render. V1 replaces it with one purpose-built learner run-screen projection.

Current submission already uses authenticated `POST /api/submissions/{attemptId}` with an
`Idempotency-Key`. A small MC request resembles
`{"response":{"kind":"multipleChoice","selected":["amide"]}}`. After loading the attempt, the
server already knows its learner, tenant, course, assignment, version, seed, timing state, expected
response family, and grading backend. Therefore the submission `kind`, question ID, version, seed,
backend, and course/assignment IDs are redundant learner assertions; v1 removes them rather than
trusting or cross-checking them. The internal Rust and browser draft models remain tagged. Only the
public answer wire becomes type-free.

The current receipt returns another full `QuestionAttempt`, including persistence and provenance
fields the browser already received, plus feedback and next-attempt data. V1 returns only the
committed attempt ID, policy-permitted outcome/feedback, and a minimal next descriptor. In the same
fixture, an illustrative recorded-only compact receipt is 125 bytes rather than 1,039 bytes.

| Current field group                                                               | Needed by active browser? | Decision                                                                                                |
| --------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------- |
| Envelope `version`, `seed`, title, prompt, public asset references/checksums      | Yes                       | Keep once in the public presentation. Fetch asset bytes separately.                                     |
| Response/content-block `kind` and public widget constraints                       | Yes                       | Keep as render discriminants. These are not learner claims.                                             |
| Durable choice/slot IDs                                                           | No                        | Replace on the wire with presentation-scoped four-hex rendered IDs.                                     |
| Numeric tolerance, text match mode, answer/rubric/weights                         | No                        | Keep in the server grading definition; expose only input constraints and displayed unit.                |
| Attempt ID and effective deadline                                                 | Yes                       | Keep in the minimal active descriptor.                                                                  |
| Attempt tenant/problem/parameter hash/status/result/provenance/implementation IDs | No                        | Remove from active learner routes and receipts.                                                         |
| Course/assignment IDs, course theme, run number/mode                              | Yes                       | Return once in the consolidated run-screen shell.                                                       |
| Full enrollment, assignment items/policies, course role/tenant                    | No                        | Resolve server-side; use separate authorized resource routes where another screen genuinely needs them. |

Current prefetch is a bodyless, authenticated request for the next unattempted position. The server
chooses the fresh seed, creates a tenant/actor/run/predecessor-bound durable reservation, renders it,
and returns only its answer-free public envelope. The browser cannot choose a source, backend, seed,
or grading state. This plan retains that server authority and narrows when a prepared envelope may be
revealed for timed work.

Local LibreTexts ADAPT is useful comparison evidence, not a compatibility target. Its student view
retrieves rich assignment/question records and its native submission carries `assignment_id`,
`question_id`, serialized `submission`, client-selected `technology`, and currently also constructs
client `max_score`. ADAPT's server reads stored QTI JSON to infer `questionType` and computes native
partial scores. Simple choice submits one identifier, but matching submits the full mutated
`termsToMatch` objects. Its WeBWorK route uses an iframe and signed/encrypted problem/answer JWT
exchange, where rich renderer score/answer objects cross the browser-facing workflow before an
unconfirmed result is confirmed. The inspected ADAPT code has no server-issued attempt binding or
canonical presentation digest. PLE adopts answer stripping, server grading, and deterministic
generation; it intentionally avoids browser-selected technology, resource-ID-only authority,
whole-assignment learner payloads, and public upstream result transport.

| Boundary      | LibreTexts ADAPT observed behavior                                                                                                                                  | PLE v1 decision                                                                                                        |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Native render | Fetches rich whole-assignment question records with seeded, student-sanitized `qti_json`, prior response, timing, media, and iframe state.                          | Fetch one active learner screen plus the public envelope; keep full attempt/provenance server-side.                    |
| Native submit | Browser sends `assignment_id`, `question_id`, `technology`, and serialized answer. It does not send question type.                                                  | Browser path carries only attempt ID; body carries digest and schema-selected answer.                                  |
| Native grade  | Server loads stored `qti_json`, infers `questionType`, and grades partial components server-side.                                                                   | Server loads immutable attempt definition/backend, infers rich internal type, and grades server-side.                  |
| Binding       | No attempt ID, version binding, presentation checksum, or ETag was found in the inspected route.                                                                    | Session, RLS-visible attempt, idempotency key, immutable binding, rendered IDs, and descriptor digest bind the answer. |
| WeBWorK       | Signed/encrypted `problemJWT` iframe produces signed `answerJWT`, an unconfirmed result, then browser confirmation; rich score/answer objects traverse the browser. | PLE privately calls the renderer. Browser submits only PLE answer data; renderer result and mapping stay server-side.  |
| Adopt         | Answer stripping, deterministic seed, server grading, and protected external context.                                                                               | Preserve these properties.                                                                                             |
| Avoid         | Whole-assignment mandatory payload, client technology, resource-ID-only versionless binding, provider result through browser, and iframe-origin heuristics.         | Keep an attempt-bound, small explicit contract.                                                                        |

The byte figures above are fixtures rather than production percentiles. WP-P6 measures the complete
path: browser-to-PLE round trip; bounded parse/validation; database read; native grading; PLE-to-
WeBWorK round trip; WeBWorK execution; persistence; and response serialization. Asset delivery,
network RTT, database work, and private renderer execution are expected to matter more than a few
JSON bytes; measurements, not this expectation, decide later operational tuning.

A recorded two-choice WeBWorK contract fixture measured a 310-byte private render form, a 334-byte
private grade form, a 1,221-byte upstream JSON response, a 400-byte answer-free public envelope, and
an 89-byte current browser submission. Real PG source is the meaningful transfer variable: PLE
permits at most 256 KiB of raw source, whose base64 form is about 341 KiB before form escaping. The
browser never sends that source. On a cache hit, a question GET makes no private renderer call;
current grading performs a same-seed rerender plus the grade call. WP-P4 replaces that normal two-call
grade path with bounded server-only replay state and one private grade call. WP-P6 records p50/p95
stage times and representative sizes before making an optimization claim.

### Responsiveness priorities

| Stage                        | Current observation                                                                    | V1 action                                                                                                      |
| ---------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Initial active render        | At least seven browser GETs in four dependent waves before asset bodies.               | One learner-screen GET; fetch independently cacheable asset bytes by logical asset URL.                        |
| Native submit                | One small browser request; parsing and native grading are local to PLE.                | Keep one request; do not optimize away clear field names for single-digit byte savings.                        |
| WeBWorK submit               | PLE performs a same-seed private rerender and then a private grade RPC.                | Persist bounded mapping at issue and make one normal grade RPC.                                                |
| Immutable source/cache reads | WeBWorK source and safe render cache are server-side; materiality is not yet measured. | Instrument first; consider bounded immutable-object caching only if evidence warrants it.                      |
| Images and other assets      | Binary media can dwarf JSON and requires independent decode/readiness.                 | Keep IDs/checksums in JSON, immutable HTTP caching, bounded one-question prefetch, and no base64 inline media. |
| Result                       | Current receipt repeats a full attempt/provenance projection.                          | Return a compact policy-projected receipt and next descriptor.                                                 |

### Review decisions

| Question                                                    | Decision                                                                                                                                            |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Does render `kind` remain?                                  | Yes. Response-schema and content-block discriminants are required to choose safe browser components.                                                |
| Does submission `kind` remain?                              | No. The RLS-visible attempt and reproduced response schema select the strict answer decoder.                                                        |
| Are four-character CRC16 item IDs accepted?                 | Yes. They are attempt-presentation-scoped routing/consistency IDs, globally unique within one question presentation, with nonce retry on collision. |
| Does CRC16 replace security or the full digest?             | No. Session, attempt ownership, lifecycle, RLS, idempotency, and SHA-256 descriptor binding remain authoritative.                                   |
| Is smaller answer JSON the main latency target?             | No. The answer remains under roughly 100 bytes; run-screen request fan-out and private WeBWorK execution are material.                              |
| Does the browser receive full attempts or renderer results? | No. Full attempts remain server/store history and support evidence; private renderer state and grading results stay server-side.                    |

## Objectives

- Replace the current seven-request active run-screen fan-out with one minimal learner projection.
- Make `QuestionAttemptId`, authenticated session, and `Idempotency-Key` the only grading
  authority supplied by the learner.
- Send a compact answer containing only an issued rendered-object ID or lexical response plus a
  whole-presentation digest.
- Give every selectable rendered object a four-lowercase-hex, attempt-presentation-scoped
  `RenderedItemIdV1` without weakening durable internal identities.
- Retain a SHA-256 whole-presentation digest for render-state consistency that item IDs cannot
  establish alone.
- Persist enough server-only WeBWorK state to make normal grade one private RPC.
- Ship an atomic, measured contract cutover with deterministic Rust/Wasm evidence.

## Design philosophy

This plan applies **Fix the design, not the symptom**, **Long-term over short-term**, and **Design
for adaptability**. The server derives rich typed state from the attempt once; the browser sends
only the learner's choice or text. CRC16 is deliberately a compact rendered-object consistency and
routing identifier, never a security boundary. The session, forced RLS, attempt lifecycle, digest,
and idempotency record remain the security boundary.

The plan uses one Rust-owned binary codec exported to Wasm. TypeScript calls that codec and never
reimplements canonicalization, CRC16, or SHA-256. A future incompatible field layout requires a
new version; it cannot silently reinterpret v1.

## Scope

- Implement `LearnerRunScreenV1`, `PresentationDescriptorV1`, rendered IDs, minimal attempt
  descriptors, type-free wire decoding, compact receipts, and browser recovery.
- Add the `2026080908_secure_question_grading_payloads.sql` prerequisite migration and the exact
  attempt/prefetch/replay-state bindings below.
- Cut native flat questions and current WeBWorK MC to API v1 atomically.
- Add deterministic vectors, Store/RLS tests, private-renderer request-count tests, browser traces,
  and redacted stage instrumentation.

## Non-goals

- File-upload and external-tool response payloads are out of v1 because neither is an accepted
  WP-RC5 family; each needs its own object-transfer or broker contract.
- The digest and CRC do not authenticate a browser, replace TLS, prove a pixel painted, prove an
  image decoded, or prevent a malicious authenticated learner from submitting an allowed answer.
- Current v1 flat questions and RC3 WeBWorK MC do not gain partial-credit rules here. Future
  server-only rubrics require no wire change.
- Production numeric latency SLOs are not invented before representative pilot evidence. Complete
  instrumentation and gates are the decision for this version.

## Contract decisions

### Authority and projections

The route path `POST /api/submissions/{attemptId}`, authenticated session, and bounded
`Idempotency-Key` identify a submission. The browser sends no question, course, version, seed,
backend, submission `kind`, points, component score, or weight. The handler performs these steps in
order: authenticate and bound the top-level body; RLS-load the attempt and owning run; load the
immutable question and reproduce its exact public response schema; verify the stored and submitted
presentation digest; strictly decode `answer` through that schema; create the typed canonical
request hash; check idempotent replay; then, only for a new request, enforce lifecycle/timing, grade,
and commit once. An exact retry therefore returns its recorded receipt before a closed-attempt error,
while a changed retry conflicts before grading.

`GET /api/runs/{run}/screen` replaces browser assembly of resource endpoints. Its
`LearnerRunScreenV1` is exactly the course and assignment IDs needed for navigation, the selected
course theme, learner-visible run number/mode, the active attempt ID/deadline/digest, and the public
envelope:

```json
{
  "scope": { "course": "...", "assignment": "...", "theme": "grass" },
  "run": { "number": 4, "mode": "practice" },
  "attempt": { "id": "...", "deadline": null, "presentationDigest": "pd1_..." },
  "envelope": {
    "version": "...",
    "seed": 0,
    "presentationNonce": "...",
    "...": "public render only"
  }
}
```

The run ID is already in the request path. Tenant, enrollment, problem, assignment policies, full
course/assignment records, attempt status/provenance, and duplicate version/seed are absent. A
completed run uses the existing policy-redacted summary route rather than manufacturing an active
attempt. Learner history uses the bounded run summary and never receives raw `QuestionAttempt`
provenance. V1 retires raw full-attempt learner JSON routes; support evidence remains in the
server/store audit boundary rather than creating an unneeded browser projection. An illustrative
conversion of the current fixture is about 1,559 JSON body bytes in one response instead of 3,757
bytes across seven responses. That estimate is one-time design evidence, not a contract size test.

`PresentationDescriptorV1` does not include `AttemptId`; it is reusable for a privately prepared
next attempt. The server binds its resulting full digest to the `QuestionAttemptId` only when it
promotes that prepared attempt. A prefetch with no attempt returns
`{ "presentationDigest":"pd1_...", "envelope":{...} }`; promotion and the later receipt require
that exact digest before reuse. This permits safe deterministic prefetch without pretending the
descriptor is an authorization token.

Browser draft recovery keys by `QuestionAttemptId` plus `presentationDigest`; it does not repeat
tenant, run, version, or seed in the storage key. A recovered draft is accepted only when its exact
answer shape and every rendered item ID remain valid for the refreshed same-attempt presentation.

### PresentationDescriptorV1 codec

`PresentationDescriptorV1` is one Rust-owned, tagged, length-prefixed binary codec. Its exact byte
sequence is normative:

1. ASCII domain bytes `ple:presentation:v1\0`.
2. One `u8` descriptor-version tag (`1`).
3. `VersionId` as 16 raw UUID bytes.
4. `seed` as unsigned `u64` big endian.
5. `presentationNonce` as exactly 16 raw bytes.
6. Title, prompt blocks, response schema, ordered `PresentedItemV1` values, logical assets and
   rendition SHA-256 values, and hotspot intrinsic metadata, in that declared order.

Every enum is one closed `u8` tag. Every optional field starts with `u8` `0` or `1`; a present value
immediately follows `1`. Strings are a `u32` big-endian byte length followed by exact UTF-8 bytes.
Vectors are a `u32` big-endian element count followed by ordered encodings. SHA-256 values are 32
raw bytes. UUIDs are 16 raw bytes. Unsigned integers are big endian. Maps and floating-point values
are prohibited. Text is neither Unicode-normalized nor reinterpreted. The public schema represents
hotspot coordinates as integers in a 0..1,000,000 plane. Adding, removing, or reordering any field
requires `PresentationDescriptorV2` and a new domain tag.

`ResponseSchemaV1` has fixed family tags: `singleChoice=0`, `multipleAnswer=1`, `fillIn=2`,
`multiFillIn=3`, `numerical=4`, `matching=5`, `ordering=6`, and `hotspot=7`. It encodes only public
widget constraints, never tolerances or answers: single choice has no extra value; multiple answer
has minimum and maximum selection counts; fill-in has maximum characters; multi-fill has an ordered
vector of blank item ordinals and a maximum-character value per blank; numerical has maximum
characters and optional displayed unit; matching has ordered prompt and choice item ordinals plus a
`u8` choice-reuse flag; ordering has ordered item ordinals; hotspot has the surface ordinal and
minimum/maximum point counts. Every referenced ordinal must resolve to exactly one item with the
required role.

`ContentBlockV1` uses fixed tags and fields: `Text=0` plus Markdown string; `Math=1` plus LaTeX and
description strings; `Image=2` plus 16-byte logical `AssetId`, 32-byte authored checksum, and
description; `Code=3` plus language and source strings; and `Table=4` plus header-string vector,
row-vector-of-string-vectors, and description. Hex JSON checksums are decoded to exactly 32 bytes or
issuance refuses. `AssetBindingV1` is the ordered tuple of logical `AssetId`, authored checksum,
selected rendition checksum, optional intrinsic width, and optional intrinsic height. A hotspot
requires both nonzero dimensions; other assets require both or neither. These declared tags and
orders, rather than Rust enum source order, are the wire contract.

Item construction is deliberately two-stage. `RenderableItemBasisV1` is encoded with the same
primitives and contains, in order: item-role `u8`, ordinal `u32`, optional public label, ordered public
`ContentBlockV1` values, ordered `AssetBindingV1` values, and optional hotspot intrinsic width/height
plus the fixed 0..1,000,000 coordinate-plane tag. It contains neither the durable internal ID nor
`RenderedItemIdV1`. Item-role tags are `choice=0`, `blank=1`, `matchPrompt=2`, `matchChoice=3`,
`orderItem=4`, and `hotspotSurface=5`.

The server hashes the exact `RenderableItemBasisV1` bytes for the CRC input below. After deriving and
globally de-duplicating every rendered ID, it encodes each `PresentedItemV1` in the descriptor as the
two raw CRC bytes followed by the length-prefixed basis bytes. The public JSON exposes those two bytes
as four lowercase hexadecimal characters. Consequently the CRC derivation is not circular, while a
changed rendered ID, role, order, content, asset, or geometry changes the full presentation digest.
Fixed vectors cover every content and item-role tag, plus a deliberately changed rendered ID.

`presentationDigest` is SHA-256 of the complete descriptor bytes. The database stores all 32 bytes;
the public value is the first 16 bytes, base64url without padding, prefixed `pd1_`. Rust exports
descriptor construction and digest verification through Wasm; TypeScript passes typed public values
to Wasm and never creates an alternative JSON or hash encoding. Vectors include empty strings,
embedded NUL/escape bytes, non-ASCII UTF-8, every option state, item ordering, asset changes,
hotspot dimensions, and one mutation of every encoded field. The vectors also prove the CRC test
vector below and byte-identical Rust/Wasm results.

### RenderedItemIdV1

Durable `ChoiceId`, blank, prompt, and ordering IDs remain server-domain identities. A rendered
selectable object additionally receives a browser-facing `RenderedItemIdV1`: exactly four lowercase
ASCII hexadecimal characters. The browser normally displays labels and content, never this code.

For each issued presentation, the server computes CRC-16/CCITT-FALSE over this length-framed binary
input:

1. ASCII domain bytes `ple:rendered-item:v1\0`.
2. The 16-byte `presentationNonce`.
3. `VersionId` as 16 raw UUID bytes.
4. `seed` as unsigned `u64` big endian.
5. One closed item-role `u8` tag.
6. The item ordinal as `u32` big endian.
7. Durable internal-ID bytes as `u32` length plus bytes.
8. SHA-256 of the exact `RenderableItemBasisV1` bytes as 32 raw bytes.

CRC parameters are polynomial `0x1021`, initial value `0xffff`, `refin=false`, `refout=false`, and
`xorout=0`. The required vector is ASCII `123456789` -> `29b1`. Values serialize as four lowercase
hex digits. Issuance derives every rendered ID across the whole presentation and requires global
uniqueness, including matching prompts and choices. On any duplicate it mints a fresh OS-random
16-byte nonce and retries the complete presentation; it fails closed after eight attempts. The
stored nonce makes resume and grading deterministic.

CRC16 is accepted because a question has at most a small number of rendered items, collisions are
detected at issuance, and retry is free. Ten independently distributed IDs have 45 possible pairs,
so the approximate pre-check probability of any collision is `45 / 65,536`, or about 1 in 1,456;
the learner never sees a colliding presentation because issuance regenerates the nonce. CRC16
complements rather than replaces the SHA-256 digest:
an item ID says which issued object was selected; the full digest detects omitted or reordered
prompt/schema/assets and inconsistent render state. Neither is authentication. Native mappings are
deterministically reproduced from immutable version, seed, and nonce. WeBWorK persists only its
validated rendered-ID to upstream field/value mapping.

### Exact learner wire

`QuestionAttemptId` appears once in `POST /api/submissions/{attemptId}`. Version, problem,
assignment, learner, and durable choice UUIDs are not repeated in the body. The canonical UUID route
spelling costs 36 ASCII characters; selectable objects use four-character `RenderedItemIdV1`
values instead. Changing the one route identifier to a denser encoding would save only a few bytes
and is not a v1 latency target.

The API v1 body is:

```json
{ "presentationDigest": "pd1_...", "answer": { "...": "definition-selected shape" } }
```

The explicit answer object is intentional. A scalar could save a handful of characters for MC but
would make the family contracts less legible and harder to extend. In the current fixture the old MC
body is 59 bytes; an illustrative v1 body with a full 128-bit public presentation digest and a
four-character item ID is 80 bytes. The request becomes slightly larger because it adds a useful
whole-render consistency assertion. It remains operationally tiny; the performance gains come from
removing broad projections/request waves and one private WeBWorK call.

The server knows the answer family before parsing `answer`. It accepts only the following shapes:

| Family            | `answer` wire value                                                         |
| ----------------- | --------------------------------------------------------------------------- |
| MC and WeBWorK MC | `{ "selected": "4ef3" }`                                                    |
| MA                | `{ "selected": ["4ef3", "91c2"] }`                                          |
| FIB               | `{ "text": "as typed" }`                                                    |
| MULTI-FIB         | `{ "blanks": [{"slot":"4ef3","text":"..."}] }` in issued order              |
| NUM               | `{ "text": "1.25e-3" }`                                                     |
| MATCH             | `{ "matches": [{"prompt":"12a4","choice":"ef32"}] }` in issued prompt order |
| ORDER             | `{ "order": ["91c2", "4ef3"] }`                                             |
| HOTSPOT           | `{ "surface":"4ef3", "points":[{"x":512,"y":233}] }`                        |

FIB and NUM carry text because each has one target. Matching carries IDs for both rendered sides;
ordering carries the ordered rendered IDs; multi-FIB carries rendered slot IDs; hotspot carries the
rendered surface ID and unsigned normalized coordinates. It never exposes IDs or geometry for the
correct hotspot regions. Bounded parsing rejects duplicate IDs, unknown properties, invalid hex,
wrong order/cardinality, out-of-schema IDs, and coordinates outside the issued schema. The browser
may check shape but never grades. The internal `StudentResponse` remains a rich tagged enum after
this decoder; `kind` is absent from the public wire.

The browser never sends component weights or scores. Current flat v1 and RC3 WeBWorK MC grade
all-or-nothing according to their existing policy. A later partial-credit rubric reads the same
server-loaded definition and returns a policy-projected aggregate result without a new answer shape.
The compact receipt is `{ accepted, attempt, outcome, feedback, next }`; `attempt` is the one attempt
ID, not a full attempt record. `outcome` is either a policy-permitted aggregate or
`{ "state":"recorded" }`. `next`, when present, contains only the next attempt ID, deadline, and
presentation digest. The browser reuses its matching prefetched envelope; if none is available it
loads the consolidated run screen. Deferred feedback reveals neither component scores nor correct
answers. The response never echoes the submitted answer because the browser already owns it.

### Idempotency and consistency recovery

| Signal                             | Detects                                                                                                                                               | Does not establish                                                 |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `RenderedItemIdV1` membership/role | Selection from another presentation, stale or wrong-order choice mapping, wrong blank/match side, unknown surface, and malformed item correspondence. | Authentication, correctness, or whether pixels were visible.       |
| `presentationDigest`               | Stale or mixed envelope state; changed prompt/schema/item order/content/assets/geometry; wrong cached version/seed/nonce.                             | Transport security, browser integrity, or successful media decode. |
| Asset checksum plus readiness      | Wrong asset bytes, failed required asset load, and missing hotspot intrinsic dimensions.                                                              | Semantic correctness of the authored image or human perception.    |

CRC membership failure is an invalid answer shape and never reaches grading. A digest mismatch is a
recoverable same-attempt state problem unless server reproduction disagrees with its own persisted
descriptor, which is recorded as a server defect and fails closed.

After bounded parse and before idempotency replay lookup, the server creates `request_sha256` from
the typed canonical pair `(presentationDigest, answer)` under request-contract version 1. The same
idempotency key and identical hash returns the original compact receipt. The same key with a
different hash returns `409 idempotency_key_reused` before a backend call. Contract version 0 is
historical; v1 is new issuance.

The browser recomputes the full descriptor with Wasm before enabling Submit. Its independent asset
readiness gate waits for required assets and known hotspot intrinsic dimensions. A local digest
mismatch disables submit, preserves the editable answer, and offers same-attempt refresh. A server
mismatch returns `409 presentation_mismatch`, performs no grade or mutation, and logs only attempt
ID, expected/received digest prefixes, client build, descriptor version, and time. The browser
refetches the same attempt, restores only schema-compatible draft values, requires review/resubmit,
and never silently changes seed or issues another attempt.

If the Rust/Wasm bridge is unavailable, TypeScript does not reimplement the descriptor codec. The
degraded path sends `{ "presentationDigest":"pd1_...", "envelope":{...} }` once to
`POST /api/attempts/{attemptId}/validate-presentation`; the server returns only
`{ "matches":true }` or the same `409 presentation_mismatch` before enabling the ordinary compact
submission. The body is the already-public envelope, bounded by the normal envelope limit. That rare
extra round trip is preferable to divergent hashing code and does not change grading authority. It
is not used on the healthy path and is measured separately.

### Native and WeBWorK grading

Native grading receives the typed internal response resolved from `QuestionAttemptId`; its answer
key never crosses the learner boundary. WeBWorK issuance privately renders immutable source using
server credentials, projects the answer-free envelope, verifies that projection, then persists a
bounded replay record. Normal grade converts `RenderedItemIdV1` through that record and makes one
private authenticated `render_rpc` grade call. Browser-to-PLE remains the compact PLE body above.

`webwork_grade_replay_state` contains no source text, credentials, session key, correct answer, or
raw upstream response. A missing record may perform one verified private rerender and recreate
state only after every binding validates; a mismatch or second recovery refuses before grading.
This is more private than ADAPT's browser-mediated JWT answer exchange while still allowing the
renderer to receive the source and protected course credential it requires.

The current HTTP adapter grades through two official `render_rpc` calls: a same-seed render recovers
and validates the upstream radio field/value mapping, then a grade call resends source, seed, private
course credentials, the selected upstream value, and `WWsubmit=1`. The public render cache stores
only the answer-free envelope and sanitized markup, so it correctly cannot supply that private
mapping. WP-P4 records the validated mapping at issuance and removes the first normal grade call.
The official renderer remains stateless, so source and private credentials still cross the private
PLE-to-WeBWorK boundary once per grade. A new opaque upstream render-handle service would be a
different maintained protocol and is out of v1. V1 also does not add an in-process source-byte cache:
one object-store read is acceptable for correctness, and no evidence currently shows it is a
material stage. If WP-P6 later proves otherwise, that evidence owns a separate bounded cache package
keyed and revalidated by immutable object ID/SHA.

### Prefetch and caching

The server always reserves and pre-renders a next question privately. The strict
`PrefetchedQuestionPayloadV1` record contains `descriptorVersion`, the 16-byte nonce, the 32-byte
digest, answer-free envelope, immutable version/seed/parameter binding, and answer-free provenance.
Its Rust decoder denies unknown fields and validates those fields against both the envelope and the
physical `question_prefetch` binding columns before any promotion. Promotion copies the physical
version/nonce/digest to the new attempt, binds the full digest to that attempt, and refuses any
missing, legacy, malformed, or payload-versus-column mismatch without issuing an attempt. Browser
full-envelope prefetch is permitted only when
`TimingPolicy::Untimed`; `PerQuestion` and `PerAttempt` remain server-only until the predecessor
commits. This protects timed/exam exposure without a new policy or later decision. Immutable public
assets and non-answer render artifacts may cache by version/seed/nonce; private WeBWorK replay state
is separate from public render cache and attempt-bound.

## Migration and persistence

WP-P2 owns forward-only `schemas/migrations/2026080908_secure_question_grading_payloads.sql`.
It executes before WP-RC5. The migration adds nullable historical-compatible columns to
`question_attempt`: `presentation_descriptor_version smallint`, `presentation_nonce bytea` with
length 16, and `presentation_digest bytea` with length 32, plus one check requiring all three null
or all three present and a second check requiring a present descriptor version to equal `1`. The
Store decoder also refuses any non-null version other than `1`, with a malformed-version SQL and
Memory/PostgreSQL parity test. Because the cutover gate requires zero prefetch rows,
`question_prefetch` gains
`presentation_descriptor_version smallint NOT NULL CHECK (presentation_descriptor_version = 1)`,
`presentation_nonce bytea NOT NULL CHECK (octet_length(presentation_nonce) = 16)`, and
`presentation_digest bytea NOT NULL CHECK (octet_length(presentation_digest) = 32)`. Its payload is
strict `PrefetchedQuestionPayloadV1`; Store insertion and promotion compare its version/nonce/digest,
immutable version/seed/parameter binding, envelope, and provenance to the columns and expected
predecessor before write or promotion. Memory/PostgreSQL parity tests prove missing, malformed, and
mismatched payload/column combinations refuse without creating an attempt.

It renames `submission_idempotency.response_sha256` to `request_sha256` and adds
`request_contract_version smallint NOT NULL DEFAULT 0 CHECK (request_contract_version IN (0,1))`.
The rename preserves the existing `character(64)` hash bytes. The migration adds the version column
with default `0`, thereby backfilling every historical row, then enforces the check and `NOT NULL`.
All v1 writers explicitly insert `1`; they never rely on the historical default. The same atomic
release migration removes the default only after the zero-active-attempt gate and before traffic is
enabled, so an unversioned later writer fails closed. Contract `0` retains historical replay
interpretation; contract `1` means the canonical typed request digest defined above, not a raw JSON
hash. Tests replay a pre-migration row unchanged and prove a v1 same-key/different-digest request
returns `409` before grading.

It creates capability-specific `webwork_grade_replay_state` with `tenant_id`, `attempt_id`,
`attempt_occurred_at`, `course_id`, immutable problem/version/`source_object_id` and SHA-256, `seed`,
renderer ID/version, presentation digest, `state_version`, bounded mapping JSONB, mapping SHA-256,
and `created_at`. `(tenant_id, attempt_id, attempt_occurred_at)` is a composite foreign key to the
partitioned `question_attempt` and uses `ON DELETE CASCADE`; `course_id` is server-bound and foreign
key constrained. Its primary key is `(tenant_id, attempt_id, attempt_occurred_at)` and its retention
lookup index is `(tenant_id, course_id, created_at)`. Mapping JSONB is at most 32,768 bytes and v1
permits at most 32 choices, group/name values at most 128 bytes, and each upstream value at most
512 bytes. It represents only rendered ID to upstream field/value data and contains no source,
credential, session key, correct answer, or raw response.

The table has forced tenant RLS, the existing `ple_bind_course_from_attempt` course-binding function,
the existing `ple_fence_learner_record_write` retention/write fence, and backup/restore coverage.
`ple_app` receives exactly `SELECT`, `INSERT`, and `DELETE`; it receives no `UPDATE`, so state is
immutable and is replaced only by delete-plus-insert after exact binding validation. No
application-level encryption is added: the record contains no credential, answer key, source, or
correct answer. Database and object-storage encryption, forced RLS, narrow grants, and encrypted
backups are its at-rest boundary. Successful or terminal submission deletes replay state atomically;
retention cascade deletes it with its attempt.

The ledger reserves the remaining uncreated migrations in this order:

1. `2026080908_secure_question_grading_payloads.sql` (WP-P2 prerequisite).
2. `2026080909_object_reconciliation.sql` (WP-RC7).
3. `2026080910_oidc_identity.sql` (WP-RC8).
4. `2026080911_lti_advantage.sql` (WP-RC9).

WP-RC7 schema work starts only after WP-P2; its non-schema object inventory work may proceed in
parallel. Accepted prior migrations are never renamed.

## Milestone plan

| Milestone | Workstreams  | Outcome                                          | Parallel-plan ready                                                        |
| --------- | ------------ | ------------------------------------------------ | -------------------------------------------------------------------------- |
| M1        | WP-P1, WP-P2 | Codec, IDs, descriptor bindings, Store state     | Yes; model/Wasm and migration have separate owners after contract handoff. |
| M2        | WP-P3, WP-P4 | Atomic native and WeBWorK server cutover         | Yes; both consume accepted M1 interfaces.                                  |
| M3        | WP-P5, WP-P6 | Browser recovery, measurements, closure evidence | Yes; browser and evidence documentation own distinct files.                |

### Work package WP-P1: Contracts and codec

- Owner: `rust-code-expert`, reviewed by `wasm-rust-expert`.
- Depends on: none.
- Files: new question-model presentation/learner descriptor modules, response model, Wasm export,
  generated API types, and fixed vector fixtures/tests.
- Behavior: implement the normative codecs, ID generator, global collision retry, public descriptors,
  and type-free wire types.
- Success: Rust and Wasm produce byte-identical vectors; every meaningful descriptor mutation changes
  the digest; collision injection proves new-nonce retry and eight-attempt fail-closed behavior.
- Validation: focused Rust/Wasm/vector tests, strict Clippy, generated-binding freshness, and fresh
  independent Wasm review.

### Work package WP-P2: Persistent bindings

- Owner: `postgresql-expert`.
- Depends on: WP-P1 contract types.
- Files: `2026080908_secure_question_grading_payloads.sql`, Store interface, Memory/PostgreSQL
  implementations, prefetch promotion, retention/backup/restore owners, and conformance tests.
- Behavior: store descriptor binding and private replay state exactly as specified above.
- Success: RLS refuses foreign tenant access, constraints reject malformed bindings/state, retention
  cascades cleanly, and Memory/PostgreSQL conformance agrees.
- Validation: fresh/no-op migration, forced-RLS, Store parity, replay bounds, backup/restore, and
  independent PostgreSQL review.

### Work package WP-P3: Native issuance and submission

- Owner: `rust-code-expert`.
- Depends on: WP-P1 and WP-P2.
- Files: the server run facade and new learner-screen/submission projection modules, native
  backend/validation modules, route tests, API fixtures, and generated client contracts.
- Behavior: serve one `GET /api/runs/{run}/screen` projection, issue minimal descriptors, decode
  type-free answers after attempt load, enforce digest and exact idempotency, grade once, and return
  compact policy-projected receipts.
- Success: all eight wire vectors reject invalid shape/ID, exact retry returns receipt, changed retry
  is `409` before grade, mismatch is non-mutating, and the active learner route exposes no raw
  `QuestionAttempt` or provenance.
- Validation: focused Axum/security tests, native MC regression, all family vectors, and fresh server
  security review.

### Work package WP-P4: One-call WeBWorK grade

- Owner: `rust-code-expert`, independently security reviewed.
- Depends on: WP-P1, WP-P2, and accepted WP-RC3.
- Files: WebWork adapter/renderer contract, server backend, Store replay API, request-count tests,
  and private live test.
- Behavior: persist issued mapping and make normal grade one private RPC; allow only one fully
  validated rerender self-heal.
- Success: normal, retry, self-heal, and mismatch traces prove request count and no private material
  enters browser/cache/receipt/replay state.
- Validation: recorded upstream contract tests, private-container trace, state scans, and independent
  security review.

### Work package WP-P5: Browser recovery

- Owner: `solid-js-expert`, reviewed by `human-interact-expert`.
- Depends on: WP-P1 and WP-P3.
- Files: browser API/decoders/query owner, run page, attempt state, response widgets, Wasm bridge,
  and Playwright scenarios.
- Behavior: consume the single learner-screen projection, compute digest through Wasm, gate asset
  readiness, submit the compact body, and restore compatible drafts after same-attempt mismatch
  refresh. The browser no longer pages attempts or joins enrollment/assignment/course records to
  discover the active question.
- Success: keyboard-only MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, HOTSPOT, and WeBWorK MC flows
  pass; network traces contain no private fields or submission `kind`; initial active render uses
  one learner-screen request before separate asset fetches.
- Validation: built-browser Playwright, accessibility review, stale mismatch and offline retry tests.

### Work package WP-P6: Measurement and closure

- Owner: `integrator`.
- Depends on: WP-P3, WP-P4, and WP-P5.
- Files: project tools, E2E traces, payload evidence report, contracts/architecture/file-map/usage,
  status and changelog updates.
- Behavior: record redacted bytes and p50/p95 durations for browser round trip, PLE validation,
  database, native grading, private WeBWorK round trip/execution, persistence, and response.
- Success: evidence states actual fixture and representative measurements, identifies meaningful
  latency stages, and explicitly records no numerical SLO until representative pilot evidence.
- Validation: reproducible local-stack collection, full repository gate, and independent docs review.

## Cutover and verification

The public cutover is atomic. A pre-production maintenance gate disables new issuance, permits
old-contract active runs to finish on the old release, and requires zero active attempts and zero
prefetch rows before migration. It does not close or delete learner records. Then it applies the
migration and deploys server and browser together. Historical learner records remain available only
through bounded history/summary projections; raw full-attempt persistence DTOs do not remain public.
Old active route/body usage returns stable `410 contract_retired`. Local synthetic stacks may be
explicitly recreated; production data is never recreated as a shortcut.

- Per-patch: focused owner tests, formatter/linter/type checks, generated bindings, Markdown/ASCII/
  whitespace checks, `git diff --check`, and `git diff --cached --check`.
- Contract: codec vectors cover ASCII, escapes, non-ASCII, options, ordering, CRC vector, rendered
  ID collision retry, and every semantic descriptor mutation.
- Persistence: fresh/no-op migration, constraints, Memory/PostgreSQL parity, forced-RLS, retention,
  and backup/restore coverage pass.
- Integration: all-family native wires, idempotency ordering, mismatch non-mutation, compact receipt,
  and one-call normal WeBWorK grading pass.
- Browser: built application proves keyboard access, asset readiness, stale recovery, and a capture
  proving only digest/answer body plus path attempt ID and idempotency header are sent.
- Permanent tests protect semantic behavior: strict family decoding selected by the attempt,
  CRC collision retry/fail-closed issuance, digest mismatch non-mutation, idempotent replay, compact
  projection allowlists, prefetch promotion, and one-call normal WeBWorK grading. They do not assert
  fixture byte totals, exact request counts for unrelated shell resources, or arbitrary latency
  thresholds.
- One-time acceptance evidence records representative JSON/form/media bytes, run-screen request
  waves, and p50/p95 stage timings in the payload evidence report. Those diagnostic measurements do
  not remain as brittle permanent tests unless they enforce an explicit security or resource bound.
- Closure: independent Rust/security, PostgreSQL, and HCI reviewers find no unresolved P0/P1 before
  WP-RC5 starts.

## Risk register

| Risk                     | Trigger                                     | Owner    | Control                                                                   |
| ------------------------ | ------------------------------------------- | -------- | ------------------------------------------------------------------------- |
| Codec divergence         | Vector mismatch                             | WP-P1    | Rust owns codec; Wasm is the only browser implementation.                 |
| CRC duplicate            | Issuance detects duplicate                  | WP-P1    | Global uniqueness, OS nonce retry, bounded fail-closed issue.             |
| Render disagreement      | Digest mismatch                             | WP-P3/P5 | Same-attempt recovery, preserved draft, no grade/mutation.                |
| Replay private-data leak | State scan finds prohibited data            | WP-P2/P4 | Narrow record, bounds, RLS, grants, and security review.                  |
| Timed-content exposure   | Envelope arrives early                      | WP-P5    | Server-only pre-render for timed policies; Untimed-only browser prefetch. |
| False latency claim      | Metrics attribute speed to JSON bytes alone | WP-P6    | Stage measurements and no numeric SLO before pilot evidence.              |

## Documentation close-out

WP-P6 updates this plan, `docs/active_plans/active/release_completion_plan.md`,
`docs/active_plans/implementation_status.md`, `docs/DATABASE_STRUCTURE.md`, `docs/CONTRACTS.md`,
`docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, `docs/USAGE.md`, and `docs/CHANGELOG.md`.
It writes `docs/active_plans/reports/secure_question_grading_payload_evidence.md` with commands,
redacted timings, migration result, and independent-review findings. The plan is complete only when
these named artifacts and all listed gates are present; no additional policy decision is needed.
