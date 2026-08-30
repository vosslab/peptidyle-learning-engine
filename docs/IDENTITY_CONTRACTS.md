# Identity contracts

## Binding single-installation model

PLE is one installation with global accounts. The binding SD1 product contract
requires each account to have one global `UserId` and exactly one immutable
Student, Instructor, or Sysadmin role; a person who needs multiple roles uses
separate accounts. A session then establishes one account and its one role, and
an operation derives authorization from the exact course membership, Student
ownership, workspace relationship, approved-Instructor state, or narrowly typed
platform capability that applies to that operation. This target remains pending
implementation and acceptance; pre-SD1 plural account/session role source is
cutover input.

Every published assignment question is shared Instructor-visible catalog
content. A private draft has no catalog identity and remains visible only
through its workspace relationship until validated publication creates a new
immutable published question identity. Shared catalog content is answer-free
and contains no Student records.

This document maps identities and their scopes. It supplements
[USER_ROLES.md](USER_ROLES.md), [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md). The active
[single_installation_authorization_plan.md](active_plans/active/single_installation_authorization_plan.md)
owns the migration from the former installation-scope model to these identities.

## Rules that apply everywhere

- A durable ID names one stored thing. It does not prove that its holder may
  read, change, or discover that thing.
- The server resolves the session and constructs `ActorContext`; browser
  requests never establish a user, approval state, course membership, Student
  ownership, workspace relationship, job target, or role by supplying an ID.
- `UserId`, `CourseId`, `WorkspaceId`, and published `QuestionId` are
  globally unique. Parent relationships, lifecycle state, and operation-specific
  predicates establish access.
- Educational records are owned by their exact course and Student
  membership/enrollment relationships. They do not inherit authority from an
  account role or a visible course reference.
- Published catalog content is immutable and shared. Courses, memberships,
  enrollments, runs, attempts, jobs, and protected objects are independent
  records that may refer to it.
- Rust uses distinct newtypes where mixing values would be a correctness risk.
  UUID strings appear only at a trusted server or defined browser boundary.
- A checksum or digest detects disagreement in otherwise valid data. It is not
  authentication, authorization, transport security, or an answer key.

## Account, session, and actor identities

| Identity or value                      | Scope                           | Intended use                                                                                                                                       |
| -------------------------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `UserId`                               | Global, durable                 | Names one PLE account/person across courses and workspaces. It is distinct from Student membership and enrollment identity.                        |
| Account role                            | Pending SD1 global account state | Stores exactly one closed Student, Instructor, or Sysadmin role. Target account/session storage never combines roles.                                |
| `SessionId`                            | Global, durable session record  | Names one server-tracked login session, including expiry and revocation state.                                                                     |
| `SessionTokenHash`                     | Server-only session record      | Stores the hash of the opaque browser credential. The raw credential is never a DTO, record locator, or log value.                                 |
| `ActorContext { user_id, session_id }` | Server-derived request context  | Carries the authenticated actor into domain, Store, and authorization operations. It has no ambient course, workspace, or Student grant.           |
| Approved-Instructor state              | Global, revocable account state | `approved_instructor(user_id, now)` establishes current Instructor product capabilities and is re-evaluated for protected operations.              |
| `Sysadmin` account role                | Pending SD1 global account state | Names limited platform operations. It has no course membership; teaching and FERPA reads use direct Instructor-account authority or audited support. |

The server resolves the opaque first-party session credential to
`ActorContext`. The browser receives only its own answer-free account/session
projection. It never receives another person's `UserId`, a raw session
token, or an authority-bearing approval claim.

### Session authority ownership

[`learning_data_access::session`](../crates/learning-data-access/src/session.rs)
is the sole owner of server-only session and actor identities: `SessionId`,
`SessionTokenHash`, `SessionLifetime`, `SessionRecord`, `ActorContext`, and
`SessionStore`. `SessionId` is a separate durable record identity, not a token
hash, token-derived value, or browser locator. `ActorContext` with `user_id` and
`session_id` is constructed only after the session store resolves the opaque
credential. It has no course, workspace, Student, or other operation grant and
has no browser serialization shape.
Neither type belongs in `question_model` or generated browser contracts.

