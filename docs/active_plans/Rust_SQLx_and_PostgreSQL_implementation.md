# Independent static review: Rust, SQLx, and PostgreSQL implementation

> **Historical and concluded review (2026-08-08).** This document preserves the static-review
> evidence and the original remediation queue; it is not an active implementation plan and does not
> authorize edits to the accepted SQLx baseline. The current migration policy is
> [decisions/database_schema_evolution_plan.md](decisions/database_schema_evolution_plan.md), and
> the current execution handoff is [implementation_status.md](implementation_status.md). Applied
> migrations, including the six-file baseline, are immutable; future schema changes use a new
> forward migration.

## Context

`docs/active_plans/decisions/database_schema_evolution_plan.md`
specified a consolidated pre-data PostgreSQL baseline: six domain-owned SQLx migrations, forced
tenant RLS, least-privilege principals, bounded partitions, and `cargo tools database` as the only
apply path. That work landed 2026-08-08 (commits `fdac14e`, `ac91688`).

This is a **static, read-only** post-implementation review. No database was started, no migration
applied, no query executed. Every finding below is derived from reading the SQL, the Rust, and the
reference corpus. Where a finding would benefit from measurement, it is named as validation work for
whoever implements the fix, not claimed as evidence here.

The current canonical browser path is the single production-shaped live-demo application. Its
seeded people and records are fictional live data, but this historical review remains static and
does not claim runtime, migration, backup, restore, or fault-injection proof for that path.

At the time of this review, the six-file epoch was still the pre-data baseline and the findings were
candidate consolidation work. That condition has concluded: the accepted baseline and every later
applied migration are immutable, and current schema work uses forward migrations under the current
authority above.

## Method and skill routes

