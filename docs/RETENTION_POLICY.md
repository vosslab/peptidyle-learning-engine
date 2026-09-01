# Student-record retention policy

Peptidyle is one installation with global accounts and a shared Question Library. It
separates reusable learning content from course-owned educational records. A `BlueprintCourse`
is the reusable, answer-free course definition; it has no Students, enrollments, deadlines, or
retention state. A `CourseInstance` is the exact teaching `CourseId` created from one immutable
BlueprintCourse parent and applied revision. It owns copied Assignment Content, Students,
deadlines, releases, delivery settings, and the resulting educational records.

Retention targets one exact `CourseInstance`. A published question, its immutable versions, the
Question Library, and a `BlueprintCourse` remain outside that CourseInstance lifecycle. A
Published Question can remain in the Question Library after every student record for a CourseInstance
is gone. This is both the sharing model and the deletion boundary.

This document defines the intended product retention boundary. The current SD1 baseline implements
Course Retention Plans and immutable lifecycle evidence; Store/worker execution, object cleanup,
Student-access fencing, browser routes, and connected acceptance remain incomplete work. The later
sections specify that required completion path without claiming it is mounted today.

The canonical live-demo baseline uses fictional people and course records as ordinary PLE data.
They are disposable because the complete database and storage can be regenerated, not because
retention, authorization, deletion, or audit behavior has a special demo path.

## Default CourseInstance lifecycle

Ending a `CourseInstance` snapshots the current deployment/operator policy and server-authoritative
end time. Later policy edits do not silently rewrite that CourseInstance's original schedule. The
snapshot has a positive generation; an extension creates a later generation and makes prior
scheduled work stale.

The deployment operator may configure any strictly increasing, whole-day notify, archive, and
delete windows from 1 through 36,500 days. This is installation-wide deployment or explicit product
configuration; it is not an institution selector, account setting, installation identity, or
authorization boundary. When no explicit configuration is present, PLE uses these privacy-first
defaults:

| Time after course end | Persisted action                           | Student-visible result                                             |
| --------------------- | ------------------------------------------ | ------------------------------------------------------------------ |
| 30 days               | Create the in-app instructor notification. | Records remain available.                                          |
| 100 days              | Archive student records.                   | Student record aliases and StudentRecord deliveries are concealed. |
| 365 days              | Permanently delete student records.        | The terminal `studentRecordsDeleted` lifecycle is recorded.        |

The fixed current notification says:

> This course ended 30 days ago. Student records are still available. If they are no longer needed,
> archive or delete the course now. Student records will be automatically removed after 100 days
> unless the course is archived or the retention period is extended by a sysadmin.

In that copy, "removed after 100 days" means archived from ordinary student access. It is not a
claim that the relational student graph is permanently deleted at day 100; the delete stage remains
scheduled for day 365 by default.

## Authority and API contract

Only a stored authenticated direct CourseInstance Instructor or a Sysadmin with
an active exact-course `retention_lifecycle_support` capability may read
retention status or request archive/delete. Only that capability's registered
Sysadmin operation may extend a schedule. Instructor authority derives from the
global account session and stored membership in the exact CourseInstance;
Sysadmin support authority derives from the closed registry in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md#sysadmin-support-capability-registry).
There is no institution lookup. A missing, foreign, expired, revoked,
or wrong-kind capability and a missing, foreign, archived, or revoked
CourseInstance/Student relationship fail closed and are concealed. A request
never supplies Account, course, Student, role, or support-capability authority.

The retention API exposes only a coarse lifecycle state, Assignment Content disposition, a strong
revision ETag, and the fixed notification projection. It never exposes student identities, policy
deadlines, object IDs, Object Addresses, queue jobs, leases, or generations.

- Archive and delete require `If-Match` with the current strong revision and create a durable
  replay receipt bound to the authenticated Account, action, requested disposition, expected generation, and
  resulting stage. A retry reports `scheduled`, `inProgress`, or `completed`; it cannot enqueue a
  second current-stage job.
- Extension also requires the current strong revision, but is a conditional
  schedule change rather than an archive/delete replay receipt. It uses the
  registered payload-free Sysadmin support operation and supersedes
  still-scheduled prior-generation work.
