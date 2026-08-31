> **Historical review input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). The M0 result is concluded evidence.

LLM reviewer feedback

1. The revised direction is sound. ADAPT confirms that a simple SQL-plus-S3 split is not enough. The design also needs content integrity, versioning, and secure separation of grading material.
2. Treat answer-bearing content as a separate security class. Question text, media, answer keys, grading logic, generated parameters, and student-visible renders should not share the same access path.
3. Decide which source_object_reference is authoritative for each backend. For example, QTI ZIP, parsed QTI model, WeBWorK source, generated question instance, and rendered output should each have a clear source-of-truth role.
4. Add immutable object identity early. Use checksums, version IDs, content type, size, ownership, license, and provenance for every stored source_object_reference, including text stored in PostgreSQL.
5. Reconsider storing large XML, JSON, and source payloads directly in operational tables. Metadata and searchable fields belong in PostgreSQL. Immutable source packages and large versioned payloads may fit better in object storage.
6. Define reproducibility as a requirement. A historical attempt should identify the exact question revision, source source_object_reference, renderer version, seed, parameters, assets, and grading implementation.
7. Make secure rendering a first-class service boundary. The student-facing application should receive only the minimum rendered content and attempt token needed for the active question.
8. The rewritten plan should convert these decisions into explicit tasks with files or components, expected behavior, success conditions, and validation cases before implementation begins.

These are LLM reviewer comments, not final design decisions.

That suggests separating identity from publication status.

ADAPT gives every saved problem a durable official ID. That makes reuse easy, but it also elevates drafts, experiments, imports, and junk into permanent catalog objects.

A better model would use two layers:

- Draft object: private, disposable, editable, and not globally discoverable
- Published problem version: immutable, shareable, and assigned a durable public ID

Possible lifecycle:

draft
-> validated
-> published
-> deprecated
-> archived

Only publication would create the long-lived problem identity used across courses.

You could also distinguish:

problem_id
version_id
workspace_id

- workspace_id identifies the instructor sandbox item
- problem_id identifies the reusable published problem
- version_id identifies the exact immutable revision

This preserves ADAPT's strongest feature, easy sharing across classes, without filling the official catalog with abandoned experiments.

A useful rule would be:

Drafts are cheap and temporary. Published problems are durable and versioned.

The sandbox could still assign internal UUIDs immediately, but those would not be presented as official problem numbers.

binary blobs always concern me. databases sizes grow quickly, with modern storage seeking is quick, but still concerns me

That concern is valid. I would keep binary data out of PostgreSQL unless the files are tiny and transactional coupling clearly matters.

A practical rule:

- PostgreSQL: metadata, normalized text, JSON, grading rules, checksums, version links
- Object storage: ZIP, images, audio, PDFs, DOCX, H5P packages, large XML, large source bundles
- Temporary disk: extraction, conversion, scanning, rendering

The database then stores only references such as:

object_id
sha256
size_bytes
media_type
storage_key
storage_class
status

This keeps database backups, replication, vacuuming, and restore operations smaller and faster.

For responsiveness, object storage is not usually the bottleneck if:

- assets use immutable content-hash URLs
- CloudFront or another CDN caches them
- the application never lists buckets during requests
- the database already knows the exact object key
- QTI ZIP files are parsed at import time, not during student access

I would also set thresholds. For example:

Under 256 KB:
PostgreSQL is acceptable for normalized text or JSON
Over 256 KB:
Prefer object storage
Any binary:
Prefer object storage
Original imported package:
Always object storage

The exact threshold can change after profiling. The architectural rule matters more than the number.

The main exception is small, frequently accessed text. Keeping a 20 KB normalized question payload in PostgreSQL is reasonable. Moving every small JSON document to S3 would add complexity and extra network requests without much benefit.

So the conservative design is:

PostgreSQL stores compact operational state. Object storage stores all binary and archival content.

That avoids database bloat without making ordinary question loading dependent on object storage.

human: did the manager mention that adapt uses a hybrid problem SQL storage and only goes to filesystem for large files? I dunno. worth exploring

Yes, the manager described ADAPT as a hybrid:

- QTI XML, parsed QTI JSON, and WeBWorK source are stored in MySQL.
- Uploaded media is stored in S3.
- MySQL stores the S3 key and basic file metadata.

However, the manager did not establish that ADAPT uses a size threshold or moves only large files to S3. The apparent split is by content type, not size:

structured text and source code in SQL, media files in S3

That distinction is worth exploring. Useful questions for the manager:

- Are small images also stored in S3?
- Are large XML or JSON payloads ever moved out of MySQL?
- Are H5P and QTI ZIP packages stored in S3 or unpacked into SQL?
- Does ADAPT ever store binary data in MySQL?
- Is there a configured size threshold anywhere in the upload or import code?
- Are imported files preserved, or only their parsed representation?

Based on the evidence provided so far, I would describe ADAPT as type-based hybrid storage, not size-based hybrid storage.
