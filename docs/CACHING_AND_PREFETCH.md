# Caching

## Status and scope

Caching in PLE reduces repeated work; it never creates assessment truth or authorization. Published
content, Assignment Attempts, saved responses, timing, grading, feedback, and exact Course and
Student ownership remain server-owned.

The former browser next-Question prefetch, reservation, promotion, `nextIssued`, and `nextPending`
design is retired. The current Assignment Attempt starts with its issued Question set and presents
one position at a time. Responses are saved to the active Attempt, and recovery is server-owned
auto-submission of that durable saved state at expiry. No cache entry submits, grades, or recovers a
Student response.

Related authorities are [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md),
[DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md), and
[SECURITY_MODEL.md](SECURITY_MODEL.md).

## Cache ownership

| Layer                        | May contain                                                                                           | Key or binding                                                                     | Never contains                                                                                           |
| ---------------------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Browser route memory         | Current decoded, answer-free Assignment Attempt screen state                                          | Current route and active Assignment Attempt                                        | Answers, grading authority, durable Student Work, or an offline submission queue                         |
| Browser asset cache          | Delivered image and other asset bytes                                                                 | Delivery URL and content checksum                                                  | Private source or a signed protected URL retained by PLE                                                 |
| CDN public assets            | Public immutable `QuestionAsset` renditions in `PublicAssets`                                         | Typed immutable public Object Address and checksum                                 | `PrivateContent`, `StudentRecords`, Question Source archives, restricted assets, renders, or Answer Keys |
| iMathAS render cache         | Schema version, Source Object Reference and checksum, answer-free presentation, and renderer identity | Immutable Question Revision Reference and Question Seed                            | Answers, private rubrics, credentials, raw Question Backend output, Student records, or grading results  |
| Attempt presentation records | Retained Question Attempt Reproduction Details, presentation bindings, and backend-owned documents    | Exact Assignment Attempt, position, Question Revision Reference, and Question Seed | A browser-writable substitute for the Attempt or a speculative successor                                 |

Browser API JSON is `Cache-Control: no-store`. That includes Assignment Attempt context, progress,
presentations, saved-response acknowledgements, finalization results, feedback, and
protected asset delivery. Route memory may improve rendering but is disposable and never replaces
PostgreSQL Student Work.

## Asset delivery

Public immutable asset delivery can redirect only a `Ready` Question Library `QuestionAsset` from
the physically separate `PublicAssets` domain. Its CDN response may use long-lived immutable caching
and a checksum ETag. A pending record, restricted asset, nonexistent delivery ID, or checksum
mismatch is not deliverable.

Protected assets use a same-origin authenticated operation that reauthorizes and audits the exact
typed record before returning a bounded signed URL with `no-store`, `Pragma: no-cache`, and
`Referrer-Policy: no-referrer`. The signed URL is transient and is never retained as a reusable PLE
cache entry.

## Immutable render keys

An iMathAS render is reusable only when it is a pure, answer-free projection of an immutable
Published Question Revision and stored Question Seed. Its typed `QuestionRender` Object Address
includes that revision and seed and lives in `PrivateContent`. The deterministic object identity uses
an adapter-specific SHA-256 domain separator.

Cache identity never includes a Student, Course Membership, session, response, deadline, or browser
input. The same Question Revision and Question Seed must reproduce the same canonical output. A
source or behavior change creates new immutable revision evidence; PLE never overwrites an existing
render or asset cache entry.

An invalid, missing, schema-mismatched, checksum-mismatched, or source-mismatched entry is a refusal
or safe cache miss. It cannot bypass authorization, Attempt lifecycle checks, server timing,
response validation, or grading.

## Adapter behavior

### PLE Question Backend

PLE Questions generate an answer-free Question Presentation at issue time. The Assignment Attempt
retains the exact public presentation binding and matching server-only Question Grading Input. Saved
response validation and automatic grading use those retained artifacts, not a current Published
Question or speculative cache entry.

### WeBWorK

WeBWorK has no shared browser render cache. At issue, the adapter calls the configured renderer and
PLE persists the exact backend-owned document with the Question Attempt. An authorized document read
returns that stored document and does not rerender it.

Saving captures the bounded opaque form payload. Whole-Attempt finalization sends that immutable
accepted payload with the retained source, seed, and backend document to WeBWorK, which returns
the credit fraction PLE records. A narrow background pass uses the same finalization path only for
an abandoned expired Attempt or a backend that explicitly needs deferred completion. No WeBWorK
field mapping, replay state, or cache entry becomes browser authority.

### iMathAS

The iMathAS adapter may reuse the immutable answer-free `QuestionRender` object after validating its
pinned Source Object Reference, source checksum, profile, Question Revision, Question Seed, renderer
identity, and response shape. An `AlreadyExists` write race rereads and validates the winning
immutable object.

iMathAS result verification remains a server-to-server operation bound to exact Course, Student,
Assignment Attempt, Question Attempt, immutable Question Revision, Question Seed, and server
correlation. Raw responses, credentials, Answer Keys, and Question Grading Input stay server-only.

## Refusal behavior

| Condition                                                                    | Required behavior                                                                            |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Foreign Account, Course, Student Record, or Assignment Attempt               | Return not found or conflict without disclosing protected state                              |
| Mismatched Assignment Attempt, position, Question Revision, or Question Seed | Refuse before cache delivery, response persistence, grading, or mutation                     |
| Cache schema, checksum, source, revision, seed, or renderer mismatch         | Refuse the entry; rerender only where the adapter contract permits                           |
| WeBWorK backend document or saved opaque response is missing                 | Refuse question-locally; do not rerender or synthesize a response                            |
| Renderer or Question Backend is unavailable                                  | Do not substitute a Question or guess a grade; expose only the bounded backend-local failure |
| Protected asset delivery is not authorized                                   | Return no signed URL and disclose no object-store address                                    |

## Observability and future work

Measure browser-to-PLE time, authorization and Store access, adapter render/cache lookup, private
Question Backend time, grading, persistence, asset transfer, and browser completion. Metrics remain
bounded, aggregate, low-cardinality, and free of responses, answer-bearing content, Student identity,
signed URLs, raw provider payloads, Job IDs, and lease tokens.

Future cache work begins from measured latency. Permanent tests prove deterministic cache identity,
validation refusal, answer secrecy, and cross-user/cross-course denial. Representative byte and
timing measurements are one-time evidence rather than exact-performance permanent assertions.
