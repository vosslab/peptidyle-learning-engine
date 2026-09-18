# PLE base schema

This directory is the canonical structural build for a fresh PLE database.
`install.sql` is deliberately small: it is an ordered `psql` manifest, not a
combined schema file. Source is layered by kind, then by domain, so a reader
can audit table shape without opening a function body:

- `00_roles.sql` -- cluster roles, schemas, default-deny ACLs
- `10_types.sql` -- enums and domains
- `20_tables/<aggregate>.sql` -- `CREATE TABLE` and `COMMENT ON` only
- `30_constraints.sql` -- late and circular foreign keys
- `40_indexes.sql` -- explicit indexes
- `50_functions/<domain>.sql` -- functions, triggers, and views
- `60_policies.sql` -- RLS enablement and policies (includes `60_policies/`)
- `70_grants.sql` -- table and routine privileges (includes `70_grants/`)

`api_compatibility.sql` now lives under `50_functions/` and finishes the
restricted runtime projection.

The database administration command installs the manifest with PostgreSQL 17
`psql -X --set=ON_ERROR_STOP=1 --single-transaction`:

```bash
cargo tools database initialize
```

The base is DDL only. Required teaching data belongs to the separate
installation-data phase, never to a structural correction.

Before this command, the platform's single privileged bootstrap transaction
creates every PLE role. The base validates that graph and then owns PLE
schemas, database objects, and grants. The `NOCREATEROLE` migrator can assume
only the database owner, four schema owners, and Unrelease executor through
direct `SET`-only memberships; it has no application, authentication, or
student capability membership.

## Before the production freeze

This source remains editable until the first human-approved production
deployment. Change the owning module directly, then rebuild a disposable
database from empty. That keeps the base readable and prevents a new
pre-production patch history.

## After the production freeze

The first human-approved production deployment freezes these base files.
Subsequent structural changes are timestamped, immutable SQLx forward
migrations in `../migrations/`, applied with:

```bash
cargo tools database migrate
```

Use `cargo tools database verify` with a restricted application connection to
confirm the installed schema without granting runtime DDL or ledger access.
