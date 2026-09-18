# Plan: SQL base schema restructure

Remediation plan for [sql_schema_quality_audit.md](../audits/sql_schema_quality_audit.md). The
audit is evidence and stays fixed; this plan is the work and is updated as milestones land. Every
work package must satisfy [docs/DATABASE_STYLE.md](../../DATABASE_STYLE.md).

Status: not started. Opened 2026-09-18. Companion plan:
[schema_style_checker_plan.md](schema_style_checker_plan.md) ships the measuring tool first.

## Context

The base schema under `schemas/base_schema/` (146 tables, 27,306 lines, installs in one second)
stores derived, duplicated, and stringly typed data on the rows that multiply fastest (Assessment
Attempts, Issued Questions, Question Attempts, notification fan-out); mixes tables with functions,
policies, and grants in 36 of 39 table-bearing files so the structure cannot be read in one place;
and carries shapes that Human Guidance has since retired (dead Assessment lifecycle values, a
privileged Course owner column, a second concurrency token, deferred iMathAS scaffolding, four
identity conventions). PLE is pre-production with no durable data
(HUMAN_GUIDANCE.md, "Codebase development rules"), so each change edits the owning module and is
verified from a clean install; no forward migration is written. Table shape is the part that
cannot be fixed after production data exists; functions and indexes can follow the freeze.

## Objectives

- Every table passes the 18-question checklist in `docs/DATABASE_STYLE.md`.
- A reader audits the complete table structure from `schemas/base_schema/20_tables/` and the
  generated `docs/SCHEMA_TABLES.md` without opening a function body.
- Frozen Student Work facts are referenced from immutable snapshot rows, never copied per row.
- Closed vocabularies are native types declared once.
- The schema contains no lifecycle value, owner column, token, or backend scaffolding that Human
  Guidance has retired.

## Design philosophy

Tables are forever; functions are replaceable. Spend the effort on table shape now, in the layout
that will be audited later, and accept that function bodies and grants move mechanically. This is
**Fix the design, not the symptom** and **Long-term over short-term** from `docs/REPO_STYLE.md`.
Rejected alternative: fix per-row copies with triggers that keep the copies consistent; that
preserves the redundancy and adds a second thing to audit.

- Evidence strategy for uncertain methods: every milestone gate is a fresh install plus a catalog
  count or a connected probe in `tests/_temp/`; index work is decided only by
  `EXPLAIN (ANALYZE, BUFFERS)` on a seeded Course, never by the audit's candidate list alone.

## Scope

- Reorganize `schemas/base_schema/` into the layered layout in `docs/DATABASE_STYLE.md`.
- Add catalog comments to all 146 tables and a generated `docs/SCHEMA_TABLES.md`.
- Replace text-plus-CHECK vocabularies with enums and domains declared once.
- Introduce content-addressed Assessment policy and entry snapshots; remove per-row copies.
- Remove derived and duplicated Student Work columns and the grading wrapper table.
- Unify identity, revision-number width, and checksum types; remove retired lifecycle values,
  the assigned-Instructor column, `metadata_etag`, and constant columns.
- Collapse notification fan-out, remove iMathAS scaffolding, share one lease shape.
- Update the Rust data-access readers and projections each change touches.
- Add referencing-side indexes and generic immutability guards where measured.

## Non-goals

- Write forward migrations for populated databases; the base schema is edited directly.
- Redesign authorization: roles, RLS policies, and `SECURITY DEFINER` boundaries keep their
  semantics and only move files.
- Deduplicate function bodies beyond what a changed table forces.
- Change browser DTOs or API contracts, with one exception: the Library statistics projection
  gains the new counters and loses choice counts (WP-3.8). Every other projection returns the
  same JSON.
- Add iMathAS, H5P, or AI-backend tables; they return with their approved designs.
- Tune autovacuum, partitioning, or pooling; no workload evidence supports it.

## Current state summary

From the audit's installed-catalog inspection (2026-09-17): 146 tables, 281 FKs (166 without a
referencing-side index), 497 CHECKs, 282 indexes (38 explicit), 599 routines, 158 triggers, 290
policies, 3 enums against ~110 text-plus-CHECK vocabularies, 32 catalog comments, 92 hand-listed
immutability comparisons. `assessment_attempt` copies title, 50 KB instructions, and nine policy
strings per Attempt; `question_attempt` repeats a seven-string toolchain; `question_attempt_state`,
`finalization_kind`, and `question_response_grading` are derived; finalized responses are stored
twice; Library Watch notifications store each event three times. Rust readers of removed columns:
`crates/learning-data-access/src/archived_student_work_recovery.rs` and
`.../postgres/assessment_delivery_finalization.rs`; policy readers: five
`.../postgres/assessment_*` modules.

## Resolved decisions

Stated so the plan finishes without further human input. Change one by editing it here before the
dependent milestone starts, and record it in `docs/DESIGN_DECISIONS.md` at close-out.

1. Feedback disclosure rules are part of the Attempt snapshot (frozen at start, HUMAN_GUIDANCE.md:848).
2. A public object's public ID is its primary key and sole FK target (HUMAN_GUIDANCE.md,
   "Human-facing reference IDs", as revised 2026-09-18). Published Questions already comply;
   Question Pools, Course Instances, Assessments, Accounts, and Blueprint Courses re-key from
   `uuid` (Blueprints: from `reference_number bigint`) to the public-ID domain. Every
   `reference_number` column is dropped; a reader found in `crates/` or `schemas/` is rewritten to
   the public ID, not preserved.
