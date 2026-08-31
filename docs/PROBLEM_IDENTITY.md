# Problem identity and lifecycle

This document is the durable map of identities that name questions and their
related records. It answers two easy-to-confuse questions:

1. What durable record is this?
2. Which stable, human-usable Question ID names that published question lineage?

Durable records use typed UUID identities where appropriate. A Question Version
uses one validated `AAA-BBBB` Crockford Base32 Question ID and a positive
version number. Presentation-scoped response IDs are a third, deliberately
temporary contract.

The model lives primarily in `crates/question_model/src/identity.rs`,
`activity.rs`, `catalog.rs`, `definition.rs`, and `presentation/`. The exact
Student wire contract is owned by
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md); this document
does not turn an identifier into authorization or grading authority.

## The maintainer rule

An editable draft is workspace-owned and has neither a `QuestionId` nor a
`QuestionVersionNumber`. Initial publication mints a stable `QuestionId` lineage and its
first immutable `QuestionVersionNumber`. A controlled same-lineage update keeps the
`QuestionId` and mints another immutable `QuestionVersionNumber`; a substantive fork gets
a new `QuestionId` and a new immutable version. An attempt pins its exact
question/version pair, seed, and server-owned provenance.

This is a type boundary, not a `draft: bool` convention. A
`DraftQuestionDefinition` contains a `WorkspaceId` and no published identity.
A `QuestionDefinition` requires both `QuestionId` and `QuestionVersionNumber`. A caller
cannot accidentally pass the draft type to a published-only API without an
explicit publication step.

## Identity domains

The record identifiers in this table are distinct Rust newtypes over a
PostgreSQL `uuid`. `QuestionId` and `QuestionVersionNumber` instead model the
canonical `(question_id, version_number)` database key. They are not aliases:
a function asking for a Question Version Number cannot be given a Question ID,
and a `CourseId` cannot be given an `AssignmentId`.

Fresh server-created record identities are UUIDv7. Their random portion avoids
sequential enumeration and their time ordering is friendlier to indexes. The
reader and wire contract accept any canonical UUID, including deterministic
fixtures and pre-existing values; the version restriction applies to minting,
not deserialization. Generation is behind the server-enabled `generate`
feature and is absent from the browser/Wasm build.

### Authoring and shared content

| Identifier          | Names                                                  | Ownership and lifetime                                                      |
| ------------------- | ------------------------------------------------------ | --------------------------------------------------------------------------- |
| `WorkspaceId`       | One private Instructor authoring workspace             | Workspace-owned, mutable draft boundary                                     |
| `WorkspaceImportId` | One staged import in a workspace                       | Workspace-owned until publication resolves it                               |
| `QuestionId` | Stable human-facing Question lineage | Shared immutable-content provenance and retrieval |
| `QuestionVersionNumber` | Positive monotonic version within one Question | Shared content; referenced by assignments and attempts |
| `AssetId`           | One logical published asset                            | Shared content identity, independent of physical placement                  |
| `ObjectId`          | One immutable object-store record                      | Physical/source/rendition object identity; never a substitute for `AssetId` |

`QuestionVersionReference { question_id, version_number }` is the complete durable
reference to immutable published evidence. Assignment items, attempts,
exported manifests, and provenance use that pair rather than resolving a
latest version. Instructor version history may show the safe available choices,
but a future assignment update must select and pin an exact version.

`AssetId` answers "which logical asset does content cite?" `ObjectId` answers
"which immutable stored bytes or source artifact reproduce this record?" An
asset can point at an object, but keeping the names separate permits storage
deduplication or rendition replacement without rewriting question content.

### Course and Student teaching records

| Identifier                                       | Names                                              | Why it must remain distinct                                                   |
| ------------------------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------- |
| `CourseId`, `CourseMembershipId`                 | A Course Instance and one exact membership         | Course membership is not assignment ownership                                 |
| `AssignmentId`                                   | One course assignment                              | Defines learning activity and policy, not content                             |
| `AssignmentItemId`, `AssignmentSelectionGroupId` | A stable assignment item or random-selection group | Preserves position and selection semantics when content repeats               |
| `StudentRecordId`                                 | One Student's protected course record              | Separates course-local educational records from global Account identity        |
| `AssignmentAttemptId`                             | One pass through an Assignment                     | Binds one exact Student Record and Assignment while preserving continued practice |
| `IssuedQuestionId`                                | One selected Question in an Assignment Attempt     | Freezes source entry, exact Question Version, order, selection evidence, and scoring treatment |
| `QuestionAttemptId`                               | One server-issued try under an Issued Question     | Binds a response to retained selection, seed, timing, and backend              |
| `StudentRecordId`, `AccountId`                   | Protected Student Record and authenticated Account | A Student Record is course-scoped; an Account is installation-global, so they are not interchangeable concepts |

