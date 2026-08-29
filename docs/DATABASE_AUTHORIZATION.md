# Database authorization

## Intended database model

PLE is one installation. Global accounts use `UserId`; there is no institution selector, tenant
identity, tenant-leading key, or client-selected database context. This reference is the sole durable
PostgreSQL authorization authority and the canonical database authorization target for the fresh SD1
epoch. [SECURITY_MODEL.md](SECURITY_MODEL.md) provides the cross-cutting security model and points
here for all durable PostgreSQL authorization detail. The active
[single-installation authorization plan](active_plans/active/single_installation_authorization_plan.md)
and [scope register](active_plans/active/single_installation_scope_register.md) allocate its
implementation; existing pre-epoch schema documents are migration input, not an alternate model.

The server derives `ActorContext { user_id, session_id }` from a valid global account session. Each
protected database transaction sets only the trusted, transaction-local `ple.actor_user_id` value.
An absent or malformed actor value is not a fallback identity: forced RLS refuses the operation.
Routes, browser fields, queue payloads, object keys, and provider responses are evidence or input;
they never establish actor, membership, workspace, course, or worker authority.

## Authority relationships

`approved_instructor(user_id, now)` is the one current, manually approved Instructor predicate. It
authorizes global Instructor capabilities: course creation, publication, shared-catalog discovery,
collections, favorites, saved searches, reuse, and improvement. Approval withdrawal closes each of
those capabilities in the protected transaction.

The `sysadmin` platform role does not satisfy `approved_instructor`. A Sysadmin
must complete the explicit operator-led Instructor approval path before creating
or teaching a course. Course creation then atomically creates the first ordinary
Instructor membership; Sysadmin status does not add creator authority.

`current_course_instructor(user_id, course_id, now)` requires both current `approved_instructor`
and a current direct Instructor membership for that exact course. Course creation atomically creates
the first ordinary Instructor membership. It does not create a creator, owner, or privileged
course-authority row. Every current co-Instructor receives the same teaching mutation and FERPA-read
decision for equivalent state; audit rows identify the actor without changing authority.

Student work requires the exact course relationship and Student ownership of the durable child
record. A private draft, curriculum, or authoring input requires its current workspace owner or
collaborator relationship. A published question has exactly one Instructor-visible shared-catalog
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
| Draft or curriculum workspace           | Workspace owner/collaborator                   | Unshared source and author preview                    |
| Course, roster, assignment              | `current_course_instructor`                    | Other courses and former memberships                  |
| Run, attempt, response, grade, artifact | Student ownership or current course Instructor | Other Students, courses, and inactive records         |
| Job, export, object, provider state     | Locked typed lease and durable target          | Caller-supplied scope and foreign targets             |

Lifecycle does not narrow approved-Instructor discovery. The catalog safely
returns the lifecycle state on every published question. Assignment creation
and other ordinary new-selection operations require `active`; exact historical
resolution and retained assignment references may resolve `deprecated` or
`archived` questions without making them newly selectable.

## Course relationships

Current live `course_member` rows represent only Student and Instructor membership. Invitation
acceptance verifies the target's current Instructor approval, invitation state, and roster revision
in one transaction. Revocation serializes with protected reads and writes, so it takes effect
immediately. Approval withdrawal likewise closes course-Instructor operations immediately.

Future Grader, Course Observer, and Student Observer access uses a distinct
`course_relationship` plus `course_capability_grant`. Each grant records a subject `UserId`, exact
`CourseId`, relationship kind, bounded capability set, issuer, lifecycle/revocation state, revision,
audit identity, and required consent or disclosure policy. It is not a `course_member` row and does
not satisfy current Student-owner, Instructor, roster, Gradebook, response, export, artifact,
assignment-write, or worker predicates.

- A Grader receives only the bounded grading work in its completed relationship package.
- A Course Observer receives a separately typed anonymous aggregate projection with disclosure
  thresholds; it contains no subject, enrollment, row, small-cell, or linkable metadata.
- A Student Observer receives a distinct one-Student projection only with explicit revocable consent
  and its own disclosure contract.

Fabricated, expired, and revoked future grants fail all current FERPA predicates.

## Row-level security

Every protected table enables and forces PostgreSQL RLS. Policies use the transaction-local actor
and operation-specific predicates for current Instructor membership, Student ownership, workspace
relationship, or a leased capability. A policy must deny when required context or relationship is
missing. The protected Store/PostgreSQL operation performs the predicate and data operation in the
same transaction, preventing a route-level check from outliving revocation.

The application uses least-privilege roles. Runtime logins are `NOINHERIT`, `NOSUPERUSER`, and do
not have `BYPASSRLS`; table owners, superusers, and broad broker membership are not runtime actor
identities. The public schema is private by default, and grants are explicit per table, view, and
function.

Security-definer brokers implement only a registered capability whose arguments, actor, durable
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
receive the same actor-scoped RLS, minimum-field, audit, retention, incident-response, and backup
handling. Partition children, views, staging relations, query results, exports, diagnostics, and
restores inherit the highest label of their inputs.