3. `metadata_etag` becomes an Edit Number; change-proposal FKs re-key to `(reference, edit_number)`.
4. Finalized responses are finalized in place; `question_response` is removed.
5. iMathAS tables, columns, and enum values are removed.
6. Course themes stay a reference table; every other closed vocabulary becomes an enum.
7. Question Pools are current state with an Edit Number, not a Revision family (HUMAN_GUIDANCE.md
   "Common revision and history specifications" and "Question Pool specifications", revised
   2026-09-18). Student Work pins four values per Pool-served Question: Published Question ID,
   its Revision Number, Question Pool ID, and the Pool's Edit Number at selection. The Edit
   Number pin is evidence, not a foreign key.

## Architecture boundaries and ownership

| Component | Responsibility | Owner class |
| --- | --- | --- |
| `schemas/base_schema/10_types.sql` | every enum and domain | schema coder |
| `schemas/base_schema/20_tables/*.sql` | table DDL and catalog comments only | schema coder |
| `schemas/base_schema/30_constraints.sql`, `40_indexes.sql` | late FKs; measured indexes | schema coder |
| `schemas/base_schema/50_functions/*.sql`, `60_policies.sql`, `70_grants.sql` | behavior, RLS, grants | schema coder |
| `devel/generate_schema_tables_doc.py`, `docs/SCHEMA_TABLES.md` | generated structure doc | tooling coder |
| `devel/check_schema_style.py`, `devel/schema_catalog_lib.py`, `schemas/catalog_snapshot.json` | style checker over the committed snapshot or a live database | tooling coder |
| `crates/learning-data-access/src/postgres/*` | Rust readers of changed columns | Rust coder |
| `schemas/installation_data/live_demo.sql` | seed must load after every milestone | schema coder |

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M0 / WS-layout | all of `schemas/base_schema/`, `install.sql` | one patch, mechanical move |
| M0 / WS-docgen | `devel/generate_schema_tables_doc.py`, `devel/check_schema_style.py` | one patch |
| M1 / WS-types | `10_types.sql`, every table file, Rust enum mappings | one patch per vocabulary family group |
| M1 / WS-identity | Blueprint, Course, Assessment table files; `30_constraints.sql` | one patch |
| M2 / WS-snapshots | `20_tables/assessment.sql`, `assessment_attempt.sql`, `50_functions/assessment_*` , Rust readers | two patches (policy, entry) |
| M3 / WS-derived | `20_tables/assessment_attempt.sql`, finalization and recovery functions, Rust readers | one patch |
| M3 / WS-fanout | `20_tables/library_watch.sql`, `delivery_backends`, lease tables | one patch |
| M4 / WS-indexes | `40_indexes.sql`, `60_policies.sql`, immutability functions | one patch per measured group |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M0 | Layered layout | Move DDL by kind, add comments, generate structure doc | Structure readable in one place; no semantic change |
| M1 | Types and identity | Enums, domains, one identity convention, retired values gone | Checklist items 4, 5, 7, 8, 16 pass everywhere |
| M2 | Snapshots | Policy and entry snapshot tables; copies removed | Checklist items 2, 3, 10, 11 pass for Student Work |
| M3 | Derived data and fan-out | Derived columns, grading wrapper, response copy, notification copy, iMathAS, leases | Checklist items 2, 13 pass; deferred scaffolding gone |
| M4 | Indexes and guards (post-freeze allowed) | Measured FK indexes; generic immutability | Checklist items 12, 14 pass |

### Milestone: M0 layered layout

- Depends on: none.
- Deliverables: layered `schemas/base_schema/` tree; `install.sql` layer manifest; `COMMENT ON`
  for 146 tables; `devel/generate_schema_tables_doc.py`; committed `docs/SCHEMA_TABLES.md`;
  `devel/check_schema_style.py`.
- Workstreams: WS-layout (serial, owns every SQL file), WS-docgen (independent, owns the two new
  Python files; runs against the pre-move tree first, then re-runs after).
- Entry criteria: none.
- Exit criteria: fresh install yields the audit's catalog counts exactly (146 / 281 / 497 / 282 /
  290 / 599 / 158); Live Demo seed loads; `tests/e2e/database_baseline_security_catalog.sql`
  result unchanged; layout test passes.
- Parallel-plan ready: yes, WS-layout and WS-docgen, maximum 2. WS-layout is internally serial
  because every file moves.

### Milestone: M1 types and identity

- Depends on: M0 (edits must land in the final layout once).
- Deliverables: `10_types.sql` with one type per vocabulary family and the public-ID, checksum,
  and bounded-name domains; every `text CHECK (IN ...)` replaced; the five uuid-keyed public
  aggregates and their children re-keyed to the public ID; `revision_number` `integer` everywhere; `source_object_checksum` `bytea`
  everywhere; `assessment_status` without `closed`/`archived`; `assigned_instructor_*` removed;
  `metadata_etag` replaced; non-carrier constant columns removed; every table has a creation
  instant (43 lack one today); Rust mappings updated.
