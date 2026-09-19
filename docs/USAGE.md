# Usage

PLE has a disposable local stack for development and a small set of canonical
database administration commands. The default local stack provisions ordinary
installation-data Live Demo records; it is not a mock or a separate schema.

## Start the Live Demo

From the repository root, start the normal local developer entry point:

```bash
./launchers/run_live_demo.sh
```

The launcher installs missing TypeScript dependencies through its existing helper,
builds the production `dist/` bundle, starts the fixed HTTPS stack, and prints the
Live Demo entry URL. New launches choose an HTTPS gateway port in the 8000-8399
range. Open an already-running demo, or start and open a fresh one:

```bash
./launchers/run_live_demo.sh open
./launchers/run_live_demo.sh start --open
```

Start and open always print the URL before attempting to open a browser, including
when the browser opener fails. Use `--headless` when the command must not open a browser. Each start replaces the
fixed disposable stack and its previous local data; unrelated Podman projects remain
outside this lifecycle. Stop it through the same owner:

```bash
./launchers/run_live_demo.sh stop
```

The local identity selector replaces only identity verification. The server still
derives each authenticated session, Product Role, and Course relationship from stored
PLE records. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the ordinary teaching
graph and its data boundaries.

## Work with Blueprint Courses

Open a Public Blueprint Course and choose **Create Course Instance from this
Blueprint**, or create an empty Course Instance. The adoption form selects one
exact saved Revision; enter Course names and term dates, then choose **Create
Course Instance**. Blueprint Assessments become independently editable
Unreleased Course Instance Assessments. Their Types, Questions, Pools, points,
instructions, and teaching defaults are retained; delivery dates start unset.
Set dates in the Course Instance and review the Assessments before release.
Adoption either completes in full or creates no Course Instance.

The Blueprint overview lists Assessments without editable fields. For a Blueprint
you own, choose **Open Course Editor**, then **Edit assignment** beside one assignment.
The quoted old labels identify current implementation controls; the target copy
is **Edit Assessment** and **Return to Assessment list**. Local edits stay available
until **Save Blueprint Course** creates a Revision or you discard them. Course names
and Private/Public/Archived lifecycle are current metadata and do not create a
Revision.

## Choose installation data

The default local stack creates the Pilot Question publication and ordinary Live Demo
graph after its canonical structure is installed. To start an ordinary product stack
without that data, use the controller directly before provisioning:

```bash
source source_me.sh && python3 local_stack.py start --headless --without-live-demo
```

This opt-out changes only the installation-data choice. It does not select another
schema, identity model, project, or data lifecycle.

## Inspect the local stack

Run controller commands through the repository Python environment:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py logs --tail 120
source source_me.sh && python3 local_stack.py validate
```

`doctor`, `status`, `projects`, `logs`, and `validate` are diagnostics; `--json` is
available on `doctor`, `status`, `projects`, and `validate`. Read
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for ownership, recovery, and
Podman details.

## Administer the database

The database command has exactly three lifecycle actions. `initialize` and `migrate`
read `PLE_MIGRATION_DATABASE_URL` and require the migration role. `verify` reads
`DATABASE_URL` through the restricted application connection:

```bash
cargo tools database initialize
cargo tools database migrate
cargo tools database verify
```

Use `initialize` for a genuinely empty PLE database. Use `migrate` only after the
post-freeze migration lifecycle has pending forward changes. Use `verify` to check the
application-safe schema projection without granting DDL or migration-ledger access.
The complete default installation-data operation runs after services are ready:

```bash
cargo tools installation-data provision
```

`provision` first runs the convergent Pilot publication and database-owned
teaching graph, then creates cross-system Student Work and grading effects
through their owning paths. `apply` is available when only that database-owned
graph is needed; it is not complete Live Demo provisioning. Use
`cargo tools installation-data provision --without-live-demo` to explicitly
skip the Live Demo before it is created. [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md)
explains the base schema and post-freeze rule; [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md)
explains the restricted-role boundary.

## Validate a change

Use the smallest applicable gate first:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
```

The first three are local code and hygiene gates. `local_stack.py acceptance` exercises
its declared connected service lanes, not a complete visible teaching journey. The
aggregate front door and evidence classifications are in
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Find oversized documentation sections

```bash
source source_me.sh && python3 devel/markdown_section_sizes.py -i docs/HUMAN_GUIDANCE.md -n 15
```

Prints every heading with its line number and the number of `- ` bullets directly under it,
largest first; `-n` limits the listing and `-m` hides sections below a bullet count. Use it to
pick the sections of a long guidance document that want splitting.

## Generate schema tables documentation

```bash
source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py
```

Reads `schemas/base_schema/` (override with `-s`/`--source-dir`) or a live
database (`-d`/`--database`) through
[../schema_style/schema_catalog_lib.py](../schema_style/schema_catalog_lib.py)
and writes [SCHEMA_TABLES.md](SCHEMA_TABLES.md) (`-o`/`--output`) plus
`schemas/catalog_snapshot.json`
(`-j`/`--snapshot`). One Markdown section per `20_tables/*.sql` file when that
directory exists, otherwise one section per source file that contains
`CREATE TABLE`. Each table lists its qualified name, role tag, columns, types,
nullability, constraints, foreign keys, indexes, and catalog comments.

The checker loads `schemas/catalog_snapshot.json` when that file exists.
`-j`/`--snapshot` selects another path and raises `FileNotFoundError` if it
is missing. Snapshot and database runs also apply Tier 3 rules.

## Check schema style

Regenerate the catalog snapshot, then check it. The checker reads
`schemas/catalog_snapshot.json` on its own when that file exists:

```bash
source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py
```

`-j`/`--snapshot` still selects an explicit snapshot path. Omit the snapshot
file (and `-j`) for a source-only run.

Reports mechanical rules from docs/DATABASE_STYLE.md against schemas/base_schema/ (override with -s/--source-dir). Default stdout is one count, tab, rule_##_title line per rule with findings (two-digit numbers), skip notes, and N findings in M rules only. One finding line per violation (rule_##_title, location, message, source file:line) is written to output/schema_style_findings.txt. -v/--verbose also prints those finding lines to stdout. -r/--report includes advisory findings (rule_14_unindexed_fk, and rule_layout only when 20_tables/ is absent) in verbose stdout. -j/--snapshot reads a catalog snapshot; -d/--database reads a live database. Exit 1 on any findings, 0 if clean.

## Course retention intervals

The installation-wide FERPA schedule is PostgreSQL settings, not a table.
Defaults are 14 days inactive warning, 70 days archive notice, 100 days from
retention start to archive, and 265 days from that archive cutoff to deletion.
Override per database:

```sql
ALTER DATABASE ple SET ple.retention_archive_after_retention_start = '100 days';
```

`ple_data.retention_schedule()` is the SQL read path for those values.

## Evidence boundaries

The completed baseline reset has connected PostgreSQL, service, browser,
installation-data, and backup/restore evidence recorded in
[CHANGELOG.md](CHANGELOG.md). Use the narrowest owning gate while changing the
system: PostgreSQL-only for database-owned invariants, focused service checks for
cross-system boundaries, and a Live Demo journey when it adds end-to-end evidence.
See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the permanent-test and
connected-evidence boundary.
