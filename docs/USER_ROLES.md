# User roles

PLE has exactly three human user roles: **Student**, **Instructor**, and
**Sysadmin**. There is no Manager, Administrator, Publisher, or other human
role. A person may hold more than one of the three roles.

This document owns the role vocabulary. The operation-specific rules remain
in [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and the data
handling rules remain in [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Human roles

| Role       | How it is established                                                                         | Authority                                                                                                                                                                                                                                   | Explicit limits                                                                                                                                                                                                                                                                                          |
| ---------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Student    | A verified PLE account claims an instructor-issued course invitation.                         | Work assigned activities and view only that student's currently accessible educational records.                                                                                                                                             | Cannot enumerate classmates, author content, manage a course, inspect grading material, or use another student's identifiers.                                                                                                                                                                            |
| Instructor | A real person is manually approved and receives a direct `Instructor` membership in a course. | Create courses and content; manage only courses where the person has a current direct Instructor membership; view the FERPA records needed to teach those courses.                                                                          | A coarse Instructor session role does not grant access to every course. Instructor access ends when the direct course membership is revoked.                                                                                                                                                             |
| Sysadmin   | An operator manually adds the `sysadmin` platform role to the verified PLE account.           | Operate the platform and perform the narrow sysadmin-only lifecycle actions documented by each contract. A sysadmin with an established tenant context may create a course, which also creates direct Instructor membership in that course. | Sysadmin status never makes the person a course member or grants general access to teaching records. Audited roster support and coarse retention lifecycle actions are explicit exceptions; grades, responses, runs, exports, and item analysis remain unavailable without direct Instructor membership. |

The repository owner is both a Sysadmin and an Instructor. Other instructors
are approved only after real-person validation. There is no self-service
promotion to Instructor or Sysadmin.

## Course membership

Course membership does not add more types of users. It records which of the
same human roles relates a person to one exact course:

- `Student` means the person may use only their own active Student paths.
- `Instructor` means the person may teach and administer that exact course.

`Sysadmin` is deliberately not a course-membership value. A sysadmin who needs
teaching records must also have a current direct Instructor membership. The
one support exception is the closed roster capability below. This keeps a
platform credential from becoming ambient access to grades, responses,
attempts, or other student data.

The platform role also does not select a tenant. Initial production tenant and
course bootstrap remains an operator-owned provisioning step; after a direct
course relationship exists, the ordinary account flow can derive that tenant
without accepting a browser-supplied tenant ID.

The production passwordless flow derives Student or Instructor from the
persisted course membership selected by the account. The separately stored
`sysadmin` account attribute is operator-controlled: application database
credentials may read it to mint a session but cannot grant it.

## FERPA and student data

Treat course-linked student educational records as **radioactive**. Collect
the minimum, keep it course- and tenant-scoped, expose it only for the exact
teaching operation, exclude it from logs and general analytics, and delete it
under the course retention lifecycle.

This includes, at minimum:

- roster membership, roster email, institutional student ID, and group data;
- assignment enrollments, accommodations, runs, attempts, responses, and
  feedback;
- grades, item analysis, manual grading state, and grade exports;
- student uploads, generated student artifacts, protected-delivery grants,
  and educational audit evidence; and
- opaque identifiers or metadata when they link a person to any of the above.

An account email, passkey public credential, account label, and authentication
ceremony are sensitive account/security data. They are not automatically an
educational record merely because the account may later join a course. Once a
value is copied into or linked with a course record, the course-linked copy is
FERPA data and follows the radioactive handling rule.

The [database radioactive table map](DATABASE_TENANCY.md#radioactive-table-map)
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

- Authenticate the PLE account first; derive tenant and actor only on the
  server.
- Check direct course membership at the Store/database boundary for every
  FERPA-bearing operation unless its contract defines a narrower audited
  Sysadmin support or lifecycle capability.
- Permit Sysadmin roster support only through the closed list/invite/policy/
  revoke/import operations. Record actor, course, action, and time whenever
  Sysadmin authority opens that boundary; never put roster PII or invitation
  secrets in the audit payload.
- Keep the Sysadmin retention exception coarse and payload-free; it may change
  lifecycle state but never return roster, response, grade, or artifact data.
- Keep Student-owner and Instructor-history capabilities separate.
- Conceal missing and unauthorized FERPA records with the same result where
  existence itself is sensitive.
- Serialize Instructor membership revocation with response disclosure,
  roster mutation, grading, exports, and other protected operations.
- Do not accept a role, tenant, user ID, or approval assertion from browser
  input.

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
  membership, revocation serialization, RLS context, and actor-scoped Store
  capabilities.
- [the canonical role migration](../schemas/migrations/2026080928_user_roles.sql)
  owns PostgreSQL wire values, operator-only sysadmin approval, and the rule
  that sysadmin is not general FERPA course authority and that its roster-help
  exception is closed and audited.
