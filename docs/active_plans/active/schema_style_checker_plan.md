# Plan: schema style checker

A Python maintainer tool, `devel/check_schema_style.py`, that reports every mechanical rule in
[docs/DATABASE_STYLE.md](../../DATABASE_STYLE.md) against the SQL source today and against the
installed catalog once [sql_schema_restructure_plan.md](sql_schema_restructure_plan.md) lands its
layout. The two plans are separate: this one ships a tool; that one changes tables. The tool
lands first so every restructure patch watches its finding counts fall.

Status: not started. Opened 2026-09-18.

## Context

The schema audit ([sql_schema_quality_audit.md](../audits/sql_schema_quality_audit.md)) measured
its findings with a throwaway parser in a scratch directory: 72 FK columns that hide their parent
table, ~110 `text` columns with literal `IN (...)` CHECKs, 17 constant columns, 43 tables with no
clock, 166 FK edges with no referencing-side index. Those numbers are the restructure plan's
progress meter, and nothing in the repository can reproduce them. Off-the-shelf linters (SQLFluff,
Squawk, schemalint, pgTAP) cover formatting, migration safety, or seven generic rules; none knows
this repository's naming, clock, or key rules. The human's preference is a `devel/` Python tool
run directly, with a fast turnaround, ahead of any pytest wrapper.

## Objectives

- One command prints every mechanical style finding with a location and exits non-zero when any
  blocking rule fires.
- The command works today on the current `schemas/base_schema/` source, before the layout moves.
- The same command reads `schemas/catalog_snapshot.json` or a live database once those exist,
  so catalog-only rules join without a second tool.
- Rules are one function each, named for the checklist item they enforce, so a reader maps
  output to `docs/DATABASE_STYLE.md` without a lookup table.

## Design philosophy

Ship the rules that a regex over `CREATE TABLE` text can decide, then add rules as their inputs
appear (role tags, then the catalog). A tool that reports 72 findings on day one and 0 after M1
is worth more than a complete tool that arrives after the tables have moved. This is **Perfect is
the enemy of good** and **Use the scientific method** from `docs/REPO_STYLE.md`: the counts are
the measurement.

- Evidence strategy for uncertain methods: each rule's first run is compared with the audit's
  hand-verified counts; a rule that disagrees is fixed before it blocks anything.

## Scope

- Write `devel/schema_catalog_lib.py`: parse `CREATE TABLE`, `CREATE TYPE ... AS ENUM`,
  `CREATE DOMAIN`, `CREATE INDEX`, `ALTER TABLE ... ADD FOREIGN KEY`, and `COMMENT ON TABLE`
  from the SQL source into one plain-dict model; load the same model from
  `schemas/catalog_snapshot.json`; load it from a live database through `psql` in a Podman
  container when `--database` is given.
- Write `devel/check_schema_style.py`: run every rule over the model, print findings, exit code.
- Implement the Tier 1 rules now, Tier 2 rules gated on role tags, Tier 3 rules gated on catalog
  input.
- Document usage in `docs/USAGE.md` and the tool's place in `docs/DATABASE_STYLE.md`.

## Non-goals

- Move, rename, or edit any table; the restructure plan owns schema changes.
- Add a pytest wrapper; that is a later two-line call once the rules are stable.
- Replace `devel/generate_schema_tables_doc.py`; the generator reuses the library and lands with
  the restructure plan's M0.
- Judge the human-only checklist items (1, 2, 3, 10, 15, 18).

## Approach

1. Move the scratch parser into `devel/schema_catalog_lib.py` with a stable model:
   `{"tables": {qualified_name: {"file", "line", "columns": [{name, type, not_null, check}],
   "primary_key": [...], "uniques": [[...]], "foreign_keys": [{columns, parent, parent_columns,
   file, line}], "comment", "role"}}, "enums": {...}, "domains": {...}, "indexes": [...]}`.
   Comment-strip, top-level-comma split, and `ALTER TABLE` FK capture carry over from the
   audit's parser; add `COMMENT ON TABLE` capture and a `role` field parsed from a comment that
   begins with `role:`.
2. Write `check_schema_style.py` with `parse_args` (`-s/--source-dir` default
   `schemas/base_schema`, `-j/--snapshot` path, `-d/--database` name, `-r/--report` for advisory
   rules, `-q/--quiet` summary only), `main`, and one `rule_<id>` function per rule returning a
   list of `Finding(rule, location, message)`.
3. Tier 1 rules (source or snapshot):
   - `rule_layout`: `CREATE TABLE` outside `20_tables/`; `CREATE FUNCTION`, `CREATE POLICY`,
     `GRANT`, `REVOKE` inside a `20_tables/` file; enum or domain outside `10_types.sql`. Reports
     only until the `20_tables/` directory exists, then blocks.
   - `rule_7_key_names`: every single-column FK column ends with `<parent_table>_id`; every
     column of a composite FK is either a public/uuid key named for its parent or one of
     `revision_number`, `*_position`, `edit_number`; every PK column follows the same rule.
   - `rule_4_types`: `timestamp` without zone, `timestamptz(n)`, `varchar(`, `char(`, `serial`,
     `bigserial`, `money`, `json ` (not `jsonb`); `text` column whose CHECK is a literal
     `IN (...)` list.
   - `rule_2_constant_columns`: CHECK admitting exactly one literal.
   - `rule_5_duplicate_literal_sets`: identical `IN (...)` sets on two or more columns.
   - `rule_16_clock_present`: table without a `timestamptz` or `date` column.
   - `rule_14_unindexed_fk` (advisory): FK edge with no index or PK/UNIQUE whose leading columns
     match.
