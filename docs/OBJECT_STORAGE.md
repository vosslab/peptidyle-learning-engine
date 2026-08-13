# Object storage

PLE treats object storage as four separate security domains, not as one bucket
with naming conventions. `ObjectKey` is the only physical-key constructor;
routes and browser payloads name logical delivery IDs, never buckets, paths, or
client-selected filenames.

## Physical domains

| Domain | Typed bucket | Contents | Delivery rule |
| --- | --- | --- | --- |
| Public assets | `PublicAssets` | Only public, immutable `ProblemAsset` renditions | CDN-readable after the durable registry is `Ready` and the object has the exact immutable-public tag. |
| Private content | `PrivateContent` | Authoring, source archives, answer-bearing provenance, renders, institution-only `RestrictedProblemAsset` objects, and course banners | Never CDN-readable. A protected delivery is authorized per request. |
| Student records | `StudentRecords` | Tenant-owned exports and the reserved future student-artifact class | Never public; a protected delivery requires RLS-scoped authorization and an explicit grant. |
| Temporary processing | `TempProcessing` | Conversion workspaces and course-banner candidates | Never signable or browser-served. |

Each production domain has its own bucket and KMS key. This physical split is
an enforcement boundary: a public CDN policy cannot accidentally expose an
institution-only asset merely because both are published content. Local MinIO
uses four correspondingly named buckets to preserve the same routing contract,
but is not evidence of AWS IAM, KMS, bucket-policy, Object Lock, or recovery
configuration.

## Typed immutable objects

`crates/objects/src/bucket.rs` derives a key's bucket, path, category,
version, and object identity from typed server values. There is no raw-string
key variant. Important mappings are:

| Key family | Domain | Browser delivery |
| --- | --- | --- |
| `WorkspaceSource`, `WorkspaceQuestionSource`, `WorkspaceAsset`, `WorkspaceQuestionAsset`, `ProblemSource`, `PublishedImportArchive`, `ProblemRender` | Private content | Never generic delivery. |
| `ProblemAsset` | Public assets | Public CDN path only after publication activation. |
| `RestrictedProblemAsset` | Private content | Protected delivery only; never a CDN key. |
| `CourseBanner` | Private content | Protected delivery only when it is the exact current course banner. |
| `StudentRecord` | Student records | Protected delivery only through an explicit grant. |
| `CourseBannerCandidate`, `Temporary` | Temporary processing | Never delivered. |

Objects are immutable. A write to an existing typed key is refused; replacement
uses a new identity and, for published content, a new immutable version. The
object record carries server-computed SHA-256, size, verified media type,
category, provenance, and creation time. Reads recompute SHA-256 and reject a
mismatch. The checksum detects storage corruption or a substituted object; it
does not authenticate a writer or authorize a reader. Database ownership,
bucket/IAM policy, TLS, and publication immutability provide those properties.

## Instructional image boundary

Images are hostile input even when they came from an instructor. The shared
`objects::image_validation` boundary accepts only complete, single-container
PNG, JPEG, or WebP still images. It enforces an 8 MiB input limit and a
20-million-pixel decoded limit before a bounded full decode; rejects GIF and
other formats, animation, zero dimensions, malformed containers, and trailing
container bytes (including JPEG data after EOI). The measured media type and
dimensions, rather than a filename or request `Content-Type`, are registered
with the immutable asset.

This is a content-safety and parser-confusion boundary, not a malware scanner.
The system does not claim to make arbitrary files safe for every downstream
consumer; it only admits the strict still-image formats that its own rendering
paths support.

## Delivery boundary

`GET /api/assets/{id}` can return only an already-ready public catalog asset.
It resolves an opaque registry ID, verifies the complete trusted
`ProblemAsset`/`PublicAssets` record shape, then redirects to a configured
immutable CDN URL. It cannot authorize, audit, or issue a protected bearer
URL, and returns the same not-found response for protected and absent IDs.

`POST /api/assets/{id}/delivery` is the separate protected path. It requires a
same-origin authenticated session, reauthorizes the tenant/actor/object
relationship, records a minimized access event, and returns a short-lived URL
in JSON. It refuses public assets so there is no second, stateful public path.
Private-content URLs are at most 60 minutes; student-record URLs are at most
five minutes. Protected responses use `no-store`, `Pragma: no-cache`, and
`Referrer-Policy: no-referrer`. Temporary objects are never signable.

The route never accepts a bucket, object key, checksum, or filename. A signed
URL is a short-lived bearer capability, not a durable browser datum: clients
must not place it in browser storage, analytics, a referrer chain, or logs.

## Public-asset publication

Public publication is intentionally not a pre-commit S3 upload. PostgreSQL and
object storage do not share a transaction, so catalog publication atomically:

1. commits immutable catalog state, `Pending` asset-delivery records, and a
   closed `PublishPublicAssets` outbox job; and
2. makes no final public object or CDN-visible registry transition in that
   transaction.

The dedicated publisher subsequently claims only that job kind, re-reads each
pending record and its exact private source from PostgreSQL, verifies the
source record and SHA-256, and writes the final public key. It uses immutable
creation semantics. A retry accepts an existing final key only when its exact
record and checksum agree. Finally, a lease-conditional database function
changes the complete batch from `Pending` to `Ready` and completes that same
job atomically. Pending records have no public route result.

The pending source is an exact allowlist: a private `WorkspaceAsset` or
`WorkspaceQuestionAsset` with the expected tenant/workspace/object/category
and no published version. It is never a public object, arbitrary private key,
browser value, or queue payload byte sequence. The dedicated publisher has a
separate database capability and production IAM role; ordinary API and worker
roles cannot materialize public objects. This closes both pre-commit CDN
orphans and a confused deputy that could copy arbitrary private data into the
public domain.

Production infrastructure enforces immutable publication tags, conditional
create (`If-None-Match: *`), and bucket/IAM policies. The public bucket is
created with Object Lock enabled, but has no default legal-retention period:
the active append-only guarantee is the immutable-tag policy so disposable
rehearsals remain recoverable. Any legal-retention rule is a separate reviewed
operations decision. Code requests and verifies the exact public-object tag
before public use, but an AWS deployment is not verified until its
infrastructure tests and live policy inspection pass.

## Encryption and lifecycle evidence

Production S3 composition requires SSE-KMS for every object write and verifies
the returned encryption headers. Encryption at rest is the baseline for all
four domains and their backups. PLE deliberately does not encrypt every
public asset again in application code: public objects must be CDN-readable,
and a blanket application layer would add key-handling risk without supplying
an access-control property that the public domain intentionally lacks.

Private or student-specific application payload encryption is a separate
design decision when a field needs protection from storage administrators or a
specific downstream processor; it is not implied by the object checksum.
Secrets and provider state use their own server-side protection boundaries.

The current repository validates typed routing, immutable writes, checksums,
strict image admission, delivery separation, pending-publication behavior, and
publisher lease/retry behavior. General object reconciliation remains planned:
an orphan is never served, and a missing or checksum-mismatched referenced
object fails closed and retains its database evidence until repair. Production
KMS rotation, bucket policies, Object Lock retention, backup restore, and IAM
are deployment evidence, not properties demonstrated by `MemoryObjectStore`.
