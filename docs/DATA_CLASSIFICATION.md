# Data classification

This reference classifies the information PLE handles so contributors can make
safe storage, API, cache, logging, and deletion decisions before adding a new
field or artifact. It complements the enforcement details in
[SECURITY_MODEL.md](SECURITY_MODEL.md), the API and module register in
[CONTRACTS.md](CONTRACTS.md), and the concrete object rules in
[OBJECT_STORAGE.md](OBJECT_STORAGE.md). It does not grant an exception to any
of those contracts.

Classification follows the information's meaning, not its representation. A
UUID, checksum, object ID, or opaque handle can still be sensitive when it
links a Student to a protected record. A value copied from a private object to
a log, cache, URL, or browser field retains its original classification.

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) supersedes this document for
the meaning of PLE-owned terms. This document owns classification, disclosure,
and retention consequences for those terms.

## Decision procedure

Classify a new datum before choosing its Rust type, table, Object Address, API
reader result, or telemetry event.

1. Decide whether it is answer-bearing or could reveal a correct answer before
   a Student submits. If so, it is server-only Answer Key, Question Feedback,
   Question Answer Explanation, or format-specific Question Grading Input.
2. Decide whether it identifies or describes a Student, course relationship,
   or course record. If so, apply the radioactive educational-record rules.
3. Decide whether it is a published answer-free presentation asset, private
   workspace Question Source, Question Grading Input, credential, or audit data.
   Do not infer delivery permission from its bucket.
4. Give browser code only the narrowest data needed for the visible
   operation. The authenticated attempt and `AuthenticatedSession`, with exact
   course and Student relationships, determine grading
   authority.
5. State the retention owner and deletion behavior when the datum is created.
   A record without a deletion owner does not become exempt from retention.

## Classification matrix

"Browser exposure" means ordinary Student-facing and public browser
contracts. The explicit Instructor-only canonical-source and author-preview
routes remain narrow exceptions described in [SECURITY_MODEL.md](SECURITY_MODEL.md).
"Current source-proven" describes behavior demonstrated by current source or
tests. "Future target" describes design intent pending implementation and
acceptance; it is not privacy evidence.

## Current evidence caveat

This matrix distinguishes intended boundaries from current evidence. Historical
installation-scoped types and related scope remain in current source. Those
paths do not prove a single-installation `AuthenticatedSession`, stable Question
Library lineage, exact Student ownership, Star/Watch privacy, or thresholded
Question Statistics. Domain and authorization contracts, PostgreSQL
schema/RLS/protected functions, Store implementation, server/worker/object/adapter
implementation, API/generated-contract/browser implementation, and connected
real-stack/release evidence each require their own acceptance. Until the relevant
capability accepts its boundary, a target row is design intent rather than a
privacy assurance.