| Skill                | Route                                                                                                                                                                                                                                                                                                                                                                                     | Applicability                                                                                                                                                                                                                                                                                                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/postgresql-expert` | `topic_index.md` rows for table/constraint design, index selection, partitioning, isolation, and migration planning. Corpus used as a static inspection checklist: `PostgreSQL_16.0_Documentation-2023.md` for behavior and syntax authority, `PostgreSQL_Mistakes_and_How_to_Avoid_Them-2025.md` for production failure modes (FK indexing, data types, connections, long transactions). | Primary. The skill's execution-oriented steps (representative-workload baselines, `EXPLAIN (ANALYZE, BUFFERS)` before/after) are **out of scope for a read-only review** and are carried into remediation validation instead.                                                                                                                           |
| `/rust-code-expert`  | Contract classification, then ownership and error-contract review of the store boundary.                                                                                                                                                                                                                                                                                                  | Primary for the Rust layer.                                                                                                                                                                                                                                                                                                                             |
| `/wasm-rust-expert`  | Checked and scoped out.                                                                                                                                                                                                                                                                                                                                                                   | **Not applicable to the database path.** `crates/wasm/Cargo.toml:14-18` depends only on `question_model`, `domain`, and `wasm-bindgen` - no `store`, no `sqlx`. It becomes relevant to exactly one remediation task (task 8), because `AttemptResult` lives in `question_model`, which ships in the browser bundle via `web = ["question_model/wasm"]`. |

Artifacts inspected: all six files in `schemas/migrations/` in full (6362 lines; 66 tables, 90
foreign keys, 71 indexes, 52 functions, 150 policies, 53 triggers); `crates/learning-data-access/src/postgres.rs`
(11329 lines, ~247 query sites), `rls.rs`, `session.rs`, `jobs.rs`, `retention.rs`, `pagination.rs`,
`lib.rs`, `build.rs`; `crates/project-tools/src/database.rs`, `e2e_seed.rs`;
`crates/server/src/composition.rs`; `check_codebase.sh`; root `Cargo.toml`.

## Phase 1: Original design objectives

Objectives testable against shipped artifacts:

- Six migrations, no historical repair chain (plan lines 395-422).
- Forced RLS with default-deny; application connections never own tables or bypass RLS (342-346).
- `tenant_id` leading every private key, foreign key, and important index (344).
- `NUMERIC` for points and credit, never floating point (69).
- Monthly partitions on high-volume append-only data; pre-create partitions and **alert on
  default-partition writes** (366-369).
- Full-text **and trigram** indexes on a denormalized `catalog_search_document` projection (138).
- SQLx `Migrator`; a **dedicated migration role**; startup does a read-only compatibility check
  only (379-385).

## Phase 2: PostgreSQL schema and migrations

### Delivered well

- Nine roles, all `NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT`, created idempotently
  (`principals.sql:5-44`). `REVOKE ALL ON SCHEMA public FROM PUBLIC` then explicit `GRANT USAGE`.
- Every `SECURITY DEFINER` function is revoked from `PUBLIC`, reassigned to a narrow broker role,
  and pinned with `SET search_path` (41 of 52 functions are definer). Volatility is deliberate:
  `STABLE` on readers, `IMMUTABLE STRICT` on the two CHECK-constraint helpers.
- `ENABLE` plus `FORCE ROW LEVEL SECURITY` on essentially every tenant table. `FORCE` is
  load-bearing: tables are owned by the migration runner, not a `ple_*` role, and PG16 `ALTER TABLE`
  states RLS "will not be applied when the user is the table owner" unless forced.
- Points columns are `numeric(16,4)` / `numeric(16,12)`. `double precision` appears only in
  statistics aggregates, fenced by NaN and signed-zero CHECK constraints.
- `problem.public_id bigint GENERATED ALWAYS AS IDENTITY`, unique, sequence granted only to
  `ple_app`.
- Immutability enforced by `BEFORE` triggers diffing `to_jsonb(NEW)` against `to_jsonb(OLD)`, not by
  grants alone.
- Partition keys are correctly inside every partitioned table's primary key.
- UUIDs are genuinely time-ordered: `Uuid::now_v7()` (`question_model/src/identity.rs:90`,
  `activity.rs:83`, `auth.rs:24`), so index locality on the hot append-only tables is good and the
  classic v4-random B-tree spread problem does not apply.
- No `DROP`/`ALTER` repair sequences, no milestone-named helpers, no `CONCURRENTLY` inside a
  transaction-wrapped migration.

### Systematic static passes

**Foreign key versus supporting index.** PostgreSQL does not index the referencing side of a
foreign key. Of 90 FKs, **75 have leading-prefix index coverage and 15 do not.** The ones that
matter, because they are `ON DELETE CASCADE` (the cascade fires the scan automatically) or point at
a high-volume parent:

| Referencing table                  | Referencing columns                              | Parent                       | Why it matters                                                                     | Cite                            |
| ---------------------------------- | ------------------------------------------------ | ---------------------------- | ---------------------------------------------------------------------------------- | ------------------------------- |
| `catalog_tenant_grant`             | `(problem_id, version_id)`                       | `problem_version`            | CASCADE with **no supporting index of any kind**                                   | `catalog_authoring.sql:482`     |
| `attempt_timing_current`           | `(tenant_id, attempt_id, attempt_occurred_at)`   | `question_attempt`           | CASCADE into the partitioned, highest-volume table; only a 2-of-3 PK prefix exists | `operations_analytics.sql:1084` |
| `assignment_attempt_score_staging` | `(job_id)`                                       | `worker_job`                 | CASCADE; every index on the staging tables leads with `tenant_id`                  | `operations_analytics.sql:1103` |
| `assignment_scoring_staging`       | `(job_id)`                                       | `worker_job`                 | same                                                                               | `operations_analytics.sql:1112` |
| `assignment_summary_staging`       | `(job_id)`                                       | `worker_job`                 | same                                                                               | `operations_analytics.sql:1118` |
| `assignment_summary_staging`       | `(tenant_id, enrollment_id)`                     | `enrollment`                 | CASCADE                                                                            | `operations_analytics.sql:1124` |
| `course_group_member`              | `(tenant_id, course_id, course_group_id)`        | `course_group`               | CASCADE; an index exists but with `user_id` in position 3, so it is not a prefix   | `courses_assignments.sql:318`   |
| `assignment_policy_exception`      | `(tenant_id, course_id, course_group_id)`        | `course_group`               | CASCADE                                                                            | `courses_assignments.sql:333`   |
| `assignment_selection_candidate`   | `(tenant_id, assignment_id, selection_group_id)` | `assignment_selection_group` | CASCADE                                                                            | `courses_assignments.sql:306`   |

Also: `manual_grade_receipt` carries **no secondary index at all** - only its PK - yet has two FKs
(`activity_feedback.sql:467,470`). Five more uncovered FKs are `RESTRICT`/no-action references to
`problem_version(problem_id, version_id)` from `assignment_item`, `assignment_run_item`,
`problem_collection_member`, `assignment_selection_candidate`, and
`question_statistics_contribution_receipt` - lower urgency (catalog versions are immutable and
rarely deleted) but they make every version-retirement check a scan.

**Duplicate indexes.** Two indexes exactly reproduce a constraint index that already exists on the
same table with no partial predicate: `course_retention_cleanup_manifest_object_idx` reproduces its
PK column list (`retention.sql:2425` vs `:2388`), and `course_retention_dispatch_job_idx` reproduces
the `job_id` UNIQUE (`retention.sql:2427` vs `:2394`). These are pure duplication - any query either
could serve is already served by the constraint index, so they carry cost with no present or future
upside. Three further indexes share a column list with a PK/UNIQUE but add a partial predicate,
making them narrower rather than redundant; those are fine.

**Indexes without a current matching query.** Five indexes have no matching access path among the
inspected Rust queries and PL/pgSQL bodies: `problem_version_catalog_search_text_idx` (GIN
tsvector), `problem_version_catalog_idx (lifecycle, title, ...)`, `problem_version_metadata_idx` and
`problem_version_capabilities_idx` (GIN jsonb_path_ops), and
`catalog_search_document_public_id_idx`. All catalog reads currently go through
`catalog_search_document` / `catalog_search_view`.

State the epistemic limit plainly: **static inspection cannot establish that an index is dead.** It
establishes only that no currently inspected query appears able to use it. These may be intentional
forward-looking indexes, or support for administrative, debugging, and reporting access paths, or
features whose Rust call sites do not exist yet - the shapes they serve (capability, metadata,
category, and keyword facets, plus a lifecycle-filtered title browse) are precisely the discovery
facets the plan names at lines 134-138. They may equally be unnecessary maintenance cost. The review
recommended documenting and reconsidering their intended access paths during historical
consolidation; static evidence alone did not justify deleting them.

Cost context for that judgment: `problem_version` is append-only and immutable - the immutability
trigger permits only `lifecycle` transitions - so each index there costs roughly one write per
publish plus storage. An index without a matching query on a **high-churn** table would be a
materially different case, because every UPDATE maintains it and compounds bloat. None of these five
sit on a churn-heavy table; `worker_job` carries five indexes against non-HOT churn (phase 3), but
all five are used.

**Trigram support is specified and absent.** Plan line 138 requires "full-text **and trigram**
indexes". There is no `CREATE EXTENSION` statement anywhere in the six migrations, no `pg_trgm`, and
no `gin_trgm_ops` index. Additionally, all text search uses `to_tsvector('simple', ...)` /
`websearch_to_tsquery('simple', ...)` (`catalog_authoring.sql:460,468,711`;
`postgres.rs:6483,6517,6540,6550,6562`), and the `simple` configuration performs no stemming. The
delivered discovery therefore supports neither substring/typo-tolerant matching nor linguistic
matching - a searcher must type a whole word exactly. For a feature the plan calls "a primary
database concern", this is a functional gap, not a tuning nit.

**Column types.** Hash columns are `character(64)` (`principals.sql:65`,
`catalog_authoring.sql:139,169,202`, and elsewhere), each already carrying a
`CHECK (... ~ '^[0-9a-f]{64}$')` that fixes the length. PG16 is explicit: "there is no such
advantage in PostgreSQL; in fact `character(n)` is usually the slowest of the three because of its
additional storage costs. In most situations `text` or `character varying` should be used instead."
The blank-padding semantics also mean the RLS policy comparison
`session_hash = (NULLIF(current_setting('ple.session_hash', true), ''))::character(64)`
(`principals.sql:88`) relies on `bpchar` trailing-space equality rather than exact string equality.
`text` plus the existing CHECK is strictly better on both counts.

**Partition key versus predicates.** The four range-partitioned tables key on `occurred_at`, and
their PKs are `(tenant_id, <entity>_id, occurred_at)`. Consequently every FK pointing at
`question_attempt` must carry `occurred_at` as a third column, which is exactly why the
`attempt_timing_current` CASCADE above is uncovered. Any lookup that knows only `attempt_id` cannot
prune and must touch every partition. This is an inherent, accepted cost of the partitioning choice,
but it should be recorded as a design constraint so callers are written to carry `occurred_at`.

Defects from this phase are D2, D3, D4, D7, and D8 below.

## Phase 3: Rust and SQLx integration

### Delivered well

- **Tenant context is correct.** `SELECT set_config('ple.tenant_id', $1, true)` is parameterized and
  transaction-local, issued inside the transaction it protects (`postgres.rs:341-352`), as is
  `SET LOCAL ROLE ple_app`. No `after_connect` hook carries session state, so nothing leaks across a
  pooled checkout. The most important thing to get right is right.
- **Zero dynamic SQL.** All 247 sites pass `&'static str` with `$N` binds. No `QueryBuilder`, no
  `format!` into SQL text, no interpolated identifiers, `ORDER BY`, or `LIMIT`. `PageSize` clamps to
  `1..=100` before binding.
