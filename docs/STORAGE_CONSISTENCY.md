# Storage consistency

PLE uses PostgreSQL and object storage as one durable system, but does not pretend that they share
one transaction manager. This document defines the cross-storage contract: what is authoritative,
how an operation becomes visible, how failures are represented, and how the system repairs them.

It complements [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
[DATABASE_TENANCY.md](DATABASE_TENANCY.md), [OBJECT_STORAGE.md](OBJECT_STORAGE.md), and
[RETENTION_POLICY.md](RETENTION_POLICY.md). Those documents own schema, authorization, object
shape, and lifecycle detail. The active implementation and release plans remain the source of
truth for unfinished work.

## Scope and vocabulary

An **object** is immutable bytes plus its typed `ObjectRecord`. A **reference** is durable database
state that intentionally makes an object relevant to PLE. An object can be readable only after its
reference and delivery policy allow it; storing bytes never grants browser access.

| Term | Meaning |
| --- | --- |
| Object ID | Opaque `ObjectId` assigned by the server and carried by an `ObjectRecord`. |
| Typed key | Server-derived `ObjectKey`, including bucket, immutable path, category, and identity components. |
| Object record | Metadata returned by `ObjectStore::put`: ID, key, SHA-256, size, media type, category, provenance, and creation time. |
| Reference | A database row or validated JSON field that binds one business fact to one exact object record. |
| Orphan | Bucket bytes for which no intended database reference exists. |
| Broken reference | Intended database reference for which bytes are missing or fail integrity verification. |
| Manifest | A frozen, typed set of objects that one retention or worker operation must process exactly. |

Object IDs and keys are different on purpose. The ID is the durable logical identity used in
records and delivery registries. The key is an implementation-owned physical location derived from
typed components. Browser requests use an opaque delivery ID, never an object key, bucket name,
or client-selected filename.

## Authority and visibility

PostgreSQL is authoritative for whether PLE intends an object to exist. The object store is
authoritative for whether the corresponding bytes exist and what they contain. This asymmetric
rule gives every disagreement a safe interpretation.

| Observed state | Meaning | Required treatment |
| --- | --- | --- |
| Record/reference and verified bytes | Healthy object. | Authorize and deliver according to its category and scope. |
| Verified bytes, no record/reference | Orphan. | Keep out of delivery; reconcile after quarantine. |
| Record/reference, missing bytes | Broken reference. | Alert and quarantine delivery; never delete the database record to conceal it. |
| Record/reference, wrong digest | Corruption or wrong bytes. | Treat as a broken reference; do not serve it. |

This rule is intentionally more conservative than "whichever system responds first wins." A
database record is evidence that content, provenance, a student artifact, or a deletion obligation
was intentionally created. Removing it because object storage is temporarily unavailable would
erase evidence and can make a record impossible to explain. Conversely, a byte object without a
database reference must not become visible merely because it exists in a bucket.

The browser reaches objects only through `GET /api/assets/{id}`. The server resolves the opaque
delivery ID to a trusted record, applies tenant/course/student authorization, audits a protected
grant, and only then obtains a short-lived signed URL where that is allowed. This keeps physical
storage topology out of browser contracts. See [OBJECT_STORAGE.md](OBJECT_STORAGE.md#delivery-grants)
and [SECURITY_MODEL.md](SECURITY_MODEL.md#asset-delivery-boundary).

## Typed identities and immutable bytes

`crates/objects/src/bucket.rs` is the sole owner of object-key construction. Callers provide typed
identities such as tenant, workspace, course, problem version, asset, import, seed, or object ID;
they do not concatenate paths. The resulting `ObjectKey` chooses one of three buckets:

| Bucket | Current object classes | Visibility |
| --- | --- | --- |
| `content` | Workspace sources/assets, published source archives/assets/renders, course banners | Category- and authorization-specific; it is not automatically public. |
| `student-records` | Tenant-owned exports and other `StudentRecord` artifacts | Authenticated, tenant-scoped, explicit grant only. |
| `temp-processing` | Conversion intermediates and banner candidates | Never signable or browser-served. |

Object writes are immutable. A `put` for an existing key fails rather than overwriting a previous
artifact. A replacement is a new typed key and object record. That preserves the chain from an
attempt or publication back to the exact source, assets, seed, and renderer/generator version that
produced it. It also prevents a later authoring change from changing what a learner previously saw.

The authoritative implementation boundary is [bucket.rs](../crates/objects/src/bucket.rs),
[lib.rs](../crates/objects/src/lib.rs), and the `MemoryObjectStore` and S3 implementations. The
database takes opaque `ObjectRecord` values; it does not reconstruct keys from untrusted strings.

## Write and publication protocol

Because PostgreSQL and S3-compatible storage cannot participate in a single atomic transaction,
PLE uses an ordered, idempotent protocol rather than a distributed two-phase commit:

```text
allocate typed identity
        |
        v
put immutable bytes and compute SHA-256
        |
        +-- failure --> no database reference; operation fails
        |
        v
validate returned ObjectRecord and expected typed binding
        |
        v
commit database transition that references exact record
        |
        +-- failure --> unreachable orphan; reconciliation owns later cleanup
        |
        v
authorize delivery only through committed database state
```

Bytes-first, record-second is the required order. A crash before the database transition creates an
unreachable orphan, which is safe to quarantine and collect. Reversing the order could create a
broken reference that promises content or evidence PLE cannot retrieve.

The database transition is the visibility cutover. A source object is not a published question
merely because it has been uploaded. Publication verifies the exact source object, checksum,
canonical source, answer-free model, and grader-only material, then records the immutable
version-pinned binding in one database transaction. The relevant broker functions and provenance
fields are in [2026080805_operations_analytics.sql](../schemas/migrations/2026080805_operations_analytics.sql);
the server conversion path is [qti_profile_conversion.rs](../crates/server/src/qti_profile_conversion.rs).

The same rule applies to deterministic renders and assets. A render may be cached after it is
produced, but it cannot replace the pinned question source. A course-banner candidate stays in
`temp-processing` until a verified compare-and-swap save promotes distinct immutable bytes to the
current course-banner record. The course appearance migration is
[2026080907_course_appearance.sql](../schemas/migrations/2026080907_course_appearance.sql).

## Source artifacts and provenance

PLE keeps source artifacts separate from learner render data and grading authority.

| Backend or source | Authoritative durable fact | Derived values |
| --- | --- | --- |
| Native static | Canonical versioned PLE flat-question JSON source object | Answer-free render projection and grader-only material. |
| Native algorithmic | Pinned generator ID/version and seed specification | Parameters, rendered output, and cached render. |
| WeBWorK | Pinned PG source/version plus replay inputs | Rendered HTML, images, and cached render. |
| QTI | Original unchanged package object | Parsed model, extracted assets, and accepted/rejected report. |
| Student artifact | Typed tenant/course `StudentRecord` object and authorized record link | Delivery grant and export presentation. |

Attempt evidence binds problem/version, source artifact ID and SHA-256, adapter and generator or
renderer versions, seed, parameter hash, asset IDs, grading implementation version, and rendered
question hash. This is sufficient to explain a result without copying answer-bearing source into a
learner response or browser cache. The fuller reproducibility record is specified in
[implementation_plan.md](active_plans/implementation_plan.md#reproducibility-record).

Published content is shared and immutable. Workspace sources, imports, and draft artifacts are
tenant-private. Student records are tenant-owned and retention-bound. A retention purge must never
follow a student record reference into shared published source or catalog content.

## Checksums are evidence, not authority

Several integrity values appear in PLE. They answer different questions and must not be conflated
with authorization or transport security.

| Value | Scope | Purpose | Not a substitute for |
| --- | --- | --- | --- |
| Object SHA-256 | Immutable bytes | Detect corrupt, substituted, or mismatched stored bytes on read. | Authorization, signatures, or a database transaction. |
| Source/payload checksum | Canonical immutable content | Bind a published revision to exact persisted source/model data. | A learner-facing answer check. |
| Rendered-question hash | Attempt presentation | Detect that replay/regeneration differs from recorded presentation. | A secure session or grading key. |
| Presentation CRC-16 ID | One rendered selectable item | Compact consistency signal for submitted choice/item identity. | Authentication or collision-resistant integrity. |

The secure grading payload design keeps browser submissions small while preserving server-only
grading. Attempt-specific selectable IDs must be uniqueness-checked within the issued question;
the server keeps the authoritative mapping. A separate descriptor digest detects a whole-render
mismatch. Neither value permits a browser to choose a different tenant, object, grading key, or
attempt. See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Retention as a cross-storage transaction

Retention is the current implemented example of the cross-storage protocol. It uses durable state
and replay, not a best-effort bucket delete after a relational purge.

```text
authorize schedule/action and fence learner-facing access
        |
        v
freeze tenant/course/stage/generation object manifest in PostgreSQL
        |
        v
claim retention job under its lease and process only that manifest
        |
        v
idempotently delete or confirm absence of every typed object
        |
        +-- any object failure --> keep archive/fence; retry same manifest
        |
        v
delete tenant learner graph in one ordered database transaction
        |
        v
record terminal studentRecordsDeleted tombstone
```

The manifest primary identity is `(tenant_id, course_id, generation, stage)`, with one typed object
row per expected object. It prevents a retry from discovering a newly written object or deleting an
unrelated one. The worker queue carries only bounded identifiers, stage, and generation; it does
not carry student names, object keys, raw responses, or grading content.

Archive fences learner aliases and delivery before cleanup. Permanent deletion is terminal only
after all manifest objects are absent and the relational learner graph has been removed in its
verified foreign-key order. A partial object-store failure leaves the course archived and retries
the same frozen manifest; it never reports successful deletion early. Published content, drafts,
and anonymous question aggregates remain outside this student-record deletion boundary.

Current Store contracts are in [retention.rs](../crates/learning-data-access/src/retention.rs) and
the durable PostgreSQL mechanism is in
[2026080806_retention.sql](../schemas/migrations/2026080806_retention.sql). The policy and default
timeline are in [RETENTION_POLICY.md](RETENTION_POLICY.md).

## Failure and repair matrix

| Failure | Durable state after failure | Safe response | Repair owner |
| --- | --- | --- | --- |
| Object `put` fails | No intended reference. | Return failure; do not start publication or delivery. | Caller may retry with the same typed identity only if the backend reports a safe replay. |
| Object `put` succeeds; DB transition fails | Unreferenced immutable bytes. | Keep unreachable and non-deliverable. | Planned reconciliation quarantines then deletes as an orphan. |
| DB reference commits; bytes later disappear | Broken reference. | Stop delivery, alert, preserve database evidence. | Storage incident/recovery; never auto-delete the reference. |
| DB reference commits; digest differs | Broken reference/corruption. | Refuse delivery and record diagnostic context. | Storage incident/recovery; preserve evidence. |
| Publication request repeats | Existing immutable source and committed transition may already exist. | Validate exact identity/checksum and return the same outcome; reject divergence. | Publication broker/Store. |
| Retention worker crashes mid-delete | Prepared manifest and job lease/generation remain durable. | Reclaim/replay only the same stage and manifest. | Retention worker. |
| Retention object delete fails | Course stays archive-fenced; relational purge is not complete. | Retry exact manifest after the failure clears. | Retention worker. |
| Stale worker attempts a commit | Lease or generation no longer matches. | Reject stale completion without changing current state. | Store transaction. |

"Repair" never means silently inventing bytes from a JSON projection or silently dropping a
reference. Restore procedures require the authoritative source, audit evidence, and normal
publication or delivery validation. Production backup/restore objectives and drills remain
deployment work, not a completed local-code claim.

## Reconciliation status

The write order and retention manifests are implemented. General bucket-to-database reconciliation
is intentionally **planned, not implemented**. WP-RC7 reserves
`2026080909_object_reconciliation.sql` and the following behavior:

- page through bounded bucket inventory without making raw listing data browser-visible;
- register deterministic render/cache references before they can be treated as orphans;
- mark an unreferenced object on first observation, then quarantine/delete only if a later pass
  still observes it beyond the configured policy window;
- cancel a planned deletion if a valid reference appears before removal;
- alert and quarantine delivery for missing or mismatched referenced bytes; and
- make inventory, decisions, and worker replay idempotent, tenant-safe, and safe under concurrent
  creation.

Until that package is accepted, operators must not describe orphan cleanup as automatic. The
planned worker is a repair and observability mechanism, not a license to treat an object-store
listing as an authorization source. Its package, validation, and acceptance gates are owned by
[release_completion_plan.md](active_plans/active/release_completion_plan.md#wp-rc7-reconcile-objects-and-close-m2-through-m5).

## Change rules

Any change to a cross-storage contract is a frozen-contract change. The patch must update the
typed object owner, database transition, direct consumers, conformance/security evidence, and the
contract register together. A migration after durable data uses expand, backfill, verify, switch,
and contract stages; it does not rewrite an applied baseline.

Use [CONTRACTS.md](CONTRACTS.md#frozen-contract-change-rule) for the atomic change rule,
[DATABASE_TENANCY.md](DATABASE_TENANCY.md#context-and-transactions) for transaction/RLS ownership,
and [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md#migration-ledger) for migration compatibility.
