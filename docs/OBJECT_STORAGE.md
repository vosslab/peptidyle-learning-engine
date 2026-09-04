# Object storage

PLE treats object storage as four separate security domains, not as one bucket
with naming conventions. `ObjectAddress` is the only physical-address constructor;
routes and browser payloads name logical delivery IDs, never buckets, paths, or
client-selected filenames.

PLE has one installation-wide Question Library. Storage classification
does not create a second publication audience or a publication tier. The
canonical live-demo path uses these same domains and delivery rules.

## Physical domains

| Domain                        | Object Storage Area | Contents                                                                                                                                                                    | Delivery rule                                                                                                                                                                                                                                                                             |
| ----------------------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Published presentation assets | `PublicAssets`      | Only immutable, answer-free renditions of Published Questions                                                                                                               | CDN-backed delivery is available only after the Question Library publication decision and durable registry are `Ready`, with the exact immutable-public tag and approved-Instructor Question Library access or an allowed Assignment Access decision for the Student's assigned activity. |
| Private content               | `PrivateContent`    | Private workspace Question Source and assets, generation and grader keys or payloads, Question Attempt Reproduction Details, renders, and course-record presentation assets | Never CDN-readable. A protected delivery uses its exact server-derived authority.                                                                                                                                                                                                         |
| Student records               | `StudentRecords`    | Student work, protected course-record artifacts, and annotations                                                                                                            | Never public; delivery requires the exact Student, course, or typed support authority for that record.                                                                                                                                                                                    |
| Temporary processing          | `TempProcessing`    | Conversion workspaces and short-lived course-banner entries                                                                                                                 | Never signable or browser-served.                                                                                                                                                                                                                                                         |

Each Object Storage Area maps to its own provider bucket and KMS key. This physical split is
an enforcement boundary: a public CDN policy cannot expose private workspace
source, Answer Key, Question Feedback, Question Answer Explanation, Question
Grading Input, Student work, or course records. Local MinIO uses
four correspondingly named buckets to preserve the routing contract, but it is
not evidence of AWS IAM, KMS, bucket-policy, Object Lock, or recovery
configuration.

## Typed immutable objects

`crates/objects/src/bucket.rs` derives an Object Address's Object Storage Area, path, and version,
and object identity from typed server values. There is no raw-string address
variant. Important mappings are:

| Object class                                                        | Object Address variants                                                               | Domain and delivery authority                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Private workspace source and imported assets                        | `WorkspaceImportSource`, `WorkspaceQuestionSource`, `WorkspaceImportAsset`            | `PrivateContent`; the Authoring Workspace Owner relationship is required for a private workspace View. Collaboration is a future separately designed capability, not current authority.                                                                                                           |
| Published answer-free presentation asset                            | `QuestionAsset`                                                                       | `PublicAssets`; approved-Instructor Question Library access or an allowed Assignment Access decision for the Student's assigned activity selects the immutable CDN rendition. This does not expose source, Answer Key, Question Feedback, Question Answer Explanation, or Question Grading Input. |
| Published Question Source, import archive, and private render state | `QuestionSource`, `PublishedImportArchive`, `QuestionRender`                          | `PrivateContent`; only an exact server capability or the authorized private workspace Question Source operation may read it.                                                                                                                                                                      |
| Generation/grader keys and payloads                                 | Server-only private records and any typed private object written by its owning worker | `PrivateContent`; only the exact grader, generation, worker lease, or capability may read it.                                                                                                                                                                                                     |
| Course-record presentation asset                                    | `CourseBanner`                                                                        | `PrivateContent`; delivery rechecks the exact current course record and its course relationship.                                                                                                                                                                                                  |
| Student work or protected artifact                                  | `StudentRecord`                                                                       | `StudentRecords`; delivery rechecks exact Student ownership, course Instructor authority, or a narrow audited support capability.                                                                                                                                                                 |
| Course Banner Upload or processing scratch object                   | `CourseBannerUpload`, `Temporary`                                                     | `TempProcessing`; never delivered, signed, or used as a public publication result.                                                                                                                                                                                                                |

Objects are immutable. A write to an existing typed Object Address is refused; replacement
uses a new identity and, for published content, a new immutable version. The
object record carries server-computed SHA-256, size, verified media type,
Source Object Checksum, and creation time. Reads recompute SHA-256 and reject a
mismatch. The checksum detects storage corruption or a substituted object; it
does not authenticate a writer or authorize a reader. Database ownership,
provider bucket/IAM policy, TLS, and publication immutability provide those properties.