[`learning_data_access::rls`](../crates/learning-data-access/src/rls.rs) owns
only the transaction adapter that installs an already-resolved `ActorContext`
in a protected database transaction. It does not mint, define, re-export, or
authorize `SessionId`, `ActorContext`, `UserId`, course membership, workspace
relationships, or Student ownership. The adapter applies transaction-local
actor context and forced-RLS denial; domain and Store owners evaluate the exact
relationship or typed capability.
The current legacy installation-scope context in this module is migration input
for SD1, not a second session or actor contract and not a global replacement identity.

## Course, Student, and relationship identities

| Identity              | Owns or names                                        | Authority and relation                                                                                                                      |
| --------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `CourseId`            | One teaching course or section                       | Global durable course identity. Course-scoped records carry this exact parent.                                                              |
| `CourseMembershipId`  | One immutable course-membership episode              | Binds one `UserId`, one `CourseId`, a role, lifecycle, and roster revision. Revocation preserves evidence; rejoining creates a new episode. |
| Student membership    | Current `CourseMembershipId` with Student role       | Participates in exact Student ownership checks for course work and educational records.                                                     |
| Instructor membership | Current `CourseMembershipId` with Instructor role    | Together with current approval, establishes `current_course_instructor(user_id, course_id, now)`.                                           |
| `EnrollmentId`        | One assignment enrollment for one Student membership | Binds a Student's course relationship to an assignment. It supports ownership and history; it is not a session or role substitute.          |
| `CourseGroupId`       | One typed group inside a course                      | Groups membership episodes for an explicit course purpose such as section, lab, cohort, accommodation, or work.                             |
| `AssignmentId`        | One course assignment                                | Has one exact `CourseId` parent and owns its current policy and ordered items.                                                              |
| `AssignmentItemId`    | One current assignment item                          | Retains item identity while a future assignment definition changes.                                                                         |
| `RunId`               | One pass through an assignment                       | Belongs to one enrollment; later practice uses a new run.                                                                                   |
| `QuestionAttemptId`   | One issued question instance                         | Binds a run to exact immutable content, seed, timing, status, provenance, and grading backend.                                              |

Under the binding pending SD1 product contract, the closed Sysadmin
course-instance provisioning command binds an exact BlueprintCourse source and
revision, an explicitly assigned approved Instructor account, and a
server-reserved CourseInstance identity. One transaction creates the
CourseInstance, that account's first ordinary Instructor membership, and an
append-only audit event; it gives the Sysadmin account no membership. Every
current co-Instructor has the same teaching and FERPA-read predicates. A
current course Instructor may invite an approved Instructor account, and
acceptance rechecks role agreement, approval, invitation state, and roster
revision atomically.

Student work is authorized by the authenticated `UserId` owning the active
Student membership and enrollment for the exact course. Direct current
co-Instructors use the same course predicate for permitted teaching-record
reads; neither another course nor a visible record ID extends that authority.

## Workspace and publication identities

| Identity                    | Scope                                        | Intended use                                                                                                                                                     |
| --------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `WorkspaceId`               | Global durable private-authoring root        | Names one draft workspace. Its owner/collaborator relationships, rather than its ID, authorize draft, import, source, asset, preview, and publication actions.   |
| Workspace relationship      | Durable `UserId` to `WorkspaceId` binding    | Records owner or explicit collaborator access and its lifecycle/revision. It owns private draft visibility.                                                      |
| `WorkspaceImportId`         | One private staged import                    | Names an import within its workspace. It never becomes a public question locator.                                                                                |
| `QuestionId`                | Global immutable published question identity | Human-facing catalog locator for one published question. Every published assignment question is discoverable by approved Instructors through the shared catalog. |
| `ProblemId` and `VersionId` | Server-only immutable content evidence       | Exact hidden identity for replay, grading, audit, provenance, and transport. It never lets a browser choose a version or resolve a latest question.              |
| `AssetId`                   | Logical published content asset              | Names a published logical asset; it does not grant object delivery.                                                                                              |
| `ObjectId`                  | Immutable stored bytes                       | Names stored source, asset, export, or learner-record bytes under an exact typed scope.                                                                          |

