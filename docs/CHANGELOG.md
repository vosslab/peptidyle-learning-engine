# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 16 entries are archived in [CHANGELOG-2026-09l.md](CHANGELOG-2026-09l.md).

## 2026-09-19

### Fixes and Maintenance

- Align Student Course landing, invitation, Assessment list, and Attempt
  context JSON public-ID fields to `id`. Live Demo activity provisioning
  reads those `id` fields. Human Guidance now states that a Pool fork
  starts at Edit Number 1, not Revision 1. Gate: `cargo test -p server
  --lib live_student_course_landing` and `cargo test -p question_model
  --lib blueprint_course`.

- Live Demo and bundled publisher Accounts insert the mint placeholder so
  `ple_private.assign_human_reference` issues a random public ID. Email and
  authoring workspace remain the stable lookup. After provision, the local
  stack records the minted persona IDs into the env file and recreates the
  API before Morgan TOTP. Gate: `tests/test_live_demo_target.py`.

- `launchers/run_fast_checks.sh` regenerates schema tables docs and runs
  `schema_style/check_schema_style.py` first. The checker now exits 1 on
  any finding, including `rule_14_unindexed_fk`.
  `launchers/all_test.sh` uses the same schema gate. Docs-only plus launcher
  and checker exit policy.

- Refresh living identity and architecture docs against Human Guidance: Question
  Pools are current membership with an Edit Number, public IDs are the only
  stored identity and JSON `id`, and saved responses finalize in place.
  Touched [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md),
  [FILE_STRUCTURE.md](FILE_STRUCTURE.md),
  [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md),
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
  [QUESTION_MODEL.md](QUESTION_MODEL.md),
  [CONTRACTS.md](CONTRACTS.md),
  [API_CONTRACTS.md](API_CONTRACTS.md),
  [DATA_CONTRACTS.md](DATA_CONTRACTS.md),
  [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md),
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
  [BLOOM_TAXONOMY_GUIDE.md](BLOOM_TAXONOMY_GUIDE.md),
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md),
  [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md), and related lifecycle
  docs. Historical changelog and archive files are unchanged. Docs-only.

## 2026-09-18

### Additions and New Features

- Pre-production identity cutover: public-ID JSON fields are `id`, not a
  parallel `reference` property. Dual `id`+`reference` is gone from
  Course, Assessment, Attempt, and Blueprint views. Gate:
  `cargo tsgen` and `./launchers/run_fast_checks.sh`.

- Live Demo and bundled publisher Account IDs are public `UXXXXXXXZ`
  values (`U0000035E` and kin). The public-ID mint trigger keeps a
  supplied canonical ID and only replaces the mint placeholders
  `U00000009`, `CI0000000Y`, `A0000000A`, and `BP0000000C`. Gate:
  `tests/test_live_demo_target.py`.

- Student Work API grants start as `ple_private_owner` so REVOKE/GRANT on
  `ple_private` SECURITY DEFINER functions apply. Replica durability counts
  `assessment_attempt_saved_response` instead of dropped `question_response`
  tables. The replica count validator accepts one integer per query, not a
  fixed five-field row. The stored Question fixture set uses public Course
  Instance and Assessment IDs for `cargo tools fixtures`. Live Demo composition tests
  use public Account IDs. Markdown schema links follow the layered
  `schemas/base_schema/` layout; gitignored `catalog_snapshot.json` is not
  a browsable link. Course Banner HTTP lives in
  `crates/server/src/course_appearance/banner.rs`. The Assessment Question
  Editor view lives in
  `assessment_workspace_questions_view.tsx`. Clock type and Library
  statistic increment rules moved from Human Guidance into
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md). Gate:
  `tests/test_disposable_stack_replica_adapter.py` and
  `./launchers/run_fast_checks.sh`.

- Close-out gates: disposable `postgres:17` install plus Live Demo teaching
  graph (publisher psql variables supplied; eight Pilot Question pins and a
  public Blueprint so `live_demo.sql` loads). Connected Student Work oracles
  `attempt_expiry_connected_oracle.sql` and `unrelease_connected_oracle.sql`
  pass. M4 Unrelease purge `EXPLAIN` uses
  `assessment_attempt_assessment_id_idx`. Domain CHECK helpers
  `is_canonical_prefixed_public_id` are `EXECUTE` to `PUBLIC`. Gate:
  `tests/_temp/run_m4_install_and_explain.py` and
  `./schema_style/check_schema_style.py` twice.

- WP-3.8 Library projection JSON: `QuestionStatistics` keeps `{state:"unavailable"}`
  and adds Instructor `available` with snake_case counts and rates
  (`issued_count`, `blank_count`, `answered_count`, outcome counts,
  `credit_sum`, `credit_sum_sq`, omitted rates when the denominator is 0).
  Bulk search uses the all-Revision rollup; Question detail adds `revisions`.
  Pool detail `evidence` uses current members' all-Revision rollup (mean
  credit weighted by `answered_count`) plus `pool_issued_count` from
  `question_pool_statistics`. SQL:
  `ple_api.read_question_library_usage_statistics`,
  `read_question_library_revision_usage_statistics`,
  `read_question_pool_library_usage_statistics`. Students never receive
  these aggregates; Sysadmin Library reads stay unavailable. Gate:
  `cargo test -p question_model --offline --lib question_library_statistics`
  and `node --test tests/test_question_statistics_decoder.mjs`.

- M4 WP-4.1, WP-4.2: Recorded `EXPLAIN (ANALYZE, BUFFERS)` for Unrelease
  purge, FERPA purge, Student landing, gradebook, Watch feed, and Pool
  difficulty (member join to `question_revision_statistics`) in
  `{SCRATCH}/explain_baseline.log`. Live Demo teaching graph was not
  loaded (publisher psql variables absent), so plans ran on an empty
  Student Work catalog; execution times were sub-millisecond. FERPA
  purge already uses `assessment_attempt_student_assessment_lookup_idx`;
  gradebook uses the course-leading unique key. Unrelease
  `DELETE ... WHERE assessment_id = ...` seq-scans `assessment_attempt`
  because the PK leads with `course_instance_id`; added
  `assessment_attempt_assessment_id_idx`. No cached Pool difficulty
  column (empty-catalog join does not miss a latency budget). Remaining
  advisory `rule_14` unindexed FKs deferred. WP-4.3 generic `to_jsonb`
  immutability guards deferred: column-list `IS DISTINCT FROM OLD`
  triggers remain on Student Work and Course lifecycle. Gate:
  `tests/_temp/run_m4_install_and_explain.py` (install.sql, M2 probes
  pass, M3 revision counters pass).

