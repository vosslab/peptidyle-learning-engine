# Object storage

PLE treats object storage as four separate security domains, not as one bucket
with naming conventions. `ObjectKey` is the only physical-key constructor;
routes and browser payloads name logical delivery IDs, never buckets, paths, or
client-selected filenames.

PLE has one installation-wide published-question catalog. Storage classification
does not create a second publication audience or a publication tier. The
canonical live-demo path uses these same domains and delivery rules.

## Physical domains

| Domain | Typed bucket | Contents | Delivery rule |
| --- | --- | --- | --- |
| Published presentation assets | `PublicAssets` | Only immutable, answer-free renditions of published questions | CDN-backed delivery is available only after the catalog publication decision and durable registry are `Ready`, with the exact immutable-public tag and an approved delivery authority. |
| Private content | `PrivateContent` | Private workspace source and assets, generation and grader keys or payloads, provenance, renders, and course-record presentation assets | Never CDN-readable. A protected delivery uses its exact server-derived authority. |
| Student records | `StudentRecords` | Student work and protected course-record artifacts, exports, and annotations | Never public; delivery requires the exact Student, course, or typed support authority for that record. |
| Temporary processing | `TempProcessing` | Conversion workspaces and short-lived course-banner candidates | Never signable or browser-served. |

Each production domain has its own bucket and KMS key. This physical split is
an enforcement boundary: a public CDN policy cannot expose private workspace
source, grading material, Student work, or course records. Local MinIO uses
four correspondingly named buckets to preserve the routing contract, but it is
not evidence of AWS IAM, KMS, bucket-policy, Object Lock, or recovery
configuration.

## Typed immutable objects

`crates/objects/src/bucket.rs` derives a key's bucket, path, category, version,
and object identity from typed server values. There is no raw-string key
variant. Important mappings are:

| Object class | Key family | Domain and delivery authority |
| --- | --- | --- |
| Private workspace source and authoring assets | `WorkspaceSource`, `WorkspaceQuestionSource`, `WorkspaceAsset`, `WorkspaceQuestionAsset` | `PrivateContent`; the creating Instructor's exact workspace ownership is required for a private workspace projection. Collaboration is a future separately designed capability, not current authority. |
| Published answer-free presentation asset | `ProblemAsset` | `PublicAssets`; approved-Instructor catalog authority or exact Student assignment entitlement selects the immutable CDN rendition. This does not expose source or grading material. |
| Published source, provenance, and private render state | `ProblemSource`, `PublishedImportArchive`, `ProblemRender` | `PrivateContent`; only an exact server capability or the authorized private workspace/provenance operation may read it. |
| Generation/grader keys and payloads | Server-only private records and any typed private object used by their owning worker | `PrivateContent` when materialized; only the exact grader, generation, worker lease, or capability may read it. |
| Course-record presentation asset | `CourseBanner` | `PrivateContent`; delivery rechecks the exact current course record and its course relationship. |
| Student work or protected artifact | `StudentRecord` | `StudentRecords`; delivery rechecks exact Student ownership, course Instructor authority, or a narrow audited support capability. |
| Processing candidate or scratch object | `CourseBannerCandidate`, `Temporary` | `TempProcessing`; never delivered, signed, or used as a public publication result. |

Objects are immutable. A write to an existing typed key is refused; replacement
uses a new identity and, for published content, a new immutable version. The
object record carries server-computed SHA-256, size, verified media type,
category, provenance, and creation time. Reads recompute SHA-256 and reject a
mismatch. The checksum detects storage corruption or a substituted object; it
does not authenticate a writer or authorize a reader. Database ownership,
bucket/IAM policy, TLS, and publication immutability provide those properties.

## Instructional image boundary

Images are hostile input even when they came from an Instructor. The shared
`objects::image_validation` boundary accepts only complete, single-container
PNG, JPEG, or WebP still images. It enforces an 8 MiB input limit and a
20-million-pixel decoded limit before a bounded full decode; rejects GIF and
other formats, animation, zero dimensions, malformed containers, and trailing
container bytes (including JPEG data after EOI). The measured media type and
dimensions, rather than a filename or request `Content-Type`, are registered
with the immutable asset.

This is a content-safety and parser-confusion boundary, not a malware scanner.
The system does not claim to make arbitrary files safe for every downstream
consumer; it admits the strict still-image formats that its own presentation
paths support.

## Delivery authority

Every delivery selects one server-derived authority. The bucket and an opaque
object or delivery ID never supply authority by themselves:

1. The approved-Instructor catalog authority delivers the safe catalog
   projection and the published presentation assets that it references.