Validated publication either starts a new immutable catalog identity for a new
question or records a new immutable `QuestionVersion` under an existing stable
`QuestionId` lineage. A correction or compatible material improvement does not
mint a new `QuestionId`; it preserves the lineage and creates exact new
`ProblemId`/`VersionId` evidence. A full fork for an incompatible objective,
task, response family, or educational purpose creates a private draft and,
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
  version to a validated replacement `QuestionVersion` in the stable lineage
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

Published catalog discovery and reuse use current approved-Instructor state.
The catalog projection releases only its answer-free, content-focused fields.
It excludes Student-linked data, accepted responses, grades, source packages,
private grader payloads, provider identifiers and credentials, object keys,
signed URLs, and workspace identifiers.

## Current and future course relationships

Current `course_member` relationships provide the closed Student and
Instructor membership model. Future least-authority relationships are separate
records; each carries subject `UserId`, exact `CourseId`, relationship kind,
explicit capability set, issuer and issue time, lifecycle/revision, audit ID,
and its required disclosure policy.

| Relationship                         | Intended projection                  | Identity boundary                                                                                                                                              |
| ------------------------------------ | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grader                               | Bounded grading work                 | Uses an explicit grant and exact grading target; it does not become a course manager.                                                                          |
| Course Observer (for example, ADAPT) | Anonymous aggregate grades           | Uses a typed aggregate projection with disclosure thresholds and no Student subject, enrollment, row, small-cell, linkable metadata, answers, or FERPA record. |
| Student Observer                     | A consent-backed view of one Student | Binds observer, one Student, and one explicit revocable consent/disclosure record.                                                                             |

These relationships complement rather than replace course membership. They
remain separate from Student ownership, Instructor teaching, roster,
Gradebook, response, export, artifact, assignment-write, and worker predicates
until each workflow has its complete privacy and disclosure contract.

## Typed operational identities and scopes

| Identity                 | Scope                                    | Intended use                                                                                                                                                                                        |
| ------------------------ | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `JobId`                  | Durable queue record                     | Names one durable work unit. It does not establish a worker lease or target authorization.                                                                                                          |
| `JobLeaseToken`          | One worker claim                         | Opaque server/worker capability for the current lease. It is replaced on reclaim and never enters a browser contract.                                                                               |
| Job target scope         | Locked job manifest                      | A tagged `course`, `workspace`, `catalog`, `object`, or `provider` target resolved from immutable job metadata. Handler family, target type, generation, and broker grant agree before work starts. |
| `ExportId`               | One authorized export request            | Browser may inspect coarse status; a worker resolves frozen private input from the exact authorized scope.                                                                                          |
| `AssetDeliveryId`        | Protected delivery lookup                | Refers to an authorized `AssetId`, `ObjectId`, or course banner. It does not mint another logical object or grant raw storage access.                                                               |
| `AttemptSupportActionId` | One idempotent Instructor support action | Audits a sensitive action against its exact course and attempt scope.                                                                                                                               |
| `ScoringGeneration`      | Current-score fence                      | Positive monotonic generation that makes obsolete work harmless without deleting history.                                                                                                           |

A worker derives every target from its locked current lease and immutable job
manifest. Queue payload, retry input, provider response, object reference, and
caller input are evidence; they do not establish course, workspace, catalog,
object, or provider authority.

## Human-facing references and browser identifiers

