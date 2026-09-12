# PLE base schema

This directory is the canonical structural build for a fresh PLE database.
`install.sql` is deliberately small: it is an ordered `psql` manifest, not a
combined schema file. Each included module owns the current final tables,
constraints, indexes, functions, row-security policies, and grants for one
domain. `cross_domain_constraints.sql` owns relationships between domains and
`api_compatibility.sql` finishes the restricted runtime projection.

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
