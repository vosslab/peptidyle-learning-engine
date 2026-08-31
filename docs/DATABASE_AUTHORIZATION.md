# Database authorization

## Intended database model

PLE is one installation. Global accounts use `AccountId`; there is no institution selector, installation
identity, leading scope key, or client-selected database context. This reference is the sole durable
PostgreSQL authorization authority and the canonical database authorization target for the fresh SD1
epoch. [SECURITY_MODEL.md](SECURITY_MODEL.md) provides the cross-cutting security model and points
here for all durable PostgreSQL authorization detail. The
[implementation status](active_plans/implementation_status.md) allocates its implementation;
existing pre-epoch schema documents are migration input, not an alternate model.

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) supersedes this document for
the meaning of PLE-owned terms. This document owns PostgreSQL authorization
implementation only.

The server derives `AuthenticatedSession { account_id, session_id }` from a valid global Authenticated Session. Each
protected database transaction sets only the trusted, transaction-local `ple.session_account_id` value.
Forced RLS accepts the resolved authenticated Account value for the operation.
Routes, browser fields, queue payloads, object keys, and provider responses are evidence or input;
they establish only their exact membership, workspace, course, or worker authority.

## Authority relationships

`approved_instructor(account_id, now)` is the one current, manually approved Instructor predicate. It
authorizes global Instructor capabilities: course creation, publication, shared-catalog discovery,
Question Collections, Stars, saved searches, reuse, and improvement. Approval withdrawal closes each of
those capabilities in the protected transaction.

The `sysadmin` platform role does not satisfy `approved_instructor`. A Sysadmin
must complete the explicit operator-led Instructor approval path before creating
or teaching a course. Course creation then atomically creates the first ordinary
Instructor membership; Sysadmin status does not add creator authority.

`current_course_instructor(account_id, course_id, now)` requires both current `approved_instructor`
and a current direct Instructor membership for that exact course. Course creation atomically creates
the first ordinary Instructor membership. It does not create a creator, owner, or privileged
course-authority row. Every current Teaching Team Member receives the same teaching mutation and FERPA-read
decision for equivalent state; audit rows identify the authenticated account without changing authority.

Student work requires the exact course relationship and Student ownership of the durable child
record. A private authoring input requires its current Authoring Workspace owner or Workspace
Collaborator relationship; a Draft Blueprint Revision requires its own Blueprint Collaborator
relationship. A published question has exactly one Instructor-visible shared-catalog
state: every approved Instructor can discover and reuse its safe projection while its visible
lifecycle is `active`, `deprecated`, or `archived`. Selection eligibility is separate: only `active`
questions are eligible for ordinary new selection; deprecated and archived questions remain available
for discovery and exact historical references but are excluded from ordinary new selection. Drafts
remain private until successful validated publication.

`Sysadmin` is a platform role, not ambient FERPA authority. A Sysadmin reads or changes Student work
only through a narrow, audited support capability or an ordinary current Instructor membership.

| Durable target                          | Database authority                             | Boundary that remains private                         |
| --------------------------------------- | ---------------------------------------------- | ----------------------------------------------------- |
| Account, session, passkey               | Exact global account/session                   | Credentials and authentication evidence               |
| Published catalog question              | `approved_instructor`                          | Answer keys, private grading, source, and credentials |
| Draft Question authoring                | Authoring Workspace Owner/Workspace Collaborator | Unshared source and author preview                    |
| Draft Blueprint Revision contribution   | Blueprint Course Owner/Blueprint Collaborator    | Other Blueprint Courses, revisions, and Course Instances |
| Course, roster, assignment              | `current_course_instructor`                    | Other courses and former memberships                  |
| Run, attempt, response, grade, artifact | Student ownership or current course Instructor | Other Students, courses, and inactive records         |
| Job, export, object, provider state     | Locked typed lease and durable target          | Caller-supplied scope and foreign targets             |

Lifecycle does not narrow approved-Instructor discovery. The catalog safely
returns the lifecycle state on every published question. Assignment creation
and other ordinary new-selection operations require `active`; exact historical
resolution and retained assignment references may resolve `deprecated` or
`archived` questions without making them newly selectable.