- M3 WP-3.1, WP-3.2, WP-3.8 (WS-derived): Dropped `question_attempt_state`,
  `finalization_kind`, `question_response`, `question_response_grading`,
  submission `receipt`, and `ple_private.imathas_question_backend_session`.
  Saved responses finalize in place (`finalized_at`,
  `assessment_submission_id`). `grading_result` FKs
  `(course_instance_id, question_attempt_id)` only; receipts FK that PK.
  `ple_private.delivery_toolchain` holds the seven toolchain values;
  `question_attempt.delivery_toolchain_id` is the NOT NULL FK. Library statistics
  store `issued_count`, `blank_count`, `answered_count`, outcome buckets,
  and credit sums; dropped choice statistics; added pool issued and member
  selected counts. JSON field names stay the same except Library stats
  (snake_case). Gate: `tests/_temp/m3_library_stats_probe.sql` and
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`. GRANT
  `question_source_binding_fields_are_valid` now matches the three-argument
  CREATE so disposable `postgres:17` `install.sql` succeeds.

- M3 WP-3.4 to WP-3.7 (WS-fanout): Watch inbox JOINs
  `library_watch_event_recipient` to `library_watch_event` (same JSON
  columns; no `read_at`). Dropped `ple_private.library_watch_notification`,
  `ple_private.imathas_render_cache_entry`, iMathAS source-binding columns
  and enum labels (`question_backend`, `question_format`, `import_format`),
  delivery SQL wrappers, and `ple_audit.object_delivery_access_event` plus
  unused `ple_data.access_decision`. Postgres iMathAS session store returns
  Unavailable. Shared work lease is `lease_token` plus `lease_expires_at`
  with `ple_private.work_lease_pair_is_valid` on `job`,
  `course_retention_notification`, `course_banner_work`, and
  `profile_image_work` (banner/profile keep `lease_state` payload). Claim
  stays per-worker. Workers still needed: public-asset job, FERPA notifier,
  banner work, profile image work. Retention worker sweeps expired/revoked
  `authenticated_session`, expired/consumed `email_authentication_challenge`
  and `passkey_ceremony`, and old `authentication_rate_limit` windows after
  a hardcoded 7-day grace (`ple_api.sweep_expired_authentication_growth`).
  Partial sweep indexes sit on those predicates. `question_backend` is
  `ple` and `webwork` only. Gate:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`.

- M2 WP-2.5 and WP-2.3: Question Pools are current state. Dropped
  `question_pool_revision`, `question_pool_revision_member`, and
  `question_pool_revision_bloom`. Members live in `question_pool_member`
  keyed by `(question_pool_id, member_position)`. Attestation and Edit
  Number live on `question_pool`. `save_question_pool_members` CAS-es the
  Edit Number, no-ops an identical ordered list, and re-attests on change.
  Forks copy current members once. Assessment pool entries and snapshots
  pin `question_pool_id` only. `question_pool_selection` stores
  `question_pool_id` plus `question_pool_edit_number`; selected items keep
  exact Question pins. Watch emits `members_changed`. Student Work tables
  in `assessment_attempt.sql` (and ple_audit correction targets) carry
  `course_instance_id NOT NULL` leading every PRIMARY KEY and UNIQUE, bound
  in parent FKs. Accommodation stays teaching config with a uuid PK and
  `course_instance_id` in Attempt FK tuples. JSON field names are unchanged.
  Gate: `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py` twice exits 0 (advisory
  `rule_14` 175). `tests/_temp/m2_snapshot_probes.sql` on disposable
  `postgres:17` shows equal policies share one snapshot, distinct policies
  do not, and an Attempt keeps the original snapshot after the Assessment
  policy changes (`student_work_oracles.log`).

- M2 WP-2.2: `assessment_entry` is a parent of common columns; kind-specific
  facts live in `assessment_entry_question` and `assessment_entry_pool`.
  Frozen issue facts live in `ple_private.assessment_entry_snapshot`
  (content-addressed SHA-256 PK). `issued_question` stores
  `assessment_entry_snapshot_id` and drops `point_value`, `scoring_rule`,
  `question_attempt_limit`, `question_attempt_time_limit_seconds`, and
  `question_attempt_grace_seconds`. Save and issue call
  `ple_private.ensure_assessment_entry_snapshot`; the scorer reads points
  through the snapshot. JSON field names are unchanged. Gate:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py` exits 0 (advisory `rule_11`
  15, `rule_14` 172).

- Human Guidance public-ID names: Rust/TypeScript types are `AccountId`,
  `CourseInstanceId`, `AssessmentId`, and `BlueprintCourseId` (no parallel
  `*Reference` or `CourseId` alias). `docs/TERMINOLOGY_CONTRACT.md` matches
  HUMAN_GUIDANCE: public IDs are the primary keys; Question Pools are current
  state with an Edit Number; Student Work pins Pool ID plus Pool Edit Number.
  Gate: `cargo test -p question_model --offline --lib` (146 passed);
  `cargo check -p learning-data-access -p server_core -p project-tools
  --offline`; `cargo tsgen` wrote 395 types.

- M2 WP-2.1: frozen Assessment policy lives in
  `ple_data.assessment_policy_snapshot` (content-addressed SHA-256 PK).
  `assessment`, `assessment_template`, and `assessment_attempt` store
  `assessment_policy_snapshot_id` instead of copying title, instructions,
  schedule, limits, rules, and feedback. `ple_private.ensure_assessment_policy_snapshot`
  inserts-or-reuses the row; Attempt start copies the Assessment's current
  snapshot id. Quiz/Exam attempt-limit = 1 is enforced in the helper.
  JSON field names are unchanged. Style checker allows `sha256_digest` PKs
  on `role: snapshot` tables. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py` twice exits 0
  (advisory `rule_11` 15, `rule_14` 170).



- M1 WP-1.4: Account, Course Instance, Blueprint Course, Assessment, and
  Question Pool primary keys are the public ID (`ple_data.account_id`,
  `course_instance_id`, `blueprint_course_id`, `assessment_id`,
  `question_family_id` domains in `10_types.sql`). Foreign keys target those
  columns. GRANT identities were rewritten to match CREATE FUNCTION argument
  types. Disposable `postgres:17` `install.sql` completes (146 tables, 281 FKs,
  600 routines including public-ID domain helpers; CHECK/index/policy/trigger
  counts changed with enums and clocks). Gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  exits 0 with advisory `rule_11` and `rule_14`. Catalog snapshot type display
  keeps schema-qualified enums (`ple_data.product_role`). Live Demo seed and
  Rust `AccountId`/`CourseInstanceId`/`AssessmentId` mappings still follow.

- M1 WP-1.6: the concurrency token is the Edit Number only. SQL columns are
  `course_edit_number`, `blueprint_edit_number`, and
  `question_pool_edit_number`. JSON uses those names (`courseEditNumber` /
  `blueprint_edit_number` / `blueprintEditNumber` depending on the contract's
  serde case). HTTP `ETag`/`If-Match` is the decimal integer as a strong
  validator (`"42"`), not a UUID. `cargo tsgen` wrote 395 types including
  `BlueprintEditNumber`. Gate: `cargo check -p learning-data-access -p
  server_core --offline` after the Course JSON field rename.

- M1 Live Demo: `installation_data_activity` issues temporary sessions by
  looking up `ple_private.account_authentication_email.normalized_email` as
  `ple_migrator`/`ple_private_owner` through `PLE_MIGRATION_DATABASE_URL`.
  Fictional emails stay the stable seed keys; minted Account public IDs are
  not compiled in. Gate: `cargo check -p project-tools --offline`.

- Account, Course Instance, and Assessment identities in Rust are the
  canonical public ID string (`AccountId` / `CourseInstanceId` /
  `AssessmentId`). `AccountId`, `CourseInstanceId`, and `AssessmentId` are
  aliases of those types so SQL, Rust, and TypeScript share one value. Postgres
  binds and decodes that text. Live Demo env vars parse public IDs, not UUIDs.