- An end-course request has an empty body. Archive and extension accept only their closed JSON
  bodies. Delete has an empty body. Request bodies cannot establish Account, course, Student, object, job,
  lease, stage, or generation.

These routes return `Cache-Control: no-store`. Foreign, missing, archived, and deleted student
records are concealed at the normal record boundary rather than revealing a retention distinction.

## Durable state machine

The storage lifecycle is `active -> archived -> deleted`. A deadline makes a stage eligible; it
does not fabricate a lifecycle result. The private dispatcher alone creates a closed retention job
payload containing only `course`, `stage`, and `generation`. The worker derives the authenticated Account and exact
course scope from the claimed typed job and supplies the job ID and active lease only to the Store boundary.

```text
course end snapshots policy and generation
        |
        +-- due scheduler dispatches one closed stage job
        |
        +-- worker proves account/course/stage/generation/job/lease binding
                |
                +-- notify: persist one in-app notification
                |
                +-- archive: fence access, freeze manifest, remove typed objects, mark archived
                |
                +-- delete: freeze delete manifest, remove objects and student graph, mark deleted
```

Preparation and commit verify the current generation, exact stage, leased worker job, unexpired
lease token, and course-retention row. A stale generation, reclaimed lease, mismatched job, or
malformed payload cannot commit an old worker's result. The worker accepts only exact course-scoped typed
`StudentRecord` keys; an already absent object is an idempotent success.

The archive access predicate is reused by CourseInstance student records, runs, summaries, feedback,
exports, external-tool paths, and protected StudentRecord assets. It also denies access as soon as
the current archive/delete stage has started, preventing a cleanup race from leaking a record.
BlueprintCourse reads never grant CourseInstance or Student access.

## Archive and permanent deletion

Archive is an access-and-cleanup transition, not an assertion that relational records have already
been removed. Under the fenced course state, the Store terminalizes resurrection paths, freezes an
exact course/stage object manifest in PostgreSQL, revokes protected student-record delivery, and
the worker deletes those typed objects. Only after the prepared manifest is complete does the
lifecycle become `archived`.

Permanent deletion creates and replays its own manifest. It has an independent object set because
newly discovered objects must not be silently added to a retry of the archive-stage manifest. The
delete preparation also records private, indexed run, attempt, and export work sets. They avoid
whole-course ID arrays in process memory, fence only the course being purged, and are erased before
the terminal tombstone is written.

After all delete-stage objects are absent, one PostgreSQL transaction removes the complete
CourseInstance-owned student graph in verified foreign-key order and then records
`studentRecordsDeleted`. A partial object-store failure leaves the course archive-fenced and retries
the same prepared delete manifest. It cannot report permanent deletion early.

The deleted student graph includes:

- enrollments, student course membership, assignment summaries, runs, attempts, submissions,
  evaluations, grades, timers, feedback, and item-analysis rows;
- prefetch, provider replay, idempotency, scoring, and per-student statistics receipts;
- student-record audit events, exports, protected deliveries, and external-tool sessions and
  transcripts; and
- CourseInstance Assignment Content only when the archive-time disposition is `delete`.

The purge retains:

- Published Questions, immutable versions, Source Object References, Question Library metadata, Question
  Classifications, and
  licensing;
- the Question Library and every `BlueprintCourse`, including its reusable
  Assignment Content and immutable revisions;
- instructor drafts and private workspaces;
- backend capability metadata;
- anonymous question-statistics aggregates; and
- CourseInstance Assignment Content when the frozen disposition is `retain`, the default.

Deletion never follows a CourseInstance assignment's immutable problem references into shared
content or into its BlueprintCourse parent. The owner default is to retain those CourseInstance
Assignment Content when Student records are archived or deleted; the closed archive disposition
is the only explicit choice that can change that treatment.

## Aggregate survival and disclosure

Question statistics are aggregated while the corresponding student records exist, then survive as
identity-free shared-content aggregates. They contain neither Account nor student identifiers, and
the browser suppresses a statistic below the k-anonymity disclosure threshold of five observations.
This means deletion removes the educational evidence that created an aggregate without removing the
non-identifying signal used to improve a published question library.

