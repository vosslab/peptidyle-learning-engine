# Caching and prefetch

## Status and scope

Caching in PLE reduces repeated work; it does not create an alternate source of
assessment truth. Published content, the authoritative attempt, timing,
grading, feedback, exact CourseId/Student ownership, and promotion remain
server-owned. This document records the durable cache and prefetch boundary implemented across
the object store, adapters, run routes, and browser. It distinguishes current
behavior from planned optimization so a cache hit is never mistaken for a
permission or grading decision.

The related contracts are [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md),
[DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md), and
[SECURITY_MODEL.md](SECURITY_MODEL.md). The delivery order and open work remain
in [implementation_plan.md](active_plans/implementation_plan.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Cache ownership

PLE uses distinct caches with deliberately different contents and lifetimes.

| Layer                     | May contain                                                                           | Key or binding                                                                                                              | Never contains                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Browser run state         | Current authoritative screen and one speculative envelope                             | Current route and active attempt                                                                                            | Answers, grade keys, durable prefetch reservation                                                   |
| Browser asset cache       | Delivered image and other asset bytes                                                 | Delivery URL and content checksum                                                                                           | Private source or a signed protected URL retained by PLE                                            |
| CDN public assets         | Public immutable `QuestionAsset` renditions in `PublicAssets`                         | Typed immutable public Object Address and checksum                                                                              | `PrivateContent`, `StudentRecords`, source archives, restricted assets, renders, or answer material |
| Adapter render cache      | Answer-free envelope, safe markup, source binding, renderer identity                  | Immutable QuestionRevisionReference and seed                                                                                 | Answer keys, private rubrics, credentials, raw provider output                                      |
| Attempt and prefetch rows | Question Attempt Reproduction Details, binding, and private replay state where needed | Exact CourseId, StudentRecordId, AssignmentAttemptId, predecessor/attempt, position, and QuestionRevisionReference plus seed | A browser-writable substitute for the attempt record                                                |

The browser treats every API JSON response as `Cache-Control: no-store`.
This includes run screens, submissions, prefetch responses, feedback, and
protected asset delivery responses. It keeps a successfully decoded prefetched envelope
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
immutable published version and its stored seed. Its Object Address is
`QuestionRender { question_revision, seed, object }`; it lives in
`PrivateContent`, and the object identity is
deterministically derived with an adapter-specific SHA-256 domain separator.
The typed key therefore includes the problem even where the compact object ID
is derived from version and seed. Cache identity never includes a Student, course membership, session,
response, deadline, or browser input. The shared safe render cache is not a record cache and cannot
authorize a Student.

The deterministic cache rule relies on the exact seeded-generation contract:
the same `(question_id, revision_number, seed)` must reproduce the same canonical output. A new
generation behavior, source revision, renderer compatibility version, or
authored edit requires a new immutable published question with a fresh Question
ID and hidden exact evidence rather than cache deletion or overwriting an
existing entry. A changed object is refused by its
checksum, typed key, schema, Source Object Reference binding, version, seed, title,
and backend-specific validation.

This gives cache invalidation a simple rule:

- Never mutate an existing published render or asset cache entry.
- Publish a new immutable question for a content or behavior change.
- Treat an invalid, missing, checksum-mismatched, or provenance-mismatched
  entry as a refusal or a safe cache miss, not as content that may be served.
- Do not use a cache result to bypass authorization, attempt lifecycle checks,
  server timing, response validation, or grading.

Object storage is a cache backing store, not a browser authorization path.
`ProblemRender` objects are render-category objects and cannot be converted to
a public asset URL by the asset route.

### Authorization and RLS

The safe render cache is global immutable content. A cache hit grants no access to a Student record
and cannot satisfy a run, attempt, or assignment check. Protected attempt, replay, and prefetch rows
use forced RLS and an operation-specific predicate over the server-derived Account plus exact
`CourseId`, `StudentRecordId`, `AssignmentAttemptId`, `QuestionAttemptId`, `QuestionRevisionReference`, and seed. A missing authenticated Account
context, an absent Student relationship, a revoked membership, or a mismatch in any binding returns
no protected row. A worker obtains the same target from a locked typed lease; it never accepts a
course, Student, attempt, or reference from queue input.

## Adapter behavior

### PLE questions

PLE questions generate an answer-free envelope at issue time. A
presentation-bearing attempt retains that exact public snapshot and matching
server-only grading envelope; submit and submitted reads validate those
persisted artifacts rather than recomputing a renderer output. PLE Question Implementations
without an envelope remain explicitly `NotApplicable`.

### WeBWorK

The WeBWorK adapter stores a safe cache object containing the answer-free
envelope, sanitized HTML, published Source Object Reference binding, and renderer
identity. It validates all of those fields before serving it and records a
non-sensitive `ple.webwork.cache` `renderer_call` or `cache_hit` witness for
adapter cache work. The raw PG source, renderer password, upstream URL, hidden
fields, field/value mapping, raw RPC response, and grading result are excluded.

There are two different issue-time or envelope-less WeBWorK reuse cases:

1. `reproduce` reads the safe cache and does not need a renderer call when an
   explicit active, envelope-less workflow needs it. It is not a submission,
   receipt-replay, or submitted-attempt delivery path.
2. A current `issue` cache hit rereads the safe cache but also re-renders once
   to capture and verify a fresh private replay mapping for the newly issued
   attempt. It compares the reproduced safe output to the immutable cached
   output before accepting the mapping.

The second call remains necessary for each newly issued attempt because the
shared cache deliberately excludes private replay material. PLE persists the
bounded, validated mapping under the exact CourseId/StudentRecordId/AssignmentAttemptId/AttemptId and immutable
QuestionRevisionReference plus seed, along with the exact public snapshot and server-only grading envelope.
Every normal active or
submitted attempt `GET` replays that persisted snapshot directly: it does not
call adapter `reproduce`, consult the adapter safe-render cache, call the
renderer, or emit `ple.webwork.cache` `renderer_call` or `cache_hit`. Normal
grading reads the same attempt-bound artifacts and makes one private grade
RPC; it neither rerenders nor repairs missing replay state. Missing or
mismatched state fails question-locally and closed. Do not place replay
mappings in the public render cache; they are server-only Question Grading Input.

### iMathAS

The iMathAS adapter uses the same immutable `ProblemRender` shape for an
answer-free external-tool envelope. It validates the pinned Source Object Reference,
provider, integration profile, version, seed, and response shape on every
read. A cache miss asks the configured verified provider for a safe render;
an `AlreadyExists` write race rereads and validates the winning immutable
object. Grade verification remains a server-to-provider operation bound to exact
CourseId/StudentRecordId/AssignmentAttemptId/AttemptId, immutable QuestionRevisionReference, seed, and server correlation. It has
no process-local grade cache.

Provider metadata is retained only as external protocol data. Provider names, integration profiles,
renderer identities, upstream handles, and field/value mappings may be sent on the private
PLE-to-provider exchange, but cannot establish CourseId, StudentRecordId, Assignment Attempt, authorization, or
cache identity. Raw provider responses, credentials, and answer-bearing material remain server-only.

## Reservation and promotion

Next-question prefetch is an issuance preparation protocol, not an early
attempt. The browser sends an empty same-origin `POST` to
`/api/courses/{course}/assignments/{assignment}/attempts/{predecessor}/prefetch-next`;
the path supplies routing context only, and the browser cannot choose a seed, question position,
version, backend, provenance, or timer.

The server authenticates the Student, resolves the exact Student through the
CourseId membership, verifies ownership of the unresolved predecessor and run,
rejects a second active question, selects the first unattempted assignment
position, chooses a fresh seed, issues the backend projection, creates a
presentation binding, and persists a key-free reservation. The reservation
binds CourseId, StudentRecordId, AssignmentAttemptId, predecessor QuestionAttemptId, position,
immutable QuestionRevisionReference, seed, parameter hash, complete backend provenance,
explicit presentation capability, presentation binding, exact answer-free public
snapshot, and matching server-only grading envelope. An identical request is
idempotent; a conflicting request cannot rewrite its immutable variation.

The reservation's private execution material is not a browser capability. The Store keeps
Question Backend-specific grading contracts and replay mappings behind the server-owned typed capability, or derives
them from a locked worker lease whose target is the same exact `CourseId`, `StudentRecordId`, `AssignmentAttemptId`,
predecessor `QuestionAttemptId`, `QuestionRevisionReference`, and seed. No caller-supplied scope or provider
metadata can widen that lease.

PLE Question JSON and WeBWorK reservations additionally retain their typed,
checksummed first-grade contracts. PLE Question JSON carries its private definition;
WeBWorK carries its private definition and replay mapping. Promotion refuses a
missing or mismatched required contract, so submit never consults a current
published Question, grader, or renderer to recreate it.

No `QuestionAttemptId`, response, grade, or timer exists for a reservation.
Only successful submission of the predecessor promotes the exact reservation
into the next attempt and records either an immutable `nextIssued` descriptor
or durable `nextPending` receipt state. An idempotent submission replay returns
that stored state; it must not scan later run state and invent a different
successor. Initial recovery alone may heal the one committed-but-unlinked
predecessor caused by an interrupted process.

The client accepts a speculative envelope only when the committed receipt and
prefetch descriptor exactly agree on predecessor, run, assignment position,
version, seed, and backend-owned rendered hash. On mismatch, late completion,
network failure, route teardown, or any decode failure, it discards the
speculative data and reloads the authoritative run screen. Route teardown
aborts the outstanding prefetch request.

## Privacy and assessment policy

Prefetch may prepare server-side work whenever needed, but early Student
disclosure is a policy decision. Untimed mastery and practice may deliver one
answer-free next envelope and warm its assets. Timed or exam work may render
privately but must not reveal the next envelope until the current attempt has
committed. Prefetch never starts the next timer, grades an answer, or changes
completion.

The current route creates and returns a safe envelope after its ownership and
lifecycle checks; it does not itself branch on timing or exam policy. Therefore
the plan's timed/exam withholding rule is an implementation requirement, not
evidence that all current route configurations enforce it. Until the policy
gate is present, callers must expose this route only for modes explicitly
allowed to reveal the next question early.

Asset warming is likewise bounded and conservative. The browser extracts only
same-origin image asset IDs from the prefetched envelope, deduplicates them,
and warms at most 12 with `credentials: "same-origin"` and `cache:
"force-cache"`. It does not warm arbitrary URLs, embed binary bytes in the
envelope, or persist a speculative asset list.

## Refusal and recovery

The following outcomes are intentional safety behavior:

| Condition                                                                                        | Required behavior                                                                              |
| ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| Another AccountId, another course, or a foreign attempt/predecessor                              | Return not found or conflict; do not disclose state                                            |
| Missing or mismatched CourseId/StudentRecordId/Assignment Attempt/attempt/reference/seed binding | Refuse before cache, grading, or mutation                                                      |
| Active predecessor already answered or run completed                                             | Reject prefetch; do not start a successor                                                      |
| Conflicting duplicate reservation                                                                | Preserve the first reservation and reject rewrite                                              |
| Cache schema, checksum, source, version, seed, title, or renderer mismatch                       | Refuse the entry; re-render only where the adapter contract permits                            |
| WeBWorK replay state missing                                                                     | Refuse question-locally; receipt-era attempts have no rerender or self-heal compatibility path |
| Prefetch descriptor differs from receipt                                                         | Drop browser memory and use the ordinary run-screen route                                      |
| Renderer or provider outage                                                                      | Do not substitute a new question or guess a grade; surface the backend-local failure           |
| Protected asset delivery                                                                         | Authorize and audit every request; do not place the signed URL in a reusable cache             |

## Observability and future work

Measure meaningful work before reducing JSON fields by a few bytes. The
relevant stages are browser-to-PLE time, route authorization and Store access,
PLE issue or adapter cache lookup, PLE-to-provider/renderer time, grading,
promotion and persistence, asset transfer, and return to the browser. Record
bounded aggregate latency and hit/miss/error counts without attempt IDs,
responses, asset URLs, provider payloads, or answer-bearing content.

Current WeBWorK adapter-cache witnesses intentionally expose only
`renderer_call` and `cache_hit`; persisted attempt-snapshot reads emit neither.
Future operational metrics should preserve that low-cardinality, non-sensitive
approach while adding p50/p95 stage timing, cache validation refusals, prefetch
reservations/promotions/mismatches, and bounded asset-warm outcomes.
Representative payload sizes and latency measurements belong to WP-P6 rather
than fragile exact-byte permanent tests.

The next cache work should follow the payload plan in this order:

1. Complete attempt-bound presentation and replay persistence before relying
   on cache hits for WeBWorK issuance latency.
2. Enforce the timed/exam prefetch disclosure policy at the route boundary.
3. Replace broad Student DTOs with the minimal screen, answer, and receipt
   projections while retaining rich server-side provenance.
4. Add aggregate observability and evaluate cache warming from measured
   latency, not assumed payload savings.

Permanent tests should prove deterministic cache identity, validation refusal,
no-answer disclosure, cache-hit renderer behavior, reservation idempotency,
atomic promotion, strict receipt matching, timed-content withholding, and
cross-user, cross-course, and foreign-attempt refusal. One-time load tests and representative timing
measurements
are implementation evidence, not permanent exact-performance assertions.