- WP-1.4 leftover in `50_functions`: bodies that still read
  `public_reference` / `reference_number` on Account, Course Instance,
  Blueprint Course, Assessment, and Question Pool now use the text `*_id`
  PK. Locals and parameters that held those IDs as `uuid` or `bigint`
  (including `v_actor` from `current_session_account_id()`) are `text`.
  API `RETURNS TABLE (public_reference text, ...)` aliases remain.
  `assessment_attempt.reference_number` and
  `authoring_workspace.reference_number` are unchanged. 60_policies needed
  no further public-ID column rewrites.

- After dropping `ple_private.assessment_attempt.reference_number` and
  `ple_private.authoring_workspace.reference_number`, `50_functions` readers
  and writers look up Assessment Attempts by `assessment_attempt_id uuid`.
  Parameters, `RETURNS TABLE` columns, GRANT EXECUTE identities, and JSON
  `assessmentAttempt` values use that PK text rather than `'R-' ||
  reference_number`. Archived recovery keyset pagination uses
  `p_after_assessment_attempt_id uuid`.

- Removed `ple_data.course_retention_policy`. FERPA notice, archive, recovery,
  and deletion intervals are installation GUCs
  (`ple.retention_inactive_warning_lead_time`,
  `ple.retention_archive_notice_lead_time`,
  `ple.retention_archive_after_retention_start`,
  `ple.retention_delete_after_archive`) read by `ple_data.retention_schedule()`,
  with the previous 14/70/100/265-day defaults. Callers no longer select a
  named policy row. Table grants and RLS for that object are gone.

- Identity PKs that were bigint IDENTITY, boolean, or a Blueprint
  reference-number PK are now `uuid` (or `text` for the singleton retention
  policy). The SQLx ledger in `ple_migration` stays outside `rule_08`.
  Nullable columns have `COMMENT ON COLUMN` stating that NULL means the
  optional fact is absent. Table-level `CHECK` clauses after `--` comments
  are no longer parsed as a column named CHECK. Gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  exits 0 with only advisory `rule_11` and `rule_14`.

- `schema_style/check_schema_style.py` loads `schemas/catalog_snapshot.json`
  when that file exists, so
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  applies snapshot Tier 3 without `-j`. An explicit `-j` path still errors if
  missing.

- M1 types, clocks, and key names: closed vocabularies are PostgreSQL enums in
  `10_types.sql` (snake_case labels); `text CHECK (IN ...)` columns now use
  those types; every table has a creation clock and current-state/aggregate
  tables have `updated_at`/`updated_on`; single-column FKs name their parent
  table (`course_instance_id`, `published_question_id`, `object_record_id`)
  per DATABASE_STYLE.md. JSON emitted by SQL stays snake_case
  (NAMING_CONVENTIONS.md). Assigned-Instructor Course columns are gone;
  Instructor membership is the Course teaching team. `course_theme` is a
  vocabulary table. Disposable `postgres:17` `install.sql` completes.
  Style gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py -j schemas/catalog_snapshot.json`.
  Public-ID primary keys (WP-1.4) and Rust mappings remain.

- M0 install census (disposable `postgres:17`, bootstrap through
  `migration_principal_bootstrap_sql`, then `psql --single-transaction -f
  install.sql` as `ple_migrator`): 146 tables / 281 FKs / 497 CHECKs / 282
  indexes / 290 policies / 599 routines / 158 triggers, matching the
  2026-09-17 audit. `content_vocabulary.sql` loaded. `live_demo.sql` still
  requires publisher `psql` variables (`pilot_question_publications` and
  Blueprint references) from `cargo tools installation-data apply`. CHECK
  helpers live in `15_table_check_functions.sql`; default REFERENCES/EXECUTE
  grants let later layers create cross-schema FKs and triggers. Gate log:
  `{SCRATCH}/install_seed.log`.

- M0 WP-0.1 to WP-0.3: moved `schemas/base_schema/` into the DATABASE_STYLE.md
  layered layout with `git mv` for every mixed module that survived as one
  destination (`00_roles.sql`, `10_types.sql`, `20_tables/<aggregate>.sql`,
  `30_constraints.sql`, `40_indexes.sql`, `50_functions/<domain>.sql`,
  `60_policies/` plus a short include manifest, `70_grants/` plus a short
  include manifest). Every table has a `COMMENT ON TABLE` that begins with
  `role:`. Policies and privileges are split by domain so no layer file
  crosses the 999-line source budget. `rule_layout` now matches GRANT/REVOKE
  as statement keywords, not substrings of `revoked_at`. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py` exits 0 with
  zero blocking findings (M1/M2 rules remain advisory). `provided_avatar_catalog.sql`
  was inlined into `20_tables/profile_media.sql` and removed as a path.

