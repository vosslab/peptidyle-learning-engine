# Roadmap: production-baseline readiness

Status: implementation gates for the pre-production database reset are complete.
This roadmap records the release boundary; it does not authorize deployment.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority,
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) is the semantic implementation
contract, and [TODO.md](TODO.md) owns genuinely unfinished product work.

## Database lifecycle

Before the first human-approved production deployment, the modular base schema
is the editable source of truth. Structural corrections change the owning file
under `schemas/base_schema/`; no corrective pre-production migration chain is
maintained. `install.sql` stays a small ordered `psql` manifest, while its
domain modules own the current tables, constraints, functions, RLS policies,
and grants.

The first human-approved production deployment records the freeze in the
release decision and [CHANGELOG.md](CHANGELOG.md). From that point, the base is
immutable and each structural change is one bounded, forward-only SQLx
migration in `schemas/migrations/`, recorded by
`ple_migration._sqlx_migrations`. This rule, rather than a migration count,
filename inventory, or product-version label, prevents a return to
patches-on-patches.

[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) describes the resulting
catalog. [CONTRACTS.md](CONTRACTS.md) owns the administration and installation
contracts.

## Completed production-baseline evidence

The canonical administration path is deliberately small:

1. `cargo tools database initialize` installs a fresh eligible database from
   the base manifest.
2. `cargo tools database migrate` applies any pending forward migration.
3. `cargo tools installation-data provision` ordinarily provisions the complete
   known-good Live Demo as ordinary product data. `provision --without-live-demo`
   explicitly omits that data phase; `apply` is only the narrower
   database-owned graph operation.
4. `cargo tools database verify`, using the application role, confirms the
   restricted schema-state contract.

The final material tree has passed the following release-readiness evidence:

- PostgreSQL 17 fresh initialization, replay, restricted application-role
  verification, schema ownership/RLS/DDL checks, and the populated Unrelease
  closure and lock race;
- default full Live Demo provisioning and replay, plus the connected explicit
  opt-out that leaves no product-data roots and exposes no demo surface;
- released Assignment edits with retained existing-Attempt evidence and
  accepted-current-state later Attempts; Question and Blueprint archive/restore;
  Blueprint Draft publication and replay; and Assignment Unrelease;
- authoring API/S3/browser publication and WebWork worker-grading boundaries;
  and
- backup restore followed by the ordinary migrate-and-verify path, together with
  the aggregate Rust, TypeScript/Node, and Python gates.

The Live Demo uses the production schema and ordinary records. Database-owned
facts belong in its idempotent data manifest; effects that leave PostgreSQL use
their normal owning path. It has no parallel demo schema, special lifecycle,
or teardown subsystem.

## Product work that remains separate

The reset establishes the current model: Question Revisions and Blueprint
Revisions are immutable publication evidence; Course and Assignment
configuration are current state; Attempts and Issued Questions retain the
facts needed to interpret Student Work. It removes retired Assignment, Course
Schedule, Question Change Proposal, and Course Retention Revision scaffolding
instead of presenting incomplete capabilities as launch work.

Future product capabilities are allocated only when they have a bounded
workflow, Store/Server authorization, user contract, and evidence plan. The
current candidates and their priority belong in [TODO.md](TODO.md), not in
schema reservations or compatibility layers.

## Evidence and release boundary

Keep fast tests deterministic, offline, and behavior-focused. PostgreSQL,
Podman, browser, real-stack, and restore exercises remain explicit acceptance
evidence rather than hidden unit-test machinery. Retain a test only when it
protects a durable behavior, authorization boundary, evidence-integrity rule,
or schema lifecycle requirement; record one-time rebuild investigations in
the changelog rather than making them permanent suites.

The remaining release decision is human approval of the first production
deployment. It freezes the base and opens the forward-migration era. Preservation
or upgrade of an existing user-data database is outside this pre-production
release boundary.

## Related documentation

- [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) - product authority and lifecycle decision.
- [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) - Revision and evidence semantics.
- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) - final database catalog and ownership.
- [CONTRACTS.md](CONTRACTS.md) - administration and installation interfaces.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) - durable versus one-time evidence.
- [TODO.md](TODO.md) - bounded, unfinished product work.
