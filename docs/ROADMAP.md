# Roadmap: pre-production database and release readiness

Status: foundational pre-production database work is accepted, while browser restoration,
data-separation, release, and production-readiness gates remain open. This is the durable forward
roadmap, not an implementation authorization or a production-release claim. [CONTRACTS.md](CONTRACTS.md)
owns durable boundaries, [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) owns validation rules,
and [TODO.md](TODO.md) routes unfinished work. Bounded execution notes do not replace these
permanent authorities.

## Evidence boundary

The current pre-production reset is the 36-file foundational migration baseline
(`2026082901` through `2026082936`). A clean PostgreSQL 17 volume applies that exact
domain-ordered baseline, accepts a second no-op run, and passes the restricted-login
Question Library probes. Earlier migration epochs are historical evidence only; they are not part of
the material schema contract. [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) owns the checked-in
migration sequence and forward allocation rule.

The automated-grading operation boundary is accepted. Its seven predecessor migrations and four-file
`2026081866`-`2026081869` closeout sequence are present, and final material-tree Validation passed on
the 99-migration tree. The completed wire-naming and terminology cutovers are recorded in
[CHANGELOG.md](CHANGELOG.md) and governed by [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md). Student-work inspection and grade-scheme-aware
calculated Gradebook work remain acceptance-open for their remaining visual and documentation gates.
Provider/mailbox, unrelated passkey, multi-replica, security, HCI, and release work remain acceptance-open.
Instructor live-demo acceptance does not imply production onboarding,
deployment, or release acceptance.

## Accepted/current/future

### Accepted

- The foundational clean-cluster baseline and its explicit migration administration boundary.
- The forward-only migration allocation policy and accepted feature migrations recorded in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- The real live-demo deployment and session boundary: a seeded Account selector, ordinary
  server-owned Authenticated Session resolution, and the explicitly absent teaching routes
  specified in [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).
- Historical automated-grading HCI evidence as contextual evidence only. It does not establish a
  current Student, Instructor, or Gradebook browser workflow.
- Existing normalized operational models only where [CHANGELOG.md](CHANGELOG.md) records accepted
  evidence. This roadmap does not broaden those claims.

### Current and acceptance-open

- Restore the Course, authoring, delivery, grading, Gradebook, and administration route surface,
  then establish separate browser, visual, accessibility, and task-completion evidence.
- Rerun the complete named Validation suite on each final material tree. Focused or historical
  migration counts do not establish release acceptance.
- Keep documentation links GitHub-browsable through the material-tree Markdown-link gate.
- Keep schema administration explicit and privileged: `cargo tools database migrate` uses
  `PLE_MIGRATION_DATABASE_URL`; application startup and browser capabilities do not own DDL.
- Close live-demo data separation. The typed lifecycle may apply migrations first, but the
  baseline installer currently still accepts `--apply-migrations` and therefore retains a
  duplicate schema authority. The target is a compatible pre-migrated database followed by a
  data-only installer for fictional, disposable teaching data.
- Complete the remaining QSOM1 product capability under one bounded owner: same-lineage
  publication, Draft Question expiration, orphan cleanup, Question Search isolation, Server
  Routes, Browser Surfaces, and connected acceptance. This is feature delivery, not a vocabulary
  replacement gate; settle and allocate the operation contract before implementation.
- Complete clean-volume real-stack, browser, backup/restore, and independent security/operations
  review gates before any production deployment decision.
- Continue the broader version 1 platform goal through bounded work items; this roadmap records the
  durable database and release-readiness boundary.

### Future

- Allocate currently absent vocabulary-adjacent capabilities only when their product work becomes
  current: Watched Questions, Blueprint Updates, Course Invitation Email Delivery, a configured
  Question Backend selector, Course Banner upload/persistence, durable Blueprint-operation replay,
  and Job Kind registration/readiness. Their reserved names do not create implementation work in
  the Vocabulary Replacement Checklist.
