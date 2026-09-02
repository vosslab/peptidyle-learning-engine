# Authorization contracts

## Binding single-installation model

PLE is one installation with global accounts. The implemented SD1 Account and
Authenticated Session boundary carries exactly one immutable Student, Instructor,
or Sysadmin role; a person needing multiple roles uses separate accounts. Full
service, database, and release acceptance remains incomplete. The installation has one
immutable shared Question Library of Published Questions used in Assignments, private Instructor
authoring workspaces, and course-scoped Student educational records. It has no
institution selector, publication-visibility tier, or creator-owned course
authority.

This document is the binding authorization contract. It defines the authority
that routes, Store methods, PostgreSQL policies and protected authorization functions, workers, object
delivery, browser DTOs, and audits must use. Product scope and dependency order
remain in [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) and the
[implementation status](active_plans/implementation_status.md).

Authorization is distinct from authentication, structural validation, revision
and lifecycle checks, and grading. An identifier, a request field, a browser
projection, a coarse role label, or a valid response shape does not establish
authority.

## Account and request boundary

The authenticated-session boundary derives the complete server-only account context:

```text
AuthenticatedSession {
    account_id: AccountId,
    session_id: SessionId,
}
```

`AuthenticatedSession` has no caller-controlled constructor. The server resolves its
opaque first-party session before it reads a protected resource, opens a Store
transaction, or performs expensive work. The session credential is HttpOnly;
only its database-authoritative hash, expiry, and revocation state are stored.
Missing, malformed, expired, revoked, and unknown sessions receive the same
unauthenticated result.

Every protected operation follows this sequence:

```text
resolve session -> derive AuthenticatedSession -> load and authorize exact durable scope
-> validate request/revision/lifecycle -> perform operation in one transaction
```

The protected transaction reevaluates every current authority predicate that it
uses. Authorization therefore precedes a revision conflict, request validation
where feasible, publication work, object signing, provider dispatch, and other
observable work. PostgreSQL receives transaction-local authenticated-account context; forced
RLS and narrow authorization functions enforce the same durable scope. Application,
worker, and grader roles are least-privilege roles and do not bypass RLS.

## Canonical predicates

All global Instructor capability checks require one Active Instructor Account:

```text
account.role = instructor AND account state = active
```

Sysadmin vetting occurs before Account creation; it is not a second Account
lifecycle. The resulting Active Instructor Account
authorizes Question Library discovery, Question Folders, Stars, Watches, and Saved Question
Searches, course creation, publication, reuse, and improvement. Every active
Instructor has the same global product capabilities.

Sysadmin status alone does not create Instructor authority. A Sysadmin creates a
Course Instance only for an explicitly assigned active Instructor account; it receives no
course membership. A person who needs teaching authority uses an active Instructor
account.

Every course-teaching operation uses one canonical predicate:

```text
current_course_instructor(account_id, course) =
    current direct Instructor membership(account_id, course)
```

The predicate authorizes the complete registered course-Instructor operation
set, including course definitions, roster administration, assignments,
gradebook, permitted Student-work inspection, exports, and course-record
assets. Course creation atomically creates the first ordinary Instructor
membership; it creates no additional creator or owner power.

Account deactivation takes effect immediately for global Instructor operations,
and Membership revocation takes effect immediately for the affected course. Each
protected transaction locks or otherwise safely evaluates the current Account
and Membership state before it commits, so a previously authorized request
cannot win a concurrent revocation.
Past membership and audit evidence remain durable records; they do not retain
active authority.

## Course authority matrix

The course creator and every accepted Teaching Team Member have equal authority. They
receive identical allow/deny results for the same current course state; audit
entries retain the acting `AccountId` and distinguish who performed the action.

| Course operation                                                | Creator                                   | Current accepted Teaching Team Member     | Student                                        | Sysadmin without membership                                 |
| --------------------------------------------------------------- | ----------------------------------------- | ----------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------- |
| Read or change course, roster, schedule, appearance, assignment | Allow through `current_course_instructor` | Allow through `current_course_instructor` | Read only the Student projection in own course | No general course authority                                 |
| Read Gradebook, authorized Student-work, permitted export       | Allow through `current_course_instructor` | Allow through `current_course_instructor` | Own answer-free work only                      | No general FERPA authority                                  |
| Invite or revoke a Teaching Team Member                         | Allow through `current_course_instructor` | Allow through `current_course_instructor` | No                                             | Narrow audited roster support only where separately granted |
| Create or publish a question                                    | Allow for active Instructor Account       | Allow for active Instructor Account       | No                                             | Platform operation only when separately authorized          |

