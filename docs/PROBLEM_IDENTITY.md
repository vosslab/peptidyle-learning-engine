# Problem identity and lifecycle

This document is the durable map of identities that name questions and their
related records. It answers two easy-to-confuse questions:

1. What durable record is this?
2. Which stable, human-usable Question ID names that exact published question?

The first answer uses typed UUID identities. The second uses one validated
`AAA-BBBB` Crockford Base32 Question ID. Presentation-scoped response IDs are
a third, deliberately temporary contract.

The model lives primarily in `crates/question_model/src/identity.rs`,
`activity.rs`, `catalog.rs`, `definition.rs`, and `presentation/`. The exact
learner wire contract is owned by
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md); this document
does not turn an identifier into authorization or grading authority.

## The maintainer rule

An editable draft is tenant-owned and has neither a `ProblemId` nor a
`VersionId`; successful publication mints a new immutable
`(ProblemId, VersionId)` pair. An attempt pins that published pair, its seed,
and its server-owned provenance.

This is a type boundary, not a `draft: bool` convention. A
`DraftQuestionDefinition` contains a `WorkspaceId` and no published identity.
A `QuestionDefinition` requires both `ProblemId` and `VersionId`. A caller
cannot accidentally pass the draft type to a published-only API without an
explicit publication step.

## Identity domains

All identifiers in the first two tables are distinct Rust newtypes over a
PostgreSQL `uuid`, serialized as canonical 36-character UUID strings at JSON
boundaries. They are not aliases: a function asking for `VersionId` cannot be
given a `ProblemId`, and a `CourseId` cannot be given an `AssignmentId`.

Fresh server-created identities are UUIDv7. Their random portion avoids
sequential enumeration and their time ordering is friendlier to indexes. The
reader and wire contract accept any canonical UUID, including deterministic
fixtures and pre-existing values; the version restriction applies to minting,
not deserialization. Generation is behind the server-enabled `generate`
feature and is absent from the browser/Wasm build.

### Authoring and shared content

| Identifier          | Names                                                  | Ownership and lifetime                                                      |
| ------------------- | ------------------------------------------------------ | --------------------------------------------------------------------------- |
| `WorkspaceId`       | One private instructor authoring workspace             | Tenant-owned, mutable draft boundary                                        |
| `WorkspaceImportId` | One staged import in a workspace                       | Tenant-owned until publication resolves it                                  |
| `ProblemId`         | Internal identity for one immutable published question | Shared immutable-content provenance and retrieval                           |
| `VersionId`         | Internal exact immutable publication evidence          | Shared content; referenced by assignments and attempts                      |
| `AssetId`           | One logical published asset                            | Shared content identity, independent of physical placement                  |
| `ObjectId`          | One immutable object-store record                      | Physical/source/rendition object identity; never a substitute for `AssetId` |

`ProblemVersionRef { problem, version }` is the complete internal durable
reference to immutable published evidence. Assignment items, attempts,
exported manifests, and provenance use that pair rather than resolving a
latest version. It is not an instructor-facing version picker.

`AssetId` answers "which logical asset does content cite?" `ObjectId` answers
"which immutable stored bytes or source artifact reproduce this record?" An
asset can point at an object, but keeping the names separate permits storage
deduplication or rendition replacement without rewriting question content.

### Tenant-owned teaching records

| Identifier                                       | Names                                              | Why it must remain distinct                                                   |
| ------------------------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------- |
| `TenantId`                                       | One RLS and institutional boundary                 | Every educational record carries it directly                                  |
| `CourseId`, `CourseGroupId`                      | A course/section and its current group             | Course membership is not assignment ownership                                 |
| `AssignmentId`                                   | One tenant assignment                              | Defines learning activity and policy, not content                             |
| `AssignmentItemId`, `AssignmentSelectionGroupId` | A stable assignment item or random-selection group | Preserves position and selection semantics when content repeats               |
| `EnrollmentId`                                   | One student's relationship to one assignment       | Keeps repeated runs on one durable educational record                         |
| `RunId`                                          | One pass through an assignment                     | Preserves earlier completed runs while allowing continued practice            |
| `QuestionAttemptId`                              | One issued question in one run                     | Binds an answer to the learner, run, exact version, seed, timing, and backend |
| `StudentId`, `UserId`                            | Pedagogical student and authenticated person       | They may map to the same provider UUID, but are not interchangeable concepts  |