- Added `devel/generate_schema_tables_doc.py` (WP-0.4): reads the schema catalog
  through `schema_style.schema_catalog_lib` from `schemas/base_schema/` or
  `-d`/`--database`, and writes [SCHEMA_TABLES.md](SCHEMA_TABLES.md) plus
  `catalog_snapshot.json`. One
  Markdown section per `20_tables/*.sql` file when that directory exists,
  otherwise one section per source file that contains `CREATE TABLE`. Each table
  lists qualified name, role tag, columns, constraints, foreign keys, indexes,
  and catalog comments. Command:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py`. Documented
  in [USAGE.md](USAGE.md).

- Added the SQL base schema restructure plan
  ([sql_schema_restructure_plan.md](archive/sql_schema_restructure_plan.md)) in the
  `blueprint-plan-drafter` multi-workstream form: five milestones (M0 layered layout and catalog
  comments, M1 types and identity, M2 snapshots, M3 derived data and fan-out, M4 measured indexes
  and generic immutability guards), eight workstreams with parallel-readiness stated per
  milestone, 28 one-owner work packages numbered in dependency order, six resolved decisions so execution needs no further
  human input (feedback rules frozen in the Attempt snapshot, the public ID as primary key
  wherever one exists, Edit Numbers replace `metadata_etag`, responses finalized in place,
  iMathAS removed, course themes stay a reference table), gates, risk register, and patch plan. The audit now holds
  evidence only and points to the plan.

- Renumbered the `DATABASE_STYLE.md` checklist to a straight 1-18 (the inserted `6b` and `9b`
  became 7 and 11) and the restructure plan's M2 work packages to dependency order (partition
  keys are WP-2.3, the probe WP-2.4); every cross-reference in the style doc, both plans, the
  audit, and this changelog uses the new numbers.
- Added `devel/markdown_section_sizes.py`: lists a Markdown file's headings with line numbers
  sorted by the count of `- ` bullets directly under each (`-n` top N, `-m` minimum), so
  oversized Human Guidance sections are easy to find. First run on `HUMAN_GUIDANCE.md`: 116
  headings, 1,118 bullets; largest are Content classification (36), Search Question Library
  interface (29), and Agent working principles (23). Documented in `USAGE.md`.
- Split three oversized Human Guidance sections with every bullet preserved: "Human-facing
  reference IDs" (48) into the parent plus alphabet and canonical form, checksum, formats by
  object, and database keys and clocks; "Content classification" (36) into the hierarchy plus
  vocabulary management, selection and discovery, and tags and names; "Search Question Library
  interface" (29) into interface, filters, and syntax siblings. Human Guidance now states the
  under-25-bullets rule per section with the tool that checks it; no section exceeds it. The
  pre-production bullets were reduced to one "no legacy" and one "fix the foundation" bullet.
- Added a separate step-list plan for the schema style checker
  ([schema_style_checker_plan.md](active_plans/active/schema_style_checker_plan.md)):
  `devel/schema_catalog_lib.py` (one parser and model shared with the doc generator) and
  `devel/check_schema_style.py` (one `rule_<id>` function per mechanical checklist item, findings
  with locations, summary per rule, exit code, `--snapshot` / `--database` / `--report`). Tier 1
  rules run on the current source today and must reproduce the audit's counts (72 / ~110 / 17 /
  43 / 166); Tier 2 activates with role tags, Tier 3 with the catalog. The restructure plan's
  WP-0.5 now references this plan instead of restating it.
- Shipped the schema style checker as repo-root package `schema_style/` (`check_schema_style.py`
  orchestrator, `schema_style_rules.py`, `schema_catalog_lib.py` plus parse/scan/database
  modules). `devel/` is a support location, not an import package. Command:
  `source source_me.sh && python3 schema_style/check_schema_style.py`. Each finding line is
  `rule_<id>`, location, message, and source `file:line`. `-q`/`--quiet` prints per-rule
  counts only. CLI flags are `-s`/`--source-dir`, `-j`/`--snapshot`, `-d`/`--database`,
  `-r`/`--report`, and `-q`/`--quiet`; the findings file path is hardcoded to
  `output/schema_style_findings.txt`. First source run exits 1 with 612 findings in 7 rules.
  Versus the 2026-09-17 audit (72 / ~110 / 17 / 43 / 166): `rule_7_key_names` 74
  single-column FK suffix misses (audit 72 of 180; parser sees 177); `rule_4_types` 104
  (audit ~110); `rule_2_constant_columns` 17; `rule_16_clock_present` 44 (audit 43; extra
  `ple_private.object_storage_check`); `rule_14_unindexed_fk` 165 advisory (audit 166);
  `rule_5_duplicate_literal_sets` 59; `rule_layout` 149 advisory until `20_tables/` exists.
  A CREATE TABLE parse/count mismatch is blocking (`rule_table_count`). Tier 2 skipped
  (no role tags); Tier 3 skipped (source-only). `--snapshot` on a missing path raises
  FileNotFoundError.

### Behavior or Interface Changes

- WP-0.5: `schema_style/check_schema_style.py` keeps every rule implementation and
  treats M1 rules as advisory until M1 (`rule_7_key_names`, `rule_4_types`,
  `rule_2_constant_columns`, `rule_5_duplicate_literal_sets`,
  `rule_16_clock_present`, `rule_16_updated_clock`, `rule_16_clock_type`), M2
  `rule_11_student_work_keys` as advisory until M2, and `rule_14_unindexed_fk` as
  advisory. `rule_layout` stays advisory until `20_tables/` exists, then blocks.
  `rule_17_role_tag` blocks once any table carries a role tag. `table count`
  stays blocking. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py`.
- `schema_style/check_schema_style.py` default stdout is per-rule counts only. `-v`/`--verbose`
  prints one `rule_<id>` finding line per violation. The full list is always written to
  `output/schema_style_findings.txt`. Flags are `-s`/`--source-dir`, `-j`/`--snapshot`,
  `-d`/`--database`, `-r`/`--report`, and `-v`/`--verbose`.
- Schema style default summaries are `count`, tab, `rule_##_title` with two-digit
  numbers so they align and sort (` 17\trule_02_constant_columns` before
  `166\trule_14_unindexed_fk`). Verbose finding lines use the same padded ids.

- Question Pools are now current state with an Edit Number, not a Revision family. Student Work
  already pins the exact Published Question Revision served from a Pool, so a Pool Revision added
  nothing that grading, history, or Unrelease needs; Assessments own forked Pools as current
  state; Blueprint Revisions embed their Pool member lists; statistics and Bloom key by Pool.
  For every Pool-served Question, Student Work pins four values: Published Question ID, its
  Revision Number, Question Pool ID, and the Pool's Edit Number at selection (evidence, not an
  FK). Human Guidance (revision specifications, Question Pool specifications, usage statistics,
  Bloom metadata, Student Work), the FERPA policy, `DATABASE_STYLE.md` role and identity tables,
  and plan decision 7 with WP-2.5 (drop `question_pool_revision`, one `save_question_pool_members`
  with CAS and canonical no-op) record it. Patches renumbered 1-10+.
- Human Guidance "Human-facing reference IDs" now states one identity rule instead of "may":
  an object with a public ID uses that public ID as its primary key and sole foreign-key target;
  objects without one use a native UUID or a composite natural key; no table carries an
  auto-increment integer key; every table has one creation clock (`timestamptz` for enforced,
  ordered, or audited rows, `date` for authored content); table shape follows
  `DATABASE_STYLE.md`. The pre-production rule now also says to use that state to improve
  foundational schemas, contracts, abstractions, and ownership boundaries. `DATABASE_STYLE.md`
  "Identity" and checklist item 7, plan decision 2 and WP-1.4, and audit finding 2.1 record the
  same decision: Question Pools, Course Instances, Assessments, Accounts, and Blueprint Courses
  re-key to their public IDs and every `reference_number` column is dropped.

- `DATABASE_STYLE.md` gained "Partition readiness": at 50 Courses x 100 Students x 20
  Assessments x 25 Questions x 5 Attempts, Student Work tables grow ~12.5M rows per semester,
  held to a 25-40M steady state by the 365-day FERPA purge, so partitioning is unlikely; every
  Student Work table nonetheless carries `course_instance_id`, binds it in its parent FK, and
  leads every PK and UNIQUE with it, because the per-Course purge benefits now and a future
  `PARTITION BY LIST (course_instance_id)` then attaches without a re-key. Nothing is partitioned before a
  measured trigger (100M rows, per-Course purge in seconds, or autovacuum lag). Checklist item 11
  and plan WP-2.3 carry it; row estimates use five Attempts per Student.

### Fixes and Maintenance

- Connected PostgreSQL fixture SQL in
  `crates/learning-data-access/tests` now mints public IDs (`U`/`CI`/`A`/`BP`
  via `RETURNING`) and uses the restructured columns:
  `account_id` text, `course_instance_id`, `blueprint_course_id`,
  `assessment_policy_snapshot_id`, `assessment_entry_question`,
  `published_question_id`, `content_discipline_id`,
  `course_membership_id`. Assessment Access keeps
  `student_record_id` uuid `0xeb02` and `assessment_entry_id` uuid
  `0xed02`. Gate: `cargo test -p learning-data-access --features postgres
  --no-run --offline --locked`.

- Public IDs (`AccountId`, `CourseInstanceId`, `AssessmentId`) wrap `String`
  and are no longer `Copy`. Tests clone reused values at Course Banner saga,
  Course Appearance, and iMathAS Question Backend call sites. Gate:
  `source source_me.sh && cargo check --workspace --all-targets --locked --offline`.

- Browser TypeScript uses Blueprint/Course Edit Number JSON fields
  (`blueprint_edit_number`, `blueprintEditNumber`, `courseEditNumber`) and
  quoted `"42"` ETags instead of retired `metadataEtag` UUID validators.

