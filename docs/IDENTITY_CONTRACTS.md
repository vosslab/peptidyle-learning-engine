# Identity contracts

PLE uses different identifiers for different jobs. A durable record ID is not
automatically a public reference, a browser value, or a credential. Treating
those roles as interchangeable makes authorization, retention, retries, and
rendering harder to reason about.

This page is the cross-cutting identity map. It supplements, rather than
replaces, [PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md),
[QUESTION_MODEL.md](QUESTION_MODEL.md), [CONTRACTS.md](CONTRACTS.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The closed human
role model is in [USER_ROLES.md](USER_ROLES.md). The Rust types
and PostgreSQL schema linked below are authoritative when this page and code
ever disagree.

## Rules that apply everywhere

- A durable ID names one stored thing. It is opaque and never proves that the
  caller may read or change that thing.
- The authenticated PLE session establishes `TenantId` and `UserId`; routes and
  store methods derive authorization from those facts rather than trusting
  tenant or user IDs supplied in a browser body.
- Educational-record rows carry `TenantId` directly. PostgreSQL row-level
  security and store methods use it as the tenancy boundary.
- Published catalog content is immutable and shared. Tenant-owned courses,
  enrollments, runs, attempts, jobs, and protected objects remain separate
  records even when they refer to the same published content.
- Identifiers are distinct Rust newtypes wherever mixing them would be a
  correctness risk. They serialize as canonical UUID strings only at an
  appropriate trusted or browser boundary.
- A checksum or digest detects disagreement in otherwise valid data. It is
  not authentication, authorization, transport security, or an answer key.

## Durable record identities

The following UUID-backed types are persisted identities. Fresh
question-model identities use server-only UUIDv7 minting; decoding accepts an
existing canonical UUID so local fixtures and stored values can round-trip.
Several operational IDs use operating-system randomness instead of the
question-model minting helper. Both are opaque database identities, not
sequential counters and not browser secrets.

| Identity                                           | Owns or names                                          | Authority and relation                                                                                                 |
| -------------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `TenantId`                                         | Institution RLS boundary                               | Established by authentication; carried on every educational record.                                                    |
| `UserId`                                           | Authenticated person                                   | Comes from the PLE account boundary; differs from pedagogical `StudentId`.                                             |
| `WorkspaceId`                                      | Tenant-owned instructor draft workspace                | Drafts and staged imports remain private here.                                                                         |
| `WorkspaceImportId`                                | One staged workspace import                            | Never becomes a catalog number; publication creates fresh published identities.                                        |
| `ProblemId`                                        | Internal identity for one immutable published question | Exists only after publication; never identifies a draft.                                                               |
| `VersionId`                                        | Internal immutable publication evidence                | Assignments and attempts retain the exact `(ProblemId, VersionId)` pair.                                               |
| `AssetId`                                          | Logical published content asset                        | May resolve to an immutable physical object but is not its storage identity.                                           |
| `ObjectId`                                         | Immutable object-store record                          | Names stored bytes and may back source, asset, export, or learner-record artifacts.                                    |
| `CourseId`                                         | Tenant course or section                               | Owns assignment placement and course membership context.                                                               |
| `CourseGroupId`                                    | Current group within a course                          | Used for policy exceptions and group-scoped timing, not a global roster.                                               |
| `AssignmentId`                                     | Tenant assignment                                      | Owns current policy and ordered current items.                                                                         |
| `AssignmentItemId`                                 | Stable current-state assignment item                   | Retains item identity while future points, ordering, or policy change.                                                 |
| `AssignmentSelectionGroupId`                       | Random-selection group                                 | Distinct from its selected run items.                                                                                  |
| `EnrollmentId`                                     | One student's durable relationship with one assignment | Binds learner, assignment, and cross-run mastery state.                                                                |
| `RunId`                                            | One pass through an assignment                         | Belongs to one enrollment; later practice creates a new run rather than rewriting the completed one.                   |
| `QuestionAttemptId`                                | One issued question instance                           | Is the primary learner-answer route identity; binds run, exact version, seed, timing, status, provenance, and backend. |
| `AssignmentPolicyExceptionId`                      | One current policy exception                           | Auditable tenant record for a student or group exception.                                                              |
| `CourseBannerId` and `CourseBannerCandidateId`     | Immutable active banner and pre-promotion candidate    | Keep uploaded candidate material separate from the visible course pointer.                                             |
| `AssetDeliveryId`                                  | Protected asset route lookup                           | Reuses an `AssetId`, `ObjectId`, or `CourseBannerId`; it does not mint a second logical object.                        |
| `ExportId`                                         | One tenant-authorized export request                   | Browser may inspect its coarse status; workers resolve its frozen private input server-side.                           |
| `JobId`                                            | One durable queue unit                                 | Worker-facing record identity, not proof that a worker owns its lease.                                                 |
| `ManualGradeActionId` and `AttemptSupportActionId` | One idempotent instructor action                       | Audit identities for manual evaluation or sensitive attempt support actions.                                           |

The published-content newtypes are defined in
[crates/question_model/src/identity.rs](../crates/question_model/src/identity.rs).
Tenant, course, assignment, enrollment, run, and attempt identities are in
[crates/question_model/src/activity.rs](../crates/question_model/src/activity.rs).
The operational identities are owned by
[crates/learning-data-access/src/jobs.rs](../crates/learning-data-access/src/jobs.rs),
[crates/learning-data-access/src/manual_grading.rs](../crates/learning-data-access/src/manual_grading.rs),
[crates/learning-data-access/src/contracts/runs.rs](../crates/learning-data-access/src/contracts/runs.rs),
and [crates/learning-data-access/src/asset_delivery.rs](../crates/learning-data-access/src/asset_delivery.rs).

## Human-facing and semantic references

These values make a record usable in teaching or reproducible in grading; they
are not substitutes for durable ownership and authorization.

| Value                                | Purpose                                                                                                   | Do not use it as                                                                  |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `QuestionId`                         | Single non-sequential Crockford Base32 locator such as `7K3-M9QP`; names one immutable published question | Authorization, a hidden snapshot ID, or a version selector.                       |
| `ProblemVersionRef`                  | Exact internal immutable `(ProblemId, VersionId)` evidence                                                | A browser version selector, learner attempt, or mutable assignment-item identity. |
| `ChoiceId`                           | Stable internal semantic ID for an authored choice, slot, match endpoint, order item, or hotspot region   | A display label, screen position, or presentation-scoped response token.          |
| `Seed` plus generator version        | Reproduces a generated variant under immutable content                                                    | Learner authority to select a new variant.                                        |
| Source object and SHA-256 provenance | Reproduces the source interpreted for an attempt                                                          | A public download URL or browser-supplied renderer input.                         |
| `ScoringGeneration`                  | Positive monotonic stale-work fence for current assignment scores                                         | A record identity or browser-controlled score revision.                           |
| Timing and retention generations     | Fence stale auto-submit and retention workers                                                             | A learner-editable deadline or retention policy.                                  |

The catalog display reference is owned by
[crates/question_model/src/catalog.rs](../crates/question_model/src/catalog.rs).
Semantic response IDs are in
[crates/question_model/src/response.rs](../crates/question_model/src/response.rs).
`ScoringGeneration` is defined in
[crates/question_model/src/assignment.rs](../crates/question_model/src/assignment.rs),
and queued generation fences are a closed server-side contract in
[crates/learning-data-access/src/jobs.rs](../crates/learning-data-access/src/jobs.rs).

One Question ID has one immutable published content identity. A correction,
fork, or other authored content change receives a new Question ID and may
retain explicit source provenance. The hidden `(ProblemId, VersionId)` pair
remains available only for exact immutable retrieval, past-attempt replay,
grading, audit, and provenance. It never authorizes a browser to select a
version or resolve a "latest" question. Existing assignments remain exact
until an Instructor deliberately replaces an item with strong revision
checking. A replacement changes the future assignment definition; issued
snapshots retain their original exact evidence.

## Browser and retry identities

The presentation model and descriptor codec below are implemented Rust types.
Their use as compact browser submission values remains the secure-payload
**target**: the current browser still submits the tagged durable-ID
`StudentResponse` shape. At the target boundary, the browser receives an
intentional minimum and submits against the attempt path once, with a bounded
idempotency header and a compact answer. The server re-derives tenant, learner,
course, assignment, question family, version, seed, timing, and grading backend
from the authenticated attempt.

| Value                             | Scope and lifetime      | Meaning                                                                                                                                                  |
| --------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `QuestionAttemptId` in the route  | Durable attempt         | Names the already-issued instance. It is not enough by itself: session and ownership checks remain required.                                             |
| `SubmissionIdempotencyKey` header | One exact browser retry | Bounded visible ASCII key. The server records request and receipt hashes so same-request replay is harmless and a changed replay conflicts.              |
| `RenderedItemIdV1`                | One presentation        | Target four-lowercase-hex browser value for a selectable rendered object, submitted instead of a letter, position, or durable `ChoiceId`.                |
| `PresentationNonceV1`             | One presentation        | Server-minted 16-byte nonce used when deriving attempt-specific rendered-item IDs and the presentation descriptor.                                       |
| `PresentationDigestTokenV1`       | One presentation        | Target public 128-bit `pd1_` prefix of the full server-stored SHA-256 descriptor digest. It detects an attempt paired with the wrong valid presentation. |
| Normalized hotspot coordinates    | One response            | Target integers from 0 through 10,000 bound to the rendered surface, not pixels or a device-specific image size.                                         |

At that target boundary, `RenderedItemIdV1` is deliberately small and
presentation-specific. The server generates every ID from a domain-separated
CRC16 input containing the nonce, immutable version, seed, item role, ordinal,
durable item identity, and canonical public content. It rejects a presentation
with any duplicate ID and retries with a new nonce. The server retains or
regenerates the authoritative mapping back to the semantic item ID for the
lifetime of the attempt.

That design means a stale selection, swapped ordering map, or matching-side
mix-up cannot silently become a valid answer merely because it still has a
valid label such as `B`. It does **not** make the CRC16 a security primitive;
the authenticated attempt and server-side grading boundary provide security.
The full SHA-256 descriptor stored with the attempt supplies the stronger
whole-presentation consistency check.

The exact wire and recovery rules are in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The codec and
presentation model are implemented in
[crates/question_model/src/presentation/model.rs](../crates/question_model/src/presentation/model.rs),
[crates/question_model/src/presentation/builder.rs](../crates/question_model/src/presentation/builder.rs),
and [crates/question_model/src/presentation/codec.rs](../crates/question_model/src/presentation/codec.rs).
The route's authenticated attempt and idempotent replay path is
[crates/server/src/run/submission.rs](../crates/server/src/run/submission.rs).

## Capabilities are not IDs

Some opaque values must remain private because possession conveys authority.
They must never be confused with UUID record IDs or checksums.

| Capability or secret                                     | Holder and use                                    | Persistence and logging rule                                                                                            |
| -------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Raw session cookie                                       | Browser and authentication endpoint only          | Storage holds only `SessionTokenHash`; raw token is never persisted.                                                    |
| Email authentication secret                              | Initiating browser and email recipient only       | Short-lived, single-use, browser-bound; database holds only a hash and it never enters logs or analytics.               |
| Passkey public credential                                | PLE account boundary                              | The credential/public state is protected account data, not a password verifier or course-Instructor projection.         |
| `JobLeaseToken`                                          | The worker replica that claimed one job           | Replaced on reclaim; not an HTTP or browser type; debug output redacts it.                                              |
| `ExternalToolLaunchToken`, launch proof, and lease token | Server-mediated external-tool exchange            | Opaque fixed-size random bytes; never serialized to generic question or submission records.                             |
| Provider correlation                                     | Private external-tool recovery state              | Persisted only inside the broker boundary; redacted from diagnostics.                                                   |
| Signed object URL                                        | Short-lived object-storage delivery result        | Created only after authorization and audit; not a durable object identity.                                              |
| Future file-upload capability                            | Tenant/learner/attempt-bound upload authorization | Not yet issued. File-upload responses fail closed until this server-issued capability and object-commit boundary exist. |

Session storage is defined in
[crates/learning-data-access/src/session.rs](../crates/learning-data-access/src/session.rs).
Job and external-tool capability types are intentionally non-serializable in
[crates/learning-data-access/src/jobs.rs](../crates/learning-data-access/src/jobs.rs)
and [crates/learning-data-access/src/external_tool.rs](../crates/learning-data-access/src/external_tool.rs).
Protected object authorization is owned by
[crates/learning-data-access/src/asset_delivery.rs](../crates/learning-data-access/src/asset_delivery.rs).
The current fail-closed file-upload route behavior is in
[crates/server/src/run/submission.rs](../crates/server/src/run/submission.rs).

## UUID size is not a payload problem

A canonical UUID is 36 characters in JSON, but PLE sends the attempt UUID once
in a route, not once per response item. It is insignificant beside request
headers, a normal question render, or an image asset. A UUID is worth its size
when it is a durable, independently addressable record: it stays unambiguous
in logs, database joins, exports, and cross-replica work.

Compact identifiers are appropriate only when their scope is intentionally
small:

- use `RenderedItemIdV1` for attempt-presentation selection and ordering;
- use a validated `AAA-BBBB` Question ID for a human catalog locator;
- use a generation number for stale-work fencing; and
- use a nonce, digest token, or capability only for its explicit protocol
  purpose.

Do not shorten a durable ID solely to save a few JSON characters, and do not
promote a checksum, generated display number, or short renderer token into a
global database key.

## Identity and checksum decision table

| Need                                          | Correct mechanism                                                            | Why                                                                      |
| --------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Locate one stored attempt                     | `QuestionAttemptId` plus authenticated tenant context                        | Durable identity and RLS authorization are separate checks.              |
| Select an object as it appeared on screen     | `RenderedItemIdV1`                                                           | Small attempt-specific mapping catches presentation disagreement.        |
| Preserve authored semantics through shuffling | `ChoiceId`                                                                   | Stable server-side item identity is independent of display order.        |
| Detect stale or mixed presentation state      | Presentation nonce plus digest token, checked against the full stored digest | Detects state disagreement without pretending to authenticate a request. |
| Retry a lost submit response safely           | `SubmissionIdempotencyKey` plus request/receipt hashes                       | Distinguishes an exact retry from altered content.                       |
| Keep only current worker output               | A positive generation fence                                                  | Makes older job output harmless without deleting history.                |
| Authorize a protected operation               | Session, route authorization, RLS, and where needed an opaque capability     | IDs and checksums alone are never authorization.                         |

## Maintainer checklist

When adding a new identifier or protocol value, decide and document:

1. What exact thing does it name, and who owns it?
2. Is it durable, human-facing, semantic, presentation-scoped, a stale-work
   fence, a checksum, or a capability?
3. Which layer mints it, where is it persisted, and how is it serialized?
4. Does a tenant-owned record carry `TenantId` directly and receive RLS
   coverage?
5. Can a browser or worker derive it from an authenticated attempt or lease
   instead of resending it?
6. Does possession authorize anything? If yes, use a bounded opaque capability
   with expiry, redaction, and an explicit storage boundary rather than a
   general-purpose record ID.

## Related documents

- [PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md) defines publication identity and
  lifecycle.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines safe public question data and
  server-only answer material.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines the
  learner render, response, presentation-consistency, and WeBWorK boundary.
- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines the authentication, tenancy,
  grading, storage, and provider security boundaries.
- [CONTRACTS.md](CONTRACTS.md) registers the frozen inter-module contracts and
  their change rules.