| Data class                                        | Examples and authoritative owner                                                                                                                                                                                                                                                                                                                 | Storage and access owner                                                                                                                                                                    | Browser exposure                                                                                                                                                                                                                                                                                                                                                                                                         | Retention and deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Current state                                                                                                                                                 |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Published answer-free presentation assets         | A stable `QuestionId` names one Published Question lineage and mutable lineage metadata. Each immutable Question Revision carries the safe prompt and response presentation, revision-specific metadata, and renditions for assigned delivery. `question_model`, Question Library publication, and `ObjectAddress::QuestionAsset` own the shape. | Question Library records and `PublicAssets`; only `Ready` immutable registry records are eligible for an authorized CDN-backed delivery.                                                    | Approved-Instructor Question Library search and details results, an allowed Assignment Access decision for the Student's assigned activity, and logical `QuestionAssetId` may be sent. Never send answers, keys, source, a physical Object Address, bucket, or arbitrary signed URL. Published content is not anonymous internet content; delivering one asset does not grant Question Library search or details access. | Title and Description corrections update stable-lineage metadata. Source corrections or compatible improvements retain the stable `QuestionId` and publish a new immutable Question Revision. A substantive fork publishes a new `QuestionId` and first revision. A Sysadmin-approved `ForcedQuestionCorrection` applies only to `security_flaw` or `critical_correctness_flaw`, stops new selection and issuance of the flawed revision, and activates one authoritative replacement mapping while retaining immutable history. Course retention never deletes shared presentation assets. | Current schema and Object Storage establish the stable lineage/revision keys. Service and connected deployment acceptance remains required.                   |
| Private workspace source and authoring assets     | Draft Question Source, Draft Question Metadata, imported archives, editable assets, and private previews. The authoring Store and workspace contracts own them.                                                                                                                                                                                  | Separate `PrivateContent` objects, private Draft Question Metadata, and Draft Question Source Bindings require the Authoring Workspace Owner. Collaboration is future separate authority.   | A separate authorized preview or export may return a deliberate reader result. Draft source bytes, Draft Question Source Object References, Object Addresses, and hidden identifiers never enter ordinary Student delivery or Question Library search.                                                                                                                                                                   | Authoring Workspace cleanup expires inactive Draft Questions, their editable metadata, and draft source objects after the configured warning or recovery period. Publication writes a separate immutable Question Revision source object and Binding and copies accepted metadata into Published Question-owned storage.                                                                                                                                                                                                                                                                    | M1 accepts the metadata and Source Binding schema foundation. Publication, Question Search isolation, cleanup, and final parent QSOM1 acceptance remain open. |
| Generation/grader keys and payloads               | Answer Keys, accepted values, private rubrics, checker configuration, generation seeds, deterministic Question Grading Input, and Question Feedback. Grading and generation owners define correctness.                                                                                                                                           | Server-only grading records and `PrivateContent` objects, with separate least-authority Store, adapter, and worker capabilities.                                                            | Never in Student responses, Question Library data, generated TypeScript, Wasm, browser storage, URLs, logs, traces, or ordinary DTOs.                                                                                                                                                                                                                                                                                    | Retain the records required to reproduce an authorized grade or publication. They are not removed by Student-record deletion unless their only copy is a linked record that the owning retention rule removes.                                                                                                                                                                                                                                                                                                                                                                              | Current source proves the server-only grading and Wasm-closure boundary. Typed-scope Store and connected acceptance remain required.                          |
| Course records                                    | Course settings, Instructor membership, Assignment Content, schedules, course banners, and course-management history. Course Store and server routes own authorization and lifecycle.                                                                                                                                                            | PostgreSQL under forced RLS and typed course scope; current direct course-Instructor membership authorizes teaching reads and writes.                                                       | Send only the exact route response data. Student assignment presentation is delivered through Assignment Access, not general course-record access.                                                                                                                                                                                                                                                                       | Course lifecycle archives or deletes course-owned records according to [RETENTION_POLICY.md](RETENTION_POLICY.md). Shared presentation assets and private workspace drafts remain outside that deletion.                                                                                                                                                                                                                                                                                                                                                                                    | Future exact course ownership and forced-RLS acceptance are required. Current source proves only existing installation-scoped course paths.                   |
| Student work                                      | Student enrollment and membership, assignment summaries, Assignment Attempts, Question Attempts, Question Submissions, responses, feedback, grades, and linked access evidence. `learning-data-access` and `server` own authorization and lifecycle.                                                                                             | PostgreSQL under forced RLS and `StudentRecords`; exact Student ownership or current course-Instructor membership is required. Narrow audited support is the only exception.                | Only the exact teaching or Student-self result, normally `Cache-Control: no-store`. Exclude it from general logs, analytics, URLs, and browser persistence.                                                                                                                                                                                                                                                              | Course lifecycle notifies, archive-fences access, then deletes the course-owned Student graph. Backup expiry and recovery objectives remain deployment work.                                                                                                                                                                                                                                                                                                                                                                                                                                | Future exact Student ownership and Authenticated Session acceptance are required. Current source proves only existing course-scoped checks.                   |
| Account and authentication data                   | Authentication email, account label, opaque `AccountId`, passkey public credential/state, and authentication ceremonies. The auth capability owns them separately from course Stores.                                                                                                                                                            | Global account tables and authentication services; exact account/session authority is required. A course-linked copy follows Student-work classification.                                   | Only the account owner receives the minimum account-management data. Course routes receive no general account table.                                                                                                                                                                                                                                                                                                     | Account and security retention own global records. Course snapshots follow course-record or Student-work retention as applicable.                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Global Account and Authenticated Session ownership is a future connected-acceptance boundary; current source proves the passwordless/passkey foundation.      |
| Account and curation metadata                     | `Star` is a vetted-Instructor-visible endorsement. Its count and the vetted Instructor identities who starred are curation metadata. `Watch` membership and notification state belong to the watcher.                                                                                                                                            | Global account/curation records use exact `AccountId` and Published Question identity. Star endorsement visibility is limited to active Instructors; Watch state is private to the watcher. | Active Instructors may see the Star count and vetted Instructor identities who starred. Students and anonymous users receive neither Star identities nor Watch state, and curation metadata is absent from their projections.                                                                                                                                                                                            | Account and curation lifecycle owns Star and Watch records. They remain separate from Student-record retention even when a watcher or Star owner is enrolled in a course.                                                                                                                                                                                                                                                                                                                                                                                                                   | Future Star/Watch visibility and privacy acceptance is required. Current source does not prove this boundary.                                                 |
| Credentials and secrets                           | Opaque session credentials, database URLs, object-store credentials, Question Backend authentication, signing and encryption keys, and deployment secrets. Auth and deployment composition own them.                                                                                                                                             | Host-only HttpOnly cookie for the raw ordinary session credential; hashed session record; deployment secret storage and process configuration for other secrets.                            | Raw credentials, secrets, and connection strings never enter JSON, local storage, URLs, logs, traces, generated code, images, or repository examples.                                                                                                                                                                                                                                                                    | Session records expire or revoke under authentication policy. Deployment secrets follow rotation and revocation procedures, not course retention.                                                                                                                                                                                                                                                                                                                                                                                                                                           | Current source-proven account-session and secret boundary; production secret-manager delivery is deployment work.                                             |
| Private Question Backend replay and Session state | WeBWorK field/value replay mapping, renderer identity, iMathAS Session Authentication, iMathAS Launch State, source bytes, iMathAS Result Tokens, and launch cookies. The adapter/server boundary owns them.                                                                                                                                     | Attempt-bound private persistence and server-held Sessions; Question Backend calls receive trusted server-built requests only.                                                              | A Student receives a safe Question Attempt presentation and result View, never upstream field names, values, source, token, credential, or Question Backend Session state.                                                                                                                                                                                                                                               | Retain only as exact Course and Student Question Attempt evidence while needed for replay and grading. Remove it with the associated Student Record unless a separate immutable Question Source rule applies.                                                                                                                                                                                                                                                                                                                                                                               | Current source proves WeBWorK replay and iMathAS Question Backend boundaries; broader typed-scope acceptance remains required.                                |
| Anonymous aggregate statistics                    | Cohort-gated item difficulty, timing, and discrimination statistics associated with shared question revisions. `Question Statistics` owns the aggregate boundary.                                                                                                                                                                                | Shared identity-free aggregate tables, separate from course records. Course-local Assignment Question Analysis is Student work because small cohorts can identify a Student.                | Active Instructors receive only the released aggregate result after the deployment-wide k-anonymity threshold. It contains no Student, Account, course, raw response, or per-Student score.                                                                                                                                                                                                                              | It survives Student-record deletion because the released result contains no identifying record and is not reconstructed from deleted attempts.                                                                                                                                                                                                                                                                                                                                                                                                                                              | Future thresholded, version-aware Question Statistics release and privacy acceptance are required. Current source does not prove this disclosure boundary.    |
| Audit data and operational diagnostics            | Security audit events, protected-delivery authorization, bounded worker and job evidence, errors, and operational diagnostics. The producing server or worker owns each event shape.                                                                                                                                                             | Linked educational audit data uses forced RLS and the same exact course, Student, workspace, or capability scope. Deployment observability uses operations controls.                        | Browser errors are short and safe. Logs, traces, telemetry, and diagnostic attachments omit answers, keys, raw responses, object URLs, credentials, Question Backend tokens, session values, and source bytes.                                                                                                                                                                                                           | Audit data linked to Student work follows that record's retention. Operational logs require a documented deployment retention policy and never become an undeclared record archive.                                                                                                                                                                                                                                                                                                                                                                                                         | Current source proves application audit controls; exact-linkage acceptance and production observability retention remain separate work.                       |

