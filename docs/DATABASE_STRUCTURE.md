# Database structure

This document is the durable map of PLE's PostgreSQL schema: what owns each kind of
fact, which migration state is accepted, and where to find the detailed contracts. It
does not authorize changing an accepted migration. The release plan and the migration
ledger remain the authority for unfinished work and deployment acceptance.

## Schema authority

The checked-in SQLx migrations in [schemas/migrations/](../schemas/migrations/) define
the schema. The Rust Store contracts and PostgreSQL implementation must agree with those
migrations, but do not replace them as schema authority. The release plan owns migration
order, package status, and acceptance evidence:

- [implementation plan](active_plans/implementation_plan.md) defines the platform
  architecture and storage boundaries.
- [release completion plan](active_plans/active/release_completion_plan.md) defines
  the active migration sequence and release gates.
- [database schema evolution plan](active_plans/decisions/database_schema_evolution_plan.md)
  defines the forward-only process.
- [CONTRACTS.md](CONTRACTS.md) registers frozen cross-module contracts.
- [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) owns the PostgreSQL/object-storage
  commit, repair, and reconciliation contract.

The browser never connects to PostgreSQL. It receives answer-free public envelopes from
the API; server code establishes authenticated transaction context and calls Store
capabilities.

## Migration ledger

The accepted epoch has seven migrations and 80 static application relations. The first
six are the accepted pre-data baseline. `2026080907_course_appearance.sql` is the first
accepted forward migration. The 80-relation count deliberately excludes SQLx's own
ledger and runtime-created partition children; it is a useful inventory check, not a
capacity metric.

| Version | File | State | Owns |
| --- | --- | --- | --- |
| 2026080801 | [principals](../schemas/migrations/2026080801_principals.sql) | Accepted baseline | Roles, tenant context, session lookup, migration-state projection |
| 2026080802 | [catalog authoring](../schemas/migrations/2026080802_catalog_authoring.sql) | Accepted baseline | Private drafts, immutable catalog versions, authoring/import evidence |
| 2026080803 | [courses assignments](../schemas/migrations/2026080803_courses_assignments.sql) | Accepted baseline | Courses, membership, assignment configuration, enrollment, summaries |
| 2026080804 | [activity feedback](../schemas/migrations/2026080804_activity_feedback.sql) | Accepted baseline | Runs, attempts, submissions, feedback, current scores, protected logs |
| 2026080805 | [operations analytics](../schemas/migrations/2026080805_operations_analytics.sql) | Accepted baseline | Worker queue, timing, export, analytics, staging, object delivery |
| 2026080806 | [retention](../schemas/migrations/2026080806_retention.sql) | Accepted baseline | Archive/delete lifecycle and frozen cleanup manifests |
| 2026080907 | [course appearance](../schemas/migrations/2026080907_course_appearance.sql) | Accepted forward | Course theme and banner candidate/current presentation state |

`2026080908_secure_question_grading_payloads.sql` is checked in as the WP-P2
prerequisite, but is not an accepted migration or a claim about a deployed database. It
adds presentation-binding columns, request-contract versioning, and the
`webwork_grade_replay_state` relation. Do not count it in the accepted 80 relations.

### Reserved sequence

Later work takes the next ordered forward migration; it never inserts, renames, or edits
an accepted version.

| Version | Planned owner | Intended scope | Current source state |
| --- | --- | --- | --- |
| 2026080908 | WP-P2 | Secure learner grading payload binding and WeBWorK replay state | File present; acceptance open |
| 2026080909 | WP-RC7 | Object inventory and reconciliation | Reserved; no migration file yet |
| 2026080910 | WP-RC8 | Institutional OIDC identity binding | Reserved; no migration file yet |
| 2026080911 | WP-RC9 | LTI 1.3 / Advantage launches and passback | Reserved; no migration file yet |
| 2026080912 | WP-FU | Secure learner upload capability | Reserved; no migration file yet |

The upload migration follows the reserved OIDC/LTI sequence and remains planned while
learner file responses fail closed. See the
[secure learner upload plan](active_plans/active/secure_learner_file_upload_plan.md).

## Data ownership