- **Ownership contract enforced by types.** `TenantContext` has no `Default` and one construction
  site, so omitting tenancy fails to compile (`rls.rs`).
- Optimistic concurrency is single-statement CAS, never read-then-write.
- Job leasing is `FOR UPDATE SKIP LOCKED` with lease-token fencing and a replay-safe export commit.
- 13 `.expect()` sites, no bare `.unwrap()`, no `panic!`; all assert internal invariants, not raw
  query results.

### Systematic static passes

**Non-sargable predicates.** Four query shapes wrap indexed columns in expressions with no matching
expression index:

- Catalog keyset paging orders and filters on `problem_id::text || '/' || version_id::text`
  (`postgres.rs:6378-6389`, `:6473-6482`) - the PK columns exist, but the concatenated text key
  cannot use `catalog_search_document_pkey`.
- Taxonomy listing orders on `encode(convert_to(...))` (`postgres.rs:6417-6427`) - unindexable by
  construction.
- Member course list orders on `course_id::text` (`postgres.rs:303`) while the supporting index is
  on the `uuid` column, forcing a sort.
- Assignment item filtering uses `NOT (assignment_item_id = ANY($3))` (`postgres.rs:9775,9791,9809`),
  an anti-join that cannot seek.

The worker-job claim CTE is an `OR` across two differently-indexed branches
(`operations_analytics.sql:65-72`): the `state='ready'` branch matches
`worker_job_claim_ready_idx(available_at, job_id)`, but the expired-lease branch is indexed on
`lease_expires_at` while the shared `ORDER BY` is `available_at, job_id`, so one ordered index scan
cannot serve both.