An ID names a record; it does not grant access to it. Authentication, tenant
context, authorization, RLS, lifecycle checks, and server-side ownership checks
decide whether a caller may read or change the record.

### Human-facing Question ID

`QuestionId` is the only catalog identity presented to a person. Its canonical
form is `AAA-BBBB`: six random Crockford Base32 identity characters plus one
server-validated HMAC-SHA256 character. It is non-sequential and copyable. One
Question ID names one immutable published question. Every authored content
change, including an original-owner correction, publishes a new Question ID;
an explicit provenance link may identify its source question. A Question ID is
not authorization evidence and never replaces `ProblemVersionRef` in hidden
storage, attempt provenance, or historical grading.

## Publication and provenance

The lifecycle state machine is a small pure model:

| State        | Published pair present | Discovery | New assignments         | Exact historical resolution |
| ------------ | ---------------------- | --------- | ----------------------- | --------------------------- |
| `Draft`      | No                     | No        | No                      | No                          |
| `Validated`  | No                     | No        | No                      | No                          |
| `Published`  | Yes                    | Yes       | Yes                     | Yes                         |
| `Deprecated` | Yes                    | No        | Yes, by exact reference | Yes                         |
| `Archived`   | Yes                    | No        | No                      | Yes                         |

Only these forward transitions are legal:

| From         | Event                     | To           |
| ------------ | ------------------------- | ------------ |
| `Draft`      | `Validate`                | `Validated`  |
| `Validated`  | `Publish { publication }` | `Published`  |
| `Published`  | `Deprecate { reason }`    | `Deprecated` |
| `Deprecated` | `Archive`                 | `Archived`   |

`question_model::lifecycle::apply` rejects skips, reversals, and empty
deprecation explanations. The pure transition receives the publication pair;
the server publication flow records the immutable payload, scope, authorship,
and provenance atomically. There is no restore transition.

PLE does not support problem drift. A correction or other content change is a
new published question with a new Question ID, not a successor of the existing
Question ID. It may record explicit `derivedFrom` provenance to the exact
source question, preserving attribution and license lineage without granting
the derivative author write access to the source.

Existing assignments and issued runs retain their exact references. An
Instructor may deliberately replace an assignment item only through an
explicit, revision-checked edit; publication, correction, lifecycle work, and
background processing never advance an assignment automatically. This is the
durable no-drift contract. The Store and server keep assignment replacement
separate from publication: a replacement changes the future assignment
definition only after a revision check, while issued snapshots continue to use
their original exact pair. The accepted WP-R2 boundary enforces this contract.

Publication scope is `Institution` or `Public`. It applies only after
publication. There is intentionally no "private published problem" scope:
private editable work remains a tenant-owned draft.

## Attempts: durable authority, not a large request body

`QuestionAttemptId` is the primary identity for an ordinary learner submission.
An issued attempt already binds, server-side:

- authenticated learner and tenant;
- run, assignment position, and policy snapshot;
- exact `ProblemVersionRef` and generated seed;
- deadline and lifecycle/submission state; and
- adapter, renderer, generator, source-object, asset-object, and grading
  provenance necessary for reproducibility.

The compact route `POST /api/submissions/{attemptId}` therefore needs the
attempt UUID once, an idempotency key in the request header, a presentation
consistency token, and the learner's answer. The browser must not resend the
problem, version, assignment, course, seed, backend, grading mode, or a
response `kind` as authority. The server resolves the strict answer decoder
from the issued attempt's response schema.

A UUID's 36-character JSON spelling is not a useful latency target when it is
sent once amid HTTP headers and a render payload. Repeating durable UUIDs for
each selectable item would be wasteful, which is why presentation identity has
a separate compact representation.