## Boundary rules by medium

### PostgreSQL

- Shared Published Question lineages and immutable Question Revision source have global ownership. Course records and
  Student work use server-derived `AuthenticatedSession` plus exact course,
  Student, or workspace relationships.
- Forced RLS, transaction-local authenticated Account context, and narrow roles are the access
  boundary. A browser header, URL component, or JSON field never supplies
  account, membership, workspace, course, Student, or capability authority.
- A grading-reader connection is a separate least-privilege capability. The
  ordinary application Store does not acquire grading-read access by changing
  role inside a request.
- A table or JSON container does not make an answer-bearing value browser-safe.
  Private grading payloads stay opaque outside their authorized grader path.

### Object storage

- `PublicAssets`, `PrivateContent`, `StudentRecords`, and `TempProcessing` are
  distinct Object Storage Areas because delivery, encryption, IAM, and lifecycle
  policies differ. Each area maps to a provider bucket; neither concept authorizes
  a browser request.
- `ObjectAddress` derives physical paths from typed server IDs. Callers do not
  build raw storage paths, and a browser-provided string never becomes an
  Object Address.
- Published presentation assets are the only CDN-readable class. Workspace
  Question Source, Question Grading Input, Question Attempt Reproduction Details, renders, course-record
  assets, and Student work stay in their private typed classes.