**N+1 patterns inside transactions.** Four confirmed:

- One `SELECT lifecycle FROM problem_version ... FOR SHARE` per assignment reference, per save
  (`postgres.rs:9654-9673`).
- One `UPDATE assignment_item` per item and one `UPDATE assignment_selection_candidate` per
  candidate (`postgres.rs:9852-9925`).
- Up to four INSERTs per QTI item plus one per asset and per unsupported feature
  (`postgres.rs:2719-2795`).
- One INSERT per export artifact kind (`postgres.rs:1304-1314`, fixed N=4, minor).

**Transaction scope and lock hold time.** `commit_assignment_scoring` (`postgres.rs:1024-1199`)
issues at least eleven statements in one transaction - including a `SELECT ... FOR UPDATE` on
`assignment.scoring_generation` at `:1056`, a seven-relation join at `:1066`, a bulk DELETE, three
bulk `INSERT`/`UPDATE ... FROM staging` statements, and three cleanup DELETEs - so the assignment
row lock is held across all of it. `replace_postgres_assignment_items` holds a transaction across
O(N) per-item statements. All three advisory locks are the blocking `pg_advisory_xact_lock`, never
`pg_try_advisory_xact_lock`, so contention queues rather than failing fast.

**Repeated work in search.** `search_catalog` runs five near-duplicate facet queries
(`postgres.rs:6472-6567`), each re-applying the identical filter stack and re-running the
`ple_question_statistics_view` LATERAL join, rather than sharing one filtered CTE.

**MVCC and churn.** Reviewed per table from the write statements:

| Table                        | Pattern                                              | HOT-eligible? | Note                                                                                                                           |
| ---------------------------- | ---------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `worker_job`                 | INSERT + repeated UPDATE of `state`                  | **No**        | `state` is the predicate of two partial indexes, so every transition moves the row in and out of an index. Five indexes total. |
| `course_retention_stage`     | 9 UPDATE sites, all on `state`                       | **No**        | Same partial-index-predicate pattern.                                                                                          |
| `attempt_timing_current`     | UPDATE `job_id = NULL`                               | **No**        | `job_id` carries a UNIQUE index.                                                                                               |
| `attempt_score_current`      | bulk DELETE + bulk re-INSERT per recalculation       | n/a           | Generates a full generation of dead tuples on every recalculation.                                                             |
| `student_assignment_summary` | set-based UPDATE of `payload`, `updated_at`          | **Yes**       | No index on the updated columns. Good.                                                                                         |
| `assignment`                 | low-frequency UPDATE of `scoring_status`, `revision` | **Yes**       | Updated columns unindexed. Good.                                                                                               |
| `*_staging` (3)              | INSERT + bulk DELETE per job                         | n/a           | Clean scratch pattern.                                                                                                         |

The queue is the problem: `worker_job` rows are **never deleted** for the common job kinds. The only
two DELETEs are retention-scoped (`retention.sql:1344`, `:1423`), covering `autoSubmitAttempt` and
export jobs for a purged course. Renders, imports, and `recalculateAssignment` jobs accumulate
forever in `completed`/`dead` state, on a table where every row also churned through several
non-HOT updates and five indexes. Stale-lease reaping also happens only as a side effect of another
worker calling claim (`operations_analytics.sql:50-59`) - there is no scheduled reaper, so an idle
queue never self-heals.

Defects from this phase are D1, D5, D6, D9, and D10 below.

## Phase 4: Comparison against upstream guidance

