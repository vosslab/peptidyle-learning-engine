# Plan: Pre-production database baseline

Status: planned. This document records a future release gate; it authorizes no implementation
today and makes no production claim.

## Context

PLE has 28 checked-in SQLx migrations in `schemas/migrations/`. The current chain is the correct
source of truth during active feature acceptance. PLE has no users or durable production data, so
before its first production deployment the unreleased history can become one reviewed, empty-cluster
baseline. Once that baseline ships, each later schema change becomes an immutable forward migration.

Migrations establish schema, roles, policies, views, grants, and compatibility projections. They do
not create teaching data. The local typed lifecycle applies migrations first, but the current seed command
still requires `--apply-migrations` and invokes the same ledger a second time, normally as a no-op.
The baseline work must make the seed require a compatible pre-migrated database so schema and
teaching-data ownership are structurally separate before production.

## Objectives

- Replace the unreleased SQLx history with one reviewed clean-cluster baseline before production.
- Preserve the exact current schema behavior, role boundaries, and security properties in that baseline.
- Establish the shipped baseline as the first entry in the durable forward-only migration ledger.
- Demonstrate that an empty cluster, the local stack, and browser workflows remain correct.

## Design philosophy

This is a clean pre-production redesign, not an upgrade exercise. It follows **Fix the design, not
the symptom**, **Long-term over short-term**, and **Design for adaptability** from
[REPO_STYLE.md](REPO_STYLE.md): consolidate only before real data exists, then retain an auditable
forward ledger forever.

## Scope

- Inventory the accepted schema represented by the complete current migration chain.
- Generate and review one SQLx baseline migration for an empty PostgreSQL cluster.
- Preserve schema objects and their security-relevant definitions exactly where behavior requires it.
- Reset the disposable E2E database path to exercise the new single-baseline ledger.
- Convert the development seed into a data-only operation that requires a compatible pre-migrated
  database.
- Record the new forward-migration policy and acceptance evidence in durable documentation.

## Non-goals

- Do not alter the current migration chain during active feature acceptance.
- Do not migrate, backfill, transform, or retain hypothetical legacy data.
- Do not change application behavior, teaching semantics, identities, grading, or tenant boundaries.
- Do not claim a production deployment or use this roadmap as a release authorization.

## Current state summary

- `schemas/migrations/` contains 28 SQLx migration files, versions `2026080801` through
  `2026080932` with intentional reserved-version gaps.
- The migration command is explicit and privileged: `cargo tools database migrate` requires
  `PLE_MIGRATION_DATABASE_URL`; the application role cannot apply DDL.
- `cargo tools database status` reports ledger state; `verify` checks the application-visible
  migration projection through restricted roles.
- `tests/e2e/e2e_database_baseline.sh` already proves an empty database, a second no-op apply,
  status, verification, checksum detection, RLS/grant behavior, and selected live database contracts.
- The private typed local-stack lifecycle runs migrations and then the host-only deterministic E2E seed. Today the
  seed requires `--apply-migrations` and invokes the migration ledger again; this is normally a
  no-op, but it leaves schema authority in a command intended to own demonstration data.
- The active release plan remains the authority for package acceptance and deployment order.

## Architecture boundaries and ownership

The authoritative schema is SQL in `schemas/migrations/`; SQLx's `_sqlx_migrations` table is the
applied-ledger record. `learning-data-access` embeds and verifies the schema epoch. `project-tools`
offers the explicit administrative commands. The application and browser consume only verified
capabilities; neither owns DDL. The baseline work removes migration authority from the local seed so
it owns only disposable teaching data outside the baseline migration.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / workstream | Component | Review boundary |
| --- | --- | --- |
| M1 / DB-BL1 | Schema inventory and baseline SQL | PostgreSQL owner + security reviewer |
| M2 / DB-BL2 | SQLx embedding, ledger, and E2E oracle | Rust/data-access owner |
| M3 / DB-BL3 | Local stack, seed, browser acceptance, and documentation | Integrator + independent reviewer |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Freeze and model | Freeze accepted sources and compare clean-cluster schemas | A reviewed baseline candidate |
| M2 | Replace and prove | Replace the pre-production chain and run schema/security gates | Exact empty-cluster behavior |
| M3 | Release readiness | Exercise the real stack, seed, and browser before deployment | Shippable durable ledger |

