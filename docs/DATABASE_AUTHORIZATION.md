# Database authorization

This document explains how PostgreSQL enforces the product decisions in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Current table and function identifiers
are implementation evidence. A legacy `assignment` or lifecycle name does not
override the product contract.

## Principle

PLE Accounts are not PostgreSQL roles. An authenticated request installs its
trusted Account identity for a protected transaction. Database policies and
functions resolve current Account state and the exact Course, Student,
workspace, Blueprint-owner, or narrow service relationship before reading or
changing protected data.

The database is default-deny. Protected relations use forced row-level security
where appropriate; runtime roles receive explicit grants and do not own the
database, bypass RLS, or inherit broad migration authority. Browser fields,
URLs, queue messages, and object paths are untrusted selectors.

## Bootstrap and runtime roles

Privileged bootstrap and migration run outside the application. The canonical
base schema owns pre-production structure; after a production baseline, later
structural changes use reviewed forward migrations.

Application and service logins receive only the capability their process needs:

| Capability | Purpose |
| --- | --- |
| application | Authenticated product operations and schema compatibility checks |
| authentication | Session resolution and account-state operations |
| Student | Narrow Student-facing operations when separate database privilege is useful |
| service | One exact public-asset, retention, backend, or other approved technical operation |
| migration | Bootstrap/schema change only; unavailable to runtime requests |

A generic worker or queue role is not product authority. Each service function
must bind its exact target and cannot infer a grading, recovery, audit, or
compatibility workflow merely because job infrastructure exists.

## Product authorization

- A Student reads and changes only the Student's own record through an active
  relationship to the exact Course Instance.
- Every current co-Instructor has equal teaching and FERPA authority for the
  exact Course Instance. The creator or first Instructor has no extra power.
- Private Draft Question operations require the current authoring-workspace
  relationship.
- A Blueprint Course's owner alone saves its content and changes its Private,
  Public, or Archived lifecycle state.
- Vetted Instructors may discover Public Blueprints and may explicitly include
  Archived Blueprints in read-only discovery; Private is owner-only.
- Current co-Instructors of a daughter Course decide whether to apply offered
  changes to its existing Assessments. Automatic creation of a newly added
  Blueprint Assessment does not grant the Blueprint owner Course access.
- Any vetted Instructor may create a Blueprint Course Change Proposal; only
  the receiving Blueprint owner accepts changes into a new Revision.
- A Sysadmin product role is not ambient Course membership or FERPA authority.
  Support access is deliberate, scoped, and recorded.

Account deactivation closes new access while preserving authorship, Course
relationships, Student Work, and history. Course membership removal does not
delete Student records. Retention and permanent closure are separate
operations.

## Revision and current-state boundaries

Published Question Revisions and Blueprint Revisions are immutable. Draft
Questions, Course Instances, Assessments, Attempts, and Student Work do not gain
revision histories from database Edit Numbers or event rows.

A Blueprint is created Private with Revision 1. A content Save creates a new
Revision only when canonical content changes. Names and lifecycle state remain
current lineage metadata.

Update-review records and Blueprint Course Change Proposals do not become
Revision families. Accepted Blueprint content changes create an ordinary new
Blueprint Revision.

A Course Instance Assessment is current state and is Unreleased or Released.
Date-derived availability is not a Closed or Archived stored state.

## Student Work and Assessment Unrelease

Ordinary runtime roles cannot mutate submitted Student Work. An open Assessment
Attempt permits saved-response replacement through the authorized product
operation. Whole-Assessment submission makes the response evidence immutable.

Assessment Unrelease is a narrow destructive operation. It:

1. verifies an equal co-Instructor relationship, Released state, current Edit
   Number when used, and typed confirmation of the exact Assessment title;
2. changes the Assessment to Unreleased;
3. deletes all Student Work owned by that Assessment atomically; and
4. preserves the Assessment definition, Course relationships, and shared
   Published Questions and Pools.

The implementation may use a dedicated no-login function to constrain this
authority. Human Guidance does not require generalized audit events, grading
receipts, correction links, or statistics-rebuild machinery as part of the
product definition.

## Retention authorization

The final Assessment deadline starts the Course retention clock and later
Student activity resets it. An idempotent background process can send the
Instructor notice, remove FERPA-protected records from normal interfaces,
preserve recoverability during the retention period, and permanently delete
them at expiry.

Database enforcement must prevent Student and ordinary Instructor routes from
bypassing the archive stage. Course metadata, Assessment definitions,
Questions, and settings remain after FERPA-protected Student records are
deleted. Numeric durations and exact job/event/table shapes are not decided in
Human Guidance.

## Trusted function seams

Some operations need a small privileged seam for session resolution,
cross-table invariants, exact destructive operations, or narrow service work.
Such functions:

- use a fixed trusted `search_path`;
- are revoked from `PUBLIC`;
- receive only the minimum grants needed;
- recheck exact Account and relationship authority; and
- perform the protected read/write in one transaction.

## Deployment boundary

Runtime images do not need schema-installation tools or DDL authority.
Provisioning supplies separate TLS-verified credentials for migration,
application, authentication, and any narrow service capability. Successful
infrastructure provisioning alone is not authorization evidence.

## Verification scope

Permanent checks protect default-deny grants, forced RLS, owner/runtime
separation, trusted-function configuration, non-enumerating foreign-record
failures, equal co-Instructor access, scoped Sysadmin support, whole-Attempt
submission, Unrelease closure, and retention fences. Connected PostgreSQL
exercises remain distinct from catalog-only or mocked evidence.