| Implementation choice                                | Guidance                                                                                                                                        | Verdict                                                                                                                                                                                                           |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `set_config(..., true)` per transaction              | PG16: RLS is default-deny when enabled with no policy; `BYPASSRLS` and superuser always bypass                                                  | Correct. The transient no-policy windows between migrations 000300 and 000500 fail closed.                                                                                                                        |
| `REPEATABLE READ` with no retry                      | PG16 Transaction Isolation: "Applications using this level **must be prepared to retry** transactions due to serialization failures"            | **Violated.** D5.                                                                                                                                                                                                 |
| `numeric` columns, `String` binds with `$N::numeric` | SQLx: `NUMERIC` maps to `bigdecimal::BigDecimal` or `rust_decimal::Decimal`; String is **not** an accepted encoding                             | Works only via the explicit casts. `rust_decimal` sqlx feature is enabled (`Cargo.toml:81`) and unused at the boundary.                                                                                           |
| Pool sets only `max_connections`                     | SQLx `PoolOptions`: `test_before_acquire` defaults true; `acquire_timeout`, `idle_timeout`, `max_lifetime` defaulted. Mistakes book ch. 6.3-6.4 | Under-specified. D5.                                                                                                                                                                                              |
| Range partitions with a `DEFAULT` partition          | PG16 `CREATE TABLE`: adding a partition when a default exists **scans the default partition**; `ATTACH` takes `ACCESS EXCLUSIVE` on it          | Upgrades D4 from hygiene to outage risk.                                                                                                                                                                          |
| Six `NOT VALID` constraints                          | PG16 `ALTER TABLE`: skips verification of existing rows; `VALIDATE CONSTRAINT` completes it                                                     | Tables were empty, so validation was free and skipped anyway. D3.                                                                                                                                                 |
| Referencing-side FK columns unindexed                | Mistakes book: PostgreSQL does not create indexes on referencing FK columns                                                                     | 15 of 90 uncovered. Not automatically a defect - an index is warranted where parent deletions are expected and the child is large. The 9 CASCADE cases are where PostgreSQL searches the child automatically. D7. |
| `character(64)` for fixed-length hashes              | PG16: `character(n)` "is usually the slowest of the three"; prefer `text`                                                                       | D8.                                                                                                                                                                                                               |
| `to_tsvector('simple', ...)`, no `pg_trgm`           | Plan line 138 requires trigram; `simple` config does no stemming                                                                                | D9.                                                                                                                                                                                                               |

## Phase 5: Deviations and risks

### D1. Nothing verifies that the Rust and the schema agree (highest)

`sqlx::query!` / `query_as!` count: **zero**; all 247 sites use the runtime API. No `.sqlx/` offline
cache, no `SQLX_OFFLINE`; `build.rs` only emits a rerun trigger; the `macros` feature is enabled
(`Cargo.toml:76`) and unused. The baseline is **never applied to a real PostgreSQL in any automated
gate**: `crates/learning-data-access/tests/conformance.rs` (7464 lines) imports only `MemoryStore`, there is no
`#[sqlx::test]`, there is no `.github/`, and `check_codebase.sh:266` runs `cargo test --workspace`
with no `DATABASE_URL`. The only migration tests (`postgres.rs:2567-2649`) feed fabricated vectors
to `evaluate_migration_status`. A mistyped column, a policy that denies a legitimate read, or a
missing grant surfaces in production.

### D2. Three tables have no row-level security

| Table                           | Evidence                                                                               | Assessment                                                                                                                                                                                                                                                                                        |
| ------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `problem`                       | `catalog_authoring.sql:150-162`; `GRANT SELECT,INSERT ... TO ple_app` line 765         | **Real leak.** Rows carry `owner_tenant_id`, `owner_user_id`, `public_id`, `visibility` (which admits `'institution'`), `lifecycle`. Every tenant's `ple_app` reads every row. The child `problem_version` gates on `publication_scope` plus `catalog_tenant_grant`; the parent gates on nothing. |
| `answer_key`                    | `catalog_authoring.sql:135-140`; only `GRANT SELECT,INSERT ... TO ple_grader` line 759 | Grading secrets protected by grant alone, inconsistent with the RLS-enabled `published_qti_grading`.                                                                                                                                                                                              |
| `question_statistics_aggregate` | `operations_analytics.sql:978`                                                         | Deliberate cross-tenant aggregate. Record the intent in SQL.                                                                                                                                                                                                                                      |

`ple_qti_staging_broker` and `ple_queue_broker` hold `BYPASSRLS` (`principals.sql:28,32`) -
defensible, since the bypass is reachable only through definer functions with their own tenant
checks, but per D1 nothing proves the denial.

### D3. The baseline is a `pg_dump`, not an authored baseline

`SET check_function_bodies = false;` at line 3 of all six files, so a function body referencing a
mistyped relation applies cleanly and fails on first call. Dump-shaped syntax throughout
(`ALTER TABLE ONLY`, `USING btree`, `'...'::text`, separate `ADD CONSTRAINT`). **Six `NOT VALID`
constraints never validated** - `catalog_authoring.sql:399`, `courses_assignments.sql:268,294`,
`activity_feedback.sql:330,339,345` - with no `VALIDATE CONSTRAINT` anywhere, so they remain
`convalidated = false` on a database that was provably empty at creation.