Course Invitation, acceptance, update, and revocation are atomic,
audited course-membership operations. Invitation acceptance verifies the target
Instructor account's matching Product Role and active Account State.
Account deactivation closes a Teaching Team Member's course authority without
changing the durable membership history. A Sysadmin receives course-record access only
through a separately defined, narrow audited support
operation; platform status is not ambient FERPA authority.

## Course Instance Creation authority

Pending SD1 implementation defines `CourseInstanceCreationAuthority` as a
closed Sysadmin platform authority that exists before a CourseInstance. Its
command binds one exact `BlueprintCourse` source and revision, one exact
currently approved assigned Instructor account, and one server-reserved
`CourseInstanceId`. The authenticated Sysadmin receives no course membership
and cannot select a different source, Instructor, or resulting course after
the authority is evaluated.

One Store transaction creates the CourseInstance, its first direct Instructor
membership for the assigned account, and an append-only
`course_instance_created` audit event. The event identifies the acting
Sysadmin, exact Blueprint source and revision, assigned Instructor, reserved
CourseInstance identity, result, and time without copying Student records.
The new CourseInstance has no Students or delivery records at bootstrap. This
platform authority is the pre-course bootstrap boundary; it is not a
`SysadminSupportCapability`, and it does not create a Sysadmin membership or
general course authority.

## Sysadmin support capability registry

`SysadminSupportCapability` is the one closed authority registry for a
Sysadmin acting on a CourseInstance. A durable capability record contains an
opaque `capability_id`, exact `course_id`, acting `sysadmin_user_id`,
`purpose`, `issuer`, registered `operation_kind`, `minimum_projection`,
`issued_at`, `expires_at`, `revoked_at`, and an append-only audit-event
reference. The server derives this record after session authentication; a
browser cannot create, widen, renew, or select it.

An active capability has one exact CourseId, one closed Operation Kind, and
one stated support purpose. It is issued by a current CourseInstance Instructor
for support requested by that course. The platform retention scheduler is the
registered issuer for its payload-free lifecycle operation. The capability
registry begins after Course Instance Creation has committed an exact CourseInstance and
its first direct Instructor membership.

### Registered operations

#### `course_roster_support`

- Supports course roster, invitation, invitation-email-rule, revocation, and import actions approved for
  the exact course.
- Projects only the targeted roster and invitation records required for that command.
- Appends `sysadmin_support.roster`.

#### `course_schedule_support`

- Supports assignment schedule and Student accommodation changes for the stated purpose.
- Projects exact assignment timing and affected accommodation fields only.
- Appends `sysadmin_support.schedule`.

#### `assignment_content_support`

- Supports course assignment content, structure, release, and delivery settings.
- Projects Assignment Content and release state only.
- Appends `sysadmin_support.assignment_content`.

#### `deterministic_delivery_recovery`

- Invokes registered deterministic reissue or recalculation commands and inspects their receipts.
- Projects only the exact durable target, correction/recovery manifest, job state, and receipt
  reference.
- Appends `sysadmin_support.delivery_recovery`.

#### `retention_lifecycle_support`

- Reads coarse lifecycle state and requests an archive, delete, or extension transition.
- Projects lifecycle state, strong revision, disposition, and resulting receipt only.
- Appends `sysadmin_support.retention`.

Every capability use verifies the acting Sysadmin, exact CourseId, current
Operation Kind, purpose, expiry, and absence of revocation inside the same
Store transaction as the command. Missing, foreign, expired, revoked,
wrong-kind, or malformed capability state fails closed with the normal
concealed result. Each issuance, use, rejection, expiry, and revocation appends
an audit event that identifies the acting Sysadmin, issuer, exact CourseId,
Operation Kind, purpose, result, and time without copying roster PII,
invitation secrets, raw Student responses, answer keys, or scores.

