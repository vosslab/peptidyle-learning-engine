# User roles

PLE has three human personas: **Student**, **Instructor**, and **Sysadmin**. The implemented SD1
Account and Authenticated Session boundary carries one immutable Student, Instructor, or Sysadmin
role; a person who needs more than one role uses separate accounts. Course authorization remains
separate so future bounded Grader, Course Observer, or Student Observer relationships can be added
without weakening the personas. Full service, database, and release acceptance remains separate.

This document owns the role vocabulary. The operation-specific rules remain
in [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and the data
handling rules remain in [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Binding SD1 roles

### Student

- A Student account claims an Instructor-issued course invitation.
- A Student works assigned activities and views only that Student's currently accessible educational
  records.
- A Student cannot enumerate classmates, author content, manage a course, inspect Answer Keys,
  Question Feedback, Question Answer Explanations, or Question Grading Input,
  or use another Student's identifiers.

### Instructor

- An Instructor account belongs to a real person manually approved by a Sysadmin.
- Current approval grants the same global Instructor capabilities as every
  other active Instructor.
- Current approval plus direct course membership grants teaching authority for that course.
- A course may have multiple equal Teaching Team Members. Approval withdrawal closes global and
  course-Instructor capabilities; membership revocation closes that course's authority.

### Sysadmin

- A Sysadmin creates a platform operator Account through Account Creation.
- A Sysadmin operates the platform, bootstraps CourseInstances through the closed pre-course
  Course Instance Creation authority, and completes support work through an exact-course capability.
- The pending SD1 Course Instance Creation command binds the exact Blueprint source, approved assigned
  Instructor account, and server-reserved CourseInstance identity. One transaction creates the
  CourseInstance, that Instructor's initial direct membership, and an append-only audit event;
  the Sysadmin account receives no course membership.
- A support capability is purpose-bound, time-bounded, revocable, audited, and limited to its
  registered Operation Kind and `minimum_projection`-bounded response data. Sysadmin status supplies platform operations,
  not ambient course-record authority.

Dr. Voss may use separate Instructor and Sysadmin accounts. Instructors are
approved only after real-person validation. Manual Account Creation assigns
each role.

## Binding SD1 course membership

The binding SD1 product contract keeps course membership from adding more types
of users. It records how an account with the matching role relates to one exact
course:

- `Student` means the person may use only their own active Student paths.
- `Instructor` means the person may teach and administer that exact course.

A course may have multiple current Instructor members. They are equal Teaching Team Members for that
course; course creation only establishes the first membership and does not create a privileged
course-owner role.

## Future course relationships

The authorization model remains capable of expressing additional course relationships through
explicit, least-authority capabilities:

- a Grader may receive bounded grading work without course-management authority;
- a Course Observer may receive exact-course, named assignment-completion visibility and
  privacy-safe aggregate grades, without individual Student scores; and
- a Student Observer may receive a read-only view of one exact Student's consented records through
  explicit revocation, audit, and FERPA disclosure terms.

These are roadmap relationships rather than current live-demo personas. Their schemas and APIs land
with their complete visible workflow, revocation behavior, audit evidence, and privacy validation.

`Sysadmin` is not a course-membership value. Sysadmin accounts receive no
course membership. A person who needs teaching authority uses an approved
Instructor account with a current direct Instructor membership.
Support uses the closed `SysadminSupportCapability` registry in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md). This keeps a platform credential from
becoming ambient access to grades, responses, attempts, or other Student data.

PLE uses one installation. Pending SD1 initial course bootstrap uses the closed
Sysadmin Course Instance Creation authority; after it creates the direct course
relationship, ordinary account flows derive authority from the authenticated
account and current course membership.

The implemented passwordless foundation, owned by migrations `2026082902` through
`2026082904` and `2026082933`, issues one immutable account and session role. It confirms that
every selected Student or Instructor membership matches that role, and a
Sysadmin account cannot select a course membership. The operator assigns the
Product Role at Account Creation; the application does not change it.

## FERPA and student data

Treat course-linked student educational records as **radioactive**. Collect
the minimum, keep it course-scoped, expose it only for the exact
teaching operation, exclude it from logs and general analytics, and delete it
under the course retention lifecycle.

This includes, at minimum:

- roster membership, roster email, and optional external roster ID;
- Course Memberships, accommodations, Assignment Attempts, Question Attempts, responses, and
  feedback;
- grades, Assignment Analysis, grading-operation evidence, and grade exports;
- protected-delivery grants and educational audit evidence; and
- opaque identifiers or metadata when they link a person to any of the above.

An account email, passkey public credential, account label, and authentication
ceremony are sensitive account/security data. They are not automatically an
educational record merely because the account may later join a course. Once a
value is copied into or linked with a course record, the course-linked copy is
FERPA data and follows the radioactive handling rule.

The [database authorization reference](DATABASE_AUTHORIZATION.md#radioactive-records-and-retention)
names the current direct and linkage-bearing PostgreSQL relations and explains
how the label follows query results, backups, replicas, and restores.

## Service identities are not users

The public-asset publisher, API, worker, automated-grading service, renderer, database roles, and
cloud task roles are service identities. The word `publisher` may describe the
dedicated publication service or the act of publishing, but it is never a
human Product Role. The current `AccountRole` enum is the implementation name
for that classification. An Instructor publishes reviewed content through the
application; the dedicated publisher service writes and verifies immutable
public asset bytes before activation after the committed outbox decision.

## Authorization rules

- Authenticate the PLE Account first; resolve exact authorization on the server.
- Check direct course membership at the Store/database boundary for every
  FERPA-bearing operation unless its contract defines a narrower audited
  Sysadmin support or lifecycle capability.
- Issue a Sysadmin support capability only through the closed registry, bound
  to one exact course, a stated purpose, an issuer, an Operation Kind, an
  expiry, and `minimum_projection`-bounded response data. Record issuance, use, revocation, authenticated account,
  course, action, and time; keep roster PII and invitation secrets out of audit
  payloads.
- Route roster, schedule/accommodation, assignment-content, deterministic
  reissue/recalculation, and payload-free retention lifecycle support through
  that capability. Route initial CourseInstance bootstrap through the separate
  pre-course Sysadmin platform authority.
- Provide support results at the minimum useful detail and keep grades,
  responses, Assignment Attempts, exports, and Assignment Analysis behind direct Instructor
  membership or a separately approved narrow operation.
- Keep Student-owner and Instructor-history capabilities separate.
- Conceal missing and unauthorized FERPA records with the same result where
  existence itself is sensitive.
- Serialize Instructor membership revocation with response disclosure,
  roster mutation, grading, exports, and other protected operations.
- Accept browser requests through server-issued references and derive role,
  Account, membership, and approval from server-owned records.

## Canonical terms

Use **Student**, **Instructor**, and **Sysadmin** when referring to people.
Use **course instructor** for course-level authority. Use **publisher service**
or **public-asset publisher** only for the dedicated service identity. The
words owner and collaborator describe access to one private workspace, not
additional types of users. The words manager and administrator may still
describe software/process concepts such as a package manager or Secrets
Manager, but never a PLE human role.

Use **Student** in PLE-owned type, module, route, field, and table names whenever the subject is a
person with the Student role or that person's course work. Use **User** before a course role is
known. Use **learning** only for educational-system concepts, such as learning data or learning
outcomes. `student` is not a fourth role or an alias for Student in new PLE-owned names.

Active Instructor-roadmap work packages use the temporary `WP-INST-*` namespace. These planning
keys exist only while their owning plans remain active; completed planning can retire the labels.
Current titles, prose, code, APIs, and schemas use **Instructor** directly.

## Enforcement owners

- [question-model auth](../crates/question_model/src/auth.rs) owns the closed
  human Product Role enum, currently named `AccountRole` in Rust.
- [question-model course](../crates/question_model/src/course.rs) owns the one
  closed `CourseMembershipRole` relationship enum; it does not define more
  human roles.
- [learning-data-access](../crates/learning-data-access/) owns direct
  membership, revocation serialization, account-and-relationship-scoped RLS, and Store
  capabilities.
- The removed pre-SD1 role schema is historical evidence only. The fresh
  [global Account and session migration](../schemas/migrations/2026082902_global_account_authenticated_session.sql)
  owns fixed singular immutable Account and Authenticated Session role storage.
  Sysadmin Instructor Vetting occurs before Account Creation; Product Role and Account State
  then own the resulting Instructor Account authorization boundary.