## Course relationships

Current Course Membership episodes represent only Student and Instructor participation. Their
Active or Ended state derives from immutable Course Membership Events. Course Invitation acceptance
verifies the target's current Instructor Approval, exact Invitation state, and membership transition
in one transaction. A revoked approval or ended membership closes course-Instructor operations
immediately in the protected transaction.

Course Observer access uses the distinct Course Observer Relationship. It binds an Approved
Instructor Account to one exact Course Instance for its closed answer-free read scope and never
satisfies Student-owner, Teaching Team, Gradebook, response, export, Assignment-write, or worker
predicates. Student Observer and Grader relationships remain separate future product designs.

- A Grader receives only the bounded grading work in its completed relationship package.
- A Course Observer receives a separately typed anonymous aggregate projection with disclosure
  thresholds; it contains no subject, enrollment, row, small-cell, or linkable metadata.
- A Student Observer receives a distinct one-Student projection only with explicit revocable consent
  and its own disclosure contract.

Fabricated, expired, and revoked future grants fail all current FERPA predicates.

## Row-level security

Every protected table enables and forces PostgreSQL RLS. Policies use the transaction-local authenticated Account
and operation-specific predicates for current Instructor membership, Student ownership, workspace
relationship, or a leased capability. A policy must deny when required context or relationship is
missing. The protected Store/PostgreSQL operation performs the predicate and data operation in the
same transaction, preventing a route-level check from outliving revocation.

The application uses least-privilege roles. Runtime logins are `NOINHERIT`, `NOSUPERUSER`, and do
use no `BYPASSRLS`; table owners, superusers, and broad broker membership are not runtime account
identities. The public schema is private by default, and grants are explicit per table, view, and
function.

Security-definer brokers implement only a registered capability whose arguments, authenticated Account, durable
target, and audit effect they verify. Broker owners may have the limited privilege necessary for
that one operation, while ordinary application roles receive no direct shortcut to private grading,
retention, queue, object, or provider data. Session lookup, migration tooling, API Store work,
workers, grading, and brokers use distinct database credentials or roles with only their needed
grants.

## Typed operations and objects

A worker first locks a current lease. The immutable job manifest and lease derive the job's typed
course, workspace, catalog, object, export, retention, or system target. Handler family,
generation, broker grant, and target type must agree before a handler reads, writes, dispatches, or
finalizes anything. Queue payloads, retry input, provider responses, and object references cannot
widen that scope.

Each object metadata and delivery record has one typed scope: catalog presentation asset, private
workspace asset, or course-record asset. Public catalog presentation delivery is distinct from
private source delivery. Course-record delivery rechecks its course/Student authority and retention
fence; opaque object identifiers and signed URLs do not bypass it.

External launches, provider cache, exports, and retention operations bind to their exact course,
assignment, attempt, export, or retention target. Provider credentials and answer-bearing payloads
remain server-only.

## Radioactive records and retention

`Radioactive` is the operational label for a relation that can contain or directly locate a
Student's course record. It is not a human or PostgreSQL role. The following exact table families
receive the same account-and-relationship-scoped RLS, minimum-field, audit, retention, incident-response, and backup
handling. Partition children, views, staging relations, query results, exports, diagnostics, and
restores inherit the highest label of their inputs.

| Family                                | Radioactive relations                                                                                                                                                                                                                                                          |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Roster and invitation                 | `course_membership`, `course_membership_event`, `student_record`, `course_invitation`, `course_invitation_event`                                                                                                                                                               |
| Student work and Gradebook evidence   | `assignment_attempt`, `issued_question`, `question_attempt`, `question_submission`, `assignment_submission`, `assignment_grade_calculation`, `assignment_grade`, `assignment_grade_event`                                                                                     |
| Course analysis                       | `course_assignment_analysis`, `assignment_item_analysis`, `course_analysis_evidence`                                                                                                                                                                                            |
| Student exports                       | `assignment_export_request`, `assignment_export_artifact`                                                                                                                                                                                                                       |
| Course and attempt linkage            | `course_instance`, `assignment`, `assignment_revision`, `assignment_attempt`, `issued_question`, `question_attempt`, `question_submission`, `assignment_submission`, and protected receipt records                                                                              |
| External, delivery, and audit linkage | `external_tool_exchange`, `external_tool_launch_session`, `object_delivery_record`, `object_delivery_access_event`, `worker_job`                                                                                                                                               |
| Retention evidence                    | `course_retention_plan`, `retention_lifecycle_event`                                                                                                                                                                                                                            |

