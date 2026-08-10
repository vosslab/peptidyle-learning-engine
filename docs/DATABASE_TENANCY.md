# Database tenancy

This reference describes the database ownership and isolation boundary. It is
the durable companion to [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md), and
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). It records the implemented local
PostgreSQL baseline and separates it from release-candidate and cloud work
that still needs deployment evidence.

## Ownership boundary

PLE uses one PostgreSQL cluster with logical tenancy. Tenant identity is a
server-derived `TenantId`, not a database per instructor or a browser-selected
field.

| Shared, immutable content                                                                                   | Tenant-owned educational records                                                                                                                  |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `problem`, `problem_version`, public payloads, catalog metadata, shared source artifacts, and shared assets | Courses, memberships, assignments, drafts, imports, runs, attempts, submissions, feedback, grades, exports, audit evidence, and course appearance |
| Tenant-free published identity can be referenced by many courses.                                           | Every private row carries `tenant_id`; keys, foreign keys, and indexes lead with it where the relationship permits.                               |
| Publication creates a new immutable version; it does not promote a draft in place.                          | Assignments reference published versions but are not shared content. They remain owned by their course tenant.                                    |

This split prevents a course archive or deletion from following references into
the shared catalog. The default lifecycle retains tenant-owned assignment
definitions while student records archive or delete; it leaves shared published
content intact. Anonymous question statistics have a distinct non-identifying
publication boundary and are not student records.

## Context and transactions

`TenantContext` has no `Default` implementation. A tenant-owned `Store`
operation requires it, so an omitted tenancy boundary fails to compile. The
server constructs it only after resolving the authenticated session from shared
storage; request parameters, headers, and JSON bodies never create authority.

For each PostgreSQL transaction, `PostgresStore`:

- starts a transaction and uses `SET LOCAL ROLE ple_app`;
- sets `ple.tenant_id` with `set_config(..., true)` from that trusted context;
- performs every tenant query under that transaction-local setting; and
- commits or rolls back before the connection returns to the pool.

`ple_current_tenant()` reads the local setting. Transaction-local role and
tenant state prevent a pooled connection from retaining one request's authority
for another request. Read-only snapshot work follows the same setup after its
required isolation declaration. The dedicated grader uses a separate,
least-privilege `ple_grading_reader` connection and also sets tenant context per
transaction; the normal Store does not receive grader read capability.

API replicas are therefore stateless with respect to authority. A replica
resolves the opaque session and tenant from PostgreSQL and reconstructs durable
run, submission, and idempotency state rather than trusting a browser copy or
another replica's memory. See [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md)
for the full replica contract.

## Enforced database boundary

Every tenant-owned table enables and forces PostgreSQL row-level security
(RLS). Policies compare `tenant_id` with `ple_current_tenant()` and add more
specific checks where needed, such as course membership, learner-record
accessibility, or restricted broker actions. Enabled RLS with no permissive
policy fails closed.

The initial principals migration makes the `public` schema private by default
and uses narrow `NOINHERIT`, `NOSUPERUSER`, `NOBYPASSRLS` roles including
`ple_app`, `ple_auth`, `ple_student`, and `ple_grader`. Grants are explicit per
table or function. Protected operations use narrowly granted functions or
broker roles for catalog ownership, queues, retention, statistics, QTI staging,
and provenance; the ordinary application role does not gain broad table access
as a shortcut.

Some broker principals intentionally have narrowly scoped elevated capability
for broker functions. That is not an application bypass: the live acceptance
tests must prove the actual deployment login, role memberships, grants, forced
RLS, and foreign-tenant concealment. Table owners, superusers, and
`BYPASSRLS` roles are unsafe application identities and are not valid evidence
of tenant isolation.

Student access has two layers. RLS isolates a tenant, then Store queries bind
the authenticated user to that tenant's enrollment. Instructor access likewise
requires membership in the exact course; a tenant-wide role alone is
insufficient.

## Durable record rules

Educational records are protected FERPA-sensitive data. The local schema and
Store keep raw responses, feedback, grades, student artifacts, and access/audit
evidence tenant-owned and retention-bound. Browser-gradebook paths use current
summaries instead of exposing or scanning a learner's history. Queue payloads
contain bounded identifiers and generations, never names, raw responses,
answer keys, or grades.

The main immutable and idempotent boundaries are:

- Published catalog versions, public payloads, and answer-key associations are
  immutable. Corrections publish a new version rather than mutating a version
  pinned by an assignment or issued run.
- `question_attempt` records immutable identity/evidence with controlled state
  transitions. `submission` is append-only response evidence and binds a
  tenant-scoped idempotency key. Receipts and worker leases fence retries so an
  ambiguous replay returns the original result instead of duplicating effects.
- Current grade and feedback projections are replaceable only through their
  revision or scoring-generation boundary. They do not rewrite the append-only
  evidence used to justify them.
- Retention prepares an exact typed object manifest before cleanup, replays it
  only for the same bound job and renewed lease, and marks a deletion terminal
  only after every required relational and object effect succeeds.

The schema range-partitions `question_attempt`, `submission`,
`record_access_log`, and `audit_event` by time, with pre-created and default
partitions. `submission_idempotency` is hash-partitioned by tenant, and the
immutable problem-version payload table is hash-partitioned by problem. These
partitions serve retention and query windows; current summaries and identity
headers stay unpartitioned so ordinary gradebook reads avoid historical scans.

## Retention and FERPA isolation

The privacy-first defaults are notification after 30 days, learner-record
archive after 100 days, and permanent learner-record deletion after 365 days.
An institution may later configure its own ordered policy. Archive access is
fenced centrally: learner-facing aliases, exports, external-tool records, and
student-record-bound assets stop at the same closed boundary. Retained
definitions can remain visible to authorized managers without restoring
learner rows.

FERPA readiness is broader than schema design. The implemented controls support
institutional authentication, authorization, isolation, audit, retention, and
deletion obligations, but they do not by themselves certify a deployment. The
release checklist still requires evidence for the FERPA control checklist,
object-storage lifecycle, backup/restore, and an end-to-end course deletion
that preserves shared catalog content and anonymous aggregates.

## Baseline and future work

The implemented local PostgreSQL baseline is the six ordered SQLx migrations
`2026080801` through `2026080806`: principals, catalog/authoring,
courses/assignments, activity/feedback, operations/analytics, and retention.
`2026080907_course_appearance.sql` is an implemented forward migration after
that epoch. The application embeds the migrations, uses the SQLx ledger for
compatibility checks, and keeps startup read-only; project tooling owns
administrative status, apply, and verification.

Do not rewrite an applied baseline. Schema evolution after durable data uses
forward migrations with expand, backfill, verify, switch, and contract stages.
The disposable PostgreSQL acceptance path exercises fresh migration replay,
role/RLS denial, tenant concealment, restricted grader access, and
representative Store behavior. Offline conformance tests do not replace those
live checks.

The following remain planned RC or cloud deployment work, not claims about the
local baseline:

- deployed RDS/private-network/TLS/KMS, backup retention, point-in-time
  recovery, and restore rehearsal;
- deployed non-superuser application credentials and forced-RLS/grant evidence
  against the managed database;
- production Fargate scaling, class-start load evidence, worker soak, and
  replica/clock-skew operational proof; and
- the completed FERPA control checklist, configured institutional retention
  override, object-store lifecycle proof, and production deletion rehearsal.

See [implementation_plan.md](active_plans/implementation_plan.md) for those
release gates and [implementation_status.md](active_plans/implementation_status.md)
for the distinction between accepted code-first work and environment-dependent
evidence.