PLE keeps shared catalog facts, tenant teaching configuration, learner records, and
replaceable projections separate. JSONB holds cohesive, versioned payloads; relational
columns hold identities, ownership, constraints, joins, and lifecycle state. Object bytes
are not stored in these relations.

| Data class | Primary relations | Ownership and mutability |
| --- | --- | --- |
| Catalog and publication | `problem`, `problem_version`, `problem_version_payload`, `answer_key`, `published_source_artifact` | Shared immutable published content; a correction creates another version. |
| Private authoring | `workspace_draft` and `workspace_*` import/source relations | Tenant-private mutable work before publication. |
| Course activity | `course`, `course_member`, `assignment`, `assignment_item`, `enrollment` | Tenant/course configuration and membership. |
| Learner evidence | `assignment_run`, `assignment_run_item`, `question_attempt`, `submission`, `submission_evaluation` | Tenant-owned educational records. |
| Current projections | `attempt_score_current`, `student_assignment_summary`, `course_item_analysis_current` | Recomputed/published current state; not a substitute for source evidence. |
| Protected delivery/audit | `asset_delivery`, `student_export_*`, `record_access_log`, `audit_event` | Explicitly authorized and retention-bound record access/evidence. |

Publication pins an assignment to an exact `(problem_id, version_id)` and an issued run item
to what that learner actually received. A course edit therefore does not reinterpret an
already issued question. The detailed lifecycle is in
[ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), and the information-class boundary is in
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Assessment record chain

The relational activity spine is:

```text
course -> assignment -> enrollment -> assignment_run -> assignment_run_item
                                                   -> question_attempt
                                                        -> submission
                                                        -> submission_evaluation
                                                        -> attempt_score_current
                                                    enrollment -> student_assignment_summary
```

All learner-facing transitions derive tenant, learner, assignment position, version, seed,
policy, backend, and timing from the authenticated attempt record. A request body cannot
select those facts.

| Relation | Durable responsibility |
| --- | --- |
| `assignment_run` | One owned pass through an assignment; completion is derived from attempt state. |
| `assignment_run_item` | Exact version/order and selection evidence delivered in that run. |
| `question_attempt` | One issued question instance, including seed, provenance, timing, and controlled state. |
| `question_prefetch` | One bounded, not-yet-issued reservation; it has no started timer, response, or score. |
| `submission` | Append-only learner response evidence. |
| `submission_idempotency` and receipt relations | Exact retry/replay fence; no second grade on a conflicting retry. |
| `submission_evaluation` | Server-produced normalized result and policy-controlled feedback basis. |
| `attempt_score_current` and `student_assignment_summary` | Current scoring/gradebook projections, updated under scoring-generation rules. |

The accepted target payload design binds a response to `AttemptId`, an idempotency key, a
presentation digest, and a family-minimal response; it does not trust a browser-supplied
question kind, score, or answer key. See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)
for the current-versus-target boundary.

### Presentation and replay state

Migration 0908 introduces the persistence needed by the secure grading-payload cutover:

| Relation or columns | Purpose | Privacy boundary |
| --- | --- | --- |
| `question_attempt.presentation_*` | Versioned descriptor, nonce, and SHA-256 digest for the exact public presentation. | Consistency evidence, not authentication or a grading key. |
| `question_prefetch.presentation_*` | The same binding for one reservation before promotion to an attempt. | Prevents an unbound prefetch from becoming an issued render. |
| `submission_idempotency.request_*` | Versioned request fingerprint for safe response retries. | Server compares it before regrading. |
| `webwork_grade_replay_state` | Attempt-bound, answer-free mapping needed to reproduce a private WeBWorK grade call. | Never enters the browser envelope; contains no source text, credentials, correct answer, or raw renderer result. |

The relation is tenant-, course-, attempt-, version-, source-, seed-, renderer-, and
presentation-bound. Its mapping has an explicit size and item-count limit, an SHA-256
fingerprint, forced RLS, retention-broker access, and a foreign key to the precise
attempt. The complete render/response contract, including attempt-specific CRC-16 item IDs,
is in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md), not this schema map.

## Tenant isolation and grants