Human-facing locators help people find a permitted record. They are not durable
authorization facts. The server resolves each locator within `ActorContext` and
the appropriate parent relationship before returning a record.

| Value                                                                                                  | Browser use                                        | Server meaning                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `QuestionId` (`AAA-BBBB`)                                                                              | Instructor catalog search, display, and selection  | Resolves one immutable published question after approved-Instructor authorization; not a version selector or answer authority.                 |
| `CourseReference`, `AssignmentReference`, `RunReference`, `WorkspaceReference`, `CourseGroupReference` | Human-readable route/display locators              | Positive `C-`, `A-`, `R-`, `W-`, and `G-` locators resolved only inside the authenticated actor's authorized course or workspace relationship. |
| `QuestionAttemptId` in a route                                                                         | Names an already issued attempt                    | Server additionally verifies exact active Student ownership/enrollment or permitted current Instructor scope.                                  |
| `SubmissionIdempotencyKey` header                                                                      | Bounded ASCII key for one retry                    | Matches stored request/receipt hashes; identical replay is safe and changed replay conflicts.                                                  |
| `RenderedItemIdV1`                                                                                     | Compact presentation-specific selection value      | Maps only through server-held attempt presentation state to a semantic item identity.                                                          |
| `PresentationNonceV1` and `PresentationDigestTokenV1`                                                  | Presentation binding values                        | Bind a response to the intended server-generated presentation; neither authorizes a request.                                                   |
| Normalized hotspot coordinates                                                                         | Integer response coordinates from 0 through 10,000 | Describe a response surface, not pixels, device geometry, or record authority.                                                                 |

`ChoiceId` remains a server-side semantic identity for a choice, slot, match
endpoint, order item, or hotspot region. `Seed` plus generator version and the
full stored presentation digest reproduce an issued variant. They are not
learner authority to select another variant or browser input to define grading.

## Credentials, capabilities, and answer boundaries

| Value                                                        | Holder and use                         | Storage and disclosure boundary                                                                                          |
| ------------------------------------------------------------ | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Raw session cookie                                           | Browser and authentication endpoint    | Database stores only `SessionTokenHash`; raw token never enters DTOs, logs, or analytics.                                |
| Email authentication secret                                  | Initiating browser and email recipient | Short-lived, single-use, browser-bound; database stores only a hash.                                                     |
| Passkey credential state                                     | Account boundary                       | Protected account data, not a course membership or Instructor projection.                                                |
| `JobLeaseToken` and external-tool tokens                     | Exact worker/broker exchange           | Opaque bounded capabilities, redacted from diagnostics and never serialized into generic question or submission records. |
| Signed object URL                                            | Authorized delivery result             | Short-lived storage result, not an object identity or reusable browser capability.                                       |
| Answer keys, scoring rules, private rubrics, grader payloads | Restricted server grading boundary     | Never appear in catalog, ordinary browser, Wasm, observer, or student-response DTOs.                                     |

## Maintainer checklist

When adding an identifier or protocol value, document:

1. What exact thing it names and its globally unique or parent-bound scope.
2. Whether it is durable, human-facing, semantic, presentation-scoped, a
   stale-work fence, checksum, relationship, or capability.
3. Which layer mints it, where it is persisted, and which server boundary may
   serialize it to a browser.
4. Which exact `ActorContext` predicate, course/Student ownership, workspace
   relationship, or typed operational scope authorizes its use.
5. Whether a browser or worker can derive it from an authenticated attempt or
   current lease instead of resending it.
6. Whether possession conveys authority. If so, use a bounded opaque
   capability with expiry, redaction, and an explicit storage boundary.

## Related documents

- [USER_ROLES.md](USER_ROLES.md) defines the closed current human personas.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines operation
  authorization and its migration target.
- [PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md) defines publication lifecycle.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines public question data and
  server-only answer material.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines learner
  render, response, and presentation consistency.
- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines authentication, grading,
  storage, and provider boundaries.