4. Tier 2 rules (need role tags; skipped with a note when no table carries one):
   - `rule_17_role_tag`: every table's comment begins with one of `current state`, `revision`,
     `event`, `snapshot`, `student work`, `aggregate`, `vocabulary`.
   - `rule_16_updated_clock`: `current state` and `aggregate` tables carry `updated_at` or
     `updated_on` of the same type as their creation clock.
   - `rule_16_clock_type`: `revision`, `aggregate`, and lineage `current state` tables tagged
     `authored` use `date`; every other role uses `timestamptz`.
   - `rule_11_student_work_keys`: `student work` tables carry `course_instance_id NOT NULL` and
     lead every PK and UNIQUE with it.
5. Tier 3 rules (need the snapshot or a live database; skipped on source input):
   - `rule_8_identity`: public-ID tables have their public-ID domain as PK; internal aggregates
     have a `uuid` PK; owned children have a composite natural key; no `GENERATED ... AS
     IDENTITY` column is an FK target.
   - `rule_12_immutability`: no trigger function body contains `IS DISTINCT FROM OLD.`; tables
     tagged `revision`, `event`, `snapshot` have no `UPDATE` grant to a runtime role.
   - `rule_9_null_meaning`: every nullable column is named in a CHECK or has a column comment.
6. Output: one line per finding `rule_7  ple_data.assessment.course_id  ends with course_id;
   parent is course_instance` (source findings append `assessments.sql:9`), then a summary line
   per rule with its count, then `clean` or `N findings in M rules`. Exit 1 on any blocking
   finding, 0 otherwise; `--report` includes advisory rules in the listing without affecting the
   exit code.
7. First run on the current source: compare each rule's count with the audit (72 / ~110 / 17 /
   43 / 166) and reconcile any difference before the tool blocks anything.
8. Wire the restructure plan: its WP-0.5 becomes "extend the checker with Tier 2 and the
   snapshot input"; each restructure patch pastes the summary line into its changelog entry.

## Files to modify

- `devel/schema_catalog_lib.py` (new): SQL parser, snapshot loader, live-catalog loader, model.
- `devel/check_schema_style.py` (new, executable, shebang): rules, output, exit code.
- `docs/USAGE.md`: one section with the command, flags, output shape, and exit codes.
- `docs/DATABASE_STYLE.md`: "Organization of the SQL source" names the tool (already does) and
  each checklist row gains its `rule_<id>` name in the "Fix" column where a rule exists.
- `docs/active_plans/active/sql_schema_restructure_plan.md`: WP-0.5 references this plan.
- `docs/CHANGELOG.md`.

## Verification

- `source source_me.sh && python3 devel/check_schema_style.py` on the current source prints the
  audit's counts within a stated tolerance (exact for 7, 2, 16; +-5 for 4 and 14, whose audit
  numbers came from a looser regex) and exits 1.
- `python3 devel/check_schema_style.py --report --quiet` prints only summary lines.
- A scratch copy of `schemas/base_schema/` with one deliberate violation per rule (a
  `varchar(10)`, a `course_id` FK, a table with no clock) produces exactly one finding per rule;
  the scratch copy is deleted afterwards.
- `pyflakes` clean; `tests/test_function_typing.py`, `tests/test_shebangs.py`, and
  `tests/test_source_file_line_limit.py` pass; both files stay under 1000 lines (split the rule
  module if the Tier 3 rules push it over).
- After M0 of the restructure plan: `--snapshot schemas/catalog_snapshot.json` and
  `--database <name>` against the disposable container report the same findings as the source
  run.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Regex parser misses a DDL shape | false clean | a table or FK absent from the model | tooling coder | the model's table count is printed and compared with `grep -c 'CREATE TABLE'`; a mismatch is a finding |
| Rule disagrees with the audit count | trust lost on first run | count differs beyond tolerance | tooling coder | reconcile before the rule blocks; record the reason in the changelog |
| Tool blocks before the schema can comply | restructure patches stall | Tier 2 rule fires with no tags | tooling coder | Tier 2 and 3 rules skip with a note until their inputs exist; Tier 1 layout rule reports until `20_tables/` exists |
| Two parsers drift (checker vs generator) | inconsistent docs and findings | generator lands with its own reader | tooling coder | one library, `devel/schema_catalog_lib.py`, imported by both |

## Open questions and decisions needed

- Non-blocking follow-up: whether the Tier 3 immutability rule should read function bodies from
  the source (`$$ ... $$` blocks, available today) rather than waiting for the catalog; source is
  simpler and would move it to Tier 1.
- Non-blocking follow-up: pytest wrapper, once the rules have been stable for one milestone.