- Every object record carries a server-computed SHA-256, verified media type,
  exact Object Address, Source Object Checksum, and creation time. These integrity fields do
  not change authorization. Production writes require SSE-KMS; encryption at
  rest does not replace RLS, IAM, or route authorization.

### Browser, caches, and URLs

- Default browser persistence is in-memory UI state only. Session tokens,
  answer-bearing values, Object Addresses, grades, and Question Backend state are not
  stored in `localStorage`, `sessionStorage`, or persistent browser caches.
- Render payloads may be richer than submissions. The server derives expected
  Question Type, grading backend, seed, ownership, and policy from the
  authenticated attempt. The browser never supplies grading authority.
- Protected route responses use `no-store`. A signed object URL is an
  authorization result returned only by the protected delivery POST; it is not
  a durable browser datum.
- Presentation checksums and Presentation Response Item Reference CRC16 values are consistency
  data, not credentials or grading keys. They never authorize an action or
  authenticate a Student.

## Delivery and publication rules

These are future target rules until their owning implementation and privacy
evidence accept them.

Use one of these server-derived authorities for every delivery:

1. Approved-Instructor Question Library access for safe Question Library search
   and details results and their published presentation assets.
2. An allowed Assignment Access decision for the answer-free presentation of
   an assigned activity.
3. Exact Authoring Workspace Owner relationship for private workspace
   source, assets, previews, and authoring projections. Collaboration is a
   future separately designed capability.
4. Typed worker, active lease, or explicit capability for generation, grading,
   retention, Question Backend, and course-record operations. The typed check
   includes the exact course or Student relationship when the target is a
   course record.