### Freeze and model

- Depends on: explicit pre-production entry approval and accepted active release packages.
- Deliverables: baseline inventory, schema-diff evidence, and one proposed SQLx baseline file.
- Workstreams: DB-BL1.
- Entry criteria: no migration package remains acceptance-open; the current chain passes its full
  disposable baseline gate twice from clean clusters.
- Exit criteria: an independent reviewer accepts that the proposed baseline recreates the current
  accepted schema and security definitions without relying on seed data.
- Parallel-plan ready: no. The inventory must establish one frozen input before baseline SQL exists.

### Replace and prove

- Depends on: M1.
- Deliverables: one replacement baseline, updated embedded-ledger expectations, and E2E evidence.
- Workstreams: DB-BL2 and DB-BL2R.
- Entry criteria: accepted M1 inventory and a clean working tree checkpoint for the migration sources.
- Exit criteria: two independent empty clusters pass fresh migration, no-op reapply, status, and
  verification; schema and security comparisons are accepted.
- Parallel-plan ready: yes. DB-BL2 owns the source change while DB-BL2R independently reruns the
  old-versus-new comparison and reviews it.

### Release readiness

- Depends on: M2 and accepted release packages.
- Deliverables: local-stack evidence, browser acceptance, recovery rehearsal, and documentation.
- Workstreams: DB-BL3 and DB-BL3R.
- Entry criteria: M2 exit evidence and a newly created empty local volume.
- Exit criteria: all release gates pass, documentation is current, and independent review accepts
  the evidence before the first production deployment.
- Parallel-plan ready: yes. DB-BL3 owns runtime evidence; DB-BL3R audits artifacts and commands.

## Workstream breakdown

### DB-BL1: Schema inventory

- Goal: describe the exact accepted schema that one baseline must create.
- Owner: `postgresql-expert`.
- Work packages: DB-BL1.
- Needs: frozen 28-file migration chain and active release acceptance status.
- Provides: a schema, role, RLS, view, grant, extension, function, trigger, partition, and index
  inventory with a reproducible clean-cluster comparison procedure.
- Review boundary, when modifying the repository: SQL migration sources and database documentation.

### DB-BL2: Baseline and ledger

- Goal: make SQLx embed exactly one clean-cluster baseline and preserve administration contracts.
- Owner: `postgresql-expert` with `rust-code-expert`.
- Work packages: DB-BL2.
- Needs: DB-BL1 accepted inventory.
- Provides: replacement baseline migration, updated status/verification expectations, and a single
  E2E baseline oracle.
- Review boundary, when modifying the repository: `schemas/migrations/`, migration administration,
  and database E2E runner.

### DB-BL2R: Independent schema review

- Goal: independently compare old-chain and new-baseline empty clusters.
- Owner: independent PostgreSQL/security reviewer.
- Work packages: DB-BL2R.
- Needs: DB-BL2 candidate and DB-BL1 inventory.
- Provides: a signed-off discrepancy report or exact remediation findings.
- Review boundary, when modifying the repository: none; review evidence only.

### DB-BL3: Runtime acceptance

- Goal: make normal bootstrap migration-first and data-only-seed-second.
- Owner: local-stack integrator.
- Work packages: DB-BL3.
- Needs: accepted DB-BL2.
- Provides: clean-volume typed lifecycle, API, worker, and browser evidence.
- Review boundary, when modifying the repository: lifecycle, container, test, and operations docs.

### DB-BL3R: Independent release review

- Goal: audit final evidence and recovery instructions before the first deployment.
- Owner: independent operations/security reviewer.
- Work packages: DB-BL3R.
- Needs: DB-BL3 evidence.
- Provides: accepted release review or blocking findings.
- Review boundary, when modifying the repository: none; review evidence only.

## Work packages

### DB-BL1: Freeze the current schema

- Owner: `postgresql-expert`.
- Touch points: `schemas/migrations/`, [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md), and the
  active release plan.
- Depends on: none after the explicit pre-production entry gate.
- Acceptance criteria: record the exact old-chain commit and build two clean databases from it;
  compare their schema definition, ownership, extensions, roles, forced RLS, policies, views,
  functions, triggers, grants, and migration-state projections.
- Evidence or review, when useful: use `pg_dump --schema-only`, catalog queries, and role-specific
  probes; treat normalized diffs as review material, not a permanent exact-text test.