- Assessment Attempt identity in Rust, HTTP paths, and JSON is
  `AssessmentAttemptId` (UUID text). Store traits, Postgres binds, path
  parsing, and TypeScript route/JSON parsers no longer use the `R-`
  numeric `AssessmentAttemptReference`. JSON field names stay
  `assessmentAttempt`. Gate:
  `source source_me.sh && cargo test -p question_model student_work::identifiers --offline`
  and `cargo check -p learning-data-access -p server_core --offline`.

- Rewrote `schemas/installation_data/live_demo.sql` and `live_demo_oracle.sql`
  onto public-ID primary keys. Accounts resolve by seed email, the Course by
  short_name `BCHM 301`, the Assessment by title `Chapter 1 Pilot Practice`, and
  `ple.installation_live_demo_blueprint_public_reference` is the Blueprint
  Course ID. Course inserts set `created_at`, `active_until_at` (UTC plus six
  months), and `retention_starts_at`. Instructor teaching-team membership
  replaces assigned-instructor columns. Internal membership, invitation,
  student-record, workspace, and event keys stay uuid. Domain placeholders
  `CI0000000Y` and `A0000000A` satisfy the public-ID CHECK before
  `assign_human_reference` mints the row.

- Restored `schema_style` M1 rules as blocking (`rule_7_key_names`, `rule_4_types`,
  `rule_2_constant_columns`, `rule_5_duplicate_literal_sets`, `rule_16_clock_present`).
  Only `rule_14_unindexed_fk` and `rule_layout` (until `20_tables/` exists) are advisory.
  Default CLI prints `rule_<id>` finding lines and exits 1. Current source: 529 findings
  in 8 rules, `rule_7_key_names` 75 (audit 72 of 180).