Published presentation assets are immutable and answer-free. Their asset route
requires approved-Instructor Question Library access or an allowed Assignment Access
decision, then resolves only a `Ready` Question Library asset registry record
and a known `QuestionAssetId`; it does not expose private source, Answer Key, Question
Feedback, Question Answer Explanation, or Question Grading Input.
Published content is not anonymous internet content, and a successful asset
delivery does not grant Question Library search or details access: those views
require authenticated approved-Instructor access, while Student delivery requires
the allowed Assignment Access decision.

Private source, generation and grader payloads, Student work, course records,
credentials, and audit data each retain their own class after copying or
reader result. A new reader result must name its authority, allowed fields, and
retention owner before it is delivered.

## Curation privacy boundary

This is future target behavior pending curation implementation and privacy
acceptance.

`Star` is a vetted-Instructor-visible endorsement of a Published Question, not
Student work and not a course record.
Active Instructors may see the star count and the vetted Instructor
identities who starred. The identity list is a Question Library curation result and
does not expose Student activity, responses, grades, or course membership.

`Watch` membership and notification state are private to the watcher. Students
and anonymous users receive neither Star identities nor Watch state. Curation
metadata remains account/curation data even when the account also has a Student
membership; it does not enter Student-record retention or Student projections.

## Classification-specific rules

### Shared content is not automatically public

A safe prompt and presentation asset can be shared while a source archive,
private render state, Question Attempt Reproduction Details, generation seed, or grading payload
remains private. Store a durable shared object only when it supports immutable
identity, reproduction, Question Attempt Reproduction Details, or approved answer-free delivery. Add
delivery through the asset registry, never by exposing the underlying object.

### Student work remains radioactive

Treat Student work and any linked course record as radioactive. The rule is
broader than direct PII: an opaque attempt ID, timing event, response, score,
or delivery audit becomes protected educational data when it links a Student to
a course activity.

The [database radioactive-records and retention model](DATABASE_AUTHORIZATION.md#radioactive-records-and-retention)
classifies current PostgreSQL relations and explains how the label follows
query results, backups, replicas, and restores. Derived query results and
persistent database copies inherit the highest classification of their inputs.

An Assignment Attempt ID, Question Attempt ID, or Object Delivery ID is an opaque identifier.
Every use still rechecks the authenticated Account, exact course, Student,
Authoring Workspace Owner or Workspace Collaborator relationship, or capability relationship, lifecycle
state, and operation binding.
Opaque IDs reduce accidental disclosure; they do not replace authorization.

### Statistics stay anonymous

Removing a Student identifier is insufficient when a small cohort can identify
that Student indirectly. Aggregate computation happens while records exist,
publication enforces the k-anonymity threshold, and the released result holds
no Account or Student identifier. Course-specific Assignment Question Analysis remains
radioactive even when it uses aggregate arithmetic.

## Change checklist

Before merging a new data path, answer these questions in its owning plan or
contract test:

1. Which matrix row owns the datum, and does it need a new explicit class?
2. Who authorizes read, write, delivery, and deletion?
3. Does an authenticated attempt or another server-owned record already derive
   information the browser would otherwise resend?
4. Can browser, Wasm, generated TypeScript, a URL, a cache, a log, or a trace
   reveal more than the approved reader result?
5. Which immutable identity, checksum, version, or Question Attempt Reproduction Details prove
   that the server is acting on the intended data?
6. Which retention stage, object manifest, and Object Storage Check and Repair path remove the
   datum or deliberately preserve it?
7. Is the behavior current-source-proven and validated, or is it a named
   future work package that must remain fail-closed today?

## Related references

- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines the grading, authentication,
  browser, Question Backend, and delivery enforcement boundaries.
- [CONTRACTS.md](CONTRACTS.md) names the public module and route contracts.
- [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security)
  defines forced RLS, authenticated Account context, roles, and exact course, Student, and
  Authoring Workspace Owner relationship.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md) defines typed Object Addresses, delivery,
  publication, and Object Storage Check status.
- [RETENTION_POLICY.md](RETENTION_POLICY.md) defines course lifecycle and
  backup limitations.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines the
  render-to-submission data boundary.