Private Workspace Question Source bytes follow the bytes-first rule: the
server writes the typed object, then calls the session-authorized registration
capability with the exact Workspace Question Source Object Address and returned
metadata. PostgreSQL derives Private Content and authoring-content classification
from that address, accepts an identical retry, and rejects a changed address or
immutable record. A Question Source stores that Source Object Reference and
Source Object Checksum as its only source-data representation. Published source
registration remains part of the separate Question Publication operation.
The Draft Question Source Binding Store binds that byte evidence only to an
authorized Draft Question at its exact Edit Number and rechecks the closed Question Backend,
Question Format, and backend-location facts. It returns the earlier Question
Source Binding only when every immutable fact agrees.

The server-only new-lineage Question Publication coordinator resolves that same exact current
Draft Question Source Object Record through an Instructor-session-authorized database operation,
reads and verifies the complete immutable object record, and writes the same bytes under a fresh
Question Revision-owned `QuestionSource` address. Only then does it call the atomic publication
Store with the target record. An existing target address is accepted only when the immutable object
record and bytes agree; collisions cause the coordinator to mint a fresh publication identity.
Because PostgreSQL and object storage do not share a transaction, a later database refusal can leave
an unreachable target object. Object Cleanup and Draft Question expiration own that evidence and
removal; P2 does not claim them, and no Publication Server Route exists in that package.

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

Every delivery selects one server-derived authority. The Object Storage Area and an opaque
object or delivery ID never supply authority by themselves:

1. Approved-Instructor Question Library access delivers safe Question Library
   search and details results and the published presentation assets that they reference.
2. An allowed Assignment Access decision delivers the answer-free
   presentation needed for that Student's assigned activity.
3. The exact Authoring Workspace Owner relationship delivers a private
   workspace source, asset, author preview, or authoring data. Collaboration is
   a future separately designed capability.
4. A typed worker, active lease, or explicit capability delivers generation,
   grading, retention, provider, or other server-only bytes or records. A
   course-record operation includes its exact current course relationship in
   that typed check.

`GET /api/assets/{id}` can return only an already-ready published presentation
asset after the route proves approved-Instructor Question Library access or the exact
Assignment Access decision. It resolves an opaque registry ID, verifies
the complete trusted `QuestionAsset`/`PublicAssets` record shape, then
redirects to a configured immutable CDN URL. It cannot authorize, audit, or
issue a protected bearer URL, and it returns the same not-found response for
protected and absent IDs.

Published presentation assets are not anonymous internet content. Delivering
one through an approved authority does not grant Question Library search,
details, or delivery of another asset. Question Library search and details
require authenticated approved-Instructor access. A Student receives an
assigned presentation through an allowed Assignment Access decision and does not
receive Question Library access.

`POST /api/assets/{id}/delivery` is the separate protected path. It requires a
same-origin authenticated session, reauthorizes the exact Account, course,
Student, workspace, and object relationship required by the selected typed
scope, records a minimized access event, and returns a short-lived URL in
JSON. It refuses published presentation assets so there is no second,
stateful public path. Private-content URLs are at most 60 minutes;
Student-record URLs are at most five minutes. Protected responses use
`no-store`, `Pragma: no-cache`, and `Referrer-Policy: no-referrer`. Temporary
objects are never signable.

The route never accepts an Object Storage Area, Object Address, checksum, or filename. A signed
URL is a short-lived bearer capability, not a durable browser datum: clients
must not place it in browser storage, analytics, a referrer chain, or logs.

## Public-asset publication

Public publication is intentionally not a pre-commit object-store upload.
PostgreSQL and object storage do not share a transaction, so Question Library
publication atomically:

1. commits immutable Question Library publication state, `Pending` asset-delivery records, and a
   closed `PublishPublicAssets` outbox job; and
2. makes no final public object or CDN-visible registry transition in that
   transaction.

The dedicated publisher subsequently claims only that job kind, re-reads each
pending record and its exact Question Revision-owned private source or asset, verifies
the Source Object Reference and Source Object Checksum, and writes the final public key. It uses
immutable creation semantics. A retry accepts an existing final key only when
its exact record and checksum agree. Finally, a lease-conditional database
function changes the complete batch from `Pending` to `Ready` and completes
that same job atomically. Pending records have no public route result.

The pending publication input is an exact allowlist of Question Revision-owned
private objects created by publication. It has no Draft Question relationship
or Authoring Workspace path and remains complete after draft cleanup. It is
never an arbitrary private key, browser value, or queue payload byte sequence.
The dedicated publisher has a separate database capability and
production IAM role; ordinary API and worker roles cannot write public objects.
The publisher writes and verifies immutable public objects before activation.
This closes pre-commit CDN orphans and a confused deputy that could
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
publisher lease/retry behavior. General Object Storage Checks remain planned:
an orphan is never served, and a missing or checksum-mismatched referenced
object fails closed and retains its database evidence until repair. Production
KMS rotation, bucket policies, Object Lock retention, backup restore, and IAM
are deployment evidence, not properties demonstrated by `MemoryObjectStore`.