| Family                                | Radioactive relations                                                                                                                                                                                                                                                          |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Roster and policy                     | `course_roster_member`, `course_invitation`, `course_roster_import_row`, `enrollment`, `assignment_policy_exception`                                                                                                                                                           |
| Learner work and current results      | `student_assignment_summary`, `submission`, `submission_idempotency`, `submission_receipt_snapshot`, `submission_evaluation`, `attempt_feedback`, `attempt_score_current`                                                                                                      |
| Scoring and course analysis           | `assignment_attempt_score_staging`, `assignment_summary_staging`, `course_item_analysis_current`, `course_item_analysis_staging`                                                                                                                                               |
| Student exports                       | `student_export_request`, `student_export_artifact`, `course_grade_export_audit`                                                                                                                                                                                               |
| Course and attempt linkage            | `course_member`, `course_group_member`, `course_roster_state`, `course_roster_import`, `assignment_run`, `assignment_run_item`, `question_attempt`, `question_prefetch`, `attempt_timing_current`, `feedback_release`, `submission_next_attempt`, `webwork_grade_replay_state` |
| External, delivery, and audit linkage | `external_tool_exchange`, `external_tool_launch_session`, `question_statistics_contribution_receipt`, `asset_delivery`, `record_access_log`, `audit_event`, `worker_job`                                                                                                       |
| Retention evidence                    | `course_retention_cleanup_manifest_object`, `course_retention_purge_attempt`, `course_retention_purge_export`, `course_retention_purge_run`                                                                                                                                    |

Global account/session records are restricted account/security data, not FERPA data by themselves.
Private source and answer-bearing grading material are highly restricted for assessment integrity,
not Student records unless joined to Student activity. The global published aggregate remains
identity-free; its Student-linked contribution receipt is radioactive, and course-local analysis is
radioactive because small cohorts can be identifiable.

Retention keeps shared published catalog content and private drafts outside course-record deletion.
Course Student records move through `active -> archived -> deleted`. The database centrally fences
Student-facing records, exports, external-tool records, and course-record assets as archive or
deletion starts. Authorized current Instructors may retain course and assignment definitions without
restoring learner records. A retention broker uses the exact course/stage/generation manifest and a
renewed lease, so stale work cannot commit after a newer retention generation.

## Fresh migration epoch

SD1-C creates the single-installation schema only on freshly cleaned disposable stack data. It does
not preserve a tenant compatibility layer. The migration ledger allocates the exact next available
number in these ranges:

| Range                     | Capability family                                                     |
| ------------------------- | --------------------------------------------------------------------- |
| `2026082901`-`2026082904` | Principals, global accounts, sessions, passkeys, roles, actor context |
| `2026082905`-`2026082908` | Global immutable catalog, publication, safe discovery evidence        |
| `2026082909`-`2026082912` | Private authoring, collections, favorites, saved searches             |
| `2026082913`-`2026082916` | Courses, equal co-Instructors, Students, invitations, curricula       |
| `2026082917`-`2026082920` | Assignments, schedules, runs, attempts, submissions, artifacts        |
| `2026082921`-`2026082924` | Automated grading, Gradebook, analysis, improvement threads           |
| `2026082925`-`2026082928` | Typed jobs, exports, objects, retention, external-tool state          |
| `2026082929`-`2026082932` | Capability brokers, forced RLS, grants, schema acceptance helpers     |

Each migration owns its local relations, keys, constraints, indexes, functions, policies, grants,
and comments. It uses global content keys and exact user, workspace, course, membership, Student,
lease, and immutable-content identities rather than a tenant-shaped key.

## Validation lanes

Permanent offline tests prove domain authorization, Store conformance, strict browser contracts,
immutable evidence, grading, idempotency, revocation, and concealment. A data-driven operation
matrix proves identical creator/co-Instructor allow and deny decisions in Memory and Store
conformance.

Recurring service acceptance proves fresh migration convergence; missing-actor RLS refusal; Student
self versus other-Student and other-course denial; co-Instructor mutation and Gradebook read;
immediate membership revocation and approval-withdrawal denial; narrow audited Sysadmin support;
observer non-escalation; typed worker confused-deputy refusal; object delivery; external adapter;
export; retention; cleanup; and migration idempotency/checksum status.

Production-browser acceptance proves shared-catalog discovery/reuse, equal co-Instructor behavior,
immediate revocation, Student submission-to-Gradebook convergence, answer-free catalog responses,
accessible interaction, and role-appropriate screenshots on the canonical real stack.

Graphify maps, retired-identifier inventories, old-to-new schema allocation, clean-volume schema
fingerprints, and migration-count reconciliation are one-time evidence. They are not permanent test
cases. The final material tree runs `source source_me.sh && ./all_test.sh` only after focused and
connected required gates are green; skipped required lanes keep the package incomplete.