Global account/session records are restricted account/security data, not FERPA data by themselves.
Private source and answer-bearing grading material are highly restricted for assessment integrity,
not Student records unless joined to Student activity. The global published aggregate remains
identity-free; its Student-linked contribution receipt is radioactive, and course-local analysis is
radioactive because small cohorts can be identifiable.

Retention keeps shared published catalog content and private drafts outside course-record deletion.
Course Student records move through `active -> archived -> deleted`. The database centrally fences
Student-facing records, exports, external-tool records, and course-record assets as archive or
deletion starts. Authorized current Instructors may retain course and assignment definitions without
restoring student records. A retention broker uses the exact course/stage/generation manifest and a
renewed lease, so stale work cannot commit after a newer retention generation.

## Fresh migration epoch

SD1-C creates the single-installation schema only on freshly cleaned disposable stack data. It does
not preserve an installation-scope compatibility layer. The migration ledger allocates the exact next available
number in these ranges:

| Range                     | Capability family                                                     |
| ------------------------- | --------------------------------------------------------------------- |
| `2026082901`              | Principal baseline, schemas, capability roles, and default ACLs       |
| `2026082902`-`2026082906`, `2026082933`-`2026082934` | Accounts, passwordless credentials, Instructor vetting, authenticated-session resolution, atomic credential completion, and Sysadmin Account Creation |
| `2026082907`-`2026082909` | Global immutable catalog, publication, discovery, and stewardship     |
| `2026082910`-`2026082912` | Private authoring, Blueprints, collections, and saved searches         |
| `2026082913`-`2026082916` | Courses, equal Teaching Team Members, Students, invitations, curricula       |
| `2026082917`-`2026082920` | Assignment Attempts, schedules, Issued Questions, submissions, artifacts |
| `2026082921`-`2026082924` | Automated grading, Gradebook, analysis, improvement threads           |
| `2026082925`-`2026082928` | Typed jobs, exports, objects, retention, external-tool state          |
| `2026082929`-`2026082935` | Capability brokers, forced RLS, grants, schema acceptance helpers, Account Creation, and Draft Blueprint Revision evidence |

Each migration owns its local relations, keys, constraints, indexes, functions, policies, grants,
and comments. It uses global content keys and exact user, workspace, course, membership, Student,
lease, and immutable-content identities rather than a legacy scope key.

## Validation lanes

Permanent offline tests prove domain authorization, Store conformance, strict browser contracts,
immutable evidence, grading, idempotency, revocation, and concealment. A data-driven operation
matrix proves identical creator/Teaching Team Member allow and deny decisions in Memory and Store
conformance.

Recurring service acceptance proves fresh migration convergence; RLS refusal without a resolved Account; Student
self versus other-Student and other-course denial; Teaching Team Member mutation and Gradebook read;
immediate membership revocation and approval-withdrawal denial; narrow audited Sysadmin support;
observer non-escalation; typed worker confused-deputy refusal; object delivery; external adapter;
export; retention; cleanup; and migration idempotency/checksum status.

Production-browser acceptance proves shared-catalog discovery/reuse, equal Teaching Team Member behavior,
immediate revocation, Student submission-to-Gradebook convergence, answer-free catalog responses,
accessible interaction, and role-appropriate screenshots on the canonical real stack.

Graphify maps, retired-identifier inventories, old-to-new schema allocation, clean-volume schema
fingerprints, and migration-count reconciliation are one-time evidence. They are not permanent test
cases. The final material tree runs `source source_me.sh && ./all_test.sh` only after focused and
connected required gates are green; skipped required lanes keep the package incomplete.
