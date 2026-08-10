# Object storage

PLE's implemented object layer stores immutable binary and archival payloads through typed object
storage. The contract keeps physical paths, bucket selection, checksums, and delivery policy on
the server, so browser data names only logical assets and never a storage location. Learner file
uploads are deliberately not an exception: they remain fail-closed until their separately planned,
attempt-bound capability is implemented.

## Typed objects

[crates/objects/src/bucket.rs](../crates/objects/src/bucket.rs) is the sole constructor for an
`ObjectKey`. Callers supply typed IDs rather than a raw bucket name or path. A key derives its
bucket, immutable path, object ID, category, and, where applicable, published version. Immutable
writes reject an existing key; reads verify SHA-256 before returning bytes.

| Bucket            | Implemented roles | Delivery rule |
| ----------------- | ----------------- | ------------- |
| `content` | Private workspace sources/assets, published source archives/assets, deterministic renders, and retained course banners | The object layer can sign published assets, renders, and course banners. The delivery registry currently exposes only catalog assets and an exact current course banner. |
| `student-records` | Tenant-owned export artifacts represented by `StudentRecord` | Requires authenticated, RLS-scoped, explicit authorization. |
| `temp-processing` | Generic temporary processing data and short-lived course-banner candidates | Never signable or publicly served. |

The `student-records` bucket is intentionally ready for more educational-record classes, but it
does **not** yet accept learner uploads or general annotations. The active
[secure learner-upload plan](active_plans/active/secure_learner_file_upload_plan.md) reserves
attempt-bound candidate and durable upload key types, a distinct `LearnerSubmission` category,
streaming writes, inspection, protected delivery, retention, and reconciliation. Until that work
is accepted, a file-upload response fails closed before any object write.

The `content` bucket is not synonymous with public data. For example, a private workspace import
and a published source archive use durable `content` keys but cannot receive a generic delivery
URL. A banner candidate uses `temp-processing`; promotion produces a distinct immutable
`CourseBanner` key in `content`.

## Key and delivery matrix

`ObjectKey` has no raw-string variant. The following is the current complete key vocabulary;
adding a new object class requires a new typed variant and its exact ownership rules.

| Key family | Bucket | Category | Version-pinned | Object-store signable | Current route delivery |
| --- | --- | --- | --- | --- | --- |
| `WorkspaceSource`, `WorkspaceQuestionSource`, `WorkspaceAsset` | `content` | source or asset | no | no | never |
| `ProblemSource`, `PublishedImportArchive` | `content` | source | yes | no | never |
| `ProblemAsset` | `content` | asset | yes | yes | public catalog CDN after registry validation |
| `ProblemRender` | `content` | render | yes | yes | not registered by the current asset route |
| `CourseBannerCandidate`, `Temporary` | `temp-processing` | temporary | no | no | never |
| `CourseBanner` | `content` | course content | no | yes | protected only when it is the course's exact current banner |
| `StudentRecord` | `student-records` | export | no | yes | protected only through an explicit student-record grant |

Object-store signability is only a necessary capability. It never grants browser access by itself:
the `asset_delivery` registry decides whether a typed object currently has a browser route and the
route reauthorizes protected reads. In particular, a `ProblemRender` is signable at the storage
layer but is not a general learner-facing asset under the present delivery contract.

## Immutable record

Each successful `put` returns an [ObjectRecord](../crates/objects/src/lib.rs) with the durable
object ID, typed bucket and key, computed SHA-256, byte size, verified media type, category,
optional published version, license, provenance, and server creation time. The checksum is
computed on write and rechecked on read. `ObjectRecord` is backend-neutral immutable metadata,
not a universal database table: its owning workflow persists the required record, such as an
asset-delivery registration, published-import provenance, or retention manifest. Metadata records
provenance and handling context; it does not make object bytes browser-visible.

