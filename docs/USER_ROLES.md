# Product roles and course membership

PLE has three human Product Roles: **Student**, **Instructor**, and
**Sysadmin**. An **Account** is a global authenticated identity with exactly
one immutable Product Role. A person needing more than one role uses separate
Accounts. Account State determines whether that role is currently active.

This document owns role vocabulary. Exact authorization predicates are in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and PostgreSQL role
and service-capability boundaries are in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

## Student

- A Student joins an exact Course Instance through a Student Course Membership
  and Student Record.
- A Student can use only that Student's current course-scoped Assignment,
  Attempt, response, submission, history, and presentation paths.
- A Student cannot enumerate classmates, author shared Questions, administer a
  course, inspect another Student's work, or receive answer keys, private
  grading input, or teaching projections.

## Instructor

- An active Instructor has global Question Library and authoring capability.
- An active Instructor with a current Instructor Course Membership has the
  teaching authority for that exact Course Instance.
- Every current Instructor member is an equal Teaching Team member. Course
  creation establishes the first membership; it does not create a permanent
  creator or owner privilege.
- Membership revocation closes only that course authority. Account
  deactivation also closes the Account's Instructor capabilities.

## Sysadmin

- A Sysadmin performs platform operations and creates approved Instructor
  Accounts through the bounded account-creation operation.
- A Sysadmin does not receive Course Membership merely by being Sysadmin and
  has no ambient access to course records or Student Work.
- Course bootstrap and support work use their own exact, audited scope. They
  do not turn a platform role into teaching authority.

## Course membership

Course Membership is a current relationship between one Account and one Course
Instance. Its role must match the Account's Product Role:

| Membership role | Current authority |
| --- | --- |
| Student | The Account's own current Student Record and Student-facing course paths. |
| Instructor | The complete current Teaching Team operation set for that Course Instance. |

Membership history remains durable evidence, but only the current active
relationship grants access. The database evaluates the relationship within
the protected operation, so a prior browser result or route decision cannot
outlive a revocation.

## Service identities are not human roles

`ple_app`, `ple_auth`, `ple_student`, public-asset publication, grading,
workers, schema owners, and `ple_unrelease_executor` are PostgreSQL
capabilities or service identities. They are not Accounts, Product Roles,
Course Membership roles, browser personas, or human permissions. Each service
login can assume only the capability required by its process.

The Unrelease executor is especially narrow: it can perform the guarded,
audited Assignment Unrelease transaction, but it is not an application or
worker login and confers no ordinary Student-record access.

## Canonical language

Use **Student**, **Instructor**, and **Sysadmin** for people. Use **course
instructor** or **Teaching Team member** for exact course authority. Use
**service identity** for a non-human technical principal and **PostgreSQL
capability** for its restricted database role. A workspace owner is an access
relationship, not a fourth Product Role.

Only **Question Revision** and **Blueprint Revision** are product Revision
concepts. Assignment and Course Instance configuration are current state;
existing Student Work relies on retained Attempt and Issued Question evidence.