- Treat further database normalization as future design work, owned by a later bounded work item
  after evidence demonstrates a real need. Do not add speculative tables, bridges, down
  migrations, legacy readers, or data-adoption paths to close current release gaps.
- Choose production backup retention, restore cadence, capacity thresholds, and operational
  tuning from measured deployment evidence; do not encode those choices
  in the pre-production baseline.
- Production deployment and durable user-data migration remain outside this roadmap until human
  release approval and all required durable acceptance gates are complete.

## Architecture and ownership

The authoritative schema is SQL in `schemas/migrations/`; SQLx's `_sqlx_migrations` table is the
applied-ledger record. `learning-data-access` embeds and verifies the schema epoch. `project-tools`
owns explicit migration status, migrate, and verify commands. The application and browser consume
verified capabilities; neither owns DDL. The live-demo lifecycle should therefore be:

1. A migration principal applies and verifies the compatible schema.
2. A data-only host installation creates the known fictional, disposable
   teaching-data baseline.
3. The normal application, worker, storage, and browser paths exercise that live state.

The product path is not redesigned by this database roadmap. [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md)
and [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) remain authoritative for grading secrecy,
determinism, ordinary teaching workflows, and demo identity boundaries.

## Dependency-ordered work

| Stage | Work                                                                                 | Exit evidence                                                                      |
| ----- | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| D1    | Finish the current Instructor work item and its authority proofs                     | Focused gates and final Validation pass                                            |
| D2    | Migration Check of the final migration inventory and clean-cluster baseline evidence | Fresh/no-op/status/verify and role/RLS evidence on disposable clusters             |
| D3    | Remove migration authority from the live-demo data installer                         | Incompatible or absent baseline is refused without DDL; data-only install succeeds |
| D4    | Exercise release operations                                                          | Clean-volume lifecycle, real-stack browser, restore, and independent review pass   |
| D5    | Human release decision                                                               | No unresolved required gate; deployment approval is explicitly recorded            |

Stages are intentionally serial where later work depends on accepted schema or package
contracts. Any new schema work receives an allocation in the shared Migration Allocation Registry before
implementation; non-schema work receives no implicit migration number.

## Durable migration policy

Before v1 ships, disposable databases may be recreated from the reviewed baseline and current
forward chain. There is no user-data downgrade or hypothetical legacy-data adoption. After v1
ships, never edit a migration filename, version, SQL, or checksum. Every schema change uses one
later forward migration owned by a bounded work item, with fresh/no-op migration evidence, role and
RLS evidence, and behavior tests justified by [PYTEST_STYLE.md](PYTEST_STYLE.md).

Keep fast pytest deterministic, offline, and behavioral. Do not add brittle assertions over dates,
collection sizes, required key lists, hardcoded defaults, migration filenames, or complete Question Library
output. External-network, Podman, PostgreSQL, lifecycle, browser, and restore checks remain
explicit E2E, Playwright, or operational gates rather than hidden fast tests.

## Risks and release gates

| Risk                                            | Required response                                                              |
| ----------------------------------------------- | ------------------------------------------------------------------------------ |
| Schema or security object drift                 | Block the cutover; compare clean clusters and repeat independent review.       |
| Installer still applies DDL                     | Keep release acceptance open; remove the flag and migration application path.  |
| Current source changes during evidence capture  | Refresh the inventory and rerun the affected gates on the final material tree. |
| Recovery procedure is untested                  | Block release until a disposable restore exercise passes.                      |
| Normalization is proposed without measured need | Defer it to a future work item with an explicit owner and allocation.           |

Release is not ready until [CHANGELOG.md](CHANGELOG.md) records accepted predecessors, complete
Validation, data-only live-demo installation, clean-stack/browser evidence, recovery evidence, and
independent review. This roadmap does not authorize deployment.

## Related documentation

- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) - current schema inventory and migration inventory and allocation registry.
- [CONTRACTS.md](CONTRACTS.md) - durable service and capability contracts.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) - Validation evidence model.
- [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) - durable owner decisions.
- [CHANGELOG.md](CHANGELOG.md) - dated package history and receipts.