- Obvious follow-ons: DB-BL2.

### DB-BL2: Build the baseline ledger

- Owner: `postgresql-expert`.
- Touch points: `schemas/migrations/`, `crates/learning-data-access/src/postgres/migrations.rs`,
  `crates/project-tools/src/database.rs`, and `tests/e2e/e2e_database_baseline.sh` only where
  behavior genuinely changes.
- Depends on: DB-BL1.
- Acceptance criteria: replace the 28 unreleased files with one ordered SQLx baseline that creates
  the inventory from DB-BL1 on an empty cluster; keep migration administration explicit and keep
  application startup read-only.
- Evidence or review, when useful: apply once, apply again without change, run status and verify
  twice from separately created empty databases, then compare both results to DB-BL1.
- Obvious follow-ons: DB-BL2R and DB-BL3.

### DB-BL2R: Review security fidelity

- Owner: independent PostgreSQL/security reviewer.
- Touch points: none unless a review finding identifies a source correction.
- Depends on: DB-BL1 and DB-BL2.
- Acceptance criteria: demonstrate that the baseline preserves schema ownership, forced RLS,
  tenant isolation, least-privilege grants, `security_invoker` view behavior, answer-key isolation,
  partition ownership, and required migration-state access for every runtime role.
- Evidence or review, when useful: separately provisioned disposable clusters and role-specific SQL
  probes; report any difference as a blocking security discrepancy.
- Obvious follow-ons: DB-BL3 after acceptance.

### DB-BL3: Prove runtime and recovery

- Owner: local-stack integrator.
- Touch points: focused `local_stack_control` lifecycle modules, `crates/project-tools/src/e2e_seed.rs`, its focused modules,
  `tests/e2e/`, `tests/playwright/`, and operations docs only where the ownership contract changes.
- Depends on: DB-BL2 and DB-BL2R.
- Acceptance criteria: the seed refuses an absent or incompatible baseline without applying DDL;
  on a newly created local PostgreSQL volume, the typed lifecycle alone applies and verifies the baseline,
  then runs the data-only deterministic seed, starts every required service, and passes the
  representative browser suite.
- Evidence or review, when useful: retain command outputs, schema status before/after seed, and
  screenshots only when browser behavior changed; run the same clean-stack sequence twice.
- Obvious follow-ons: DB-BL3R and production deployment planning.

### DB-BL3R: Accept release evidence

- Owner: independent operations/security reviewer.
- Touch points: documentation and evidence reports only.
- Depends on: DB-BL3.
- Acceptance criteria: confirm that all artifacts identify one baseline and that forward migration,
  backup, restore, and failure recovery instructions are coherent before deployment.
- Evidence or review, when useful: independent command replay against a fresh disposable stack.
- Obvious follow-ons: first production deployment only after human release approval.

## Acceptance criteria and gates

- Pre-production entry gate: human approval confirms no production deployment and no durable user
  data; the active release plan confirms all included schema packages are accepted.
- Schema-fidelity gate: clean old-chain and clean baseline databases agree on all application-relevant
  schema, roles, RLS policies, views, grants, functions, triggers, indexes, partitions, and
  migration-state projections.
- Ledger gate: a fresh baseline migration succeeds; a second migrate is a no-op; `status` and
  `verify` report compatible state on two independently created empty clusters.
- Security gate: `ple_app`, graders, workers, publisher, and migration roles retain exactly their
  authorized access; cross-tenant and answer-key reads fail as before.
- Data-separation gate: the baseline is valid before any seed runs; the seed command has no
  migration flag or DDL path, refuses an incompatible database, and changes teaching data only.
- Real-stack gate: `source source_me.sh && python3 local_stack.py start --no-open` succeeds from a newly created local volume, then
  `./run_playwright_tests.sh` passes only after all required Podman services are healthy.
- Independent review gate: an independent PostgreSQL/security reviewer and an operations reviewer
  approve the comparison, runtime evidence, and recovery procedure.

## Test and verification strategy

Permanent tests are retained only when they meet the checklist in [PYTEST_STYLE.md](PYTEST_STYLE.md).
No new fast pytest should snapshot relation counts, migration filenames, or complete catalog output.

