# Object storage

PLE treats object storage as four separate security domains, not as one bucket
with naming conventions. `ObjectAddress` is the only physical-address constructor;
routes and browser payloads name logical delivery IDs, never buckets, paths, or
client-selected filenames.

PLE has one installation-wide Question Library. Storage classification
does not create a second publication audience or a publication tier. The
canonical live-demo path uses these same domains and delivery rules.

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) owns product lifecycle and retention.
The job, registry, object-address, and current `assignment` names below are
implementation mechanisms; they do not authorize new product states or a
generic background-work model.

## Physical domains

| Domain                        | Object Storage Area | Contents                                                                                                                                                                    | Delivery rule                                                                                                                                                                                                                                                                             |
| ----------------------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Published Question Image Renditions | `PublicAssets`      | Only immutable, answer-free Question Image Renditions of Published Questions                                                                                                 | Public delivery is available only after the Question Library publication decision and durable registry are `Ready`, with the exact immutable-public tag and vetted-Instructor Question Library access or an allowed Assessment access decision for the Student's Coursework. |
| Private content               | `PrivateContent`    | Private Draft Question source and Question Image Assets, backend state or grading inputs, and Course Banners                                                               | Never publicly readable. A protected delivery uses its exact server-derived authority. |
| Student records               | `StudentRecords`    | Student work, protected course-record artifacts, and annotations                                                                                                            | Never public; delivery requires the exact Student, course, or typed support authority for that record.                                                                                                                                                                                    |
| Temporary processing          | `TempProcessing`    | Conversion workspaces and short-lived course-banner entries                                                                                                                 | Never signable or browser-served.                                                                                                                                                                                                                                                         |

The current runtime supports only `PLE_STORAGE_TOPOLOGY=disposable-local`: an
explicitly configured, authenticated MinIO endpoint through the typed
S3-compatible adapter. Missing, empty, or unknown topology and configuration
values fail before a client is configured. It uses four configured bucket names
for these areas. The API/worker identity and the public-asset publisher identity
use distinct configured credentials.

The four domains remain a security requirement for any production deployment.
It must enforce separate provider buckets and encryption keys so public delivery
cannot expose private workspace source, Answer Key, Question Feedback, Question
Answer Explanation, Question Grading Input, Student work, or course records.
The disposable MinIO topology preserves typed routing; it is not evidence of
cloud IAM, KMS, bucket policy, Object Lock, backup, or recovery controls.

## Typed immutable objects

`crates/objects/src/bucket.rs` derives an Object Address's Object Storage Area, path, and version,
and object identity from typed server values. There is no raw-string address
variant. Important mappings are:

| Object class                                                        | Object Address variants                                                               | Domain and delivery authority                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Private workspace source and imported QTI media                     | `WorkspaceImportSource`, `WorkspaceQuestionSource`, `WorkspaceImportExtractedImage`   | `PrivateContent`; the Authoring Workspace Owner relationship is required for a private workspace View. Collaboration is a future separately designed capability, not current authority.                                                                                                           |
| Published answer-free Question Image                                | `QuestionImage`                                                                       | `PublicAssets`; vetted-Instructor Question Library access or an allowed Assessment access decision selects the immutable public rendition. This does not expose source, Answer Key, Question Feedback, Question Answer Explanation, or grading input. |
| Published Question Source, import archive, and private render state | `QuestionSource`, `PublishedImportArchive`, `QuestionRender`                          | `PrivateContent`; only an exact server capability or the authorized private workspace Question Source operation may read it.                                                                                                                                                                      |
| Generation/grader keys and payloads                                 | Server-only private records and any typed private object written by its owning worker | `PrivateContent`; only the exact grader, generation, worker lease, or capability may read it.                                                                                                                                                                                                     |
| Course Banner                                                       | `CourseBanner`                                                                        | `PrivateContent`; delivery rechecks the exact current course record and its course relationship.                                                                                                                                                                                                  |
| Student work or protected artifact                                  | `StudentRecord`                                                                       | `StudentRecords`; delivery rechecks exact Student ownership, course Instructor authority, or a narrow audited support capability.                                                                                                                                                                 |
| Course Banner Upload or processing scratch object                   | `CourseBannerUpload`, `Temporary`                                                     | `TempProcessing`; never delivered, signed, or used as a public publication result.                                                                                                                                                                                                                |

