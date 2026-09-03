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
Routes, browser fields, queue payloads, Object Addresses, and Question Backend responses are evidence or input;
they establish only their exact membership, workspace, course, or worker authority.

## Authority relationships

An Active Instructor Account has Product Role `instructor` and Account State `active`.
Sysadmin vetting occurs before that Account is created; it does not create a second
authorization predicate. An Active Instructor Account authorizes global Instructor capabilities:
course creation, publication, Question Library discovery, Question Folders, Question Stars,
Question Watches, Saved Question Searches, reuse, and improvement. Account deactivation closes
each capability in the protected transaction.

The `sysadmin` Product Role does not satisfy the Instructor Account predicate. A Sysadmin
creates an Instructor Account after Instructor Vetting; a person who needs both roles uses
separate Accounts. Course creation then atomically creates the first ordinary
Instructor membership; Sysadmin status does not add creator authority.

`current_course_instructor(account_id, course_id)` requires a current direct Instructor membership
for that exact course. The membership foreign key requires an Instructor Account. Course creation atomically creates
the first ordinary Instructor membership. It does not create a creator, owner, or privileged
course-authority row. Every current Teaching Team Member receives the same teaching mutation and FERPA-read
decision for equivalent state; audit rows identify the authenticated account without changing authority.

Student work requires the exact course relationship and Student ownership of the durable child
record. A private authoring input requires its current Authoring Workspace owner or Workspace
Collaborator relationship; a Draft Blueprint Revision requires its own Blueprint Collaborator
relationship. A published question has exactly one Instructor-visible Question Library
state: every active Instructor can discover and reuse its safe Question Library data while its visible
lifecycle is Question Revision Availability `Available` or `Archived`. Selection eligibility is separate: only `Available`
Question Revisions are eligible for ordinary new selection; Archived Question Revisions remain available
for discovery and exact historical references but are excluded from ordinary new selection. Drafts
remain private until successful validated publication.

`Sysadmin` is a platform role, not ambient FERPA authority. A Sysadmin reads or changes Student work
only through a narrow, audited support capability or an ordinary current Instructor membership.

| Durable target                                                  | Database authority                               | Boundary that remains private                            |
| --------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------- |
| Account, session, passkey                                       | Exact global account/session                     | Credentials and authentication evidence                  |
| Published Question                                              | Active Instructor Account                        | Answer keys, private grading, source, and credentials    |
| Draft Question authoring                                        | Authoring Workspace Owner/Workspace Collaborator | Unshared source and author preview                       |
| Draft Blueprint Revision contribution                           | Blueprint Course Owner/Blueprint Collaborator    | Other Blueprint Courses, revisions, and Course Instances |
| Course, roster, assignment                                      | `current_course_instructor`                      | Other courses and former memberships                     |
| Assignment Attempt, Question Attempt, response, grade, artifact | Student ownership or current course Instructor   | Other Students, courses, and inactive records            |
| Job, export, object, Question Backend state                     | Locked typed lease and durable target            | Caller-supplied scope and foreign targets                |

Question Revision Availability does not narrow Active Instructor Account discovery. The Question Library safely
returns Question Revision Availability on every Published Question. Assignment creation
and other ordinary new-selection operations require `Available`; exact historical
resolution and retained assignment references may resolve `Archived` Question
Revisions without making them newly selectable.

## Course relationships

Current Course Membership episodes represent only Student and Instructor participation. Their
Active or Ended state derives from immutable Course Membership Events. Course Invitation acceptance
verifies the target's Instructor Product Role, exact Invitation state, and membership transition
in one transaction. A deactivated Account or ended membership closes course-Instructor operations
immediately in the protected transaction.

Course Observer access uses the distinct Course Observer Relationship. It binds an Active
Instructor Account to one exact Course Instance for its closed answer-free read scope and never
satisfies Student-owner, Teaching Team, Gradebook, response, export, Assignment-write, or worker
predicates. Student Observer and Grader relationships remain separate future product designs.

- A Grader receives only the bounded grading work in its completed relationship package.
- A Course Observer receives a separately typed anonymous aggregate-grade result with disclosure
  thresholds; it contains no subject, enrollment, row, small-cell, or linkable metadata.
- A Student Observer receives a distinct one-Student result only with explicit revocable consent
  and its own disclosure contract.

Fabricated, expired, and revoked future grants fail all current FERPA predicates.

## Row-level security

Every protected table enables and forces PostgreSQL RLS. Policies use the transaction-local authenticated Account
and operation-specific predicates for current Instructor membership, Student ownership, workspace
relationship, or a leased capability. A policy must deny when required context or relationship is
missing. The protected Store/PostgreSQL operation performs the predicate and data operation in the
same transaction, preventing a route-level check from outliving revocation.

The application uses least-privilege roles. Runtime logins are `NOINHERIT`, `NOSUPERUSER`, and do
use no `BYPASSRLS`; table owners, superusers, and broad capability-role membership are not runtime account
identities. The public schema is private by default, and grants are explicit per table, view, and
function.

Security-definer authorization functions implement only a registered capability whose arguments, authenticated Account, durable
target, and audit effect they verify. Function owners may have the limited privilege necessary for
that one operation, while ordinary application roles receive no direct shortcut to private grading,
retention, queue, object, or Question Backend data. `ple_app` performs only authenticated Session
create, load, lease, and stage operations. `ple_worker_login` may `SET ROLE` only to
`ple_imathas_question_backend_grading_worker`; that capability executes only the grading
claim/commit `SECURITY DEFINER` procedures and has no direct protected-table access. Session lookup,
migration tooling, API Store work, workers, grading, and registered capabilities use distinct database
credentials or roles with only their needed grants.

## Typed operations and objects