- Workstreams: WS-types, WS-identity. They overlap on `20_tables/assessment.sql`,
  `blueprint_course.sql`, `course_instance.sql`; WS-types lands first on those three files.
- Entry criteria: M0 exit.
- Exit criteria: `SELECT count(*) FROM pg_constraint WHERE contype = 'c' AND
  pg_get_constraintdef(oid) ~ ' IN \('` is below 10; every FK target is a `uuid` PK or composite
  natural key; `./check_codebase.sh` and focused Rust tests pass; decoders round-trip unchanged
  string values; seed loads.
- Parallel-plan ready: yes, maximum 2, with WS-types owning the three shared files until it lands.

### Milestone: M2 snapshots

- Depends on: M1 (snapshot columns use the enum types).
- Deliverables: `ple_data.assessment_policy_snapshot` and `ple_private.assessment_entry_snapshot`;
  `assessment`, `assessment_template`, `assessment_attempt` reference `policy_id`;
  `issued_question` references `entry_snapshot_id`; `assessment_entry` split into common,
  question, and pool tables; save/start/issue functions insert-or-reuse by content hash; landing,
  history, and delivery projections and the five Rust `assessment_*` readers updated.
- Workstreams: WS-snapshots (serial; policy snapshot patch, then entry snapshot plus
  partition-ready keys patch).
- Entry criteria: M1 exit.
- Exit criteria: `tests/_temp/` connected probe shows (1) editing a Released Assessment after an
  Attempt starts leaves that Attempt's resolved policy unchanged, (2) two Assessments with equal
  policy share one snapshot row, (3) `student_assessment_landing` and `assessment_attempt_history`
  return JSON identical to the M1 baseline for the Live Demo seed; probe removed at close-out.
- Parallel-plan ready: no; both patches edit `assessment_attempt.sql` and the same projections.

### Milestone: M3 derived data and fan-out

- Depends on: M2 for WS-derived (`issued_question` and `assessment_attempt` shapes settled); M1
  only for WS-fanout.
- Deliverables: `question_attempt_state`, `finalization_kind`, receipt duplicates,
  `question_response_grading`, and `question_response` removed; saved responses finalized in
  place; `ple_private.delivery_toolchain` plus `question_attempt.toolchain_id`;
  `library_watch_event_recipient` is the notification row; iMathAS tables, columns, and enum
  values removed; one shared lease shape across `job`, `course_retention_notification`,
  `course_banner_work`, `profile_image_work`; `archived_student_work_recovery.rs` and
  `assessment_delivery_finalization.rs` updated.
- Workstreams: WS-derived, WS-fanout (disjoint files).
- Entry criteria: M2 exit for WS-derived; M1 exit for WS-fanout.
- Exit criteria: `tests/e2e/attempt_expiry_connected_oracle.sql` and
  `unrelease_connected_oracle.sql` pass with the same intent; Watch reads return the same rows for
  the seed; lease transition covered by one fast test; no `imathas` string remains in
  `schemas/base_schema/`.
- Parallel-plan ready: yes, WS-derived and WS-fanout, maximum 2; WS-fanout may start during M2.

### Milestone: M4 indexes and guards (optional before freeze)

- Depends on: M3 (final table shapes).
- Deliverables: `EXPLAIN (ANALYZE, BUFFERS)` baseline on a seeded 100-Student Course for the
  Unrelease purge, FERPA purge, Student landing, gradebook, and Watch feed; referencing-side
  indexes in `40_indexes.sql` only where a plan changed; immutable tables lose `UPDATE` grants and
  gain `WITH CHECK (false)` policies; partially mutable rows use one generic `to_jsonb` trigger;
  retention-privacy probe (audit 2.8) run once; a recorded partition decision for
  `question_attempt` and `issued_question` from the measured per-Course purge time and projected
  row counts (partition only on the `DATABASE_STYLE.md` trigger).
- Workstreams: WS-indexes (serial; measure, add, re-measure).
- Entry criteria: M3 exit.
- Exit criteria: index scans on the purge and landing paths; a scratch column added to
  `assessment_attempt` is rejected on `UPDATE` with no trigger edit; each index has its
  before/after in the changelog.
- Parallel-plan ready: no; each index decision depends on the previous measurement.

## Workstream breakdown

### Workstream: WS-layout

- Goal: every SQL object in its layer file with no semantic change.
- Owner: schema coder.
- Work packages: WP-0.1, WP-0.2, WP-0.3.
- Needs: nothing.
- Provides: the layout every later workstream edits.
- Review boundary, when modifying the repository: one patch; reviewer diffs catalog counts.

### Workstream: WS-docgen

- Goal: generated structure doc and layout gate.
- Owner: tooling coder.
- Work packages: WP-0.4, WP-0.5.
- Needs: a `20_tables/` directory to scan (may stub against the pre-move tree).
- Provides: `docs/SCHEMA_TABLES.md`, `schemas/catalog_snapshot.json`, `devel/check_schema_style.py`.
- Review boundary: one patch.

### Workstream: WS-types

- Goal: vocabularies as types declared once.
- Owner: schema coder with Rust coder for mappings.
- Work packages: WP-1.1, WP-1.2, WP-1.3.
- Needs: M0 layout.
- Provides: enum types used by M2 snapshots.
- Review boundary: one patch per vocabulary family group (roles and Assessment policy first).