- Settled the Question usage statistic in one dedicated Human Guidance section ("Question"
  Library object usage statistics"), mirrored in `FERPA_DATA_POLICY.md` and `DATABASE_STYLE.md`: per Published
  Question Revision, counters for issued, blank, answered, correct, partial, and incorrect plus
  credit sum and sum of squares (mean and standard deviation for bulk sorting); every Attempt
  counts, including practice; blank is tracked separately from incorrect and never reaches a
  backend; the row holds no Course, Student, Account, Attempt, timestamp, choice, or response
  linkage; the per-observation receipt is purged Student Work; Pool statistics are the sum of
  member Revisions at read time, while each Pool Revision keeps its own issued count and
  per-member selected counts; storage is per Revision, every bulk view displays the
  all-Revision rollup, and the Question detail page alone adds the per-Revision breakdown; the Library shows each rate beside its observation count
  with no exposure threshold. Answer-choice counts and "privacy threshold" wording were removed from
  HG and the FERPA policy. Plan WP-3.8 reshapes `question_revision_statistics` accordingly and
  drops the two choice tables.
- Rewrote `DATABASE_STYLE.md` in positive phrasing throughout: every rule states the action to
  take ("declare every column NOT NULL", "enforce immutability with privileges", "reach a
  parent's value through the foreign key") and lists of things to avoid were removed or folded
  into a "Replaces" column in the type table, following the prompt-positively principle in
  `REPO_STYLE.md`. Checklist questions now read as positive checks ("Is immutability enforced by
  privilege?"). The enum declaration file is `10_types.sql` throughout. No rule changed meaning.
- The clock rule now assigns the type by table role: full-precision `timestamptz` where the
  server enforces, orders, or audits (Student Work, sessions, leases, events, Courses,
  Accounts) and `date` for authored content (Published Questions, Pools, Blueprints, their
  Revisions, Draft Questions, usage statistics), because every `timestamptz` precision stores
  the same 8 bytes and compares at the same speed while `date` is 4 bytes and reads as the day.
  Rows mutated in place carry a second bookkeeping clock (`updated_at`/`updated_on`) of the same
  type; every further clock is a domain fact named for its event and typed by its meaning. The
  statistics aggregate carries `created_on` and `updated_on` (day of last increment), since day
  granularity on a global counter identifies no one. Integer epochs and
  reduced-precision `timestamptz(n)` are outside the rule. Recorded in
  `DATABASE_STYLE.md`, `HUMAN_GUIDANCE.md`, the FERPA policy (statistics carry the publication
  date), and plan WP-1.7.
- `DATABASE_STYLE.md` records the aggregate exception to the clock rule: anonymous statistics
  tables carry `created_on` and `updated_on` as `date`, because an increment `timestamptz` on a
  small-count aggregate is a re-identification path when joined to a roster while a calendar
  date on a global counter is not; FERPA attaches to identifiability, not timestamps. Plan
  WP-1.7 converts `question_revision_statistics.updated_at`.
- Style enforcement is designed as a Python maintainer tool rather than an external linter
  (SQLFluff and Squawk cover formatting and migration safety, schemalint and pgTAP need Node or a
  live database): the M0 generator also emits `schemas/catalog_snapshot.json`, every table
  comment begins with a role tag, and the maintainer tool `devel/check_schema_style.py`
  checks the layout and the mechanical checklist items (key names, types, clocks, Student
  Work keys, comments) against the snapshot or a live database in one command, with
  unindexed purge-path FKs under `--report`; a pytest wrapper is optional and later. schemalint (seven built-ins: casing,
  singular names, text, timestamptz, jsonb, identity, primary key) runs in parallel against the
  disposable Podman database whenever the snapshot is regenerated, as independent confirmation
  rather than a gate; the schema already satisfies all seven. Recorded in `DATABASE_STYLE.md`
  "Organization of the SQL source" and plan WP-0.4 / WP-0.5.
- The schema audit gained finding 2.11: 72 of 180 single-column FK columns are named so the
  parent table is invisible (`course_id` -> `course_instance`, `object_id` -> `object_record`,
  `thread_id`, and so on). `DATABASE_STYLE.md` "Naming" now requires PK `<table>_id` and FK
  `[<role>_]<parent_table>_id` with the full table name, checklist item 7 checks it, and plan
  WP-1.8 renames the columns while API field names stay mapped at the boundary.
- The schema audit gained finding 2.10 on growth outside the FERPA purge: grading and statistics
  receipts cascade with the Work (fine); `authenticated_session` and the other authentication
  tables are revoked but never deleted (unbounded, identifying); `ple_audit.object_delivery_access_event`
  has no writer anywhere and would log every asset load if wired (placeholder, remove); Published
  Questions are authored-scale and stay a global Library, with discovery search, not partitioning,
  as the Question-side concern. Plan WP-3.7 adds the expiry sweeps and drops the placeholder.
- The schema audit gained finding 2.9: 43 of 146 tables have no timestamp column, including the
  current-state `assessment_template` and the Student Work row `issued_question`; plan work
  package WP-1.7 adds the creation instant required by `DATABASE_STYLE.md`.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Developer Tests and Notes

- M3 WP-3.3: `tests/e2e/attempt_expiry_connected_oracle.sql` and
  `tests/e2e/unrelease_connected_oracle.sql` follow the current Student Work
  schema (minted public IDs, policy snapshot, `published_question_id`,
  `delivery_toolchain_id`, in-place saved-response finalization, observation
  capture). Same intent as before: stale snapshot fence, 0.67 credit replay
  1.34 then 2.01/3, attempt limit, landing, expired unanswered 0/3, Unrelease
  reject/accept/repeat, and anonymous stats `issued_count=2`. Not rerun on
  PostgreSQL here. Gate: review of the two oracles against `20_tables/` and
  `50_functions/`.
- WP-0.4/WP-0.5: `source source_me.sh && python3 devel/generate_schema_tables_doc.py`
  exits 0 and writes [SCHEMA_TABLES.md](SCHEMA_TABLES.md) (24 `20_tables/` sections,
  146 tables) and `catalog_snapshot.json`
  (146 tables, 282 indexes, 3 enums, all 146 tables tagged).
  `python3 -m pyflakes devel/generate_schema_tables_doc.py schema_style/*.py` exits 0.
  `source source_me.sh && ./schema_style/check_schema_style.py` exits 0 with
  `529 findings in 8 rules`, all advisory: `rule_11_student_work_keys 15`,
  `rule_14_unindexed_fk 166`, `rule_16_clock_present 44`, `rule_16_updated_clock 49`,
  `rule_2_constant_columns 17`, `rule_4_types 104`,
  `rule_5_duplicate_literal_sets 59`, `rule_7_key_names 75`. `rule_layout` is
  blocking and clean. `--snapshot` on a missing path raises `FileNotFoundError`.

## 2026-09-17

### Additions and New Features

- Added [DATABASE_STYLE.md](DATABASE_STYLE.md), the table-design rule set behind the schema
  audit: philosophy (tables are forever, one fact one place, invalid states unrepresentable, types
  carry meaning, measure before cost), shape rules for current-state/revision/event tables and
  content-addressed snapshots referenced rather than copied, the Human Guidance identity model as
  one convention, a type table (timestamptz, text plus CHECK or domain, enums for closed
  vocabularies, reference tables for retiring ones, numeric, bytea checksums, jsonb only for
  opaque documents), constraint preference order with the role-typed FK idiom kept, FK index and
  naming rules, a mandatory creation instant on every table (no blanket incremental integer key:
  PostgreSQL heaps do not cluster on the primary key), a layered source layout
  (`10_types`, `20_tables/<aggregate>`, `30_constraints`, `40_indexes`, `50_functions`,
  `60_policies`, `70_grants`) with catalog `COMMENT ON` and a generated `docs/SCHEMA_TABLES.md`
  as the audit completion gate, and an 18-question "is my table well designed" checklist with fail
  signals, scored against `assessment_attempt` as a worked example (11 of 18 fail). Sources:
  PostgreSQL 17 docs, Angelakos *PostgreSQL Mistakes*, and the normalization chapter of the local
  corpus. Listed as a durable authority in `AGENTS.md` and linked from `DATABASE_STRUCTURE.md`.

### Behavior or Interface Changes

- Refreshed the complete canonical screenshot corpus (76 captures) from a fresh Live Demo through
  `./devel/capture_screenshots.sh`, including the new Inactive Courses list and complete Student
  coverage of all eight native Question Types plus WeBWorK at laptop and phone widths. The
  `published_question_result` and `invitation_accepted` captures are retired; the invitation
  scenario now captures an invited Student who never joins so it replays on the same stack.
  Static corpus verification and the atlas regeneration passed.
- `./devel/capture_screenshots.sh` now runs against the already-running Live Demo by default
  (`--fresh` stops, starts, and stops an owned stack; `--only=<scenario,...>` stages selected
  scenarios without publishing). `./launchers/run_live_demo.sh` no longer clears a running or
  starting suite; it reports the entry URL, and only `stop` clears. A first start prints a
  30-second heartbeat and keeps supervisor output in `local_stack_state/live_demo_browser/supervisor.log`.
- B3 records source-approved Bloom Question Library discovery: two independent exact filters combine
  with every existing predicate, remain bound through saved searches, URL handoff, and opaque cursor
  continuation, and preserve existing sorts. Server facets describe the whole matching set with all
  six Cognitive Process and four Knowledge Dimension counts, including zeros and empty results.
  Connected multi-page, role, and browser proof remains open, so the Bloom discovery checklist row
  remains open.
- B2 records source-backed exact Question and Pool Revision Bloom correction
  routes. Active vetted Instructors, without an owner restriction, submit the
  complete pair under classification Edit Number CAS; stale `412` precedes
  no-op handling, a changed pair advances once, and no correction creates a
  content Revision. Sysadmins remain read-only. The client reloads the same
  exact Revision, retains the draft, and requires explicit resave without
  automatic retry or merge. Connected two-Instructor, denied-role, and browser
  proof remain open.
- B5 documents the implemented source boundary for Bloom Assessment sorting: fixed Entries project
  their exact pinned Question pair, Pool Entries project their exact Assessment-owned fork pair,
  and the stable order is cognitive process, knowledge dimension, then prior position. Sorting is
  blocked for an unavailable exact pair and remains an ordinary whole-Assessment Save under the
  existing Edit Number CAS. Source implementation is present, but review still awaits connected
  mixed-entry sort/save/reload and concurrent-save proof; Library filters and reporting also remain
  open, so the combined Bloom search-and-sorting checklist row stays open.
- B1 projects each exact Question or Pool Revision's required two-value Bloom
  Classification and independent classification Edit Number through answer-free
  Library reads. It adds exact Pool Revision GET and documents that an exact
  Question-detail response's legacy `latestQuestionRevision` field identifies the
  requested Revision. Source/model/decoder evidence is present; the connected
  actual-role read proof remains open, so this does not close the Bloom milestone.
- Clarified that Bloom Classification supports Question Library search and Assessment item sorting.
  The database stores only the two closed enum dimensions. Publication now requires a private,
  one-use receipt bound to the exact Bloom-relevant candidate content; the browser cannot supply
  either the receipt or classification. Fresh PostgreSQL 17 actual-role proof covered preparation
  privileges, kind/content binding, rollback, one-use consumption, deferred completeness, and
  Question and Pool Library admission. A configured real AI classifier remains required before
  connected publication can pass.
- Added answer-free native response previews after the Published Question prompt for MC, MA,
  FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. The Library descriptor omits author response IDs,
  grading policy, answers, and submission state; controls remain inactive and MATCH displays one
  shared choice bank. WeBWorK keeps its backend-owned preview. Rust model, strict TypeScript,
  focused decoder checks, production client build, and temporary Chromium checks at 390/1280 passed.
  Canonical connected acceptance and the official screenshot refresh remain milestone checks.
- Added the Course tools action to create a new Blueprint from a Course Instance. Its compact accessible dialog pre-fills only new Blueprint short name, long name, and classification; explains copied reusable structure and unchanged source/first Adoption; validates the canonical Course reference; sends only the metadata DTO with retry-stable idempotency; requires `201`/`no-store`; retains values and reports accessibly; returns focus on cancel; and opens the owner-visible Private Revision-1 receipt. `cargo tsgen`, focused Node, TypeScript, ESLint, Prettier, and `./check_codebase.sh` (369 Node tests) passed. The backend atomically derives ordered reusable structure, forks Course-owned Pools, and records immutable source provenance. PostgreSQL 17 actual-role proof passed authorization/no-write, stale rollback, idempotency, source preservation, exact pins, Adoption/student counts, and lifecycle rollback. Canonical HTTPS C420 proof made one child-route POST from `CI0QR41X` to Private Revision-1 `BPJD8H28`, showed Adoption count 1, counted source Students only in the statistic, copied no roster/delivery state, and left the source addressable and unchanged.
- Accepted C522 connected Student delivery and Work issuance through the production
  server/data-access/access-page chain, including denied access. Accepted C879 connected
  Question-fork proof with two Instructors, exact source attribution, private denial,
  retry/concurrency, distinct server-issued identity, and prevalidation-publication denial. A separate
  3-by-3 PostgreSQL proof preserved all three compatible CC source licenses and rejected every
  mismatch through the approved minimal SQL. Temporary probes were removed.
- Implemented the settled Discipline and Library improvement-activity source boundaries. Disciplines
  use stable UUID create/rename/retire/restore with no delete, active-only new choice, retained
  visible references, exact inheritance/copy, and concurrency locks. Vetted-Instructor retained
  threads and owner/Sysadmin Question or Sysadmin-only Pool impact administration are present with
  Sysadmin read-only Library content controls. Final review, major-milestone SQL/browser proof, and
  four-event private Watch delivery remain open.
- Accepted I08's bounded 33-line, three-file Question Library correction: explicit Subject loading/ready/empty/error states explain settled zero Subjects and route discovery to Tags, Question Types, and Search while preserving ordinary facets, cascade, keyboard use, and detail return. The compact top-aligned `12rem` scroll region passed temporary actual-component Chromium at 1280/390 without overflow; the harness was removed. Strict TypeScript, 51 focused Library/Ribbon Node tests, ESLint, scoped Prettier/diff checks, and independent review passed. Official screenshots and authenticated connected Library acceptance remain deferred; no global density-row or SQL-lock claim follows.

### Fixes and Maintenance

- Live Demo start is now visible and progress-based instead of a silent 600s deadline. Root
  causes: `start_stack` ran the launch child with `capture_output=True`, so
  `local_stack_state/live_demo_browser/supervisor.log` stayed empty for the whole build while the
  heartbeat pointed at it; the fixed `DEVELOPER_START_WAIT_SECONDS = 600` covered a cold
  Podman-machine build that measured over ten minutes, and the resulting SIGTERM only set a flag,
  so the build ran to completion and was then purged. Now the launch child's output streams into
  the supervisor log line by line (`stream_launch`), the supervisor writes `[phase]` markers and
  `lifecycle` writes `Step:` lines before every captured Compose, host-build, wait, migration, and
  provisioning step, the CLI heartbeat (every 15s) prints elapsed time, phase, and the last log
  line plus a `tail -f` hint, and the parent keeps waiting while the log keeps growing, giving up
  only after 300s of silence (`stalled during <phase>`), a 3600s ceiling, a child exit, or Ctrl-C.
  Termination now forwards SIGTERM to the launch child's process group so a stall, ceiling, or
  Ctrl-C stops the build within seconds before the suite reset; `start` also probes `podman info`
  first so a stopped engine fails in seconds with `podman machine start` named. The parent-side
  wait moved to `local_stack_control/browser_suite_developer_start.py` to keep the supervisor
  module under the 1000-line gate.
- Launch failures now keep their evidence. The first streamed run showed the earlier 16:32 start
  had not timed out at all: it failed at "waiting for the complete stack to report ready" with the
  message `PostgreSQL is starting`, which `unavailable_report` emitted for any failed gateway
  health probe, and the purge then destroyed the stack before anyone could look. The probe now
  reports `gateway health probe at <url> failed: <curl detail>`; the launch child prints redacted
  `compose ps` and 60-line service log tails to the supervisor log before the purge
  (`print_launch_failure_evidence`); `_launch_diagnostic` prefers the child's own `ERROR:` line,
  so the operator message no longer shows cpanm's harmless `GD` bail-out (apt `libgd-perl` serves
  the renderer at runtime) with the real error truncated off the end.
- Controller curl probes pin `--ipv4`. The second streamed run's evidence showed every service
  healthy while `curl https://localhost:<port>/health` reported `Recv failure: Connection reset
  by peer`; a bare Caddy container reproduced it: `http://127.0.0.1:<port>/` answered 200 and
  `http://localhost:<port>/` was reset because `localhost` resolves to `::1` first and the Podman
  machine forwarder resets IPv6 loopback instead of refusing, so curl never falls back to IPv4.
  Browser URLs keep `https://localhost:` for the internal certificate; browsers fall back on
  their own.
- Found the actual cause of today's unreachable gateway: Homebrew upgraded Podman 6.1.1 -> 6.1.2
  at 16:07 while the machine VM and its `gvproxy` forwarder kept running the deleted 6.1.1
  binaries. With that mismatch a container on two networks (the gateway sits on `gateway_api`
  plus the browser overlay's `default`) publishes a port that the host cannot reach, while a
  single-network container works; the real gateway image with the real Caddyfile reproduced both
  outcomes in isolation. `podman machine stop; podman machine start` fixed it: the two-network
  probe answered 200 afterwards. A client/server version gate was tried and removed the same
  hour: the machine image keeps its own server version (still 6.1.1 after the restart), so the
  mismatch is normal on macOS and would block every post-upgrade start. The preflight stays a
  plain reachability check.
- The machine restart was not the fix either; the live stack still reset every host connection
  while `podman exec` inside the gateway answered 200 and even the VM could not reach the
  container on either IP. The gateway's two networks (`gateway_api`, browser `default`) are both
  `internal: true`, and this netavark drops host-side port forwarding into a container whose
  every network is internal; a standalone gateway on one `--internal` network reproduced the
  reset, on a normal network it answered 200. `containers/compose.yaml` now attaches the gateway
  to a non-internal `gateway_edge` network for its published port, and the port binds on all
  host interfaces (the operator wants Tailscale access) instead of `127.0.0.1` only. Firefox's
  `PR_END_OF_FILE_ERROR` on the same URL was this reset, not a certificate problem. Verified
  live: `/` and `/health` answer 200 on loopback and the port is open on the Tailscale address;
  over Tailscale Caddy still answers with a TLS alert because the site block is
  `https://localhost:8080`. Deferred by the operator; the one-line change is `https://:8080`.
- Removed every `from __future__ import annotations` (16 files); Python 3.12 evaluates the same
  annotations natively. `tests/test_no_future_imports.py` keeps them out.
- Repaired drift that blocked the Live Demo and screenshot replay: the base schema still granted
  the pre-Bloom-receipt `publish_question_revision` signature (psql exit 3 on install); the Live
  Demo seeder rejected the Course summary's new `lifecycleState` field; the Question Library
  decoder refused `bloom: null` ("The library could not load"); `build.sh` now regenerates the
  ignored `generated/api/QuestionIdSyntaxContract.ts` instead of only checking it. Screenshot
  scenarios follow the current UI (Create Course Instance and Create a Template disclosures,
  Practice Question Assignment labels, Course-name invitation heading, gradebook roster cells)
  and stop asserting Student completion state or byte-level details the captures do not depend on.
- Reconciled the SQL Human Guidance audit and generator-owned checklist parts with the current
  Closed SQL ledger. Bloom preparation/admission, Question/Pool Watches and notifications,
  Blueprint Promoted and Change Proposal persistence, and retained lifetime totals no longer appear
  as missing SQL mechanisms. Their application, browser, worker, and connected proof remains open.
- Removed the accidental native Ollama dependency from the PLE product runtime,
  Compose environment, installation guidance, and Live Demo contract. PLE now
  composes ordinary non-publication service paths without an AI backend. The
  provider-neutral Bloom preparation boundary remains unconfigured, and the
  SQL/data-access prepared-receipt dependency remains an explicit follow-up
  before new Question or Pool publication can complete. Optional developer-side
  Graphify model support is separate and unchanged.
- Threaded trusted, one-use Bloom preparation receipts through Question Pool creation,
  Assessment Pool import/append, Blueprint Pool materialization, Course adoption, source updates,
  and daughter-Course propagation. Browser DTOs remain unchanged; missing prepared receipts stop
  Pool publication. PostgreSQL-feature compilation, focused receipt consumption, and data-access
  Clippy passed. AI preparation orchestration and connected publication acceptance remain open.
- Replaced stale short Blueprint, Course Instance, and Assessment reference
  validators with one canonical SQL predicate. It accepts only the exact
  seven-random-character, checksum-bearing `BP`, `CI`, `A`, and `U` forms.
  A disposable PostgreSQL 17 base-schema install confirmed canonical acceptance
  and rejection of legacy-short, lowercase, wrong-prefix, and bad-checksum IDs.
- Removed the watered-down BIOL 301 RNA/DNA Question variant from the active Fall 2026 Genetics
  pilot selection. The upstream BiologyProblems.org source and its source mirror remain intact;
  the nonpublished import inventory records the explicit exclusion. Course-fixture labels now use
  actual Fall 2026 pilot Courses.
- Updated inline Rust fixtures that still used pre-cutover short public-ID values or unhyphenated
  Question IDs to canonical checksum-bearing values.
- Regenerated the two chromosome-shape PGML delivery copies with their dedicated matching and
  which-one generators after the shared colored-span spacing repair. The local manifest hashes now
  identify regenerated bytes; immutable committed upstream URL/hash pins remain unchanged.

### Decisions and Failures

- Added the read-only SQL schema quality and efficiency audit
  ([sql_schema_quality_audit.md](active_plans/audits/sql_schema_quality_audit.md)) over the
  146-table base schema, installed into a disposable PostgreSQL 17 container for catalog evidence
  (281 FKs, 166 without a referencing-side index; 497 CHECKs; 3 native enums against ~110
  text-plus-CHECK closed vocabularies; 92 hand-listed immutability comparisons). It confirms and
  extends the human's repetition note: the Assessment Attempt copies the whole current Assessment
  including a 50 KB instruction body and nine policy strings; Question Attempts repeat a
  seven-string toolchain per Question; `question_attempt_state`, `finalization_kind`, and the
  `question_response_grading` wrapper are derived; the finalized response JSON is stored twice;
  Library Watch notifications store each event three times; 17 constant columns exist, of which
  the role-typed ones are legitimate composite-FK carriers and should become an enum rather than
  be removed. It also records Human Guidance misalignments (dead `closed`/`archived` Assessment
  states, a privileged `assigned_instructor` Course column, `metadata_etag` beside Edit Numbers,
  installed iMathAS scaffolding, four aggregate identity conventions) and proposes six bounded fix
  packages with a target snapshot structure. A source census (39 table-bearing files, 36 of them
  also holding functions; 57% of lines in function bodies; 32 catalog comments for 146 tables;
  Student Work tables split across four files) adds package 0, a layered layout with catalog
  comments and a generated `docs/SCHEMA_TABLES.md`, as the audit's completion gate. Verdict:
  readiness not established for a freeze; no code changed.
- Added the read-only UI density and layout audit
  ([ui_density_and_layout_audit.md](active_plans/audits/ui_density_and_layout_audit.md)) over the
  refreshed 76-capture screenshot corpus. It names four shared root causes behind the recurring
  per-page findings: a 190-260 px header stack on every page, object cards for every collection
  (`.instructor-list__row` 4.25rem floor, `.course-card` 5.75rem), stacked label/value grids that
  wrap values under labels (plus `h1 { max-width: 28ch }`), and status banners inserted in flow
  above the controls that produced them so saves and validation move the buttons. It specifies one
  shared `.ple-table` spreadsheet idiom with per-collection columns, seven fix packages (header
  budget, table migration, reserved status region, inline label/value pairs, prose cuts, Library
  entry, Student Coursework rows, Student attempt chrome and backend frame sizing), 34 per-surface
  findings with a coverage ledger over all 76 captures, and Playwright oracles for row count,
  layout shift, and value wrap. No code changed.
- The pre-build image prune is now `podman image prune -f` (dangling layers only), not `-a`.
  The `-a` form deleted every image without a running container before each start: the reviewed
  WeBWorK renderer (about eight minutes to rebuild on this Podman machine), the pulled postgres
  and minio images, and any of the operator's unrelated tagged images whose containers happened
  to be stopped. Every start was therefore a cold start, and the second streamed run today spent
  its first nine minutes rebuilding an image the first run had already built. Stale layers from
  rebuilds still go; tagged images are kept because they are either expensive inputs or not this
  controller's to remove.
- Removed the unreferenced Question-ID reinitialization preflight script. PLE is pre-production,
  and the canonical public-ID cutover changes the authoritative base schema directly rather than
  preserving a second migration or data-rewrite path.
- Clarified the universal canonical public-ID invariant: exact canonical values persist unchanged
  across all boundaries, use public SHA-256 checksum characters, reserve globally, and are never
  reused. The closed human-entry forms normalize before canonical syntax/checksum validation;
  stored, transmitted, displayed, copied, and generated values stay canonical. Implementation
  inventory and the separately planned cutover remain open, so this documentation change claims no
  production behavior.
- Clarified that internal non-user-facing objects retain native UUIDs and public references exist only
  for required human-facing workflows. Existing `R`, `W`, `D`, `M`, and `I` short-reference concepts
  are implementation drift and must be removed rather than replaced.
- Added a Human Guidance deferred-product section that overrides implementation language elsewhere.
  iMathAS and H5P remain desired but deferred secondary backends. Future H5P use is limited to
  Regular Assignments, Bonus Assignments, and Practice Question Assignments and is excluded from
  Quizzes and Exams because its runtime exposes answers and correctness to the Student browser.
- Reconciled the Human Guidance plan and audit ledgers with current evidence: current checklist
  accounting is 1,018 bullets (472 verified, 498 open, 48 N/A); Deferred product behavior is
  authority but excluded from current implementation counting; only the two HG-unlocked complete
  Ribbon layouts may remain unresolved at closeout; closed SQL rows are distinct from pending
  application acceptance; high-priority UI findings now state their current rendered disposition;
  and Course Instance-to-Blueprint documentation uses Create Blueprint from Course Instance with a
  new Private Revision 1, immutable source provenance, first Adoption, and an unchanged source Course.
  The plan also records completed removal of dormant H5P placeholders. Current production Backends
  are PLE and WeBWorK; desired iMathAS/H5P behavior is deferred and implies no current backend work.
- Recorded tests as liabilities as well as assets: bounded work runs narrow gates and removes
  temporary probes; full Podman and `source source_me.sh && ./launchers/all_test.sh` acceptance is
  reserved for major milestones.
- Recorded C528 as open and decision-blocked: text-entry completeness needs raw-versus-trimmed-nonempty-versus-optional semantics, and opaque backend capture needs a product choice between successful capture and an adapter-supplied trusted completeness verdict. No implementation, test, or checklist completion is claimed.