An ID names a record; it does not grant access to it. Authentication,
`AuthenticatedSession`, exact course/Student/workspace relationships, forced RLS,
lifecycle checks, and server-side ownership checks decide whether a caller may
read or change the record.

### Human-facing Question ID

`QuestionId` is the stable catalog identity presented to a person. Its
canonical form is `AAA-BBBB`: six random Crockford Base32 identity characters
plus one server-validated HMAC-SHA256 character. It is non-sequential and
copyable. One Question ID names one published question lineage; each immutable
version in that lineage has its own `QuestionVersionNumber` and exact publication evidence.
A Question ID is not authorization evidence and never replaces
`QuestionVersionReference` in hidden storage, assignment pins, attempt provenance, or
historical grading.

## Publication and provenance

The lifecycle state machine is a small pure model:

| State        | Published pair present | Approved-Instructor discovery | Ordinary new assignment selection | Exact historical resolution |
| ------------ | ---------------------- | ----------------------------- | --------------------------------- | ---------------------------- |
| `Draft`      | No                     | No                            | No                                | No                           |
| `Validated`  | No                     | No                            | No                                | No                           |
| `Published`  | Yes                    | Yes                           | Yes                               | Yes                          |
| `Deprecated` | Yes                    | Yes                           | No                                | Yes                          |
| `Archived`   | Yes                    | Yes                           | No                                | Yes                          |

Only these forward transitions are legal:

| From         | Event                     | To           |
| ------------ | ------------------------- | ------------ |
| `Draft`      | `Validate`                | `Validated`  |
| `Validated`  | `Publish { publication }` | `Published`  |
| `Published`  | `Deprecate { reason }`    | `Deprecated` |
| `Deprecated` | `Archive`                 | `Archived`   |

`question_model::lifecycle::apply` rejects skips, reversals, and empty
deprecation explanations. The pure transition receives the publication pair;
the server publication flow records the immutable payload, authorship, and
provenance atomically. There is no restore transition. Every published state
remains discoverable and exactly resolvable to an approved Instructor, and the
catalog visibly labels Deprecated or Archived state and its reason.

PLE controls question evolution through explicit semantic change classes.
Transport-size limits protect request handling and do not define compatibility.
A creator or designated original-lineage steward may
publish a same-Question-ID version only for an allowed correction or compatible
improvement. That publication mints a new immutable `QuestionVersionNumber`, archives the
replaced snapshot, preserves exact ancestry, and never rewrites an assignment
or issued attempt. A grading-semantic correction is an impact and recalculation
operation: it records affected exact pins, evaluates the permitted impact, and
publishes the replacement only through that controlled workflow.

Major objective, response-family, task, or other incompatible changes require
a fork. Any approved Instructor may start a fork from a published version, but
the fork draft is private to its creator until validation succeeds. Publication
then enters the global catalog with a new `QuestionId`, a new immutable
`QuestionVersionNumber`, and exact `derivedFrom` ancestry to the source Question ID and
version. The source author does not lose ownership or receive implicit write
access to the fork. Improvement threads remain attached to the preserved
lineage and ancestry even when a fork starts a new identity.

`QuestionChangeProposal` is the lightweight improvement operation. Any vetted
Instructor may submit one patch and rationale against one exact immutable base
version. Publication validation and semantic/grading-impact analysis must
succeed before the proposal is submitted for a lineage-owner decision. The
lineage owner accepts or rejects the proposal; a stale base is rejected and
requires the proposer to rebase and resubmit. An accepted `ModerateEdit`
publishes a new immutable `QuestionVersionNumber` in the original stable `QuestionId`
lineage, preserves canonical authorship and the compatible CC license, and
records contributor credit and proposal ancestry. It leaves every assignment,
attempt, and evidence pin unchanged.

`ModerateEdit`, `FullFork`, and `ForcedQuestionCorrection` are separate
operations. `ModerateEdit` covers a compatible same-lineage proposal;
`FullFork` covers a major semantic change and creates a creator-private draft
that validates before global publication with a new Question ID; and
`ForcedQuestionCorrection` remains the Sysadmin-only emergency replacement
operation described below. The user-facing action is **Suggest an improvement**.
Any GitHub comparison is documentation-only; these domain operations and
authorization rules are authoritative.

Existing assignments and issued runs retain their exact `QuestionVersionReference`
pins. A course Instructor may deliberately make a newly available version the
future assignment choice only through an explicit, revision-checked update;
publication, correction, lifecycle work, and background processing never
advance an assignment automatically. This controlled availability separates
stewardship from assignment composition: a future assignment update changes
only after its revision check, while issued snapshots continue to use their
original exact pair.

All successful publications enter one installation-wide shared catalog visible
to approved Instructors. Publication has no publication-scope field, selector,
filter, or separate branch. Private editable work remains a workspace-owned
draft until validation and publication succeed.

