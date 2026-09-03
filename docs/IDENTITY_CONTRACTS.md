# Identity contracts

## Binding single-installation model

PLE is one installation with global accounts. The intended global Account contract
requires each account to have one global `AccountId` and exactly one immutable
Student, Instructor, or Sysadmin role; a person who needs multiple roles uses
separate accounts. A session then establishes one account and its one role, and
an operation derives authorization from the exact course membership, Student
ownership, workspace relationship, approved-Instructor state, or narrowly typed
platform capability that applies to that operation. The Account and Authenticated
Session storage boundary is implemented; service, database, and release acceptance
remain separately incomplete.

Every Published Question used in an Assignment is shared Instructor-visible Question Library
content. A private draft has no Question Library identity and remains visible only
through its workspace relationship until validated publication creates a new
immutable published question identity. Shared Question Library content is answer-free
and contains no Student records.

This document maps identities and their scopes. It supplements
[USER_ROLES.md](USER_ROLES.md), [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The
[implementation status](active_plans/implementation_status.md) owns the migration from the
former installation-scope model to these identities.

## Rules that apply everywhere

- A durable ID names one stored thing. It does not prove that its holder may
  read, change, or discover that thing.
- The server resolves the session to a global account and session identity; browser
  requests never establish a user, approval state, course membership, Student
  ownership, workspace relationship, job target, or role by supplying an ID.
- `AccountId`, `CourseId`, `WorkspaceId`, and published `QuestionId` are
  globally unique. Parent relationships, lifecycle state, and operation-specific
  predicates establish access.
- Educational records are owned by their exact Course Instance and Student
  Course Membership relationships. They do not inherit authority from a
  Product Role or a visible Course Reference.
- Published Question Library content is immutable and shared. Courses, memberships,
  enrollments, Assignment Attempts, Question Attempts, jobs, and protected objects are independent
  records that may refer to it.
- Rust uses distinct newtypes where mixing values would be a correctness risk.
  UUID strings appear only at a trusted server or defined browser boundary.
- A checksum or digest detects disagreement in otherwise valid data. It is not
  authentication, authorization, transport security, or an answer key.

## Account, session, and relationship identities

| Identity or value         | Scope                            | Intended use                                                                                                                                                       |
| ------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `AccountId`               | Global, durable                  | Names one PLE login account across courses and workspaces. It is distinct from Student membership and enrollment identity.                                         |
| Product Role              | Implemented global Account state | Stores exactly one closed Student, Instructor, or Sysadmin Product Role. Account/session storage never combines roles.                                             |
| `SessionId`               | Global, durable session record   | Names one server-tracked login session, including expiry and revocation state.                                                                                     |
| `SessionTokenHash`        | Server-only session record       | Stores the hash of the opaque browser credential. The raw credential is never a DTO, record reference, or log value.                                               |
| Active Instructor Account | Global Account role and state    | An Account with Instructor Product Role and active Account State establishes current Instructor product capabilities and is re-evaluated for protected operations. |
| `Sysadmin` Product Role   | Implemented global Account state | Names limited platform operations. It has no Course Membership; teaching and FERPA reads use direct Instructor Account authority or audited support.               |

The server resolves the opaque first-party session credential to a `SessionRecord`
with its global account and session identity. The browser receives only its own answer-free
`AuthSessionResponse` Account data. It never receives another person's `AccountId`, a raw session
token, or an authority-bearing Account-State claim.

### Session authority ownership

[`learning_data_access::session`](../crates/learning-data-access/src/session.rs)
is the sole owner of server-only session identities: `SessionId`,
`SessionTokenHash`, `SessionLifetime`, `SessionRecord`, and `SessionStore`.
`SessionId` is a separate durable record identity, not a token hash,
token-derived value, or browser reference. A resolved session identifies its
global account and session, while the operation's exact relationship supplies
course, workspace, Student, or other authority. It has no browser serialization shape.
Neither type belongs in `question_model` or generated browser contracts.

[`learning_data_access::postgres::sessions`](../crates/learning-data-access/src/postgres/sessions.rs) owns
only the transaction adapter that installs already-resolved account and session facts
in a protected database transaction. It does not mint, define, re-export, or
authorize `SessionId`, `AccountId`, course membership, workspace
relationships, or Student ownership. The adapter applies transaction-local
resolved session facts and forced-RLS denial; domain and Store owners evaluate the exact
relationship or typed capability.
The current legacy installation-scope context in this module is migration input,
not a second session or Account contract and not a global replacement identity.

## Course, Student, and relationship identities

| Identity              | Owns or names                                                  | Authority and relation                                                                                                                         |
| --------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `CourseId`            | One teaching course or section                                 | Global durable course identity. Course-scoped records carry this exact parent.                                                                 |
| `CourseMembershipId`  | One immutable course-membership episode                        | Binds one `AccountId`, one `CourseId`, a role, lifecycle, and roster revision. Revocation preserves evidence; rejoining creates a new episode. |
| Student membership    | Current `CourseMembershipId` with Student role                 | Participates in exact Student ownership checks for course work and educational records.                                                        |
| Instructor membership | Current `CourseMembershipId` with Instructor role              | Together with current approval, establishes `current_course_instructor(account_id, course_id, now)`.                                           |
| `StudentRecordId`     | One Student Record for one Student Account and Course Instance | Binds a Student's course relationship to the durable educational record across membership episodes; it is not a session or role substitute.    |
| `AssignmentId`        | One course assignment                                          | Has one exact `CourseId` parent and owns its current policy and ordered Assignment Entries.                                                    |
| `AssignmentEntryId`   | One current Assignment Entry                                   | Names one Fixed Question or Question Pool in its Assignment Content.                                                                           |
| `AssignmentAttemptId` | One Assignment Attempt                                         | Target identity for one pass through one exact Student Record and Assignment; later practice creates another Assignment Attempt.               |
| `IssuedQuestionId`    | One selected Question Revision                                 | Binds an Assignment Attempt to exact immutable content, Assignment Entry, delivery order, and scoring treatment.                               |
| `QuestionAttemptId`   | One server-issued try                                          | Binds an Issued Question to its seed, timing, status, Question Attempt Reproduction Details, and grading backend.                              |

The future Store-backed Sysadmin Course Instance Creation operation binds an exact BlueprintCourse source and
revision, an explicitly assigned active Instructor account, and a
server-reserved CourseInstance identity. One transaction creates the
CourseInstance, that account's first ordinary Instructor membership, and an
append-only audit event; it gives the Sysadmin account no membership. Every
current Teaching Team Member has the same teaching and FERPA-read predicates. A
current course Instructor may invite an active Instructor account, and
acceptance rechecks role agreement, approval, invitation state, and roster
revision atomically.

Student work is authorized by the authenticated `AccountId` owning the active
Student membership and enrollment for the exact course. Direct current
Teaching Team Members use the same course predicate for permitted teaching-record
reads; neither another course nor a visible record ID extends that authority.

## Workspace and publication identities

| Identity                    | Scope                                        | Intended use                                                                                                                                                                                  |
| --------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `WorkspaceId`               | Global durable private-authoring root        | Names one draft workspace. Its owner/collaborator relationships, rather than its ID, authorize draft, import, source, asset, preview, and publication actions.                                |
| Workspace relationship      | Durable `AccountId` to `WorkspaceId` binding | Records owner or explicit collaborator access and its lifecycle/revision. It owns private draft visibility.                                                                                   |
| `WorkspaceImportId`         | One private staged import                    | Names an import within its workspace. It never becomes a public Question Library reference.                                                                                                   |
| `QuestionId`                | Global immutable Published Question identity | Human-facing Question Library reference for one Published Question. Every Published Question used in an Assignment is discoverable by active Instructors through the shared Question Library. |
| `QuestionRevisionReference` | Server-only immutable Question Revision      | Pairs one Question ID with its positive Question Revision Number for exact assignment, delivery, grading, replay, audit, and source evidence.                                                 |
| `QuestionAssetId`           | Logical published content asset              | Names a published logical asset; it does not grant object delivery.                                                                                                                           |
| `ObjectId`                  | Immutable stored bytes                       | Names stored source, asset, export, or student-record bytes under an exact typed scope.                                                                                                       |

Validated publication either starts a new immutable Question Library identity for a new
question or records a new immutable `QuestionRevision` under an existing stable
`QuestionId` lineage. A correction or compatible material improvement does not
mint a new `QuestionId`; it preserves the lineage and creates exact new
`QuestionId`/`QuestionRevisionNumber` evidence. A full fork for an incompatible objective,
task, Question Type, or educational purpose creates a private draft and,
after validation, a new `QuestionId` with source attribution and visible
ancestry.

Published-question stewardship has four distinct paths:

- An owner moderate edit passes publication validation and creates a new
  immutable version in the same `QuestionId` lineage.
- Any Instructor may submit a `QuestionChangeProposal` against one exact
  immutable base version. Validation runs before submission, semantic and
  grading impact is shown, and the owner accepts or rejects it. Acceptance
  creates a same-lineage version with contributor credit. An advanced base
  requires rebase or resubmission.
- Any Instructor may create a full fork as a private Draft Question. Validated
  publication creates a separate lineage with a new `QuestionId`, source
  attribution, and preserved ancestry.
- A `ForcedQuestionCorrection` is a separately audited Sysadmin operation for
  a critical security or correctness flaw. It maps one flawed immutable
  version to a validated replacement `QuestionRevision` in the stable lineage
  and records deterministic remediation; it is not ordinary editing or a
  change proposal.

The user-facing action for a `QuestionChangeProposal` is **Suggest an
improvement**. GitHub is a documentation analogy only; it adds no branch,
merge, reviewer, or repository semantics. Original authorship, contributor
credit, history, and compatible Creative Commons licensing remain preserved
across all four paths. Assignments and graded work retain exact immutable
version pins and are never changed automatically by a later revision. A
correction mapping may affect only future unissued resolution and its audited
remediation; issued and graded evidence remains pinned to the original.

Question Library discovery and reuse use current approved-Instructor state.
Question Search and Question Details release only answer-free, content-focused fields.
It excludes Student-linked data, accepted responses, grades, source packages,
private grader payloads, provider identifiers and credentials, Object Addresses,
signed URLs, and workspace identifiers.

## Current and future course relationships

Current `course_member` relationships provide the closed Student and
Instructor membership model. Future least-authority relationships are separate
records; each carries subject `AccountId`, exact `CourseId`, relationship kind,
explicit capability set, issuer and issue time, lifecycle/revision, audit ID,
and its required disclosure policy.

| Relationship                         | Intended returned data               | Identity boundary                                                                                                                                                |
| ------------------------------------ | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grader                               | Bounded grading work                 | Uses an explicit grant and exact grading target; it does not become a course manager.                                                                            |
| Course Observer (for example, ADAPT) | Anonymous aggregate grades           | Uses a typed aggregate-grade result with disclosure thresholds and no Student subject, enrollment, row, small-cell, linkable metadata, answers, or FERPA record. |
| Student Observer                     | A consent-backed view of one Student | Binds observer, one Student, and one explicit revocable consent/disclosure record.                                                                               |

These relationships complement rather than replace course membership. They
remain separate from Student ownership, Instructor teaching, roster,
Gradebook, response, export, artifact, assignment-write, and worker predicates
until each workflow has its complete privacy and disclosure contract.

## Typed operational identities and scopes

| Identity                 | Scope                                    | Intended use                                                                                                                                                                                                      |
| ------------------------ | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `JobId`                  | Durable queue record                     | Names one durable work unit. It does not establish a worker lease or target authorization.                                                                                                                        |
| `JobLeaseToken`          | One worker claim                         | Opaque server/worker capability for the current lease. It is replaced on reclaim and never enters a browser contract.                                                                                             |
| Job target scope         | Locked job manifest                      | Question Library work uses the exact `question_revision` Job Target resolved from immutable job metadata. Job Kind Registration, target type, generation, and Job claim-and-lease grant agree before work starts. |
| `AssetDeliveryId`        | Protected delivery lookup                | Refers to an authorized `QuestionAssetId`, `ObjectId`, or course banner. It does not mint another logical object or grant raw storage access.                                                                     |
| `AttemptSupportActionId` | One idempotent Instructor support action | Audits a sensitive action against its exact course and attempt scope.                                                                                                                                             |
| `ScoringGeneration`      | Current-score fence                      | Positive monotonic generation that makes obsolete work harmless without deleting history.                                                                                                                         |

A worker derives every target from its locked current lease and immutable job
manifest. Queue payload, retry input, provider response, object reference, and
caller input are evidence; they do not establish the exact Job Target authority.

## Human-facing references and browser identifiers

Human-facing References help people find a permitted record. They are not durable
authorization facts. The server resolves each Reference from the authenticated
session account and the appropriate parent relationship before returning a record.

| Value                                                                                                         | Browser use                                     | Server meaning                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `QuestionId` (`AAA-BBBB`)                                                                                     | Question Library search, display, and selection | Resolves one immutable published question after approved-Instructor authorization; not a version selector or answer authority.                                     |
| `CourseInstanceReference`, `AssignmentReference`, `AssignmentAttemptReference`, `AuthoringWorkspaceReference` | Human-readable route/display References         | Positive `C-`, `A-`, `R-`, and `W-` References resolve only inside the authenticated Account's authorized Course Instance or Authoring Workspace relationship.     |
| `QuestionAttemptId` in a route                                                                                | Names an already issued Question Attempt        | Server additionally verifies exact active Student Record ownership or permitted current Instructor scope.                                                          |
| `SubmissionIdempotencyKey` header                                                                             | Bounded ASCII key for one retry                 | Matches stored request/receipt hashes; identical replay is safe and changed replay conflicts.                                                                      |
| `PresentationResponseItemReference`                                                                           | Presentation-scoped Response Item Reference     | Maps only through server-held attempt presentation state to a semantic item identity.                                                                              |
| `QuestionPresentationNonce` and `QuestionPresentationToken`                                                   | Presentation binding values                     | The nonce participates in the complete server-held Question Presentation Checksum; the public token is its compact comparison value. Neither authorizes a request. |
| Student Hotspot Selection                                                                                     | One selected presentation-scoped Hotspot Region | Resolves through the exact issued presentation to a durable Hotspot Region; authored geometry remains in Question Response Format.                                 |

Response Item Reference remains a server-side semantic identity for a Question Choice, slot, match
endpoint, order item, or hotspot region. `QuestionSeed` plus generator version and the
full stored Question Presentation Checksum reproduce an issued variant. They are not
student authority to select another variant or browser input to define grading.

## Credentials, capabilities, and answer boundaries

| Value                                                        | Holder and use                         | Storage and disclosure boundary                                                                                          |
| ------------------------------------------------------------ | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Raw session cookie                                           | Browser and authentication endpoint    | Database stores only `SessionTokenHash`; raw token never enters DTOs, logs, or analytics.                                |
| Email authentication secret                                  | Initiating browser and email recipient | Short-lived, single-use, browser-bound; database stores only a hash.                                                     |
| Passkey credential state                                     | Account boundary                       | Protected account data, not a course membership or Instructor result.                                                    |
| `JobLeaseToken` and iMathAS Result Tokens                    | Exact worker/Result Exchange           | Opaque bounded capabilities, redacted from diagnostics and never serialized into generic question or submission records. |
| Signed object URL                                            | Authorized delivery result             | Short-lived storage result, not an object identity or reusable browser capability.                                       |
| Answer keys, scoring rules, private rubrics, grader payloads | Restricted server grading boundary     | Never appear in the Question Library, ordinary browser, Wasm, observer, or student-response DTOs.                        |

## Maintainer checklist

When adding an identifier or protocol value, document:

1. What exact thing it names and its globally unique or parent-bound scope.
2. Whether it is durable, human-facing, semantic, presentation-scoped, a
   stale-work fence, checksum, relationship, or capability.
3. Which layer mints it, where it is persisted, and which server boundary may
   serialize it to a browser.
4. Which exact account/session predicate, course/Student ownership, workspace
   relationship, or typed operational scope authorizes its use.
5. Whether a browser or worker can derive it from an authenticated attempt or
   current lease instead of resending it.
6. Whether possession conveys authority. If so, use a bounded opaque
   capability with expiry, redaction, and an explicit storage boundary.

+## Settled identity and Blueprint decisions

## Identity, authentication, and compliance

### Visible identifiers are human-readable References

**Decision.** Visible content, navigation URLs, documentation, and copyable links never expose
UUIDs. Published questions use one non-sequential Crockford Base32 ID displayed as `AAA-BBBB`;
internal UUIDs may remain in hidden server and transport boundaries.

**Why.** People need identifiers they can recognize and communicate. A public reference is a
Reference, not authorization, and persistence identity should not leak into the interface.

### Invitations and recovery use verified email

**Decision.** PLE accounts are global within the installation and use passwordless verified email as
the registration, invitation, sign-in, and passkey-recovery path. SMTP delivery is optional;
an Instructor may share a one-time invitation link through a trusted LMS.

**Why.** Email provides one comprehensible account authority while keeping a configured mail
provider optional for independent use.

### Authentication storage is strictly necessary

**Decision.** Production authentication uses one host-only `__Host-` HttpOnly, Secure,
`SameSite=Lax`, `Path=/` browser-session cookie with bounded server expiration and immediate
revocation. Persistent login, tracking, or embedded LTI requires separate review and consent.

**Why.** The bearer credential must remain unreadable to JavaScript and limited to providing the
requested signed-in service. Necessary-storage classification still requires clear disclosure and
deployment-specific legal review.

### Security controls preserve privacy and recovery guidance

**Decision.** PostgreSQL, object storage, backups, and deployment volumes use scoped managed
encryption at rest; application AEAD is reserved for stored secrets. Unauthorized users receive
generic unavailable outcomes with accessible guidance that does not disclose protected details.

**Why.** Concealment and humane teaching guidance are complementary. The regulatory basis includes
the EU ePrivacy Directive Article 5(3), Article 29 Working Party Opinion 04/2012, and current ICO
strictly-necessary storage guidance.

### Blueprint collaboration is revision-scoped and ends at publication

**Decision.** A Blueprint Collaborator contributes only to one exact Draft
Blueprint Revision through immutable grant and end events. An immutable Blueprint
Publication Event makes that revision reusable and closes its collaboration path.

**Why.** An Authoring Workspace and a Blueprint Course have different parents,
privacy boundaries, and consequences. Revision-scoped collaboration prevents an
unrelated private-workspace grant from becoming reusable-course or live-course
authority, while publication preserves the exact review boundary.

### Blueprint availability belongs to one published revision

**Decision.** Available or Archived selection state is derived from immutable
Blueprint Revision Availability Events. The Blueprint Course lineage has no
aggregate archive timestamp.

**Why.** Availability changes whether a particular published source revision is
eligible for ordinary new selection. Historical Course Instance references must
continue to resolve that exact source after it leaves ordinary selection.

## Related documents

- [USER_ROLES.md](USER_ROLES.md) defines the closed current human personas.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines operation
  authorization and its migration target.
- [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) defines the human-facing Question
  ID, Question Revision Number, and exact Question Revision Reference.
- [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines Question
  publication, availability, and stewardship vocabulary.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines public question data and
  server-only Answer Keys.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines student
  render, response, and presentation consistency.
- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines authentication, grading,
  storage, and provider boundaries.