Objects are immutable. A write to an existing typed Object Address is refused; replacement
uses a new identity and, for published content, a new immutable version. The
object record carries server-computed SHA-256, size, verified media type,
Source Object Checksum, and creation time. Reads recompute SHA-256 and reject a
mismatch. The checksum detects storage corruption or a substituted object; it
does not authenticate a writer or authorize a reader. Database ownership, TLS,
publication immutability, and deployment-specific provider policy provide those
properties.

Private Workspace Question Source bytes follow the bytes-first rule: the
server writes the typed object, then calls the session-authorized registration
capability with the exact Workspace Question Source Object Address and returned
metadata. PostgreSQL derives Private Content and authoring-content classification
from that address, accepts an identical retry, and rejects a changed address or
immutable record. A Question Source stores that Source Object ID and
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
   search and details results and the published Question Image Renditions that they reference.
2. An allowed Assessment access decision delivers the answer-free
   presentation needed for that Student's Coursework.
3. The exact Authoring Workspace Owner relationship delivers a private
   workspace source, asset, author preview, or authoring data. Collaboration is
   a future separately designed capability.
4. A typed worker, active lease, or explicit capability delivers generation,
   grading, retention, provider, or other server-only bytes or records. A
   course-record operation includes its exact current course relationship in
   that typed check.

`GET /api/assets/{id}` can return only an already-ready published presentation
asset after the route proves vetted-Instructor Question Library access or the exact
Assessment access decision. It resolves an opaque registry ID, verifies
the complete trusted `QuestionImage`/`PublicAssets` record shape, then
redirects to a configured immutable public URL. It cannot authorize, audit, or
issue a protected bearer URL, and it returns the same not-found response for
protected and absent IDs.

Published Question Image Renditions are not anonymous internet content. Delivering
one through an approved authority does not grant Question Library search,
details, or delivery of another asset. Question Library search and details
require authenticated approved-Instructor access. A Student receives an
Assessment presentation through an allowed Assessment access decision and does not
receive Question Library access.

`POST /api/assets/{id}/delivery` is the separate protected path. It requires a
same-origin authenticated session, reauthorizes the exact Account, course,
Student, workspace, and object relationship required by the selected typed
scope, records a minimized access event, and returns a short-lived URL in
JSON. It refuses published Question Image Renditions so there is no second,
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
2. makes no final public object or public-delivery registry transition in that
   transaction.

The dedicated publisher subsequently claims only that job kind, re-reads each
pending record and its exact Question Revision-owned private source or asset, verifies
the Source Object ID and Source Object Checksum, and writes the final public key. It uses
immutable creation semantics. A retry accepts an existing final key only when
its exact record and checksum agree. Finally, a lease-conditional database
function changes the complete batch from `Pending` to `Ready` and completes
that same job atomically. Pending records have no public route result.

The pending publication input is an exact allowlist of Question Revision-owned
private objects created by publication. It has no Draft Question relationship
or Authoring Workspace path and remains complete after draft cleanup. It is
never an arbitrary private key, browser value, or queue payload byte sequence.
The dedicated publisher has a separate database capability and distinct
S3-compatible credentials from the API/worker identity. It writes and verifies
immutable public objects before activation.
This closes pre-commit public-object orphans and a confused deputy that could
copy arbitrary private data into the public domain.

Any future production deployment must enforce immutable publication tags,
conditional create (`If-None-Match: *`), and provider policy. Object Lock and
any legal-retention rule remain separate reviewed operations decisions. The
current MinIO topology does not establish those production controls. A cloud
deployment is unimplemented and requires infrastructure tests plus live policy
inspection before it can be accepted.

## Encryption and lifecycle evidence

Future cloud production support must require encryption at rest for every
object write, verify the provider's returned encryption evidence, and protect
all four domains and their backups with independently managed keys. PLE does
not encrypt every published Question Image Rendition again in application code.
Public objects must be publicly readable after their authorization gate, and a
blanket application layer would add key-handling risk without supplying an
access-control property that this public class lacks.

Private or Student-specific application payload encryption is a separate
design decision when a field needs protection from storage administrators or a
specific downstream processor; it is not implied by the object checksum.
Credentials, provider state, and generation or grader payloads use their own
server-side protection boundaries.

The current implementation covers typed routing, immutable writes, checksums,
strict image admission, delivery separation, pending-publication behavior, and
publisher lease/retry behavior in the disposable-local MinIO topology. General
Object Storage Checks remain planned: an orphan is never served, and a missing
or checksum-mismatched referenced object fails closed and retains its database
evidence until repair. Cloud encryption-key rotation, provider policy, Object
Lock retention, backup restore, and identity authorization are future
deployment evidence, not properties demonstrated by MinIO or
`MemoryObjectStore`.