## Presentation-scoped rendered identity

The approved payload design introduces `RenderedItemIdV1` for an addressable
object in one issued presentation. It is exactly four lowercase hexadecimal
characters such as `4ef3`, derived by the server with CRC-16/CCITT-FALSE from
the presentation nonce, immutable version, seed, role, ordinal, durable
semantic item identity, and canonical public item content.

Rendered IDs are appropriate for selected choices, multi-blank slots, both
sides of matching, ordering items, and hotspot surfaces or named regions. A
single ordinary text or numeric field has no reason to carry one. The browser
shows ordinary labels and content, not the code, then submits the compact code
inside the family-specific answer shape.

| Durable semantic ID                                 | Rendered item ID                           |
| --------------------------------------------------- | ------------------------------------------ |
| `ChoiceId` (or another internal slot/item identity) | `RenderedItemIdV1`                         |
| Stable across the authored question's semantics     | Valid only inside one attempt presentation |
| Used by server grading and content models           | Used for compact browser correspondence    |
| May be longer opaque text                           | Exactly four lowercase hex characters      |
| Never inferred from a label or position             | Derived from the full rendered context     |

At issuance, PLE derives IDs for the entire presentation, requires uniqueness
across roles as well as within a response family, retries with a fresh
16-byte nonce when a collision occurs, and fails closed after the documented
retry limit. The server either reproduces the native mapping deterministically
from immutable version, seed, and nonce or persists the validated external
renderer mapping for the attempt.

CRC16 is intentionally a compact correspondence and error-detection value.
It is not secret, collision-resistant, authentication, authorization,
transport integrity, proof that a learner saw pixels, or proof that an answer
is correct. TLS protects transfer integrity; session ownership, RLS, timing,
attempt lifecycle, and idempotency remain the security boundaries.

## Whole-presentation consistency

Fine-grained IDs tell the server which issued objects the browser selected. A
separate whole-presentation SHA-256 digest binds the canonical public
descriptor: version, seed, nonce, prompt, response schema, rendered item
roles/order/content, asset identities/checksums, and hotspot geometry. The
database retains all 32 digest bytes; the public `pd1_...` token transports a
128-bit base64url prefix.

The digest is also a consistency value rather than authentication. It catches
application-state mistakes such as a stale tab, mixed cached envelope, wrong
choice ordering, or a valid render accidentally paired with the wrong attempt.
On mismatch, PLE does not grade or mutate the attempt: it returns the stable
`409 presentation_mismatch`, retains bounded diagnostics, reloads the same
attempt presentation, and lets the learner review compatible recovered input.
It must not silently issue a new seed or grade a stale answer.

## Design invariants

- Draft, shared published content, tenant educational records, and physical
  objects have different owners and identifiers.
- Publication is a new immutable identity boundary, not an edit in place.
- Each Question ID names one immutable published question. Existing assignments
  and issued runs pin their exact hidden evidence for historical replay.
- A normal browser surface presents that exact question and its Question ID,
  never a version picker, "latest" resolution, sequence number, or UUID.
- `QuestionAttemptId` identifies the server-owned grading context; it is not a
  bearer capability.
- A compact rendered ID is local to one presentation and never becomes a
  durable database or catalog identity.
- CRC16 and a presentation digest add consistency evidence only; they never
  replace authenticated server-side grading or tenant isolation.

## Related documents

- [QUESTION_MODEL.md](QUESTION_MODEL.md): public model and answer-bearing
  boundary.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md): normative
  render, response, digest, and rendered-ID wire strategy.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md): logical asset versus immutable object
  storage and delivery grants.
- [DATABASE_TENANCY.md](DATABASE_TENANCY.md): tenant ownership, RLS, and
  retention.
- [SECURITY_MODEL.md](SECURITY_MODEL.md): authorization and server-only
  grading boundaries.
- [CONTRACTS.md](CONTRACTS.md): change-control register for public contracts.
