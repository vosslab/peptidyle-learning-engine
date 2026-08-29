# User roles

PLE's current live product has three human personas: **Student**, **Instructor**, and
**Sysadmin**. A person may hold more than one current persona. Course authorization is represented
separately so future bounded Grader, Course Observer, or Student Observer relationships can be
added without weakening the current personas.

This document owns the role vocabulary. The operation-specific rules remain
in [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and the data
handling rules remain in [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Human roles

### Student

- A verified PLE account claims an Instructor-issued course invitation.
- A Student works assigned activities and views only that Student's currently accessible educational
  records.
- A Student cannot enumerate classmates, author content, manage a course, inspect grading material,
  or use another Student's identifiers.

### Instructor

- A real person is manually approved and receives direct Instructor membership in a course.
- Current approval grants the same global Instructor capabilities as every other approved Instructor.
- Current approval plus direct course membership grants teaching authority for that course.
- A course may have multiple equal co-Instructors. Approval withdrawal closes global and
  course-Instructor capabilities; membership revocation closes that course's authority.

### Sysadmin

- An operator manually adds the `sysadmin` platform role to the verified PLE account.
- A Sysadmin operates the platform and completes registered support work through an exact-course
  capability.
- A Sysadmin may create or teach a course after completing the explicit Instructor approval path;
  course creation then creates direct Instructor membership in that course.
- A support capability is purpose-bound, time-bounded, revocable, audited, and limited to its
  registered operation family and minimum projection. Sysadmin status supplies platform operations,
  not ambient course-record authority.

The repository owner is both a Sysadmin and an Instructor. Other instructors
are approved only after real-person validation. There is no self-service
promotion to Instructor or Sysadmin.

## Course membership

Course membership does not add more types of users. It records which of the
same human roles relates a person to one exact course:

- `Student` means the person may use only their own active Student paths.
- `Instructor` means the person may teach and administer that exact course.

A course may have multiple current Instructor members. They are equal co-Instructors for that
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

`Sysadmin` is not a course-membership value. A sysadmin who needs
teaching records must also have a current direct Instructor membership.
Support uses the closed `SysadminSupportCapability` registry in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md). This keeps a platform credential from
becoming ambient access to grades, responses, attempts, or other Student data.

PLE uses one installation. Initial course bootstrap is an operator-owned
provisioning step; after a direct course relationship exists, ordinary account
flows derive authority from the authenticated account and current course
membership.

The production passwordless flow derives Student or Instructor from the
persisted course membership selected by the account. The separately stored
`sysadmin` account attribute is operator-controlled: application database
credentials may read it to mint a session but cannot grant it.

## FERPA and student data

Treat course-linked student educational records as **radioactive**. Collect
the minimum, keep it course-scoped, expose it only for the exact
teaching operation, exclude it from logs and general analytics, and delete it
under the course retention lifecycle.

This includes, at minimum:

- roster membership, roster email, optional external roster ID, and group data;
- assignment enrollments, accommodations, runs, attempts, responses, and
  feedback;
- grades, item analysis, grading-operation evidence, and grade exports;
- student uploads, generated student artifacts, protected-delivery grants,
  and educational audit evidence; and
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

The public-asset publisher, API, worker, grader, renderer, database roles, and
cloud task roles are service identities. The word `publisher` may describe the
dedicated publication service or the act of publishing, but it is never a
human `UserRole`. An Instructor publishes reviewed content through the
application; the dedicated publisher service materializes immutable public
asset bytes after the committed outbox decision.

## Authorization rules

- Authenticate the PLE account first; derive the actor only on the server.
- Check direct course membership at the Store/database boundary for every
  FERPA-bearing operation unless its contract defines a narrower audited
  Sysadmin support or lifecycle capability.
- Issue a Sysadmin support capability only through the closed registry, bound
  to one exact course, a stated purpose, an issuer, an operation family, an
  expiry, and a minimum projection. Record issuance, use, revocation, actor,
  course, action, and time; keep roster PII and invitation secrets out of audit
  payloads.
- Route roster, schedule/accommodation, assignment-content, deterministic
  reissue/recalculation, and payload-free retention lifecycle support through
  that capability.
- Provide support results at the minimum useful detail and keep grades,
  responses, runs, exports, and item analysis behind direct Instructor
  membership or a separately approved narrow operation.
- Keep Student-owner and Instructor-history capabilities separate.
- Conceal missing and unauthorized FERPA records with the same result where
  existence itself is sensitive.
- Serialize Instructor membership revocation with response disclosure,
  roster mutation, grading, exports, and other protected operations.
- Accept browser requests through server-issued references and derive role,
  actor, membership, and approval from server-owned records.

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
outcomes. `learner` is not a fourth role or an alias for Student in new PLE-owned names.

Active Instructor-roadmap work packages use the temporary `WP-INST-*` namespace. These planning
keys exist only while their owning plans remain active; completed planning can retire the labels.
Current titles, prose, code, APIs, and schemas use **Instructor** directly.

## Enforcement owners

- [question-model auth](../crates/question_model/src/auth.rs) owns the closed
  human `UserRole` enum.
- [question-model course](../crates/question_model/src/course.rs) owns the one
  closed `CourseMembershipRole` relationship enum; it does not define more
  human roles.
- [learning-data-access](../crates/learning-data-access/) owns direct
  membership, revocation serialization, actor-scoped RLS, and Store
  capabilities.
- [the canonical role migration](../schemas/migrations/2026080928_user_roles.sql)
  owns PostgreSQL wire values, operator-only sysadmin approval, and the rule
  that sysadmin is not general FERPA course authority and that its roster-help
  exception is closed and audited.
