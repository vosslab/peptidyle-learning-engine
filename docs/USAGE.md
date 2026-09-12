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
Live Demo entry URL. Open an already-running demo, or start and open a fresh one:

```bash
./launchers/run_live_demo.sh open
./launchers/run_live_demo.sh start --open
```

Use `--headless` when the command must not open a browser. Each start replaces the
fixed disposable stack and its previous local data; unrelated Podman projects remain
outside this lifecycle. Stop it through the same owner:

```bash
./launchers/run_live_demo.sh stop
```

The local identity selector replaces only identity verification. The server still
derives each authenticated session, Product Role, and Course relationship from stored
PLE records. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the ordinary teaching
graph and its data boundaries.

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

## Evidence boundaries

The completed baseline reset has connected PostgreSQL, service, browser,
installation-data, and backup/restore evidence recorded in
[CHANGELOG.md](CHANGELOG.md). Use the narrowest owning gate while changing the
system: PostgreSQL-only for database-owned invariants, focused service checks for
cross-system boundaries, and a Live Demo journey when it adds end-to-end evidence.
See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the permanent-test and
connected-evidence boundary.