### Workstream: WS-identity

- Goal: one identity convention, retired values gone.
- Owner: schema coder.
- Work packages: WP-1.4, WP-1.5, WP-1.6, WP-1.7, WP-1.8.
- Needs: WS-types landed on the three shared files.
- Provides: stable FK targets for M2.
- Review boundary: one patch.

### Workstream: WS-snapshots

- Goal: frozen facts by reference.
- Owner: schema coder with Rust coder.
- Work packages: WP-2.1, WP-2.2, WP-2.3, WP-2.4, WP-2.5.
- Needs: M1 types and identity.
- Provides: final `assessment_attempt` and `issued_question` shapes for M3.
- Review boundary: two patches.

### Workstream: WS-derived

- Goal: no derived or duplicated Student Work evidence.
- Owner: schema coder with Rust coder.
- Work packages: WP-3.1, WP-3.2, WP-3.3.
- Needs: M2 shapes.
- Provides: final Student Work graph for M4 measurement.
- Review boundary: one patch.

### Workstream: WS-fanout

- Goal: lean notifications, no deferred scaffolding, one lease shape.
- Owner: schema coder.
- Work packages: WP-3.4, WP-3.5, WP-3.6, WP-3.7, WP-3.8.
- Needs: M1 enum types.
- Provides: nothing downstream.
- Review boundary: one patch.

### Workstream: WS-indexes

- Goal: measured indexes and drift-proof immutability.
- Owner: schema coder.
- Work packages: WP-4.1, WP-4.2, WP-4.3.
- Needs: M3 shapes.
- Provides: close-out evidence.
- Review boundary: one patch per measured group.

## Work packages

### Work package: WP-0.1 move DDL into layer files

- Owner: schema coder.
- Touch points: every file in `schemas/base_schema/`; new `00_roles.sql`, `10_types.sql`,
  `20_tables/<aggregate>.sql`, `30_constraints.sql`, `40_indexes.sql`, `50_functions/<domain>.sql`,
  `60_policies.sql`, `70_grants.sql`; `install.sql`. Use `git mv` where a file survives whole.
- Depends on: none.
- Acceptance criteria: fresh install matches the audit catalog counts exactly; seed loads;
  security-catalog e2e result unchanged.
- Obvious follow-ons: WP-0.2.

### Work package: WP-0.2 one table file per aggregate

- Owner: schema coder.
- Touch points: `20_tables/` files named `account`, `authentication`, `content_classification`,
  `published_question`, `question_pool`, `question_authoring`, `question_assets`,
  `library_discussion`, `library_watch`, `object_record`, `blueprint_course`, `course_instance`,
  `course_membership`, `course_media`, `profile_media`, `assessment`, `assessment_template`,
  `assessment_attempt` (all Student Work tables including grading), `statistics`, `retention`,
  `support_repair`, `jobs`, `corrections`, `audit`.
- Depends on: WP-0.1.
- Acceptance criteria: each table's children sit in its owner's file in dependency order; a role
  comment (current state / revision / event / snapshot, what deletes it, Human Guidance section)
  precedes each table.
- Obvious follow-ons: WP-0.3.

### Work package: WP-0.3 catalog comments

- Owner: schema coder.
- Touch points: `20_tables/*.sql`.
- Depends on: WP-0.2.
- Acceptance criteria: `COMMENT ON TABLE` for all 146 tables; `COMMENT ON COLUMN` where name and
  type do not explain the column; existing `--` prose moved, not duplicated.

### Work package: WP-0.4 schema tables doc generator

- Owner: tooling coder.
- Touch points: `devel/generate_schema_tables_doc.py`, `docs/SCHEMA_TABLES.md`.
- Depends on: none (stub against the current tree; re-run after WP-0.3).
- Acceptance criteria: a Python script (`source source_me.sh && python3
  devel/generate_schema_tables_doc.py`) installs into a disposable `postgres:17` container (or
  reads a named database), reads `pg_catalog`, and writes two committed artifacts:
  `docs/SCHEMA_TABLES.md` (one section per `20_tables/` file with columns, types, nullability,
  constraints, FKs, indexes, and catalog comments) and `schemas/catalog_snapshot.json` (the same
  facts as data: tables, columns with resolved types, PK / UNIQUE / FK column lists with parent
  tables, comments). Usage in `docs/USAGE.md`. Every table comment begins with a role tag
  (`role: current state | revision | event | snapshot | student work | aggregate | vocabulary`)
  so role-dependent rules are checkable.

### Work package: WP-0.5 extend the schema style checker to the new layout

- Owner: tooling coder.
- Touch points: `devel/check_schema_style.py` and `devel/schema_catalog_lib.py`, which
  [schema_style_checker_plan.md](schema_style_checker_plan.md) ships ahead of this plan against
  the current source; `schemas/catalog_snapshot.json` from WP-0.4.