An aggregate is not a backup of attempt history. It cannot recreate an individual response,
submission, score, or course membership after the student graph is deleted.

## Object and audit boundary

Object storage is not deleted by a broad bucket prefix. A typed cleanup manifest is the authority
for each archive or delete stage, with one durable row per expected object and a manifest count.
The database remains authoritative for the intended object set; a bucket listing never authorizes a
delete. The worker treats a missing exact object as success so a crash after object deletion but
before database commit remains safely replayable.

The lifecycle retains only its coarse retention row and permitted replay/operational evidence long
enough to prove a completed action. It deletes student-facing audit and access evidence with the
student graph. Operational logs, backup copies, and object-store inventory are separate deployment
data classes; they must not become undeclared student-record archives.

General bucket-to-database reconciliation remains planned in WP-RC7. Until it is accepted,
operators must not claim automatic orphan cleanup or automatic repair of a missing referenced
object. The safe response to a missing or checksum-mismatched referenced object is to stop delivery,
preserve database evidence, alert, and use a normal recovery procedure.

## Recovery and backup boundary

Application deletion is immediate and irreversible through the ordinary PLE product path, including
the canonical live-demo path. It does not rewrite
historical encrypted backups or point-in-time recovery snapshots taken before deletion. Those copies
expire under their own infrastructure lifecycle; selective deletion from an older snapshot is not a
supported claim.

A PostgreSQL backup, WAL/PITR stream, snapshot, replica, logical dump, crash copy, or restored
database that contains any relation in the
[radioactive records and retention model](DATABASE_AUTHORIZATION.md#radioactive-records-and-retention) is radioactive as a whole. The
classification survives application deletion until that copy expires. Recovery material therefore
requires restricted operator access, encryption, an explicit expiry, isolated restoration, and an
access record; it must not be reused as developer or shared-test data.

There is no deployed backup retention window or production recovery objective yet. WP-RC10 must
choose encrypted PostgreSQL point-in-time recovery, object-store recovery, backup expiry, restoration
authorization, and a tested recovery objective, then disclose the deployed values here. An
operator or deployment requirement for less total exposure must choose a shorter backup window.

The honest guarantee until then is:

> Deleted student records are immediately unrecoverable through the application. Historical backup
> copies remain subject to the deployment operator's configured encrypted-backup expiry window.

On 2026-08-09, a one-time local PostgreSQL 17 restore exercise restored a role-only backup and a
custom-format database backup into a separate empty cluster. It preserved the migration ledger,
roles, grants, forced RLS, Account/course isolation, application writes, and broker-function execution. That
proves a small logical database restore procedure; it does not deploy managed point-in-time recovery,
set an RPO/RTO, or prove object-store recovery.

## Evidence and verification

On 2026-08-09, a one-time isolated PostgreSQL and MinIO deletion exercise drove a populated permanent
deletion request through the retention worker. The completed manifest matched the exact typed
student-record object. The worker removed that object and the student enrollment, run, attempt,
submission, evaluation, score, feedback, receipt, delivery, access-log, audit, and course-analysis
rows. It retained the assignment and instructor membership, published problem/version/source,
workspace draft, and anonymous global statistics aggregate. Independent typed-object reads and
physical bucket inspection agreed with the relational result. The temporary SQL, Rust helper, and
shell harness were removed after recording the evidence.

Permanent tests remain deterministic and offline. They cover authorization, conditional revisions,
archive/delete replay, lifecycle truthfulness, lease/generation fencing, typed-object validation,
and retained-versus-deleted content. Fresh PostgreSQL role/RLS exercises, populated purge graphs,
live object-store deletion, multi-replica soaks, query plans, reconciliation, and backup restoration
are environment-dependent acceptance or deployment gates. Temporary reconstruction tests are useful
evidence but do not become permanent fixture infrastructure.

Related contracts: [CONTRACTS.md](CONTRACTS.md), [SECURITY_MODEL.md](SECURITY_MODEL.md), and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).
