# Student-record retention policy

Peptidyle separates reusable learning content from tenant-owned educational records. A published
problem can remain in the shared catalog after every student record for a course is gone. This is
both the sharing model and the deletion boundary.

This document describes the implemented code-first contract. Production infrastructure is not yet
deployed; deployment-specific backup and object-lifecycle settings belong to M6.

## Default course lifecycle

An institution may configure longer or shorter ordered windows. When it does not, Peptidyle uses
the privacy-first defaults recorded in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md):

| Time after course end | Action |
| --- | --- |
| 30 days | Notify the instructor; offer archive, delete, or administrator extension. |
| 100 days | Archive automatically; conceal learner access before cleanup. |
| 365 days | Delete student records and their typed object-storage artifacts. |

The instructor-facing notification says:

> This course ended 30 days ago. Student records are still available. If they are no longer needed,
> archive or delete the course now. Student records will be automatically removed after 100 days
> unless the course is archived or the retention period is extended by an administrator.

Only an authorized course instructor or tenant administrator may request archive or deletion. Only
a tenant administrator may extend the schedule. Mutations use a strong revision precondition and
durable replay receipt; request bodies cannot supply tenant, learner, object, job, lease, or
generation identity.

## Archive and deletion semantics

Archive is an access and cleanup transition, not a claim that relational records have already been
deleted. The Store first fences every learner-facing alias, terminalizes resurrection paths,
freezes an exact tenant/course object manifest, revokes StudentRecord delivery, and deletes the
typed objects idempotently. The lifecycle reports archived only after that exact work completes.

Permanent deletion freezes and replays its own delete-stage manifest under the current
scheduler-owned job, lease, stage, and generation. Course-owned writers share a retention-row lock,
so the purge fences only that course while unrelated courses remain writable. Private indexed work
sets replace whole-course in-memory ID arrays and are erased before completion. After all required
objects are absent, one PostgreSQL transaction removes the complete course-owned learner graph in
verified foreign-key order and records the durable `studentRecordsDeleted` tombstone. A partial
object failure leaves the course archived and retries the same delete-stage manifest; it cannot
report deletion early.

Deleted student records include:

- enrollments, summaries, runs, attempts, submissions, grades, timers, and feedback;
- prefetch, replay, idempotency, and per-student statistics receipts;
- student-record audit events, exports, deliveries, and external-tool sessions/transcripts; and
- assignment definitions only when the instructor's frozen archive-time choice is `delete`.

The purge retains:

- published problems, immutable versions, source artifacts, catalog metadata, taxonomy, and
  licensing;
- instructor drafts and private workspaces;
- backend capability metadata;
- anonymous question-statistics aggregates; and
- assignment definitions when the frozen choice is `retain`, which is the default.

Deletion never follows an assignment's immutable problem references into shared content.

## Recorded acceptance evidence

On 2026-08-09, a one-time isolated PostgreSQL and MinIO reconstruction drove the production worker
through a populated permanent-deletion request. The completed manifest contained the exact typed
student-record object; the worker removed that object and the learner's enrollment, run, attempt,
submission, evaluation, score, feedback, receipt, delivery, access-log, audit, and course-analysis
rows. It retained the assignment and instructor membership, published problem/version/source,
workspace draft, and anonymous global statistics aggregate. Independent typed-object reads and
physical bucket inspection agreed with the relational result. The temporary SQL, Rust helper, and
shell reconstruction harness were removed after this evidence was recorded.

## Backup boundary

Application deletion is immediate and irreversible through the live product. It does not rewrite
historical encrypted backups or point-in-time recovery snapshots. Those copies expire under their
own infrastructure lifecycle.

There is no deployed backup window yet. M6 must select an encrypted point-in-time recovery window,
configure it in infrastructure, and disclose the deployed value here. An institution requiring
less total exposure must shorten that backup window; selective deletion from an older database
snapshot is not a supported claim.

On 2026-08-09, a one-time local recovery rehearsal encrypted a role-only backup without password
hashes and a custom-format database backup, then restored both into a separate empty PostgreSQL 17
cluster. The restored database matched the source logical fingerprint and preserved the migration
ledger, role attributes, owners, grants, forced RLS, tenant isolation, application writes, and
broker-function execution. Backup and restore each completed in one second for this small fixture.
This establishes the logical recovery procedure; it does not set a production recovery objective,
deploy managed point-in-time recovery, prove object-store recovery, or choose the backup window.

The resulting guarantee is:

> Deleted student records are immediately unrecoverable through the application and expire from
> encrypted backups within the disclosed deployed backup window.

## Verification policy

Permanent tests are deterministic, offline behavior tests for authorization, revision fencing,
exact replay, lifecycle truthfulness, typed-object validation, and retained-versus-deleted content.
Fresh PostgreSQL role/RLS exercises, populated purge graphs, object-storage deletion, multi-replica
soaks, query plans, and backup restoration are environment-dependent one-time acceptance or
deployment gates. Temporary SQL and reconstruction tests are removed after their evidence is
recorded; they are not committed as fixture infrastructure.