- Depends on: WP-0.4; the checker plan's Tier 1 delivery.
- Acceptance criteria: the checker reads the snapshot (`--snapshot`) and a live database
  (`--database`) and reports the same findings as the source run; its Tier 2 rules (role tags,
  `updated_*` clocks, clock type by role, `course_instance_id`-led Student Work keys) activate
  because every table now carries a role tag; the layout rule blocks. The checker's summary line
  is pasted into every later restructure patch's changelog entry; a finding blocks the patch and
  the fix edits the table or its comment, never the rule. M1 rules (7, 4, 16, 11) report under
  `--report` until their milestone lands, then block.
- Independent confirmation: the generator session also runs `npx schemalint` against the same
  disposable database with its seven built-ins (`name-casing` snake, `name-inflection` singular,
  `prefer-text-to-varchar`, `prefer-timestamptz-to-timestamp`, `prefer-jsonb-to-json`,
  `prefer-identity-to-serial`, `require-primary-key`) at `error`; `.schemalintrc.js` is
  committed, `schemalint` is a devDependency, and the zero-finding output is pasted into the
  changelog entry that regenerates the snapshot. schemalint stays outside `check_codebase.sh`
  and the pytest lane because it needs Node and a live database.

### Work package: WP-1.1 declare vocabularies

- Owner: schema coder.
- Touch points: `10_types.sql`.
- Depends on: M0.
- Acceptance criteria: one enum per family (`product_role`, `assessment_type`, `late_work_rule`,
  `question_variation_rule`, `question_order_rule`, `feedback_release`, `scoring_rule`,
  `entry_kind`, `entry_availability`, `question_backend`, `question_format`, `question_type`,
  `library_object_kind`, `library_watch_event_kind`, `media_type`, `object_storage_area`,
  `object_data_class`, `lease_state`); domains for public-ID shapes, SHA-256 `bytea`, bounded
  names, IANA zone names; `course_theme` reference table.

### Work package: WP-1.2 apply types to columns

- Owner: schema coder.
- Touch points: every `20_tables/` file with a `CHECK (col IN (...))`; the 11 role-typed FK
  carriers first.
- Depends on: WP-1.1.
- Acceptance criteria: literal-set CHECK count below 10; role-typed FKs unchanged in semantics.

### Work package: WP-1.3 Rust enum mappings

- Owner: Rust coder.
- Touch points: `crates/learning-data-access` column reads and binds; `cargo tsgen` output.
- Depends on: WP-1.2.
- Acceptance criteria: `./check_codebase.sh` and focused Rust tests pass; decoders round-trip
  unchanged strings.

### Work package: WP-1.4 re-key public aggregates to their public ID

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/account.sql`, `course_instance.sql`, `blueprint_course.sql`,
  `assessment.sql`, `question_pool.sql` and every child table referencing them;
  `30_constraints.sql`; the functions in `50_functions/` that bind those keys; Rust store
  types in `crates/learning-data-access`; `schemas/installation_data/live_demo.sql`.
- Depends on: WP-1.1 (public-ID domains) and WP-1.2 on the shared files.
- Acceptance criteria: each of the five aggregates has `<entity>_id text PRIMARY KEY` typed by
  its public-ID domain; every FK to it targets that column; no `reference_number` column remains;
  Published Questions unchanged; `public_id_reservation` still guarantees global uniqueness;
  seed loads; the canonical public-ID e2e proofs pass.
- Obvious follow-ons: WP-1.6 (`metadata_etag` FKs re-key at the same time).

### Work package: WP-1.5 unify widths and checksum types

- Owner: schema coder.
- Touch points: every table with `revision_number` or `source_object_checksum`.
- Depends on: M0.
- Acceptance criteria: `revision_number` is `integer` everywhere; `source_object_checksum` is
  `bytea` with `octet_length = 32` everywhere; Rust binds updated.

### Work package: WP-1.6 remove retired shapes

- Owner: schema coder.
- Touch points: `20_tables/assessment.sql` (`assessment_status`), `course_instance.sql`
  (`assigned_instructor_*`, `metadata_etag`), `blueprint_course.sql`, `question_pool.sql`
  (`metadata_etag`), change-proposal tables, constant columns listed in audit 1.6.
- Depends on: WP-1.4.
- Acceptance criteria: no `closed`/`archived` Assessment value; Course authority is
  `course_membership` only; one concurrency token repo-wide; constant columns gone except role
  carriers.

### Work package: WP-1.7 every table has a clock

- Owner: schema coder.
- Touch points: the 43 tables listed in audit 2.9; their insert functions in `50_functions/`;
  Rust binds where the insert is application-side.
- Depends on: M0.
- Acceptance criteria: every table has one `NOT NULL` creation clock in the type
  `docs/DATABASE_STYLE.md` ("Every table has a clock") assigns to its role: full-precision
  `timestamptz` for enforced, ordered, or audited rows and `date` for authored content
  (`published_question.created_on`, `question_revision.published_on`, `question_pool`,
  `blueprint_course` and their Revisions, `draft_question`); `assessment_template`, the lineage
  rows `question_pool` and `blueprint_course` (metadata changes without a Revision), and every
  other current-state table also has `updated_at`/`updated_on` with the `>=` CHECK;
  `question_revision_statistics.updated_at` becomes `created_on date` plus `updated_on date`
  (aggregate exception: day granularity, no time of day); Rust binds follow the type change; a fast pytest over
  `schemas/catalog_snapshot.json` fails on a table without a clock (rule 14 in
  `devel/check_schema_style.py`).
- Obvious follow-ons: none.

### Work package: WP-1.8 key columns name their table

- Owner: schema coder with Rust coder.
- Touch points: every table whose PK or FK column name differs from `[<role>_]<parent_table>_id`
  (72 of 180 single-column FKs per the audit's naming census: `course_id` ->
  `course_instance_id`, `question_id` -> `published_question_id`, `object_id` /
  `source_object_id` -> `object_record_id` / `source_object_record_id`, `delivery_id` ->
  `object_delivery_id`, `workspace_id` -> `authoring_workspace_id`, `discipline_uuid` ->
  `content_discipline_id` and the other classification keys, `thread_id`, `folder_id`,
  `proposal_id`, `capability_id`, `membership_id`, `invitation_id`, `attestation_id`,
  `vetting_decision_id`, `correction_id`, `event_id` on watch tables); functions, policies, Rust
  store types and `sqlx` queries, the seed; API field names stay unchanged and are mapped in the
  Rust DTO layer.
- Depends on: WP-1.4 (the public-ID re-key already renames those columns once).
- Acceptance criteria: the naming census reports zero mismatches; `cargo tsgen` output is
  unchanged (API names untouched); fresh install; seed loads; `./check_codebase.sh` passes.
- Obvious follow-ons: none.

### Work package: WP-2.1 policy snapshot

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/assessment.sql`, `assessment_template.sql`, `assessment_attempt.sql`;
  save, template-copy, and Attempt-start functions; five Rust `assessment_*` readers.
