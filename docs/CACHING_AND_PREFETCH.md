# Caching and prefetch

## Status and scope

Caching in PLE reduces repeated work; it does not create an alternate source of
assessment truth. Published content, the authoritative attempt, timing,
grading, feedback, exact CourseId/Student ownership, and promotion remain
server-owned. This document records the durable cache and prefetch boundary implemented across
the object store, adapters, Assignment Attempt screen routes, and browser. It distinguishes current
behavior from planned optimization so a cache hit is never mistaken for a
permission or grading decision.

The related contracts are [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md),
[DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md), and
[SECURITY_MODEL.md](SECURITY_MODEL.md). The delivery order and open work remain
in [implementation_plan.md](active_plans/implementation_plan.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Cache ownership

PLE uses distinct caches with deliberately different contents and lifetimes.

| Layer                            | May contain                                                                                                                                  | Key or binding                                                                                                                          | Never contains                                                                                           |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Browser Assignment Attempt state | Current authoritative screen and one speculative Question Presentation                                                                       | Current route and active Assignment Attempt                                                                                             | Answers, grade keys, durable prefetch reservation                                                        |
| Browser asset cache              | Delivered image and other asset bytes                                                                                                        | Delivery URL and content checksum                                                                                                       | Private source or a signed protected URL retained by PLE                                                 |
| CDN public assets                | Public immutable `QuestionAsset` renditions in `PublicAssets`                                                                                | Typed immutable public Object Address and checksum                                                                                      | `PrivateContent`, `StudentRecords`, Question Source archives, restricted assets, renders, or Answer Keys |
| Adapter render cache             | Schema version, Source Object Reference, Source Object Checksum, rendered answer-free `QuestionVariationPresentation`, and renderer identity | Immutable Question Revision Reference and Question Seed                                                                                 | Answer Keys, private rubrics, credentials, raw Question Backend output                                   |
| Attempt and prefetch rows        | Question Attempt Reproduction Details, binding, and private replay state where needed                                                        | Exact CourseId, StudentRecordId, AssignmentAttemptId, predecessor/attempt, position, and Question Revision Reference plus Question Seed | A browser-writable substitute for the attempt record                                                     |

The browser treats every API JSON response as `Cache-Control: no-store`.
This includes Assignment Attempt screens, submissions, prefetch responses, feedback, and
protected asset delivery responses. It keeps a successfully decoded prefetched Question Presentation
in memory only, never in `localStorage` or `sessionStorage`.

Public immutable asset delivery is the exception. `GET /api/assets/{id}` can
resolve only a `Ready` Question Library `QuestionAsset` in the physically separate
`PublicAssets` domain, then redirects to its CDN URL with
`Cache-Control: public, max-age=31536000, immutable` and a checksum ETag. A
`Pending` record, restricted asset, and nonexistent delivery ID are all
non-deliverable on that GET path. The route does not accept an Object Address or
list a bucket.

Protected assets use `POST /api/assets/{id}/delivery`, not GET. The request is
same-origin/session authenticated, reauthorizes and audits the exact typed
record, then returns a bounded signed URL with `no-store`, `Pragma: no-cache`,
and `Referrer-Policy: no-referrer`. The URL must be used as a transient
image/download source, never retained as a reusable browser cache entry.

## Immutable render keys

An adapter render is reusable only when it is a pure, safe projection of an
immutable published Question Revision and its stored Question Seed. Its Object Address is
`QuestionRender { question_revision, question_seed, object }`; it lives in
`PrivateContent`, and the object identity is
deterministically derived with an adapter-specific SHA-256 domain separator.
The typed key therefore includes the Question Revision even where the compact object ID
is derived from Question Revision and Question Seed. Cache identity never includes a Student, course membership, session,
response, deadline, or browser input. The shared safe render cache is not a record cache and cannot
authorize a Student.

The deterministic cache rule relies on the exact seeded-generation contract:
the same `(question_id, revision_number, question_seed)` must reproduce the same canonical output. A changed
Question Source creates a new immutable Question Revision under the existing
Question ID when the change remains compatible, or a new Published Question
lineage when it is a substantive fork. Changed generation behavior or renderer
compatibility likewise creates new exact revision evidence rather than deleting
or overwriting an existing cache entry. A lineage-metadata edit such as Question
Title or Question Description changes no Question Revision or render key. A
changed object is refused by its
checksum, typed key, schema, Source Object Reference binding, Question Revision, Question Seed, Question Title,
and backend-specific validation.

This gives cache invalidation a simple rule:

- Never mutate an existing published render or asset cache entry.
- Publish a new immutable Question Revision for a compatible source or behavior change.
- Treat an invalid, missing, checksum-mismatched, or Source Object Reference-mismatched
  entry as a refusal or a safe cache miss, not as content that may be served.
- Do not use a cache result to bypass authorization, attempt lifecycle checks,
  server timing, response validation, or grading.

Object storage is a cache backing store, not a browser authorization path.
`QuestionRender` objects are render-category objects and cannot be converted to
a public asset URL by the asset route.

### Authorization and RLS

The safe render cache is global immutable content. A cache hit grants no access to a Student record
and cannot satisfy an Assignment Attempt, Question Attempt, or Assignment check. Protected Question Attempt, replay, and prefetch rows
use forced RLS and an operation-specific predicate over the server-derived Account plus exact
`CourseId`, `StudentRecordId`, `AssignmentAttemptId`, `QuestionAttemptId`, `QuestionRevisionReference`, and Question Seed. A missing authenticated Account
context, an absent Student relationship, a revoked membership, or a mismatch in any binding returns
no protected row. A worker obtains the same target from a locked typed lease; it never accepts a
course, Student, attempt, or reference from queue input.

## Adapter behavior

### PLE questions

PLE questions generate an answer-free Question Presentation at issue time. A
presentation-bearing attempt retains that exact public snapshot and matching
server-only Question Grading Input; submit and submitted reads validate those
persisted artifacts rather than recomputing a renderer output. PLE Question Implementations
without a public Question Presentation remain explicitly `NotApplicable`.

### WeBWorK

The WeBWorK adapter stores a cache object containing the answer-free typed
`QuestionVariationPresentation`, published Source Object Reference and Source
Object Checksum binding, and renderer identity. It validates those stored
fields before serving it and records a
non-sensitive `ple.webwork.cache` `renderer_call` or `cache_hit` witness for
adapter cache work. The raw PG source, renderer password, upstream URL, hidden
fields, field/value mapping, raw RPC response, and grading result are excluded.

There are two different issue-time WeBWorK reuse cases without a public Question Presentation:

1. `reproduce` reads the safe cache and does not need a renderer call when an
   explicit active workflow without a public Question Presentation needs it. It is not a submission,
   receipt-replay, or submitted-attempt delivery path.
2. A current `issue` cache hit rereads the safe cache but also re-renders once
   to capture and verify a fresh private replay mapping for the newly issued
   attempt. It compares the reproduced safe output to the immutable cached
   output before accepting the mapping.

The second call remains necessary for each newly issued attempt because the
shared cache deliberately excludes private Question Attempt Reproduction Details. PLE persists the
bounded, validated mapping under the exact CourseId/StudentRecordId/AssignmentAttemptId/AttemptId and immutable
Question Revision Reference plus Question Seed, along with the exact public snapshot and server-only Question Grading Input.
Every normal active or
submitted attempt `GET` replays that persisted snapshot directly: it does not
call adapter `reproduce`, consult the adapter safe-render cache, call the
renderer, or emit `ple.webwork.cache` `renderer_call` or `cache_hit`. Normal
grading reads the same attempt-bound artifacts and makes one private grade
RPC; it neither rerenders nor repairs missing replay state. Missing or
mismatched state fails question-locally and closed. Do not place replay
mappings in the public render cache; they are server-only Question Grading Input.

### iMathAS

The iMathAS adapter uses the same immutable `QuestionRender` shape for an
answer-free iMathAS Question Backend presentation. It validates the pinned Source Object Reference,
iMathAS Question Backend configuration, `imathas_remote_grading_v1` profile, Question Revision, Question Seed, and response shape on every
read. A cache miss asks the configured verified backend for a safe render;
an `AlreadyExists` write race rereads and validates the winning immutable
object. iMathAS Result verification remains a server-to-server operation bound to exact
CourseId/StudentRecordId/AssignmentAttemptId/AttemptId, immutable Question Revision Reference, Question Seed, and server correlation. It has
no process-local grade cache.

Question Backend metadata is retained only as iMathAS protocol data. iMathAS Deployment Reference,
the `imathas_remote_grading_v1` profile, renderer identities, upstream handles, and field/value mappings may be sent on the private
PLE-to-iMathAS exchange, but cannot establish CourseId, StudentRecordId, Assignment Attempt, authorization, or
cache identity. Raw iMathAS responses, credentials, Answer Keys, and Question
Grading Input remain server-only.

## Reservation and promotion

Next-question prefetch is an issuance preparation protocol, not an early
attempt. The browser sends an empty same-origin `POST` to
`/api/courses/{course}/assignments/{assignment}/attempts/{predecessor}/prefetch-next`;
the path supplies routing context only, and the browser cannot choose a Question Seed, Assignment position,
Question Revision, Question Backend, Question Attempt Reproduction Details, or timer.

The server authenticates the Student, resolves the exact Student through the
CourseId membership, verifies ownership of the unresolved predecessor and Assignment Attempt,
rejects a second active question, selects the first unattempted Assignment
position, chooses a fresh Question Seed, issues the backend projection, creates a
presentation binding, and persists a key-free reservation. The reservation
binds CourseId, StudentRecordId, AssignmentAttemptId, predecessor QuestionAttemptId, position,
immutable QuestionRevisionReference, Question Seed, parameter hash, complete Question Attempt Reproduction Details,
explicit presentation capability, presentation binding, exact answer-free public
snapshot, and matching server-only Question Grading Input. A matching request returns
the stored reservation; a conflicting request cannot rewrite its immutable Question Variation.

The reservation's server-only Question Backend grading contracts and Question Attempt Reproduction Details are not a browser capability. The Store keeps
Question Backend-specific grading contracts and replay mappings behind the server-owned typed capability, or derives
them from a locked worker lease whose target is the same exact `CourseId`, `StudentRecordId`, `AssignmentAttemptId`,
predecessor `QuestionAttemptId`, `QuestionRevisionReference`, and Question Seed. No caller-supplied scope or Question Backend
metadata can widen that lease.

PLE Question JSON and WeBWorK reservations additionally retain their typed,
checksummed first-grade contracts. PLE Question JSON carries its private Question Grading Input;
WeBWorK carries its private Question Grading Input and replay mapping. Promotion refuses a
missing or mismatched required contract, so submit never consults a current
published Question, grader, or renderer to recreate it.

No `QuestionAttemptId`, response, grade, or timer exists for a reservation.
Only successful submission of the predecessor promotes the exact reservation
into the next attempt and records either an immutable `nextIssued` descriptor
or durable `nextPending` receipt state. A repeat Question Submission returns
that stored state; it must not scan later Assignment Attempt state and invent a different
successor. Initial recovery alone may heal the one committed-but-unlinked
predecessor caused by an interrupted process.

The client accepts a speculative Question Presentation only when the committed receipt and
prefetch descriptor exactly agree on predecessor, Assignment Attempt, assignment position,
Question Revision, Question Seed, and backend-owned rendered hash. On mismatch, late completion,
network failure, route teardown, or any decode failure, it discards the
speculative data and reloads the authoritative Assignment Attempt screen. Route teardown
aborts the outstanding prefetch request.

## Privacy and assessment policy

Prefetch may prepare server-side work whenever needed, but early Student
disclosure is a policy decision. Untimed mastery and practice may deliver one
answer-free next Question Presentation and warm its assets. Timed or exam work may render
privately but must not reveal the next Question Presentation until the current attempt has
committed. Prefetch never starts the next timer, grades an answer, or changes
completion.

The current route creates and returns a safe Question Presentation after its ownership and
lifecycle checks; it does not itself branch on timing or exam policy. Therefore
the plan's timed/exam withholding rule is an implementation requirement, not
evidence that all current route configurations enforce it. Until the policy
gate is present, callers must expose this route only for modes explicitly
allowed to reveal the next question early.

Asset warming is likewise bounded and conservative. The browser extracts only
same-origin image asset IDs from the prefetched Question Presentation, deduplicates them,
and warms at most 12 with `credentials: "same-origin"` and `cache:
"force-cache"`. It does not warm arbitrary URLs, embed binary bytes in the
Question Presentation, or persist a speculative asset list.

## Refusal and recovery

The following outcomes are intentional safety behavior:

| Condition                                                                                                                   | Required behavior                                                                              |
| --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Another AccountId, another course, or a foreign attempt/predecessor                                                         | Return not found or conflict; do not disclose state                                            |
| Missing or mismatched CourseId/StudentRecordId/Assignment Attempt/attempt/Question Revision Reference/Question Seed binding | Refuse before cache, grading, or mutation                                                      |
| Active predecessor already answered or Assignment Attempt completed                                                         | Reject prefetch; do not start a successor                                                      |
| Conflicting duplicate reservation                                                                                           | Preserve the first reservation and reject rewrite                                              |
| Cache schema, checksum, Question Source, Question Revision, Question Seed, Question Title, or renderer mismatch             | Refuse the entry; re-render only where the adapter contract permits                            |
| WeBWorK replay state missing                                                                                                | Refuse question-locally; receipt-era attempts have no rerender or self-heal compatibility path |
| Prefetch descriptor differs from receipt                                                                                    | Drop browser memory and use the ordinary Assignment Attempt screen route                       |
| Renderer or Question Backend outage                                                                                         | Do not substitute a new question or guess a grade; surface the backend-local failure           |
| Protected asset delivery                                                                                                    | Authorize and audit every request; do not place the signed URL in a reusable cache             |

## Observability and future work

Measure meaningful work before reducing JSON fields by a few bytes. The
relevant stages are browser-to-PLE time, route authorization and Store access,
PLE issue or adapter cache lookup, PLE-to-Question-Backend/renderer time, grading,
promotion and persistence, asset transfer, and return to the browser. Record
bounded aggregate latency and hit/miss/error counts without attempt IDs,
responses, asset URLs, Question Backend payloads, or answer-bearing content.

Current WeBWorK adapter-cache witnesses intentionally expose only
`renderer_call` and `cache_hit`; persisted attempt-snapshot reads emit neither.
Future operational metrics should preserve that low-cardinality, non-sensitive
approach while adding p50/p95 stage timing, cache validation refusals, prefetch
reservations/promotions/mismatches, and bounded asset-warm outcomes.
Representative payload sizes and latency measurements belong to the acceptance-evidence record rather
than fragile exact-byte permanent tests.

The next cache work should follow the payload plan in this order:

1. Complete attempt-bound presentation and replay persistence before relying
   on cache hits for WeBWorK issuance latency.
2. Enforce the timed/exam prefetch disclosure policy at the route boundary.
3. Replace broad Student DTOs with the minimal screen, answer, and receipt
   projections while retaining complete server-side Question Attempt Reproduction Details.
4. Add aggregate observability and evaluate cache warming from measured
   latency, not assumed payload savings.

Permanent tests should prove deterministic cache identity, validation refusal,
no-answer disclosure, cache-hit renderer behavior, matching reservation-repeat behavior,
atomic promotion, strict receipt matching, timed-content withholding, and
cross-user, cross-course, and foreign-attempt refusal. One-time load tests and representative timing
measurements
are implementation evidence, not permanent exact-performance assertions.