### D4. Partition layout is deploy-date dependent, unmonitored, and has a latent outage

`activity_feedback.sql:644-647` calls
`ple_ensure_activity_partitions((date_trunc('month', current_date) - interval '1 month')::date, 26)`

- the partition set depends on the wall-clock date of first apply, so CI, staging, and production
  diverge. No scheduled maintenance extends the window, and nothing alerts on default-partition writes
  despite plan line 369. **The outage:** once the window lapses, rows land in
  `question_attempt_default` and `submission_default`; per PG16, creating the next month's partition
  then scans the default partition and holds `ACCESS EXCLUSIVE` on it. The routine fix for a missed
  partition becomes a locking full scan on the hottest tables in the system. Separately, the two
  hash-partition loops use `CREATE TABLE ... PARTITION OF` without `IF NOT EXISTS`
  (`catalog_authoring.sql:822-824`, `activity_feedback.sql:585-588`).

### D5. Rust and SQLx hardening gaps

- **No serialization-failure retry.** `begin_tenant_snapshot` opens `REPEATABLE READ`
  (`postgres.rs:362`); `map_sqlx_error` (`:11250-11272`) does not map `40001` or `40P01`; both
  collapse into `StoreError::Unavailable` and nothing retries.
- **Pool config is one line.** `max_connections(8)` only (`postgres.rs:2376-2380`); no
  `acquire_timeout`, `idle_timeout`, `max_lifetime`; `test_before_acquire` left true; no `sslmode`
  assertion, so TLS depends on an unvalidated `DATABASE_URL`.
- **Raw database text forwarded.** `23503`/`23514` map to
  `InvalidRecord(database_error.message())`; Postgres FK messages embed the offending key value.
- **Numeric boundary inconsistent.** `PointValue(i64)` fixed-point bound as `String` via `::numeric`,
  but `AttemptResult` carries `f64` and credit is a float division (`postgres.rs:9289`). Guarded
  against zero and non-finite values by `validate_attempt_result` (`lib.rs:4091-4105`), so not a
  live bug - an avoidable float hop on a stack already paying for `rust_decimal`.
- **Startup degrade asymmetric the wrong way.** In `verify_application_schema` only `pool.begin()`
  and `SET TRANSACTION READ ONLY` map to `Unavailable`; every later failure maps to `Incompatible`
  (`postgres.rs:2489-2506`), so a connection reset mid-check fails a boot the design meant to
  degrade.
- Widening `seed as i64` casts at five sites where the codebase otherwise prefers `try_from`.

### D6. Migration role separation documented, not implemented

Plan line 384 requires a dedicated migration role. `crates/project-tools/src/database.rs:17` reads the same
`DATABASE_URL` the server uses - no second variable, no assertion the connected role is not the
application principal. `crates/project-tools/src/e2e_seed.rs:148-153` applies migrations to any
`--database-url` with no guard.

### D7. Index coverage and index intent

Three separate categories, with different confidence levels:

- **Exact duplicates (defect).** Two indexes reproduce the keys and semantics of an existing PK or
  UNIQUE constraint index with no partial predicate (`retention.sql:2425`, `:2427`). These add
  maintenance cost and no adaptability, since any query they could serve is already served.
- **No current matching query (purpose unclear, not a defect).** Five indexes, all on catalog
  tables. Static evidence cannot classify these as dead; see phase 2. The action is to document the
  intended access path, not to delete.
- **Uncovered referencing-side FKs (prioritize, do not blanket-fix).** PostgreSQL does not require
  an index on the referencing side, and 15 of 90 FKs lack leading-prefix coverage. Whether each
  merits an index depends on expected parent-deletion frequency, child-table size, and workload. The
  ones worth acting on are the nine `ON DELETE CASCADE` cases, where PostgreSQL must search the
  child automatically - especially `catalog_tenant_grant` (no supporting index of any kind), the
  three `*_staging` tables cascading from `worker_job`, and `attempt_timing_current` cascading from
  the partitioned, highest-volume `question_attempt`. `manual_grade_receipt` has no secondary index
  at all despite two FKs. The five `RESTRICT`/no-action references to immutable
  `problem_version(problem_id, version_id)` are low priority precisely because catalog versions are
  rarely deleted. Detail table in phase 2.

### D8. Column type and constraint design

`character(64)` used for every hash column despite an existing length-fixing CHECK; PG16 recommends
`text` or `varchar`. The `auth_session` RLS policy compares through a `::character(64)` cast, so it
depends on blank-padded equality semantics.

### D9. Search capability does not meet the plan

No `CREATE EXTENSION` anywhere, no `pg_trgm`, no trigram index, despite plan line 138. All search
uses the `simple` text-search configuration, which does no stemming. Catalog paging is non-sargable
(`problem_id::text || '/' || version_id::text`), so the PK cannot serve it. `search_catalog` re-runs
five near-identical facet queries per call, each repeating the same filter stack and LATERAL
statistics join.