2. The exact Student assignment entitlement delivers the answer-free
   presentation needed for that Student's assigned activity.
3. The creating Instructor's exact workspace ownership delivers a private
   workspace source, asset, preview, or authoring projection. Collaboration is
   a future separately designed capability.
4. A typed worker, active lease, or explicit capability delivers generation,
   grading, export, retention, provider, or other server-only material. A
   course-record operation includes its exact current course relationship in
   that typed check.

`GET /api/assets/{id}` can return only an already-ready published presentation
asset after the route proves approved-Instructor catalog authority or the exact
Student assignment entitlement. It resolves an opaque registry ID, verifies
the complete trusted `ProblemAsset`/`PublicAssets` record shape, then
redirects to a configured immutable CDN URL. It cannot authorize, audit, or
issue a protected bearer URL, and it returns the same not-found response for
protected and absent IDs.

Published presentation assets are not anonymous internet content. Delivering
one through an approved authority does not grant catalog discovery or delivery
of another asset. Catalog list, search, and detail require authenticated
approved-Instructor authority. A Student receives an assigned presentation
through the exact assignment entitlement and does not receive catalog
authority.

`POST /api/assets/{id}/delivery` is the separate protected path. It requires a
same-origin authenticated session, reauthorizes the exact Account, course,
Student, workspace, and object relationship required by the selected typed
scope, records a minimized access event, and returns a short-lived URL in
JSON. It refuses published presentation assets so there is no second,
stateful public path. Private-content URLs are at most 60 minutes;
Student-record URLs are at most five minutes. Protected responses use
`no-store`, `Pragma: no-cache`, and `Referrer-Policy: no-referrer`. Temporary
objects are never signable.

The route never accepts a bucket, object key, checksum, or filename. A signed
URL is a short-lived bearer capability, not a durable browser datum: clients
must not place it in browser storage, analytics, a referrer chain, or logs.

## Public-asset publication

Public publication is intentionally not a pre-commit object-store upload.
PostgreSQL and object storage do not share a transaction, so catalog
publication atomically:

1. commits immutable catalog state, `Pending` asset-delivery records, and a
   closed `PublishPublicAssets` outbox job; and
2. makes no final public object or CDN-visible registry transition in that
   transaction.

The dedicated publisher subsequently claims only that job kind, re-reads each
pending record and its exact private workspace source from PostgreSQL, verifies
the source record and SHA-256, and writes the final public key. It uses
immutable creation semantics. A retry accepts an existing final key only when
its exact record and checksum agree. Finally, a lease-conditional database
function changes the complete batch from `Pending` to `Ready` and completes
that same job atomically. Pending records have no public route result.

The pending source is an exact allowlist: a private workspace asset with the
expected workspace, object, category, and no published version. It is never a
public object, arbitrary private key, browser value, or queue payload byte
sequence. The dedicated publisher has a separate database capability and
production IAM role; ordinary API and worker roles cannot materialize public
objects. This closes pre-commit CDN orphans and a confused deputy that could
copy arbitrary private data into the public domain.

Production infrastructure enforces immutable publication tags, conditional
create (`If-None-Match: *`), and bucket/IAM policies. The public bucket is
created with Object Lock enabled, but has no default legal-retention period:
the active append-only guarantee is the immutable-tag policy so disposable
exercises remain recoverable. Any legal-retention rule is a separate reviewed
operations decision. Code requests and verifies the exact public-object tag
before public use, but an AWS deployment is not verified until its
infrastructure tests and live policy inspection pass.

## Encryption and lifecycle evidence

Production S3 composition requires SSE-KMS for every object write and verifies
the returned encryption headers. Encryption at rest is the baseline for all
four domains and their backups. PLE deliberately does not encrypt every
published presentation asset again in application code: public objects must
be CDN-readable, and a blanket application layer would add key-handling risk
without supplying an access-control property that this public class lacks.

Private or Student-specific application payload encryption is a separate
design decision when a field needs protection from storage administrators or a
specific downstream processor; it is not implied by the object checksum.
Credentials, provider state, and generation or grader payloads use their own
server-side protection boundaries.

The current repository validates typed routing, immutable writes, checksums,
strict image admission, delivery separation, pending-publication behavior, and
publisher lease/retry behavior. General object reconciliation remains planned:
an orphan is never served, and a missing or checksum-mismatched referenced
object fails closed and retains its database evidence until repair. Production
KMS rotation, bucket policies, Object Lock retention, backup restore, and IAM
are deployment evidence, not properties demonstrated by `MemoryObjectStore`.
