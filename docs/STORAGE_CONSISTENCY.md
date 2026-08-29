# Storage consistency

PostgreSQL and object storage are separate durable systems. PLE never claims a distributed transaction between them. Instead it uses typed immutable objects, database-authoritative visibility, and operation-specific repair rules.

The binding target is the single-installation model in the [single-installation authorization
plan](active_plans/active/single_installation_authorization_plan.md). The currently checked-in
pre-SD1 source still contains historical `TenantId`, `TenantContext`, and tenant-shaped object and
retention fields. SD1-C owns replacing those source and schema shapes with exact domain scopes. This
document does not authorize a compatibility alias, a dual key, or a parallel tenant model while that
dependency is open.

## Authority and vocabulary

An **object** is immutable bytes and its server-created `ObjectRecord`. A **reference** is durable database state that makes that object relevant to a catalog, course, or student record. A **delivery** is a separately authorized route mapping from an opaque ID to one exact record.

| State | Meaning | Required treatment |
| --- | --- | --- |
| Reference and SHA-256-verified bytes | Healthy | Deliver only under its typed delivery policy. |
| Bytes without reference | Orphan | Never deliver. Preserve for reconciliation quarantine. |
| Reference without bytes | Broken reference | Fail closed, alert, and preserve database evidence. |
| Reference with wrong bytes/checksum | Corruption or substitution | Fail closed; do not replace it with a near match. |

The database is authoritative for intended existence and visibility. The object store is authoritative
for whether the exact bytes exist. Neither a bucket listing nor a successfully fetched object creates
an authorization right. Physical domain selection comes from `ObjectKey`, not a caller's path string.
Authorization is evaluated separately at the trusted Store/PostgreSQL boundary: a delivery needs the
exact course/Student relationship, a current workspace owner/collaborator relationship, the approved
Instructor catalog capability, or another registered typed capability/lease. An object ID, bucket,
path, or checksum never grants permission.

## Standard bytes-first protocol

For private source, provenance, protected artifacts, and non-public immutable objects, the owner follows this sequence:

```text
derive typed key and identity
  -> write immutable bytes and compute/record SHA-256
  -> validate returned record against the expected typed binding
  -> commit the database reference
  -> authorize delivery only from committed database state
```

A write failure creates no intended reference. A database failure after a successful write produces an unreachable orphan, not browser-visible content. Reversing that order risks a database promise whose bytes do not exist. An immutable-key collision is reusable only when the operation proves the exact record and checksum are the same logical replay; it is never an overwrite.

## Public assets use a transactional outbox

Public assets are the exception to the simple bytes-first protocol because a final public object must not exist before the catalog decision commits. For a public version, the catalog transaction commits all of the following together:

- immutable catalog publication state;
- one `AssetPublication::Pending` registry record per public asset; and
- a closed `PublishPublicAssets { problem, version }` job.

The registry points at its final immutable `PublicAssets` key but the `Pending` state has no public delivery. The dedicated publisher re-resolves records from the database under the active job lease, validates that each source is an exact allowed private workspace asset, reads and re-hashes the source, and writes the final public object. It never trusts queue-provided object bytes or a browser-provided path.

After all materialization succeeds, a lease-conditional database function performs the mechanical `Pending -> Ready` transitions and job completion in one database transaction. If the worker crashes after a public write but before activation, the retry accepts only an exactly matching immutable object then activates it. If it crashes before the write, the pending registry stays unavailable and the leased job is retried. Thus no pre-commit CDN orphan is created, and public visibility always follows a committed catalog decision.

The publisher's database capability can claim/read/fail only public-asset publication jobs and activate only the matching leased public version. Its production IAM role is separately constrained to the required private source read and public immutable write operations. Code-level capability tests do not substitute for a deployed IAM policy review.

## Integrity and authorization

Integrity and authorization answer different questions and require different evidence. SHA-256 binds
canonical source, object bytes, and selected publication records. It catches accidental corruption,
inconsistent copies, and a storage response that does not match the durable record. It is not a
signature, MAC, authorization check, or encryption mechanism.

| Concern | Question | Enforced by |
| --- | --- | --- |
| Integrity | Are these the exact immutable bytes and record? | SHA-256, typed immutable keys, `ObjectRecord`, and immutable-write checks |
| Authorization | May this actor or worker use this exact object now? | Exact course/Student or workspace relationship, approved-Instructor predicate, or registered typed capability/lease |

The corresponding confidentiality and history controls are:

| Property | Enforced by |
| --- | --- |
| Writer and reader authority | Typed server APIs, PostgreSQL/RLS capabilities, S3/IAM and delivery routes |
| Immutable history | Fresh Question ID and hidden exact publication evidence, conditional object creation, and deployed Object Lock/tag policy |
| Transport confidentiality | HTTPS/TLS and private network paths |
| Storage-at-rest confidentiality | Per-domain SSE-KMS and encrypted backups |
| Parser safety for images | Strict still-image type/container/dimension/full-decode validation |

Published-content immutability remains intentional. Every content change, including a correction,
publishes a new immutable question with a fresh Question ID and fresh hidden `(ProblemId, VersionId)`
evidence instead of changing object bytes referenced by an existing assignment, run, or attempt.
Optional one-way provenance may identify the source publication without changing it.

## Retention and repair

Student-record retention freezes a typed manifest scoped to one exact course, stage, and positive
generation, then processes only that manifest under a leased job. Manifest entries identify exact
typed object records; they are never a bucket prefix or a caller-selected path. Object deletion and
relational deletion do not claim completion until the required manifest checks succeed. Shared
published content, private authoring, and anonymous aggregates are outside a learner-record purge.

The current pre-SD1 retention source still carries tenant fields in its worker command and manifest
storage. SD1-C owns the source/schema replacement with the exact course/stage/generation scope above;
no compatibility tenant field is added here.

General bucket-to-database reconciliation is not yet implemented. Until it is, operators must preserve missing/mismatched reference evidence and investigate the backing store; application code must not silently delete references or serve unregistered bytes. Production backup restore, KMS rotation, Object Lock retention, lifecycle policy, and cross-region/failover claims need live deployment evidence.

## Change rule

A new object class must add a typed `ObjectKey`, physical-domain decision, database reference, visibility rule, retention/recovery owner, and behavior-focused tests together. It may not gain access by reusing a generic bucket/path or by treating a checksum as permission.