Approved Instructors may inspect the safe published question content and
metadata for a question referenced by another course. That shared content
visibility does not grant access to the other course's assignment composition,
Student assignment entitlement, Student records, or course management data.
Every assignment item references a Question ID already in this shared
published catalog. Drafts remain private until they validate and publish; an
assignment cannot contain assignment-private question content.

Star is one vetted-Instructor-visible endorsement per Question ID. Approved
Instructors may see the star count and the identities of vetted Instructors who starred;
Students and anonymous callers see neither the identity list nor star state. A
watch is a private Account-scoped notification subscription for versions, forks,
improvements, and impact events; it never grants course or Student authority.
Improvement threads are preserved as non-authoritative discussion records and
retain their source, successor, and fork ancestry.

Catalog evidence is version-specific. After the configured disclosure
threshold, the safe rollup may expose accepted-attempt count, graded-attempt
count, correct count, and eligible-choice selection counts for supported choice
families. Before the threshold, the values remain unavailable; raw responses,
small cells, and linkable cohorts never appear. Preview traffic and the
Instructor Student view contribute no catalog metrics.

### ForcedQuestionCorrection

Every published version remains immutable. A Sysadmin may approve a closed
`ForcedQuestionCorrection` only with reason `security_flaw` or
`critical_correctness_flaw`. The operation immediately activates the
authoritative mapping from the flawed version to the validated replacement, so
new selection and issuance resolve to the replacement. The old version is
preserved solely as immutable historical evidence; it is never edited or
deleted.

The replacement must validate before publication and carry a closed,
privacy-safe impact manifest. The resulting correction generation is handed to
bounded idempotent, generation-fenced workers for active-binding and
remediation materialization across every active Blueprint, CourseInstance,
assignment, selection-pool, and future-issuance reference. A
deterministic compatibility check governs reissue or excuse for in-progress
work. Issued or graded evidence retains its original exact version; completed
work receives superseding receipts and deterministic recalculation under the
correction.

The correction has no per-course approval step. Instructors receive audited,
course-authorized results, while the Sysadmin impact projection contains no
Student names, responses, grades, FERPA-bearing records, or other course
records. Every approval, validation, manifest, atomic advance, reissue,
excuse, superseding receipt, recalculation, and publication event is recorded
in an append-only audit trail.

## Attempts: durable authority, not a large request body

`QuestionAttemptId` is the primary identity for an ordinary Student submission.
An issued attempt already binds, server-side:

- authenticated Student and exact course relationship;
- run, assignment position, and policy snapshot;
- exact `QuestionVersionReference` and generated seed;
- deadline and lifecycle/submission state; and
- adapter, renderer, generator, source-object, asset-object, and grading
  provenance necessary for reproducibility.

A Student receives question content only through an exact server-authorized
assignment entitlement: the authenticated Student, active Student membership,
exact `CourseId`, exact `AssignmentId`, assignment audience and lifecycle, and
current policy must agree in the protected transaction. A Student cannot use
the shared Instructor catalog to obtain assignment content.

The compact route
`POST /api/courses/{courseId}/assignments/{assignmentId}/attempts/{attemptId}/submissions`
therefore needs the course and assignment as server-verified routing assertions, the attempt UUID
once, an idempotency key in the request header, a presentation consistency token, and the Student's
answer. The browser must not resend the problem, version, seed, backend, grading mode, or a response
`kind` as authority. Route identities do not grant access; the server resolves the strict answer
decoder from the issued attempt's response schema and verifies every relationship.

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
transport integrity, proof that a Student saw pixels, or proof that an answer
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
attempt presentation, and lets the Student review compatible recovered input.
It must not silently issue a new seed or grade a stale answer.

## Design invariants

- Draft, shared published content, course/Student educational records, and physical
  objects have different owners and identifiers.
- Publication is a new immutable identity boundary, not an edit in place.
- Each Question ID names one stable published question lineage. Each version is
  immutable, and existing assignments and issued runs pin their exact hidden
  evidence for historical replay.
- A Student browser surface presents its exact pinned version and Question ID,
  never an implicit "latest" resolution, sequence number, or UUID. Instructor
  updates choose and pin an exact version through the controlled workflow.
- `QuestionAttemptId` identifies the server-owned grading context; it is not a
  bearer capability.
- A compact rendered ID is local to one presentation and never becomes a
  durable database or catalog identity.
- CRC16 and a presentation digest add consistency evidence only; they never
  replace authenticated server-side grading or Account-scoped forced RLS.

## Related documents

- [QUESTION_MODEL.md](QUESTION_MODEL.md): public model and answer-bearing
  boundary.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md): normative
  render, response, digest, and rendered-ID wire strategy.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md): logical asset versus immutable object
  storage and delivery grants.
- [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#intended-database-model):
  global accounts, exact ownership, forced RLS, and retention.
- [SECURITY_MODEL.md](SECURITY_MODEL.md): authorization and server-only
  grading boundaries.
- [CONTRACTS.md](CONTRACTS.md): change-control register for public contracts.
