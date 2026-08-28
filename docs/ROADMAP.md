# Roadmap: pre-production database and release readiness

Status: foundational pre-production database work is accepted, while data-separation,
release, and production-readiness gates remain open. This is a forward roadmap, not an
implementation authorization or a production-release claim. The active plans own current
scope, dependency order, contracts, validation, and acceptance:

- [implementation plan](active_plans/implementation_plan.md)
- [release completion plan](active_plans/active/release_completion_plan.md)
- [implementation status registry](active_plans/implementation_status.md)

## Evidence boundary

The accepted pre-production reset established the six-file foundational baseline
(`2026080801` through `2026080806`). The material source tree now contains 99 ordered SQL
migrations through `2026081869`. The earlier 95-migration chain through `2026081865` is
historical acceptance evidence. Accepted feature packages own later migrations; accepted
migration files remain immutable.
The status registry is the authority for package allocation and its recorded evidence, which
can lag the current material tree until the named Validation gates are rerun.

The current package is `WP-PROF-G1`, stage G1-W7 closeout. Its automated-grading operation
boundary has completed W5 through W7b, its seven accepted migrations are restored, and its
implemented closeout source is complete across the four-file `2026081866`-`2026081869` sequence.
The affected live database/RLS/worker/browser evidence is green on the 99-migration material
tree. It remains acceptance-open pending repository tracking of the new owned files and exact
final-tracked-tree Validation.
`WP-RC8` remains acceptance-open for provider/mailbox, unrelated passkey, multi-replica, security,
HCI, and release work. Professor live-demo acceptance does not imply production onboarding,
deployment, or release acceptance.

## Accepted/current/future

### Accepted

- The foundational clean-cluster baseline and its explicit migration administration boundary.
- The forward-only migration allocation policy and the accepted feature migrations recorded in
  [implementation status](active_plans/implementation_status.md#shared-migration-ledger-and-allocation).
- The real live-demo product boundary: ordinary Student, Instructor, and Sysadmin workflows,
  server-owned authorization, answer-free browser contracts, and deterministic automated grading
  as specified in [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).
- The G1 HCI closeout review, which accepted the current learner status, Instructor retry, and
  Gradebook workflow with no P0/P1/P2 findings.
- Existing normalized operational models only where the active plans and current evidence mark
  their owning package accepted. This roadmap does not broaden those claims.

### Current and acceptance-open

- Close `WP-PROF-G1` by tracking its new owned files, then pass exact final-tracked-tree
  Validation and advance the professor queue to `WP-PROF-G2`. The affected live
  database/RLS/worker/browser evidence for `2026081866` through `2026081869` is already green
  on the 99-migration material tree.
- Rerun the complete named Validation suite on the final material tree. Focused or historical
  migration counts are not sufficient for release acceptance.
- Keep documentation link acceptance open until the new owned artifacts are tracked and the link
  gate is rerun on that material tree.
- Keep schema administration explicit and privileged: `cargo tools database migrate` uses
  `PLE_MIGRATION_DATABASE_URL`; application startup and browser capabilities do not own DDL.
- Close live-demo data separation. The typed lifecycle may apply migrations first, but the
  baseline installer currently still accepts `--apply-migrations` and therefore retains a
  duplicate schema authority. The target is a compatible pre-migrated database followed by a
  data-only installer for fictional, disposable teaching data.
- Complete clean-volume real-stack, browser, backup/restore, and independent security/operations
  review gates before any production deployment decision.
- Continue the broader version 1 platform goal through the release plan; this roadmap records the
  database and release-readiness boundary only.

### Future

- Treat further database normalization as future design work, owned by a later active package
  after evidence demonstrates a real need. Do not add speculative tables, bridges, down
  migrations, legacy readers, or data-adoption paths to close current release gaps.
- Choose production backup retention, restore cadence, capacity thresholds, and operational
  tuning from measured deployment evidence under the release plan; do not encode those choices
  in the pre-production baseline.
- Production deployment and durable user-data migration remain outside this roadmap until human
  release approval and all required active-plan gates are complete.

## Architecture and ownership

The authoritative schema is SQL in `schemas/migrations/`; SQLx's `_sqlx_migrations` table is the
applied-ledger record. `learning-data-access` embeds and verifies the schema epoch. `project-tools`
owns explicit migration status, migrate, and verify commands. The application and browser consume
verified capabilities; neither owns DDL. The live-demo lifecycle should therefore be:

1. A migration principal applies and verifies the compatible schema.
2. A data-only host installer reconciles fictional baseline teaching data.
3. The normal application, worker, storage, and browser paths exercise that live state.

The product path is not redesigned by this database roadmap. [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md)
and [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) remain authoritative for grading secrecy,
determinism, ordinary teaching workflows, and demo identity boundaries.

## Dependency-ordered work

| Stage | Work | Exit evidence |
| --- | --- | --- |
| D1 | Finish the current professor package and its authority proofs | Owning plan's focused gates and final Validation pass |
| D2 | Reconcile the final migration inventory and clean-cluster baseline evidence | Fresh/no-op/status/verify and role/RLS evidence on disposable clusters |
| D3 | Remove migration authority from the live-demo data installer | Incompatible or absent baseline is refused without DDL; data-only install succeeds |
| D4 | Exercise release operations | Clean-volume lifecycle, real-stack browser, restore, and independent review pass |
| D5 | Human release decision | No unresolved required gate; deployment approval is explicitly recorded |

Stages are intentionally serial where later work depends on accepted schema or package
contracts. Any new schema work receives an allocation in the shared ledger before
implementation; non-schema work receives no implicit migration number.

## Durable migration policy

Before v1 ships, disposable databases may be recreated from the reviewed baseline and current
forward chain. There is no user-data downgrade or hypothetical legacy-data adoption. After v1
ships, never edit a migration filename, version, SQL, or checksum. Every schema change uses one
later forward migration owned by an active package, with fresh/no-op migration evidence, role and
RLS evidence, and behavior tests justified by [PYTEST_STYLE.md](PYTEST_STYLE.md).

Keep fast pytest deterministic, offline, and behavioral. Do not add brittle assertions over dates,
collection sizes, required key lists, hardcoded defaults, migration filenames, or complete catalog
output. External-network, Podman, PostgreSQL, lifecycle, browser, and restore checks remain
explicit E2E, Playwright, or operational gates rather than hidden fast tests.

## Risks and release gates

| Risk | Required response |
| --- | --- |
| Schema or security object drift | Block the cutover; compare clean clusters and repeat independent review. |
| Installer still applies DDL | Keep release acceptance open; remove the flag and ledger path. |
| Current source changes during evidence capture | Refresh the inventory and rerun the affected gates on the final material tree. |
| Recovery procedure is untested | Block release until a disposable restore exercise passes. |
| Normalization is proposed without measured need | Defer it to a future package with an explicit owner and allocation. |

Release is not ready until the active plans record accepted package predecessors, complete
Validation, data-only live-demo installation, clean-stack/browser evidence, recovery evidence,
and independent review. This roadmap does not authorize deployment.

## Related documentation

- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) - current schema inventory and migration ledger.
- [CONTRACTS.md](CONTRACTS.md) - durable service and capability contracts.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) - Validation evidence model.
- [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) - durable owner decisions.
- [CHANGELOG.md](CHANGELOG.md) - dated package history and receipts.