A worker first locks a current lease. The immutable job manifest and lease derive the job's typed
course, workspace, Question Library, object, export, retention, or system target. Job Kind Registration,
generation, Job claim-and-lease grant, and target type must agree before a handler reads, writes, dispatches, or
finalizes anything. Queue payloads, retry input, Question Backend responses, and object references cannot
widen that scope.

Each object metadata and delivery record has one typed scope: Question Library presentation asset, private
workspace asset, or course-record asset. Public Question Library presentation delivery is distinct from
private source delivery. Course-record delivery rechecks its course/Student authority and retention
fence; opaque object identifiers and signed URLs do not bypass it.

iMathAS Question Backend Sessions, iMathAS Render Cache Entries, exports, and retention operations
bind to their exact course, assignment, attempt, export, or retention target. Question Backend
credentials and answer-bearing payloads remain server-only.

## Radioactive records and retention

`Radioactive` is the operational label for a relation that can contain or directly locate a
Student's course record. It is not a human or PostgreSQL role. The following exact record categories
receive the same account-and-relationship-scoped RLS, minimum-field, audit, retention, incident-response, and backup
handling. Partition children, views, staging relations, query results, exports, diagnostics, and
restores inherit the highest label of their inputs.

| Record category                                       | Radioactive relations                                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Roster and invitation                                 | `course_membership`, `course_membership_event`, `student_record`, `course_invitation`, `course_invitation_event`                                                                                                                                                                                           |
| Student work and Gradebook evidence                   | `assignment_attempt`, `issued_question`, `question_attempt`, `question_submission`, `assignment_submission`, `assignment_grade_calculation`, `assignment_grade`, `assignment_grade_event`                                                                                                                  |
| Assignment Analysis                                   | `assignment_analysis`, `assignment_question_analysis`, `assignment_analysis_receipt`                                                                                                                                                                                                                       |
| Student exports                                       | `assignment_export_request`, `assignment_export_artifact`                                                                                                                                                                                                                                                  |
| Course and attempt linkage                            | `course_instance`, `course_object_reference`, `assignment`, `assignment_revision`, `assignment_attempt`, `issued_question`, `question_attempt`, `question_submission`, `assignment_submission`, and protected receipt records                                                                              |
| iMathAS Question Backend, delivery, and audit linkage | `imathas_question_backend_result_exchange`, `imathas_question_backend_session`, iMathAS Question Backend Reference, `imathas_render_cache_entry`, `object_delivery`, exact Object Delivery owner relationships, Object Delivery Access Event (Account, allowed-or-denied decision, and access time), `job` |
| Retention evidence                                    | `course_retention_plan_revision`, `course_retention_event`                                                                                                                                                                                                                                                 |

Global account/session records are restricted account/security data, not FERPA data by themselves.
Private source, Answer Keys, Question Feedback, Question Answer Explanations,
and format-specific Question Grading Input are highly restricted for assessment integrity,
not Student records unless joined to Student activity. The global published aggregate remains
identity-free; its Student-linked contribution receipt is radioactive, and course-local analysis is
radioactive because small cohorts can be identifiable.

Retention keeps shared published Question Library content and private drafts outside course-record deletion.
Course Student records move through `active -> archived -> deleted`. The database centrally fences
Student-facing records, exports, iMathAS Question Backend records, and course-record assets as archive or
deletion starts. Authorized current Instructors may retain course and Assignment Content without
restoring student records. Retention Job prepare and commit operations use the exact course/stage/generation manifest and a
renewed lease, so stale work cannot commit after a newer retention generation.

## Fresh migration epoch

SD1-C creates the single-installation schema only on freshly cleaned disposable stack data. It does
not preserve an installation-scope compatibility layer. The Migration Allocation Registry allocates the exact next available
number in these ranges:

| Range                                                | Allocated capability scope                                                                                                                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2026082901`                                         | Principal baseline, schemas, capability roles, and default ACLs                                                                                            |
| `2026082902`-`2026082906`, `2026082933`-`2026082934` | Accounts, passwordless credentials, Instructor vetting, authenticated-session resolution, atomic credential completion, and Sysadmin Account Creation      |
| `2026082907`-`2026082909`                            | Global immutable Question Library, publication, discovery, and stewardship                                                                                 |
| `2026082910`-`2026082912`                            | Private authoring, Blueprints, Question Folders, and Saved Question Searches                                                                               |
| `2026082913`-`2026082916`                            | Courses, equal Teaching Team Members, Students, invitations, curricula                                                                                     |
| `2026082917`-`2026082920`                            | Assignment Attempts, schedules, Issued Questions, submissions, artifacts                                                                                   |
| `2026082921`-`2026082924`                            | Automated grading, Gradebook, analysis, improvement threads                                                                                                |
| `2026082925`-`2026082928`                            | Typed jobs, exports, objects, retention, iMathAS Question Backend state                                                                                    |
| `2026082929`-`2026082936`                            | Authorization Checks, forced RLS, grants, schema acceptance helpers, Account Creation, Draft Blueprint Revision, and Question Revision Statistics evidence |

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
observer non-escalation; typed worker confused-deputy refusal; object delivery; iMathAS Question Backend;
export; retention; cleanup; and migration idempotency/checksum status.

Production-browser acceptance proves Question Library discovery/reuse, equal Teaching Team Member behavior,
immediate revocation, Student submission-to-Gradebook convergence, answer-free Question Library responses,
accessible interaction, and role-appropriate screenshots on the canonical real stack.

Graphify maps, retired-identifier inventories, old-to-new schema allocation, clean-volume schema
fingerprints, and Migration Check evidence are one-time evidence. They are not permanent test
cases. The final material tree runs `source source_me.sh && ./all_test.sh` only after focused and
connected required gates are green; skipped required lanes keep the package incomplete.