Four catalog indexes on `problem_version` have no matching access path among the inspected queries,
because search reads only `catalog_search_document`. That is stated as an observation, not as
grounds for removal - see phase 2 and D7. It is plausible they were built for the trigram and facet
search this task restores, in which case implementing D9 gives them their caller.

### D10. Queue growth, non-HOT churn, and transaction scope

`worker_job` is never pruned for render, import, or `recalculateAssignment` kinds, and every state
transition is non-HOT because `state` is a partial-index predicate; five indexes multiply the cost.
Stale-lease reaping only runs as a side effect of another claim. `commit_assignment_scoring` holds
an `assignment` row lock across eleven-plus statements including bulk DML; item replacement holds a
transaction across O(N) per-row statements; four N+1 loops issue one statement per element. All
advisory locks are blocking rather than `try_`.

### D11. Minor

- `ple_current_tenant()` (`principals.sql:58-62`) is the only function without `SET search_path`;
  invoker-rights and schema-safe, but it backs ~100 policies.
- `question_attempt_run_summary_cursor_idx` and `question_attempt_run_position_idx`
  (`activity_feedback.sql:391,393`) are near-duplicate coverage.
- `ple_migration_state` relies on default definer-view semantics; that is what makes the `ple_app`
  read work. Comment it so nobody "fixes" it with `security_invoker`.

## Phase 6: Historical follow-up record

This was the original dependency-ordered remediation queue. It is retained to connect findings to
their validation evidence, not as present work direction. The later hardening record in
[partial_commit_status.md](partial_commit_status.md) records the completed baseline, RLS,
credential, partition, foreign-key, and retry work. Do not infer permission to rewrite an applied
migration from any item below.

1. **Live-PostgreSQL acceptance gate.** The recommendation was to add
   `tests/e2e/e2e_database_baseline.sh` per
   `docs/E2E_TESTS.md`, reusing the
   Postgres in `containers/compose.yaml`. Validation: six migrations apply to an empty database;
   re-run is a no-op; `cargo tools database verify` passes; a mutated migration file reports
   `modified`; a cross-tenant denial matrix proves that as each of `ple_app`, `ple_student`,
   `ple_grader`, `ple_grading_reader`, tenant A cannot read tenant B on every RLS-protected table. This
   is the oracle for tasks 2-5 and the only place the execution-oriented parts of
   `/postgresql-expert` belong.