The registry gives no Gradebook browsing, general Student-record browsing,
course-teaching authority, export authority, or ambient FERPA access. A
Sysadmin receives course-record access only through a separately registered
capability that specifies an equally narrow target and projection.
`ForcedQuestionCorrection` remains a platform-level closed
operation that produces privacy-safe impact and deterministic remediation; it
does not issue a course support capability or widen course access.

## Student self ownership and FERPA records

Current Student membership is an explicit relationship between `AccountId` and an
exact `CourseId`. A Student Record, Assignment Attempt, Question Attempt, response, grade, artifact,
or Student-record asset is bound to that exact course, the current Student
membership/owner, and its child identity. A Student may list and work only the
assignments available to that Student in that course and may read only the
answer-free projection of that Student's own records.

Each student-scoped Store operation rechecks current Student membership and
owner binding inside its transaction. This makes roster revocation immediate:
an inactive or removed Student cannot retain read, write, submission, asset,
or replay access. Course Instructors may use only their Instructor projections;
they do not obtain authority to submit or act as a Student. One Student never
receives another Student's record. Archive, retention, and deletion state add
their own fences to the authorization result.

## Private workspace ownership

Drafts are private authoring material. A draft, curriculum, source package,
QTI upload, conversion, collaborator grant, and author preview belongs to a
typed `WorkspaceId` with an explicit owner/collaborator relationship. The
stored current relationship and exact revision authorize draft read, save,
edit, sharing, deletion, preview, and publication preparation.

An author preview is an authorized Instructor workspace operation with
`no-store`; it is not a Student delivery path. Its projection is answer-free
and contains no key, private rubric, source package, provider credential,
Object Address, signed URL, or draft-to-published identity shortcut. Drafts remain
private until successful publication. Question Folders, Stars, Watches, and Saved Question
Searches may be personal or explicitly shared without changing the Question Library
visibility of any published question they reference.

## Shared publication and Instructor DTO contract

Every published question has one stable Question ID lineage and immutable
QuestionRevisions, visible to every active Instructor through exactly one shared
Question Library state. A publication mints a new Question ID only for a new lineage,
after the private workspace material validates. A same-lineage semantic change
publishes a new immutable QuestionRevision under the existing Question ID. An
incompatible objective, task, Question Type, or educational-purpose change is
a fork: its creator-private draft validates before publication with a new
Question ID and visible source ancestry. Existing Assignments and issued Assignment Attempts
retain their exact reference until a current course Instructor performs an
explicit revision-checked replacement.

Question Library visibility is authenticated approved-Instructor visibility.
Student delivery is a separate assignment-entitlement operation: a Student
receives only the exact question snapshot entitled by that assignment. Students
do not receive Question Library discovery. An anonymous web request receives no Question
Library access authority.

The caller projections are closed:

| Caller                                     | Question projection                                                                                                                           |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Authenticated approved (vetted) Instructor | Versioned answer-free Question Search, Question Details, lineage, usage, and thresholded Question Statistics DTOs.                            |
| Authenticated Student                      | No Question Library discovery view; only the exact answer-free or policy-permitted content delivered by an authorized assignment entitlement. |
| Anonymous caller                           | No Question Library discovery view and no Question ID resolution, existence, search, or lifecycle disclosure.                                 |

Lifecycle state is visible and does not change discovery authority. Every
Published Question remains discoverable to every active Instructor. Question
Search and Question Details expose its Question Revision Availability as
`Available` or `Archived`. Selection eligibility is a separate rule: only
`Available` Question Revisions may be selected for an ordinary new assignment.
`Archived` Question Revisions remain resolvable for exact historical references and retained
assignments, but are excluded from ordinary new selection.

Same-lineage publication is limited to the closed semantic classes: presentation,
accessibility, or metadata work that preserves grading meaning; compatible
student-content improvement that preserves the objective, task, and Question
Type; and a grading-semantic correction with an impact and recalculation record.
An incompatible objective, task, Question Type, or educational-purpose change
is a fork. `ModerateEdit` is available only to the question owner or original
lineage steward; it publishes a new immutable QuestionRevision in the same
Question ID lineage, preserves original authorship, and retains the existing CC
license. `FullFork` is available to any approved (vetted)
Instructor. It creates a creator-private Draft Question with that Instructor's
own authorship, source attribution, and a source-compatible CC license. Only its
creator's workspace can read or change the draft until validation and publication
succeed; publication creates a new Question ID with visible source/version
ancestry and no write access to the source.