- Depends on: M1.
- Acceptance criteria: `assessment_policy_snapshot` content-addressed and shared; 9 columns
  removed from each of `assessment`, `assessment_template`; 15 from `assessment_attempt`.

### Work package: WP-2.2 entry snapshot and entry split

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/assessment.sql` (`assessment_entry` split), `assessment_attempt.sql`
  (`assessment_entry_snapshot`, `issued_question`); issue and scoring functions.
- Depends on: WP-2.1.
- Acceptance criteria: 5 columns removed from `issued_question`; scorer reads points through the
  snapshot; entry kinds in child tables with no nullable pairing CHECK.

### Work package: WP-2.3 partition-ready Student Work keys

- Owner: schema coder with Rust coder.
- Touch points: every table in `20_tables/assessment_attempt.sql` (Attempt, Pool selection,
  selected item, Issued Question, Question Attempt, saved response, grading result) and the
  `ple_audit` correction targets; their FKs in `30_constraints.sql`; Rust store types.
- Depends on: WP-2.2 (lands in the same re-key so keys change once).
- Acceptance criteria: every Student Work table has `course_instance_id NOT NULL` bound in its parent FK;
  every PK and UNIQUE on those tables leads with `course_instance_id`; no table is partitioned; a
  `tests/_temp/` probe creates a `PARTITION BY LIST (course_instance_id)` parent for `question_attempt`
  and attaches a copy without error, then is removed. Estimate recorded: ~12.5M Issued Questions
  per semester at 50 Courses x 100 Students x 20 Assessments x 25 Questions x 5 Attempts, held to
  a steady state of ~25-40M rows per Student Work table by the 365-day FERPA purge; partitioning
  is therefore unlikely at this scale, and the keys exist for the per-Course purge and as
  insurance.

### Work package: WP-2.4 snapshot connected probe

- Owner: tester.
- Touch points: `tests/_temp/`.
- Depends on: WP-2.3.
- Acceptance criteria: the three M2 exit checks pass; probe removed at close-out.

### Work package: WP-2.5 Question Pools as current state

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/question_pool.sql` (drop `question_pool_revision`; key
  `question_pool_member` by `(question_pool_id, member_position)`; add `edit_number`,
  `interchangeability_attested_by_account_id`, `interchangeability_attested_at`, `updated_on`
  to the Pool row); `20_tables/assessment.sql` (Pool entries reference `question_pool_id` only);
  `20_tables/assessment_attempt.sql` (`question_pool_selection` carries `question_pool_id` and
  `question_pool_edit_number`; selected items keep their exact Question pins);
  `20_tables/blueprint_course.sql` (Blueprint Revisions embed Pool member lists in `content`
  plus pin rows); statistics and Bloom tables keyed by Pool; `50_functions/` replaces
  `append_question_pool_revision`, `construct_question_pool_revision_fork`, and the fork
  wrappers with one `save_question_pool_members` (CAS on Edit Number, re-attestation, canonical
  no-op advances nothing) and one fork that copies members; Watch notifications emit "members
  changed"; Rust store types and the Pool detail projection.
- Depends on: WP-2.2 (entry split gives Pool entries their own child table).
- Acceptance criteria: no `question_pool_revision` table or `revision_number` column on any Pool
  table; `question_pool_selection` rows carry all four pins; a Blueprint Revision installed from
  the seed reproduces its Pool member lists from its own content; Watch, statistics, and Bloom
  reads return the same rows for the Live Demo seed keyed by Pool; the two-Instructor
  concurrent-save probe in `tests/_temp/` shows CAS rejection on a stale Edit Number and a
  no-op save leaving the Edit Number unchanged.
- Obvious follow-ons: `docs/DATABASE_STRUCTURE.md` "Questions and Pools" and the
  `created_in_transaction xid8` paragraph are rewritten at close-out.

