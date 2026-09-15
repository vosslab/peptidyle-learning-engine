# Data classification

This reference applies the product and privacy decisions in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) to storage, APIs, caches, logs, and
retention. Classification follows meaning, not representation: an opaque ID or
checksum remains sensitive when it links a Student to a protected record.

## Decision procedure

Before adding a datum:

1. decide whether it can reveal or calculate a correct answer;
2. decide whether it identifies a Student, Course relationship, response,
   score, timing fact, or other educational record;
3. distinguish published answer-free content from private Question source,
   backend state, credentials, and Student Work;
4. expose only what the authorized reader needs; and
5. name its retention owner and deletion behavior.

## Classification matrix

| Class | Examples | Browser boundary | Retention |
| --- | --- | --- | --- |
| Published answer-free content | Published Question metadata, safe prompt content, authorized presentation assets | Vetted Instructor discovery or authorized Student delivery; not anonymous by default | Independent of Course Student-record retention |
| Private authoring content | Draft Question source, import archive, private preview, Answer Key | Authorized Instructor authoring only | Workspace policy; publication creates independent immutable content |
| Blueprint content | Blueprint metadata and immutable Revisions | Private owner-only; Public to vetted Instructors; Archived only through explicit inclusion | Independent of Course Student-record retention |
| Blueprint update and proposal data | Source and receiving Blueprint references, canonical differences, and acceptance decision | Daughter Course co-Instructors for Course updates; receiving Blueprint owner for Change Proposals | Retained with the Blueprint/Course relationship; never grants FERPA access |
| Course teaching content | Course metadata, Assessment definitions, Questions, settings | Current Course relationships | Remains after FERPA-protected Student records are deleted |
| FERPA-protected Student record | Course membership history, Attempt, response, score, timing, feedback, Student-linked analysis | The Student or equal co-Instructors for that Course; scoped support only | Notice, archive from normal interfaces, recovery period, permanent deletion |
| Global Account data | Account identity, immutable Product Role, authentication state | Account owner and narrow administration | Separate from Course record retention |
| Private Question Backend state | Opaque render state, response mapping, provider token, private result | Never directly exposed except the exact authorized answer-free document or response control | Follows the Question source or Student Work it serves |
| Credentials and secrets | Session credential, database URL, signing/encryption key | Never in ordinary DTOs, URLs, logs, screenshots, examples, or browser storage | Security rotation/revocation policy |
| Anonymous aggregate | Properly de-identified statistics with sufficient disclosure protection | Only the approved aggregate | May survive Student-record deletion only when it cannot identify or link back to a Student |
| Operational diagnostic | Bounded error, service health, support record | Safe projection only | Explicit operational policy; never an undeclared Student-record archive |

## PostgreSQL boundary

- Forced RLS and server-installed Account context enforce exact Course,
  Student, workspace, and Blueprint-owner relationships.
- A table, JSON container, UUID, or route parameter does not make private data
  browser-safe.
- All current co-Instructors have equal Course authority. The creator or first
  Instructor is not a privileged owner.
- Sysadmin support access to FERPA-protected records is deliberate, scoped, and
  recorded; the global Sysadmin role alone is insufficient.
- Submitted Student Work is immutable to ordinary application roles. Saved
  responses remain replaceable only while their whole Assessment Attempt is
  open.

## Object-storage boundary

- Published answer-free assets, private authoring content, Student records, and
  temporary processing use distinct typed scopes.
- The server constructs object keys from trusted identities. Browsers do not
  submit raw storage paths.
- Integrity checks do not grant access.
- Private Question source, Answer Keys, backend state, Student responses, and
  grading outcomes never become public assets.
- Assessment Unrelease and Course retention may delete Student-record objects;
  neither follows references into shared Published Questions, Pools, or
  Blueprint content.

## Browser, cache, URL, and log boundary

- Protected Student and Assessment responses use `no-store`.
- Session credentials, Answer Keys, raw backend state, grades, and Student Work
  do not enter persistent browser storage.
- Answer-free immutable content may be cached only under an exact safe key.
  Cache state never authorizes, grades, submits, or extends an Attempt.
- Public References, presentation checksums, and signed delivery URLs remain
  selectors or bounded delivery results, not durable authority.
- Logs and traces omit responses, grades, correct answers, credentials,
  provider tokens, private source, raw object URLs, and Student-linked payloads.

## Question delivery and grading

The server derives the Student, Course, Assessment Attempt, Question or Pool
Revision, backend, and disclosure policy. The Student sees one answer-free
Question presentation at a time and sends only a typed native response or
bounded opaque backend response.

The Question Backend owns grading and feedback and returns an immutable credit
fraction. PLE stores that fraction and calculates scores from current point
values. Saving changes only the working response and must not expose a grading
outcome before whole-Assessment submission.

## Student Work remains highly sensitive

Treat any Student-linked Attempt ID, response, score, timing value, feedback,
or Course analysis as FERPA-protected even if direct names have been removed.
Small-cell aggregates can still identify Students and remain protected.

Removing a Student from a Course or deactivating an Account does not delete
these records. The latest Assessment deadline starts the FERPA retention clock;
the six-month Active limit caps deadline movement so Course reuse cannot delay
FERPA retention indefinitely. Becoming Inactive is a separate transition and
does not itself delete Student records. See
[RETENTION_POLICY.md](RETENTION_POLICY.md).

## Shared content is not automatically public

A Published Question may be discoverable to vetted Instructors while its
source, Answer Key, grading inputs, and backend state remain private. Likewise,
a Public Blueprint is available to vetted Instructors but not anonymous users
or Students. "Published" and "Public" describe exact product audiences, not an
open internet access rule.

## Change checklist

Before merging a new data path, identify its class, read/write authority,
browser projection, cache/log behavior, retention stage, and real acceptance
evidence. Do not add a snapshot, job, audit, recovery, or compatibility record
merely because it could be useful later.

See [SECURITY_MODEL.md](SECURITY_MODEL.md),
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