| Evidence | Classification | Reason |
| --- | --- | --- |
| SQLx status and verification behavior | Permanent Rust behavior test | It protects a durable administration and startup contract. |
| Existing `e2e_database_baseline.sh` fresh/no-op/security path | Permanent E2E gate | It exercises a durable clean-cluster and role-boundary contract. |
| Old-chain versus candidate-baseline schema dump | One-time implementation evidence | The old chain disappears after the cutover; an exact comparison would become stale. |
| Candidate baseline object inventory | One-time implementation evidence | It proves the replacement rather than a continuing user-visible behavior. |
| Local typed lifecycle with empty volume and deterministic seed | Permanent E2E/operational gate | It protects the durable migration-first, data-only-seed-second contract. |
| Representative browser walkthrough after bootstrap | Permanent Playwright acceptance | It protects teaching workflows, not migration geometry. |

Run the repository fast suites separately from real service checks. Keep external-network, Podman,
PostgreSQL, lifecycle, and browser work under `tests/e2e/` or Playwright, never under `pytest tests/`.

## Migration and compatibility policy

Before the baseline ships, the 28-file history remains immutable in practice for active package
acceptance. At the explicit cutover, replace that entire unreleased history with one reviewed baseline
and rebuild only clean disposable/local databases. Do not create bridges, down migrations, legacy
readers, or data adoption paths.

After the baseline ships, never edit its filename, version, SQL, or checksum. Every schema change
gets one later forward migration with the owning active-plan package, fresh/no-op migration evidence,
role/RLS evidence, and the behavior tests justified by [PYTEST_STYLE.md](PYTEST_STYLE.md).

Before shipment, rollback means restoring the frozen old migration chain from the source-control
checkpoint and recreating only disposable clusters; there is no user-data downgrade. After shipment,
never edit or roll back the baseline with a down migration. Recover service from the documented,
tested backup/restore path and repair schema changes with a new forward migration.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Baseline omits a security object | High | Catalog or role diff | PostgreSQL owner | Block cutover; correct baseline and repeat independent review. |
| Seed retains DDL authority | High | Seed accepts `--apply-migrations` or invokes the ledger | Integrator | Remove that path; require and verify a compatible pre-migrated database. |
| Feature lands during freeze | Medium | New migration appears | Release manager | Delay cutover and refresh DB-BL1 inventory. |
| Fragile inventory test enters pytest | Medium | New exact-count test | Test owner | Use one-time evidence or durable behavior gate instead. |
| Recovery instructions are untested | High | Restore rehearsal fails | Operations reviewer | Block release until a clean-cluster recovery drill passes. |

## Rollout and release checklist

- [ ] Human approves the pre-production entry gate.
- [ ] Active release packages and schema owners are accepted.
- [ ] DB-BL1 inventory and independent comparison procedure are accepted.
- [ ] DB-BL2 baseline passes two clean-cluster fresh/no-op/status/verify cycles.
- [ ] DB-BL2R accepts role, RLS, view, and grant fidelity.
- [ ] DB-BL3 passes typed lifecycle, seed separation, API/worker readiness, and Playwright acceptance.
- [ ] DB-BL3R accepts backup, restore, and failure-recovery rehearsal.
- [ ] The baseline and all durable forward-ledger instructions are documented before deployment.

## Documentation close-out requirements

- Active plan / progress tracker: update the release completion plan with the accepted cutover and
  forward-ledger ownership.
- `docs/CHANGELOG.md` entry: record the baseline replacement, not obsolete migration filenames.
- Archive / closure notes: move this roadmap to the appropriate completed-plan archive and retain
  the accepted schema-comparison evidence location.

## Patch plan and reporting format

- Patch 1: DB-BL1 inventory and approved cutover input.
- Patch 2: DB-BL2 one-baseline SQLx replacement plus durable behavior gates.
- Patch 3: DB-BL3 operational/docs closure after independent review.
- Report each patch with owner, dependencies met, commands run, one-time evidence location,
  permanent tests retained, and unresolved risks.

## Open questions and decisions needed

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: release manager with `postgresql-expert` and independent
    security reviewer.
  - Evidence and decision rule: authorize cutover only when all active schema packages are accepted,
    two clean-cluster comparisons have no unexplained behavioral/security differences, and no durable
    production data exists.
- Non-blocking follow-up: choose the production backup-retention and restore cadence from measured
  deployment evidence under WP-RC10; do not encode operational tuning into this baseline.
