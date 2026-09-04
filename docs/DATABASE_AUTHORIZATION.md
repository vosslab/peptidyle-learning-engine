# Database authorization

## Intended database model

PLE is one installation. Global accounts use `AccountId`; there is no institution selector, installation
identity, leading scope key, or client-selected database context. This reference is the sole durable
PostgreSQL authorization authority and the database authorization target for the fresh
pre-production migration epoch. [SECURITY_MODEL.md](SECURITY_MODEL.md) provides the cross-cutting security model and points
here for all durable PostgreSQL authorization detail. [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md)
records the checked-in migration sequence; existing pre-epoch schema documents are migration input,
not an alternate model.

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) supersedes this document for
the meaning of PLE-owned terms. This document owns PostgreSQL authorization
implementation only.

## PostgreSQL service identities

PostgreSQL records database roles, role memberships, privileges, and object
ownership in its system catalogs. These principals are database security
configuration, not rows in PLE Account or application-data tables. Human access
begins with a PLE Account; database roles never create a Student, Instructor, or
Sysadmin Product Role.

Each Database Schema Owner Role is a non-login Service Identity for one physical
schema and its protected objects. For example, `ple_private_owner` owns the
`ple_private` schema. The migration login temporarily assumes the appropriate
owner through `SET LOCAL ROLE` when PostgreSQL requires the object's owner to
perform a schema change. Runtime access instead follows the explicitly granted
application role, the authenticated Account context, and the applicable
row-level-security policy. The principal-baseline migration and PostgreSQL
catalog acceptance checks are the executable authority for the exact role and
privilege set.

The server derives an Authenticated Session from a valid global Authenticated
Session. Session issuance resolves Product Role from the Account, and the
persisted session keeps that immutable, role-pinned Account fact. Account State
is checked by database authority; deactivation or closure blocks new sessions
and revokes existing ones. Each protected database transaction sets only the
trusted, transaction-local `ple.session_account_id` value.
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

Create Instructor Account captures the current session Account once and accepts
only an Active Sysadmin Account. In the same transaction it creates the
server-generated Instructor Account, immutable Product Role, initial Account
State, Authentication Email, and immutable qualified evidence naming that
Sysadmin. The audit relation has forced RLS, no runtime table access, no update
or delete path, and a narrow internal writer. Its evidence is not a credential
or browser-data store and does not create a new application authority.

`current_course_instructor(account_id, course_id)` requires a current Instructor Course Membership
for that exact course. The membership foreign key requires an Instructor Account. Course creation atomically creates
the first ordinary Instructor membership. It does not create a creator, owner, or privileged
course-authority row. Every current Teaching Team Member receives the same teaching mutation and FERPA-read
decision for equivalent state; audit rows identify the authenticated account without changing authority.

Student work requires the exact course relationship and Student ownership of the durable child
record. A private authoring input requires its current Authoring Workspace Owner or Workspace
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
| Job, object, Question Backend state                             | Locked typed lease and durable target            | Caller-supplied scope and foreign targets                |

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
course, workspace, Question Library, object, retention, or system target. Job Kind Registration,
generation, Job claim-and-lease grant, and target type must agree before a handler reads, writes, dispatches, or
finalizes anything. Queue payloads, retry input, Question Backend responses, and object references cannot
widen that scope.

Each object metadata and delivery record has one typed scope: Question Library presentation asset, private
workspace asset, or course-record asset. Public Question Library presentation delivery is distinct from
private source delivery. Course-record delivery rechecks its course/Student authority and retention
fence; opaque object identifiers and signed URLs do not bypass it.

iMathAS Question Backend Sessions and iMathAS Render Cache Entries bind to
their exact course, Assignment Attempt, and Question Attempt relationships.
Question Backend credentials and answer-bearing payloads remain server-only.
The Course Retention schema foundation records a Plan Revision, typed Job,
retention Event, Object Cleanup Manifest, and Object Cleanup Receipt. It has
no current retention Store, procedure, route, worker, reader, frozen manifest
membership, or lease-driven execution authority.

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
| Course and attempt linkage                            | `course_instance`, `course_object_reference`, `assignment`, `assignment_revision`, `assignment_attempt`, `issued_question`, `question_attempt`, `question_submission`, `assignment_submission`, and protected receipt records                                                                              |
| iMathAS Question Backend, delivery, and audit linkage | `imathas_question_backend_result_exchange`, `imathas_question_backend_session`, iMathAS Question Backend Reference, `imathas_render_cache_entry`, `object_delivery`, exact Object Delivery owner relationships, Object Delivery Access Event (Account, allowed-or-denied decision, and access time), `job` |
| Retention evidence                                    | `course_retention_plan_revision`, `course_retention_event`                                                                                                                                                                                                                                                 |

Global account/session records are restricted account/security data, not FERPA data by themselves.
Private source, Answer Keys, Question Feedback, Question Answer Explanations,
and format-specific Question Grading Input are highly restricted for assessment integrity,
not Student records unless joined to Student activity. The global published aggregate remains
identity-free; its Student-linked contribution receipt is radioactive, and course-local analysis is
radioactive because small cohorts can be identifiable.

