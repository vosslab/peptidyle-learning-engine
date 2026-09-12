# Database baseline and revision-model reset

Status: complete. This is the sole approved reset-plan record and an
implementation record, not a second architecture authority.
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) remains the product
authority, [TERMINOLOGY_CONTRACT.md](../TERMINOLOGY_CONTRACT.md) the semantic
contract, and [CONTRACTS.md](../CONTRACTS.md) the executable-boundary register.

## Resulting design

PLE is pre-production. `schemas/base_schema/` is therefore the editable,
canonical PostgreSQL source. Its short `install.sql` manifest orders domain-owned
modules; each module directly owns the current tables, constraints, indexes,
functions, RLS policies, and grants for its family. Rebuilds install that source
into an empty disposable database. A structural correction changes its owning
module directly.

The first human-approved production deployment freezes that base. Thereafter,
each structural change is one bounded, forward-only SQLx migration in
`schemas/migrations/`, recorded in `ple_migration._sqlx_migrations`. This is a
release decision, recorded in the changelog; it is not a product-version label,
baseline digest system, or compatibility layer.

The administration path remains small:

1. `cargo tools database initialize` installs the base into an eligible fresh
   database.
2. `cargo tools database migrate` applies pending post-freeze forward migrations.
3. `cargo tools database verify` reads the restricted application-visible schema
   state.

Runtime roles have neither DDL nor migration-ledger write authority. Existing
development databases and legacy migration ledgers are not upgrade inputs.

Installation data is a separate ordinary-data phase on that one schema. `cargo
tools installation-data provision` applies database-owned Live Demo facts and
then uses owning service paths for the required cross-system facts. It provisions
the complete known-good Live Demo by default; `--without-live-demo` selects an
empty product-data installation. The demo has no schema, role, marker, receipt,
or teardown subsystem of its own. Once present, it follows ordinary lifecycle,
retention, and deletion rules.

The resulting domain model is direct:

- Question Revision and Blueprint Revision are the only immutable Revision
  concepts.
- Course Instance and Assignment configuration are current state. Assignment
  uses its qualified Edit Number for concurrent saves, release, and Unrelease.
- Assignment Attempts and Issued Questions retain the exact evidence required to
  interpret Student Work after later current-state changes.
- Published Question and Blueprint availability belongs to stable lineages.
  New selection requires an Available lineage; Archive hides ordinary discovery
  while exact existing Revision references remain resolvable.
- A Blueprint Course starts with its owner-private Draft and no Revision. Every
  deliberate publish creates a new immutable Blueprint Revision; replay of the
  same accepted request returns its receipt.
- Assignment Unrelease is one guarded transaction: it confirms authority,
  lifecycle state, ETag, and title; deletes the schema-owned Student Work closure;
  rebuilds surviving statistics; and writes a redacted aggregate audit event.
- Question IDs are stored as compact seven-character uppercase Crockford Base32
  values. `AAA-BBBB` is display form only. The first six characters are generated
  from a cryptographic source and the final character is HMAC-derived. Pilot data
  uses ordinary valid IDs; `PNE-*` has no product meaning.

Question Change Proposal, Course Retention Plan, Blueprint collaboration, and
retired Course Schedule or Assignment Revision scaffolding have no baseline
schema, compatibility surface, or reserved slot. Future work begins as a
complete vertical capability when it has an approved product workflow.

## Completed evidence

The final database and application evidence establishes the intended boundaries:

- A PostgreSQL 17 baseline gate installed the canonical base, replayed the
  administration path, verified the restricted application role, exercised the
  authoring source binding, and proved the populated Unrelease closure and lock
  race.
- Default installation provisioning produced ordinary Pilot Questions and the
  complete Live Demo; focused lifecycle checks protect its one-time provisioning
  and later-replay policy. The explicit connected `--without-live-demo` path left
  no product-data roots and returned the expected non-enumerating `404` for the
  absent demo surface.
- The focused released-Assignment path proved that accepted current-state edits
  leave existing Attempt evidence intact and that later Attempts receive the
  accepted current state. Blueprint Draft creation, deliberate publication,
  publication replay, archive, and restore passed through the service boundary.
- The authoring API, S3 object path, and browser journey proved ordinary Question
  publication; the WebWork worker path proved the grading handoff.
- The ordinary backup restore followed by database migrate and application-role
  verify passed on PostgreSQL 17.
- Rust, TypeScript/Node, Python, and connected PostgreSQL gates passed under
  their owning commands. The retained suite protects durable behavior,
  authorization, evidence integrity, and lifecycle contracts rather than a
  historical migration inventory.

Temporary investigations support these checks only. Accepted decisions belong in
the canonical documents, code, and tests above; superseded reports do not become
a parallel documentation layer.