Private and learner-record relations carry `tenant_id`, enable RLS, and use forced RLS.
The application role is not a table owner, superuser, or `BYPASSRLS` role. Each server
transaction sets its tenant context locally; pooled connections must not inherit context.
RLS is the tenant fence, while Store queries additionally bind a learner to the owned
enrollment or require exact instructor course membership.

| Role/capability | Narrow purpose |
| --- | --- |
| `ple_app` | Normal API/server work through RLS and narrowly granted tables/functions. |
| `ple_student` | Read-only student projections subject to tenant and ownership predicates. |
| `ple_grader` / `ple_grading_reader` | Server-only grading material and approved reader functions; never browser access. |
| `ple_auth` | Hash-based session resolution only. |
| `ple_retention_broker` | Retention-manifest and learner-record cleanup work under its RLS policies. |
| `ple_statistics_broker` | Identity-free aggregate/statistics contribution work. |
| `ple_qti_*_broker` | Narrow staging/provenance capabilities for QTI import. |

Grants do not replace RLS, and RLS does not prove individual learner ownership by itself.
Production acceptance must exercise the deployed roles and transaction context, including
foreign-tenant and foreign-student denial. The detailed model is in
[DATABASE_TENANCY.md](DATABASE_TENANCY.md) and [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Worker, retention, and statistics

`worker_job` is a durable, tenant-scoped queue. Its claim function uses bounded job kinds,
leases, attempt counts, and `FOR UPDATE SKIP LOCKED`; handlers commit under lease and
generation fences. Worker payloads carry identifiers and generations rather than learner
names, raw responses, grades, answer keys, or storage credentials.

Retention is a database-backed lifecycle, not an ad hoc delete:

- `institution_retention_policy`, `course_retention`, notification, stage, dispatch, and
  API-receipt relations persist policy and revision-fenced actions.
- Cleanup manifest relations freeze the exact typed object set before a worker removes
  learner records, so a retry cannot discover a newly written or unrelated object.
- Purge relations record ordered deletion progress for attempts, runs, and exports.
- Shared published content, private drafts, and identity-free question statistics remain
  outside a learner-record purge.

`question_statistics_aggregate` is shared, identity-free statistical state.
`question_statistics_contribution_receipt` makes first-completed-run contribution
idempotent and supports deletion of the learner-owned receipt without deleting the aggregate.
Course item analysis remains a separate tenant-owned current projection. Detailed cross-store
failure/recovery behavior is owned by [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md); retention
policy is in [RETENTION_POLICY.md](RETENTION_POLICY.md).

## Partitioning and operations

The baseline partitions the high-churn learner evidence tables by month, including
`question_attempt`, `submission`, `record_access_log`, and `audit_event`; a default
partition catches unprepared dates. `problem_version_payload` uses hash partitions by
`problem_id`. Partition children are runtime schema objects and are not included in the
80-relation inventory above.

Keep current grade summaries and compact identity/header relations unpartitioned unless
measured query and retention behavior requires a change. New indexes, partitions, poolers,
read replicas, or external search require representative result-equality checks and
`EXPLAIN (ANALYZE, BUFFERS)` evidence. PostgreSQL remains the source of truth.

Migration startup is read-only: application startup reads the granted
`ple_migration_state` projection and refuses known-incompatible, dirty, pending, or
checksum-mismatched ledgers. Only the explicit migration command applies DDL. A fresh
database, a no-op second apply, and deployed-role RLS checks are separate acceptance
evidence; a passing Rust compile does not prove any of them.

## Change rules

For a durable schema change:

1. Confirm the next reserved/available migration number in the active release plan.
2. Add a forward migration; never rewrite an accepted filename, version, or checksum.
3. Preserve tenant keys, forced RLS, least-privilege grants, retention reachability, and
   answer-key isolation in the same change.
4. Update the Store contract and memory/PostgreSQL conformance only when behavior changes.
5. Use expand, backfill, verify, switch, and retire stages when existing durable data is
   affected.
6. Run the package's fresh/no-op migration, deployed-role, Store-conformance, and security
   gates before calling the schema accepted.

The exact acceptance commands and historical compatibility obligations belong to the
[release completion plan](active_plans/active/release_completion_plan.md), so this document
does not turn a design description into release evidence.