Bytes are written before the database record. The database is authoritative for intended object
existence: a record without its bytes is a broken reference that must be alerted on, not hidden by
deleting the record. The bucket is authoritative for bytes: bytes without a record are orphans
that may be collected after the reconciliation quarantine policy. This is the committed ownership
rule in [implementation_plan.md](active_plans/implementation_plan.md#database-or-object-store-who-owns-existence).

## Publication and record boundaries

Draft workspace source, extracted assets, and staged imports are tenant-private and use workspace
keys. Publication creates version-pinned `ProblemSource`, `PublishedImportArchive`, `ProblemAsset`,
and `ProblemRender` objects. Their typed paths bind the relevant problem, immutable version, and
logical asset or seed. Published content is shared and immutable; private source provenance is
not a learner-facing asset.

Student records are a different boundary: `StudentRecord` keys include the owning tenant, and the
`asset_delivery` schema binds their delivery to a tenant and course. Retention code validates an
exact typed-object manifest before deleting tenant-owned record artifacts, while shared published
content and drafts remain outside the student-record deletion path. See
[2026080805_operations_analytics.sql](../schemas/migrations/2026080805_operations_analytics.sql),
[2026080806_retention.sql](../schemas/migrations/2026080806_retention.sql), and
[SECURITY_MODEL.md](SECURITY_MODEL.md#asset-delivery-boundary).

Course-banner candidates are a separate short-lived authorization boundary. They are scoped to one
tenant and course, non-signable, and tracked in `course_banner_candidate`; only a verified save
can promote bytes to the immutable course-banner record. See
[2026080907_course_appearance.sql](../schemas/migrations/2026080907_course_appearance.sql) and
[CONTRACTS.md](CONTRACTS.md#course-appearance-contract).

## Delivery grants

The stable route is `GET /api/assets/{id}`. The database registry maps that opaque delivery ID to
one exact `ObjectRecord` and an `AssetDeliveryScope`; the route does not accept a bucket, key,
checksum, or filename from the client.

- A globally visible catalog `ProblemAsset` redirects to the configured immutable CDN URL only
  after the trusted record shape matches the exact published asset.
- Institution catalog content, student records, and course banners require the opaque HttpOnly
  session. Forced RLS limits the lookup; student-record access also checks the explicit user grant.
- A successful protected authorization records tenant, actor, delivery ID, object ID, bucket,
  optional course, and database time before a signed URL is requested. URLs and session data are
  never included in that audit event.
- `content` signed URLs are at most 60 minutes and `student-records` URLs at most 5 minutes.
  Protected redirects use no-store, no-cache, and no-referrer headers. `temp-processing` is
  rejected even if a caller reaches the route.

These contracts live in [asset_delivery.rs](../crates/learning-data-access/src/asset_delivery.rs)
and are enforced by [asset.rs](../crates/server/src/asset.rs). The security rationale and headers
are specified in [SECURITY_MODEL.md](SECURITY_MODEL.md#asset-delivery-boundary).

## Backends and lifecycle status

Tests use `MemoryObjectStore`. The local container stack configures an S3-compatible MinIO endpoint
through `PLE_S3_ENDPOINT`, region, credentials, and the three bucket names; the same
[S3ObjectStore](../crates/objects/src/s3.rs) implementation serves an AWS S3 endpoint without
exposing AWS SDK types through the `ObjectStore` trait. The MinIO client uses path-style requests.
Production composition requires names for all three buckets, but its current readiness route makes
one authorized `HeadBucket` request for the `content` bucket only. That proves the configured
endpoint and primary content bucket are available; it is not evidence that `student-records` and
`temp-processing` are reachable. Operations work should extend readiness to all three buckets
before treating a full object-storage outage check as complete. See
[minio.rs](../crates/objects/src/minio.rs) and
[composition/router.rs](../crates/server/src/composition/router.rs).

Object reconciliation is planned, not complete. WP-RC7 reserves bounded bucket inventory,
twice-observed orphan quarantine and deletion, missing/mismatched-byte alerts and delivery
quarantine, and an idempotent tenant-safe worker. Its reserved migration is
`2026080909_object_reconciliation.sql`; the named implementation files and acceptance gate remain
unlanded. The current release plan marks WP-RC7 unchecked, so this document does not treat
reconciliation or the combined M2-M5 gate as accepted. See
[release_completion_plan.md](active_plans/active/release_completion_plan.md#wp-rc7-reconcile-objects-and-close-m2-through-m5)
and [implementation_status.md](active_plans/implementation_status.md#dependency-ordered-remaining-work).