### Work package: WP-3.1 remove derived Student Work columns

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/assessment_attempt.sql`; finalization, history, recovery functions;
  `archived_student_work_recovery.rs`, `assessment_delivery_finalization.rs`.
- Depends on: M2.
- Acceptance criteria: `question_attempt_state`, `finalization_kind`, receipt duplicates,
  `question_response_grading`, `question_response` gone; saved responses finalized in place.

### Work package: WP-3.2 delivery toolchain table

- Owner: schema coder.
- Touch points: `20_tables/assessment_attempt.sql`; issue function.
- Depends on: WP-3.1.
- Acceptance criteria: `delivery_toolchain` with `UNIQUE` over its seven values;
  `question_attempt` has `toolchain_id` and none of the seven text columns.

### Work package: WP-3.3 Student Work oracles

- Owner: tester.
- Touch points: `tests/e2e/attempt_expiry_connected_oracle.sql`,
  `tests/e2e/unrelease_connected_oracle.sql`.
- Depends on: WP-3.2.
- Acceptance criteria: both pass with the same intent; C522 delivery proof and finalization row
  counts match.

### Work package: WP-3.4 notification fan-out

- Owner: schema coder.
- Touch points: `20_tables/library_watch.sql`; Watch functions.
- Depends on: M1.
- Acceptance criteria: `library_watch_notification` dropped; reads join recipient to event;
  `read_at` added only if a reader exists.

### Work package: WP-3.5 remove iMathAS scaffolding

- Owner: schema coder.
- Touch points: `delivery_backends` tables, both source-binding tables, `10_types.sql`.
- Depends on: M1.
- Acceptance criteria: no `imathas` string in `schemas/base_schema/`; Rust binds updated.

### Work package: WP-3.6 shared lease shape

- Owner: schema coder.
- Touch points: `job`, `course_retention_notification`, `course_banner_work`,
  `profile_image_work` tables and their transition functions.
- Depends on: M1.
- Acceptance criteria: one column set, one pairing CHECK, one transition function; each worker's
  continued need confirmed in the changelog entry.

### Work package: WP-3.7 bound the non-FERPA growth tables

- Owner: schema coder.
- Touch points: `20_tables/authentication.sql`, `50_functions/authentication.sql`,
  `20_tables/object_record.sql` (drop `ple_audit.object_delivery_access_event`), the retention
  worker in `50_functions/retention.sql`.
- Depends on: M1.
- Acceptance criteria: expired and revoked `authenticated_session`, `email_authentication_challenge`,
  `passkey_ceremony`, and `authentication_rate_limit` rows are deleted by the existing retention
  sweep after a fixed grace window (hardcoded, not configurable); each of the four has a partial
  index on the sweep predicate; `object_delivery_access_event` is gone; audit 2.10 lists no
  unbounded table.
- Obvious follow-ons: none.

### Work package: WP-3.8 Question Library object usage statistics reshape

- Owner: schema coder with Rust coder.
- Touch points: `20_tables/statistics.sql`; the increment functions in `50_functions/statistics.sql`;
  the issue, submission, and grading functions that raise observations;
  `crates/domain/src/statistics/version_counts.rs` and the Library projection; Pool detail
  projection.
- Depends on: WP-1.7 (aggregate carries `created_at` only), WP-3.1 (finalization shape).
- Acceptance criteria: `question_revision_statistics` holds exactly `issued_count`, `blank_count`,
  `answered_count`, `correct_count`, `partial_count`, `incorrect_count`, `credit_sum`,
  `credit_sum_sq`, `created_on`, `updated_on`, with CHECKs for the two identities in
  `docs/FERPA_DATA_POLICY.md` ("Question Library object usage statistics"); `question_revision_choice_statistics`
  and `question_statistics_observation_choice` are dropped; the submission transaction
  increments every counter exactly once per Issued Question (`issued_count` always,
  `blank_count` when no saved response exists, otherwise `answered_count` plus the outcome
  counter and the credit sums); a `question_pool_statistics` row per Pool holds `issued_count`
  and a `question_pool_member_statistics` row per `(question_pool_id, published_question_id)`
  holds `selected_count`, both incremented in the same submission transaction; a Pool's outcome statistics are summed from members at read time;
  every bulk view shows the all-Revision rollup with every rate beside its observation count,
  and the Question detail page alone adds the per-Revision breakdown;
  one connected probe in `tests/_temp/` covers issue, blank, correct, partial, incorrect, a Pool
  issue with member selection, and a Pool sum.
- Obvious follow-ons: none.

### Work package: WP-4.1 measured baseline

- Owner: schema coder.
- Touch points: `tests/_temp/` seed script and `EXPLAIN` captures.
- Depends on: M3.
- Acceptance criteria: before-plans recorded for purge, landing, gradebook, Watch feed, and the
  Library query that sorts every Question Pool by derived difficulty (member join to
  `question_revision_statistics`); a cached difficulty column on the Pool row is added only if
  that plan fails the Library's latency budget.

### Work package: WP-4.2 referencing-side indexes

- Owner: schema coder.
- Touch points: `40_indexes.sql`.
- Depends on: WP-4.1.
- Acceptance criteria: each index has a before/after plan in the changelog; candidate list from
  audit 3 package F is the starting set, not the target.

### Work package: WP-4.3 generic immutability guards

- Owner: schema coder.
- Touch points: `60_policies.sql`, `70_grants.sql`, `50_functions/` trigger functions.
- Depends on: M3.
- Acceptance criteria: no `IS DISTINCT FROM OLD` column list remains; scratch-column test rejects
  `UPDATE` with no trigger edit.

## Acceptance criteria and gates

- Per-patch gate: fresh install from `install.sql` in a disposable `postgres:17` container; Live
  Demo seed loads; `devel/check_schema_style.py` exits clean, `tests/test_markdown_links.py` and the focused
  Rust tests for touched readers pass; `docs/CHANGELOG.md` entry names the milestone and patch.
- Integration gate (per milestone): the milestone's exit criteria; `./check_codebase.sh`;
  `docs/SCHEMA_TABLES.md` regenerated when any table changed.
- Independent review gate: M2 and M3 patches get an `audit-code-reviewer` pass focused on
  Human Guidance alignment (HUMAN_GUIDANCE.md:844-848, :1550-1556) before merge.
- Failure plan: a failed gate blocks the next patch; the fix edits the owning module, never a
  compensating layer.

## Test and verification strategy

- Catalog counts are the M0 oracle: identical counts prove a mechanical move.
- Connected probes in `tests/_temp/` prove M2 and M3 behavior; they are removed at close-out
  unless a probe fails once, in which case it is promoted per `docs/PYTEST_STYLE.md`.
- The two existing e2e SQL oracles are the regression net for Student Work.
- `EXPLAIN (ANALYZE, BUFFERS)` before/after is the only justification for an M4 index.
- Full `./launchers/all_test.sh` acceptance runs once at close-out, not per patch.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Rust readers missed when a column is removed | runtime `sqlx` error in a path no test covers | grep of `crates/` finds a column name after the patch | Rust coder | grep column names before each removal; run the production build |
| M1 workstreams collide on shared table files | merge conflicts, rework | both edit `assessment.sql` | schema coder | WS-types lands first on the three shared files |
| Snapshot hash drifts from canonical content | duplicate snapshots, no sharing | two saves with equal policy produce two rows | schema coder | canonicalize with `jsonb` key order before hashing; M2 probe check (2) |
| Live Demo seed depends on removed columns | seed fails, Live Demo down | seed load error after a patch | schema coder | seed load is in every per-patch gate |
| Scope creep into function dedup | milestones stall | a patch rewrites bodies beyond the forced change | schema coder | Non-goals; reviewer rejects unforced body edits |
| Plan drift from audit | fixes miss findings | a finding has no work package | plan owner | audit finding numbers cited in each WP acceptance |
| Index added on a hunch | write cost with no read benefit | index without a recorded plan | schema coder | M4 gate requires before/after plans |

## Rollout and release checklist

- [ ] M0 through M3 landed; M4 landed or deferred with recorded `EXPLAIN` evidence.
- [ ] `docs/SCHEMA_TABLES.md` regenerated from the final catalog.
- [ ] `./launchers/all_test.sh` acceptance passed once.
- [ ] Live Demo started fresh from the new schema and the screenshot corpus replayed.

## Documentation close-out requirements

- Active plan / progress tracker: this file; update Status per milestone.
- docs/CHANGELOG.md entry: one per patch, citing milestone, WP, and audit finding numbers.
- Archive / closure notes: `docs/DESIGN_DECISIONS.md` gains entries for resolved decisions 1-6;
  `docs/DATABASE_STRUCTURE.md` updated for renamed concepts; `docs/USAGE.md` documents the
  generator; `git mv` this file to `docs/archive/`.

## Patch plan and reporting format

- Patch 1: M0 WP-0.1 to WP-0.3 (layout and comments).
- Patch 2: M0 WP-0.4 and WP-0.5 (generator and gate).
- Patch 3: M1 WP-1.1 to WP-1.3 (types).
- Patch 4: M1 WP-1.4 to WP-1.8 (identity, retired shapes, clocks, key column names).
- Patch 5: M2 WP-2.1 (policy snapshot).
- Patch 6: M2 WP-2.2 to WP-2.4 (entry snapshot, partition-ready keys, probe).
- Patch 7: M2 WP-2.5 (Question Pools as current state).
- Patch 8: M3 WP-3.1 to WP-3.3 (derived data).
- Patch 9: M3 WP-3.4 to WP-3.8 (fan-out, iMathAS, leases, growth sweeps, usage statistics).
- Patch 10+: M4 per measured index group and guards.
- Patch N: remaining repository-required work (docs close-out, archive).

Report each patch as `M<k> WP-<id>: <outcome>; gate: <evidence>` in the changelog.

## Open questions and decisions needed

- Manager/subagent decision procedure for `reference_number` readers:
  - Decision owner or dedicated class: schema coder in WP-1.4.
  - Evidence and decision rule: grep `crates/` and `schemas/` for each column; every reader is
    rewritten to the public ID and the column is dropped. No exception path.
- Non-blocking follow-up: whether Instructors may loosen feedback disclosure after an Attempt
  starts (decision 1 keeps it frozen); a product answer changes a projection, not a table.
- Non-blocking follow-up: function-body deduplication audit after the freeze.