`QuestionChangeProposal` is a separate contribution path. Any approved
(vetted) Instructor may submit a validated patch and rationale against one exact
base QuestionRevision. The lineage owner accepts or rejects it. Acceptance
creates a new immutable version in the original Question ID lineage, preserves
canonical authorship and the compatible CC license, and records contributor
credit and proposal ancestry. It never moves assignment or evidence pins. A
stale base requires rebase and resubmission.

Each assignment item records its visible Question ID and hidden exact
`QuestionRevisionReference { question_id, revision_number }` pin. Issued Assignment Attempts, Question Attempts, grading
evidence, and audit records retain the same exact pair, Question Seed, and
Question Attempt Reproduction Details. A publication, lifecycle transition, correction, or worker never
advances an assignment implicitly; a future version requires an explicit,
revision-checked assignment update.

The correction path is a closed `ForcedQuestionCorrection`. A Sysadmin alone
approves it after replacement validation and a privacy-safe impact manifest. The
manifest binds flawed and replacement exact versions, a permitted reason
(`security_flaw` or `critical_correctness_flaw`), generation, affected bindings,
and deterministic remediation. Approval atomically maps the flawed version to its
replacement for new selection and issuance. The flawed version remains immutable
historical evidence. Bounded, idempotent, generation-fenced workers materialize
the mapping; issued and graded evidence retains its original pin, while
in-progress work is reissued or excused and completed work receives superseding
receipts and deterministic recalculation when required. There is no per-course
approval step.

The Question Library exposes these closed Serde-generated, browser-safe data
objects. The current Question Search request and retained filter use
`snake_case`; the returned Question Library data objects use their explicit
camelCase contracts and reject unknown fields:

| Data object            | Browser fields                                                                                                                                                                                                                         |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `QuestionSummary`      | `questionId`, `latestQuestionRevision`, `backend`, `questionType`, `capabilities`, `metadata`, `authorship`, `availability`, `publishedAt`                                                                                             |
| `QuestionStatistics`   | Insufficient: `state`; available: `state`, `formulaVersion`, `observedCourseCount`, `independentLearnerObservationCount`, `difficultyIndex`, `attemptsMean`, `timeMedianSecondsEstimate`, optional `discriminationIndex`, `evidenceAt` |
| `QuestionSearchResult` | `summary`, `evidence`                                                                                                                                                                                                                  |
| `QuestionUseSummary`   | `globalCourseCount`, `globalAssignmentCount`, `ownCourseCount`, `ownAssignmentCount`                                                                                                                                                   |
| `QuestionUseDetails`   | `summary`, bounded `ownCourses`, `ownCoursesTruncated`                                                                                                                                                                                 |
| `QuestionSearchPage`   | `items`, optional `nextCursor`, `facets`                                                                                                                                                                                               |
| `QuestionDetails`      | `summary`, answer-free `prompt`, `evidence`, `usage`                                                                                                                                                                                   |

Question Classifications, Question License, Question Authorship, lifecycle, facets, and prompt blocks have
their closed shapes defined by the single-installation plan. Evidence is
released only after its formula-versioned disclosure threshold is satisfied.
Facet counts are published-question metadata counts. Usage is
assignment-to-question reference evidence: global fields carry no course
identity and `own_*` fields describe only courses already authorized to the
requesting Instructor. Neither derives from Student responses, scores,
enrollments, or identities.

These DTOs never contain Student-linked data, accepted responses, grades,
small-cell or linkable cohort data, answer keys, scoring rules, private grader
payloads, source packages, provider credentials or identifiers, workspace
identifiers, Object Addresses, signed URLs, or arbitrary metadata. Presentation
asset delivery has its own typed object authorization.

## Typed scopes for workers and objects