2. **Close the RLS gaps.** The recommendation was `ENABLE` + `FORCE` plus policies on `problem`
   (mirroring
   `problem_version`'s `publication_scope` / `catalog_tenant_grant` logic) and `answer_key`
   (grader-only); comment the deliberate scope of `question_statistics_aggregate`. Validation: the
   task 1 denial matrix, plus a catalog check that no `public` table has `relrowsecurity = false`.
3. **De-dump the baseline.** The recommendation was to replace the six `NOT VALID` constraints with
   validated ones; remove
   `SET check_function_bodies = false` or comment the forward reference requiring it; add
   `IF NOT EXISTS` to the hash-partition loops. Validation: `pg_constraint WHERE NOT convalidated`
   returns zero rows.
4. **Fix partition determinism and the default-partition trap.** The recommendation was to replace
   `current_date` with a fixed
   epoch month; add a `cargo tools database` subcommand or worker job that extends the window ahead
   of time; add a default-partition row-count check callable by the gate and by operations.
   Validation: two applies on different fixed dates produce identical partition sets.
5. **Separate the migration credential.** The recommendation was to read `PLE_MIGRATION_DATABASE_URL` in
   `crates/project-tools/src/database.rs`, falling back to `DATABASE_URL` only for `status`/`verify`; refuse
   `migrate` when the connected role is `ple_app`; add an opt-in flag to `e2e_seed.rs`.
6. **Harden the SQLx layer.** The recommendation was to set `acquire_timeout`, `idle_timeout`,
   `max_lifetime` on both pools;
   map `40001`/`40P01` to a distinct `StoreError` variant with bounded retry around the
   `REPEATABLE READ` and advisory-lock paths; replace forwarded `database_error.message()` with a
   constraint-name-keyed message; map post-`begin` connection failures in
   `verify_application_schema` to `Unavailable`. Validation: a concurrent fixture that forces a
   serialization failure commits after retry.
7. **Review index intent.** The recommendation was to remove the two exact-duplicate indexes. For the
   five indexes without a
   current matching query, document the intended access path in a SQL comment and retain those that
   support credible near-term schema or query evolution; reconsider only ones with no identified
   purpose. Evaluate the uncovered CASCADE foreign keys according to expected child-table size and
   parent-deletion behavior, and add covering indexes where that evaluation justifies them -
   starting with `catalog_tenant_grant(problem_id, version_id)`, the three staging `job_id` columns,
   and `attempt_timing_current(tenant_id, attempt_id, attempt_occurred_at)`; reorder
   `course_group_member_user_idx` or add a sibling so the `course_group` FK has a prefix. Validation:
   an FK-to-index coverage query in the task 1 gate that reports uncovered CASCADE FKs, reviewed
   rather than auto-failed.
8. **Close the numeric boundary.** The recommendation was to move `AttemptResult` points to the same
   fixed-point representation
   as `PointValue`, or bind `rust_decimal::Decimal` directly and delete the `String` + `::numeric`
   round trip, removing the float division at `postgres.rs:9289`. **Crosses the Wasm boundary** -
   `AttemptResult` lives in `question_model`, which ships in the browser bundle - so validation is
   the `/wasm-rust-expert` native-versus-Wasm parity oracle plus regenerating
   `tests/fixtures/published_problem` without diff.
9. **Search capability.** The recommendation was to add `CREATE EXTENSION IF NOT EXISTS pg_trgm` and
   the trigram index the plan
   specifies; decide deliberately between the `simple` and `english` text-search configuration and
   record why; replace the `::text`-concatenated catalog keyset with a plain
   `(problem_id, version_id)` tuple comparison so the PK is usable; collapse the five facet queries
   into one filtered CTE.
10. **Queue lifecycle and transaction scope.** The recommendation was to add a retention or scheduled
    sweep that deletes
    terminal `worker_job` rows for all job kinds; add a standalone stale-lease reaper rather than
    relying on claim-time side effects; consider `fillfactor` on `worker_job` and
    `course_retention_stage` given the non-HOT churn; convert the four N+1 loops to set-based
    statements; evaluate `pg_try_advisory_xact_lock` with an explicit conflict error where callers
    can retry.
11. **Adopt checked queries incrementally.** The recommendation was to convert one bounded module
    (`jobs.rs` or the retention reads) to `sqlx::query!` and commit `.sqlx/` offline data. The
    proposed `cargo sqlx prepare --check` automation is obsolete: current database automation uses
    the verified `cargo tools database verify` command shown below. Any checked-query pilot requires
    separate current-plan approval and must not modify an applied migration.
12. **File the review.** The recommendation was to write this document to
    `docs/active_plans/audits/database_schema_post_implementation_review.md` (snake_case, per the
    active-plans rules in
    `docs/REPO_STYLE.md` and add a
    `docs/CHANGELOG.md` entry under `### Decisions and Failures`.

## Historical validation evidence

These historical validation commands are retained as dated evidence and historical validation
instructions. Any
execution uses a disposable clean cluster or controlled fault exercise; it is not a production
operation. The database command forms below match the current CLI: `migrate` requires
`PLE_MIGRATION_DATABASE_URL`, while `status` and `verify` may read that variable or `DATABASE_URL`.

```bash
bash tests/e2e/e2e_database_baseline.sh          # task 1 gate; oracle for tasks 2-5, 7

PLE_MIGRATION_DATABASE_URL="<migration-database-url>" cargo tools database migrate
PLE_MIGRATION_DATABASE_URL="<migration-database-url>" cargo tools database migrate
PLE_MIGRATION_DATABASE_URL="<migration-database-url>" cargo tools database status
PLE_MIGRATION_DATABASE_URL="<migration-database-url>" cargo tools database verify

psql -c "SELECT conrelid::regclass, conname FROM pg_constraint WHERE NOT convalidated;"
psql -c "SELECT relname, relrowsecurity, relforcerowsecurity FROM pg_class
         WHERE relkind='r' AND relnamespace='public'::regnamespace
           AND NOT (relrowsecurity AND relforcerowsecurity);"
psql -c "SELECT relname, n_live_tup FROM pg_stat_user_tables WHERE relname LIKE '%_default';"

cargo fmt --check && cargo clippy -- -D warnings && cargo test --workspace
./check_codebase.sh
pytest tests/
```

The review also prescribed `EXPLAIN (ANALYZE, BUFFERS)` captures after tasks 4, 7, and 9 on
representative data for the gradebook summary read, one partitioned `question_attempt` range read,
the catalog search page, and one CASCADE delete - comparing against a pre-change capture. That is
the `/postgresql-expert` measured-evidence step, deferred out of this static review by scope.

## Out of scope

- Running PostgreSQL, applying migrations, or capturing query plans - this is a read-only review.
- Splitting `postgres.rs` and `memory.rs` into smaller modules; already in progress separately. Their
  size is why D1 matters more, not less.
- Rewriting all 247 query sites to compile-time macros; task 11 is a bounded pilot by design.
- Backup restore drills and PITR verification, which need durable data.
- Browser and Wasm work, except the single stated intersection in task 8.