The intended Course Retention policy keeps shared published Question Library
content and private drafts outside Course Student Record deletion. The present
database baseline is only its schema foundation:
`course_retention_plan_revision`, typed `job`,
`course_retention_event`, `object_cleanup_manifest`, and
`object_cleanup_receipt`. Those records neither determine a current Course
Retention State nor authorize archive, purge, notice, manifest membership, or
Object Cleanup execution.

A future complete Course Retention boundary must add the exact Course
Retention State, Course Retention Notice, Assignment Revision Retention Rule,
frozen manifest-membership relation, authorization, Store, PostgreSQL
procedures, route, worker, reader, renewed lease, and connected acceptance
evidence together. Until then, no current Instructor or browser capability
executes retention work. [RETENTION_POLICY.md](RETENTION_POLICY.md) is the
authoritative current-foundation and future-boundary description.

## Fresh migration epoch

The fresh pre-production migration epoch creates the single-installation schema only on freshly cleaned disposable stack data. It does
not preserve an installation-scope compatibility layer. The Migration Allocation Registry allocates the exact next available
number in these ranges:

| Range                                                                            | Allocated capability scope                                                                                                                                          |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2026082901`                                                                     | Principal baseline, schemas, capability roles, and default ACLs                                                                                                     |
| `2026082902`-`2026082904`, `2026082906`, `2026082933`-`2026082934`, `2026090401` | Accounts, credential foundations, Instructor vetting, authenticated-session resolution, Create Instructor Account, and its qualified immutable audit evidence       |
| `2026082907`-`2026082909`                                                        | Global Published Question lineages, immutable Question Revisions, publication, discovery, and stewardship                                                           |
| `2026082910`-`2026082912`                                                        | Private authoring, Blueprints, Question Folders, and Saved Question Searches                                                                                        |
| `2026082913`-`2026082916`                                                        | Courses, equal Teaching Team Members, Students, invitations, curricula                                                                                              |
| `2026082917`-`2026082920`                                                        | Assignment Attempts, schedules, Issued Questions, submissions, artifacts                                                                                            |
| `2026082921`-`2026082924`                                                        | Automated grading, Gradebook, analysis, improvement threads                                                                                                         |
| `2026082925`-`2026082926`, `2026082928`                                          | Typed Jobs and leases; Course Retention schema foundation; Object Delivery, storage checks, cleanup manifests, and cleanup receipts                                 |
| `2026082929`-`2026082936`                                                        | Authorization Checks, forced RLS, grants, schema acceptance helpers, Create Instructor Account, Draft Blueprint Revision, and Question Revision Statistics evidence |
| `2026082937`-`2026082940`                                                        | Assignment pool and released-entry snapshots, authenticated Assignment Attempt start, and Object Record/source-object authority in the fresh baseline               |
| `2026082942`                                                                     | Session-authorized Bind Question Source operation                                                                                                                   |
| `2026082943`                                                                     | Question credit and stewardship                                                                                                                                     |
| `2026082944`                                                                     | Question Revision Source Binding publication completeness predicate                                                                                                 |
| `2026082945`                                                                     | Question fork source                                                                                                                                                |
| `2026090101`                                                                     | Latest Question Revision summary                                                                                                                                    |
| `2026090102`                                                                     | iMathAS Question Backend Session                                                                                                                                    |

Each migration owns its local relations, keys, constraints, indexes, functions, policies, grants,
and comments. It uses global content keys and exact Account, Authoring Workspace,
Course Instance, Course Membership, Student Record, lease, and immutable-content
identities rather than a legacy scope key.

## Validation lanes

Permanent offline tests prove domain authorization, Store conformance, strict browser contracts,
immutable evidence, grading, repeated-operation outcomes, revocation, and concealment. A data-driven operation
matrix proves identical creator/Teaching Team Member allow and deny decisions in Memory and Store
conformance.

Recurring service acceptance proves the current connected service lanes:
fresh migration convergence; RLS refusal without a resolved Account; Student
self versus other-Student and other-course denial; Teaching Team Member
mutation and Gradebook read; immediate membership revocation and
approval-withdrawal denial; narrow audited Sysadmin support; observer
non-escalation; typed-worker confused-deputy refusal; Object Delivery;
iMathAS Question Backend; Object Cleanup foundations; and migration
repeatability/checksum status. A future Course Retention service requires its
own complete connected acceptance evidence; the schema foundation is not that
service.

Production-browser acceptance proves Question Library discovery/reuse, equal Teaching Team Member behavior,
immediate revocation, Student submission-to-Gradebook convergence, answer-free Question Library responses,
accessible interaction, and role-appropriate screenshots on the production browser against the real stack.

Graphify maps, retired-identifier inventories, old-to-new schema allocation, clean-volume schema
fingerprints, and Migration Check evidence are one-time evidence. They are not permanent test
cases. The final material tree runs `source source_me.sh && ./all_test.sh` only after focused and
connected required gates are green; skipped required lanes keep the package incomplete.