Workers have no browser-session authority and are not HTTP targets. A worker
may act only through a locked, current typed lease containing an opaque lease
token, typed durable scope (`course`, `workspace`, `catalog`, or `system`),
Job Kind, target identity, Handler/Effect Committer pair, and generation fence.
The Job claim-and-lease operation and RLS validate all of those values on claim, renewal, and
completion. A stale, foreign, superseded, or Job-Kind-mismatched lease cannot read,
commit, or repeat work. Queue messages carry bounded IDs and generations, not
names, raw responses, answer keys, grades, object URLs, or authority claims.

Object metadata and delivery likewise use exactly one typed scope: public
Question Library presentation, private workspace, or course Student record. Browser
markup names a logical `QuestionAssetId`; it never receives a bucket name, physical key,
source path, or signed URL. Public Question Library presentation assets may use immutable
public delivery. Workspace and course-record objects require the current typed
relationship, database registry binding, and retention fence. Protected
delivery writes its audit event before issuing a short-lived `no-store`
redirect; signed URLs never enter DTOs, markup, browser storage, logs, or
durable records. Source, cache, and temporary-processing objects are never
delivery targets.

## Future observer relationships

The current live product personas remain Student, Instructor, and Sysadmin.
The future extension point is a separate `course_relationship` with an explicit
`course_capability_grant`: subject, course, relationship kind, bounded
capabilities, issuer, lifecycle/revocation state, audit identity, and required
consent/disclosure policy. It is distinct from current Student and Instructor
memberships and cannot widen their predicates.

A future Course Observer receives a separately typed exact-course relationship
with a named assignment-completion projection and privacy-safe aggregate-grade
projection. The completion projection contains the assigned Student identity,
assignment identity, and completed/not-completed state only. It contains no
individual score, grade, response, attempt detail, feedback, accommodation,
small-cell aggregate, Student-record asset, or arbitrary course record. The
aggregate projection applies its disclosure threshold and contains no row-level
Student information. A future Student Observer requires explicit revocable
consent, an exact Student binding, a read-only projection, and its own audit
events. Future Grader, Course Observer, and Student Observer packages must
define their visible workflow, capability matrix, revocation behavior,
privacy/disclosure rules, and denial tests before activation.

## Deterministic grading boundary

An issued attempt is the sole student grading authority. It binds the exact
Student owner, course assignment, immutable question revision, seed, timing
state, and grading backend. The server checks that binding, current Student
authority, timing, idempotency, Question Type, presentation consistency, and
lifecycle before it loads answer-bearing material through a separately injected
restricted grader capability. Correctness, partial credit, feedback, and score
persistence are deterministic server decisions.

Browser-supplied question kind, choice label, seed, checksum, rendered-item
identifier, descriptor digest, or response tag is evidence to validate, never
authority. The normal Store, API pool, browser DTOs, Wasm closure, object
service, and client code do not receive answer tables or grading capabilities.
Provider failure is question-local and fails closed; it never authorizes a
browser checker or an unrelated grading backend.

## Concealment, audit, and enforcement

Protected-resource responses reveal only information an authorized caller needs
to recover. Missing, foreign, and unauthorized course, Student-record,
workspace, and protected-object resources normally share an absent result.
After authorization succeeds, a caller can receive an appropriate validation,
revision, lifecycle, or typed conflict result. A Student may receive a clear
forbidden result for a known action category, such as assignment creation in
that Student's own course.

Audit records capture the authenticated account, action, durable scope, result, and time for
authentication events, authorization denials, Account State changes,
membership and relationship changes, sensitive course-record reads, export,
retention, protected delivery, and Job lease transitions. They exclude
session credentials, signed URLs, raw Student responses, grades where an audit
reference suffices, answer keys, private grader material, and browser-supplied
authority. Auditing records who acted; it never grants a capability.

Each protected route, Store method, authorization function, worker action, and object delivery
must document the exact account predicate, durable target, field projection,
concealment result, audit event, and revocation point. Permanent tests cover
stable authorization behavior. Fresh PostgreSQL/RLS, provider, worker-lease,
and multi-replica exercises remain named disposable acceptance evidence under
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Related references

- [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) records the owner decisions.
- [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) explains the shared publication and relationship model.
- [SECURITY_MODEL.md](SECURITY_MODEL.md) defines session, answer-secrecy, provider, and asset safeguards.
- [CONTRACTS.md](CONTRACTS.md) maps implemented owners and module boundaries.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines student render, response, and grading payloads.
