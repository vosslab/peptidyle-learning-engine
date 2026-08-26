# Implementation status and handoff

Last updated: 2026-08-26

This is the sole mutable registry for the global current-package handoff and shared migration
allocations. The [implementation plan](implementation_plan.md), active
[professor capability plan](active/professor_capability_architecture_plan.md), and active
[release completion plan](active/release_completion_plan.md) own architecture, scope, dependency
order, validation, and acceptance. Durable product decisions remain in
[Human Guidance](../HUMAN_GUIDANCE.md); package history and detailed receipts remain in the
[changelog](../CHANGELOG.md).

## Current handoff

- **Current package:** `WP-PROF-T6` - the owner-directed Instructor assignment workspace. It makes
  the assignment title open one assignment home, separates Questions from Policies, adds focused
  revision-checked mutations and persisted incomplete drafts, and renders an Instructor-authorized
  answer-free Student view over the current live assignment. Its binding contract is
  `docs/active_plans/active/instructor_assignment_workspace_plan.md`. T6 owns forward capability
  migration `2026081848` for empty Draft/Archived definitions and Published readiness, and precedes
  `WP-PROF-G1`.
- **Current acceptance predecessor:** `WP-PROF-B2` accepted 2026-08-26. Its focused adoption boundary
  now owns preview-before-save fork and instantiation, rollover, term shifting, immutable provenance,
  controlled fast-forward, divergence recovery, and answer-free import inspection over ordinary
  teaching courses. Its final receipt is in the professor plan and changelog: all 77 migrations and
  Store/RLS oracles; all 15 production HTTPS journeys with independent Elena Instructor and Morgan
  Sysadmin passkeys; 75 privacy-validated screenshots; exact cleanup; and final Validation passed.
- **Accepted prerequisites:** `WP-PROF-S1` through `S7`, `T1` through `T3`, `BS1`, `LD1` through
  `LD3`, `T5`, `D1`, `D2`, `B1`, and `B2` are accepted. Their scopes and evidence are retained in
  the owning plans and changelog.
- **Release handoff:** `WP-RC8` remains parked and acceptance-open. It owns provider/mailbox,
  unrelated passkey, multi-replica, security, HCI, and release gates. Professor live-demo work does
  not imply production onboarding, deployment, or release acceptance.

### Active-system invariants

- Use the canonical disposable production-shaped HTTPS stack and visible UI-created product state.
- Keep grading deterministic and server-owned; browser contracts remain answer-free.
- Preserve tenant isolation, immutable published content, draft-versus-publication identity,
  immutable evidence, and stateless API replicas.
- Keep the learning engine question-agnostic. Biology examples are fixtures rather than policy.
- Retain direct-entry evidence for the five fixed seeded personas. Elena Instructor and Morgan
  Sysadmin each retain an independent generic passkey journey.

## B2 accepted evidence

The B2 implementation and focused evidence are current as of 2026-08-26. The selected Graphify
query identified the README architecture/documentation surface, `migrations.rs`,
`CurriculumAdoptionLivePage`, `createCurriculumAdoptionClient`, and the curriculum-adoption
persistence bridges as the relevant communities; source inspection confirmed those ownership
boundaries and the allocated `2026081838` through `2026081847` migration set.

- **Focused PostgreSQL/RLS oracle: passed.** The ignored
  `postgres_curriculum_adoption_live::postgres_curriculum_adoption_is_brokered_atomic_and_recoverable`
  test passed against the allocated B2 schema, including broker authority, forced RLS, atomic
  adoption and recovery, provenance/receipt persistence, and reconciliation relationships.
- **Connected browser suite: passed.** All 15 production-shaped HTTPS journeys are green, including
  direct Sysadmin and Instructor passkey entry, authorization, authoring, preview, replacement,
  item pools, grading conflicts, learner delivery, discovery evidence, curation, reusable curricula,
  adoption and rollover, WebWork, gateway recovery, and QTI import.
- **Static and deterministic gates: passed.** The five-part codebase gate, 322 Node tests, 7,361
  pytest checks, complete Rust feature/Clippy/test/doctest matrix, browser Wasm target, focused
  scenario contracts, source limits, ASCII, Markdown links, and diff hygiene are green. Independent
  post-fix review returned ACCEPT with no unresolved P0, P1, or P2 finding.
- **Real-service gates: passed.** The 77-migration PostgreSQL/RLS/persistence baseline, isolated
  WebWork scoring and outage oracle, and API replica restart/replay oracle passed with exact cleanup.
- **Screenshot publication: passed.** All 75 declared real-stack artifacts passed PNG integrity,
  privacy, provenance, atomic publication, and human visual review. Instructor and Sysadmin evidence
  uses only the 1280 by 800 desktop profile; Student evidence retains the declared variable profiles.
- **Final Validation: passed.** `source source_me.sh && ./all_test.sh` completed on the published
  material tree, including the complete Rust, Node, pytest, production-browser, PostgreSQL,
  WebWork, replica-restart, and cleanup gates.

### B2 seeded course-model correction

The approved live-demo course-model correction defines recognizable ordinary teaching courses with ordinary active
memberships and learner work: `Biochemistry: Protein Structure and Function`, `Genetics: Foundations of Inheritance`,
and `Biochemistry: Molecular Foundations`. Installer diagnostics retain an internal recipe identity, while product
surfaces use the teaching-course title. Morgan and Avery retain their separate ordinary authorization course.
Blueprints are non-enrollable personal reusable assignments, and Alpha curricula are
non-enrollable shared curricula; each name stays exclusive to its corresponding reusable aggregate.

The corrected seed distributes five deterministic learner observations across meaningful ordinary Chapter 1
assignments titled `Molecular Foundations: Charged Functional Groups` in the Genetics and Biochemistry teaching
courses. Existing item-analysis and discovery surfaces present those observations in context through the ordinary course
evidence surfaces. Course navigation presents recognizable teaching courses from active server-owned relationships:
Instructor teaching membership, Student learner membership, and the Sysadmin's direct teaching membership or audited
support relation under ASVS 8.2.2 and 8.3.1. Seeded memberships provide representative course context.

Before first production deployment, the reviewed clean-cluster baseline reissues `2026081818` with the final visible
Biochemistry teaching title, and disposable live-demo volumes are regenerated from it. The resulting checksum is the
canonical immutable v1 baseline. This is the first shipped baseline, so its coherent title and topology belong in v1;
the general accepted-migration immutability rule governs the forward-only ledger after that reset and after v1 ships.

Validation classification for this correction is explicit: focused permanent relationship tests protect course,
membership, reusable-aggregate, observation, and navigation relationships; a fresh live-stack database and visual
walkthrough supplies one-time package evidence. Screenshot publication and complete Validation are green; B2 was
accepted on 2026-08-26.

## Shared migration ledger and allocation

The release integrator owns migration ordering and this ledger. The reviewed pre-production v1 reset above is the
explicit clean-cluster baseline decision. After v1 ships, accepted files are immutable; future schema packages receive
an allocation before implementation. Non-schema packages do not receive an implicit allocation.

| Allocation                | Package               | Current disposition                                                                                               |
| ------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `2026080801`-`2026080806` | Foundational baseline | Accepted six-file baseline                                                                                        |
| `2026080907`              | `WP-RC1`              | Accepted course appearance                                                                                        |
| `2026080908`              | `WP-P2`               | Allocated secure question-grading payloads                                                                        |
| `2026080909`              | `WP-RC8`              | Allocated passwordless identity and enrollment                                                                    |
| `2026080910`              | `WP-RC7`              | Reserved object reconciliation                                                                                    |
| `2026080911`              | `WP-RC9`              | Reserved LTI Advantage                                                                                            |
| `2026080912`              | `WP-FU1`-`WP-FU6`     | Reserved secure learner uploads                                                                                   |
| `2026080914`-`2026080935` | Release packages      | Existing forward allocations                                                                                      |
| `2026081401`              | `WP-R0`               | Existing ranked-catalog allocation                                                                                |
| `2026081501`-`2026081504` | `WP-RC8` repairs      | Existing forward allocations                                                                                      |
| `2026081801`              | `WP-PROF-S2`          | Accepted term and time zone                                                                                       |
| `2026081802`              | `WP-PROF-S7`          | Accepted typed references and bylines                                                                             |
| `2026081803`              | `WP-PROF-S5`          | Accepted entitlement and materialization                                                                          |
| `2026081804`              | `WP-PROF-S3`          | Accepted effective-policy resolver                                                                                |
| `2026081805`              | `WP-PROF-S4`          | Accepted disclosure policy                                                                                        |
| `2026081806`              | `WP-PROF-S6`          | Accepted course grade scheme                                                                                      |
| `2026081807`              | `WP-PROF-T2`          | Accepted teaching operations                                                                                      |
| `2026081808`              | `WP-PROF-LD1`         | Accepted live-demo installation state                                                                             |
| `2026081809`              | `WP-PROF-LD2`         | Accepted Sysadmin candidate and completed-install brokers                                                         |
| `2026081810`              | `WP-PROF-LD2`         | Accepted Student pre-tenant context repair                                                                        |
| `2026081811`              | Reserved              | Reserved numeric identity                                                                                         |
| `2026081812`              | `WP-PROF-LD3`         | Accepted ordinary assignment mutation authority                                                                   |
| `2026081813`              | Reserved              | Reserved numeric identity                                                                                         |
| `2026081814`              | `WP-PROF-LD3`         | Accepted assignment-definition capability                                                                         |
| `2026081815`              | Reserved              | Reserved numeric identity                                                                                         |
| `2026081816`              | `WP-PROF-LD3`         | Accepted course-group mutation brokers                                                                            |
| `2026081817`              | `WP-PROF-LD3`         | Accepted learner-work source and execution snapshots                                                              |
| `2026081818`              | `WP-PROF-LD3`         | Canonical v1 course provisioning and installed-course attestation                                                 |
| `2026081819`              | `WP-PROF-LD3`         | Accepted grade control and export audit                                                                           |
| `2026081820`              | `WP-PROF-LD3`         | Accepted scoring preparation and finalization                                                                     |
| `2026081821`-`2026081822` | Reserved              | Reserved numeric identities                                                                                       |
| `2026081823`              | `WP-PROF-LD3`         | Accepted teaching-invitation mutation authority                                                                   |
| `2026081824`              | `WP-PROF-LD3`         | Accepted roster procedure ambiguity repair                                                                        |
| `2026081825`              | `WP-PROF-LD3`         | Accepted inactive-Student materialization decision                                                                |
| `2026081826`              | `WP-PROF-T5`          | Accepted pre-issue assignment-definition replacement                                                              |
| `2026081827`              | `WP-PROF-D1`          | Accepted discovery evidence and response-family projection                                                        |
| `2026081828`              | `WP-PROF-D1`          | Accepted actor usage snapshots and Library facets                                                                 |
| `2026081829`              | `WP-PROF-LD3`         | Reserved learner-work broker capability                                                                           |
| `2026081830`              | `WP-PROF-G1`          | Reserved assignment recalculation enqueue capability                                                              |
| `2026081831`              | `WP-PROF-G1`          | Reserved scoring-generation publication                                                                           |
| `2026081832`              | `WP-PROF-G3`          | Reserved item-analysis publication and cleanup                                                                    |
| `2026081833`              | `WP-PROF-T5`          | Reserved assignment-definition scratch isolation                                                                  |
| `2026081834`              | `WP-PROF-LD3`         | Reserved course-group policy broker repair                                                                        |
| `2026081835`              | `WP-PROF-LD1`         | Reserved catalog-derived Base Course freshness authority                                                          |
| `2026081836`              | `WP-PROF-D2`          | Accepted problem curation capabilities                                                                            |
| `2026081837`              | `WP-PROF-B1`          | Accepted blueprint and public Alpha capabilities                                                                  |
| `2026081838`              | `WP-PROF-B2`          | Accepted curriculum-adoption schema, lineage, schedule, provenance, receipt, integrity, and forced RLS foundation |
| `2026081839`              | `WP-PROF-B2`          | Accepted curriculum-adoption common broker authority, retention integration, and shared capability boundary       |
| `2026081840`              | `WP-PROF-B2`          | Accepted curriculum-adoption relational snapshots, locked preparation, inspection, and reconciliation helpers     |
| `2026081841`              | `WP-PROF-B2`          | Accepted canonical ordinary-course topology, issued-work fencing, and topology capability assertions              |
| `2026081842`              | `WP-PROF-B2`          | Accepted curriculum-adoption source authorization, closed request validation, and source snapshot facts           |
| `2026081843`              | `WP-PROF-B2`          | Accepted teaching-course, import, inspection, reconciliation, and controlled schedule snapshot facts              |
| `2026081844`              | `WP-PROF-B2`          | Accepted curriculum-adoption shared materializer validation, idempotency, receipt, and evidence helpers           |
| `2026081845`              | `WP-PROF-B2`          | Accepted fork, assignment adoption, fast-forward, and reconciliation materializers                                |
| `2026081846`              | `WP-PROF-B2`          | Accepted whole-course instantiation, rollover, and term-shift materializers                                       |
| `2026081847`              | `WP-PROF-B2`          | Accepted canonical public bridge completion and final broker catalog assertions                                   |
| `2026081848`              | `WP-PROF-T6`          | Allocated assignment-workspace capability migration: empty Draft/Archived definitions and Published readiness     |

`2026081803` (`S5`), `2026081804` (`S3`), and `2026081805` (`S4`) reflect the accepted
pre-file allocation reorder. Allocations `2026081811`, `1813`, `1815`, `1821`, and `1822` retain
their numeric identities. T6 owns `2026081848`; it does not consume the reserved G1/G3 migrations
`2026081830`-`2026081832`, which remain assigned to those grading capabilities. The professor plan
owns dependencies among reserved capabilities.

## Accepted package pointers

| Package                     | Current durable result                                              | Owning evidence                                                                                  |
| --------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `WP-PROF-LD1`               | Base Course installation lifecycle and retained-state rules         | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-LD2`               | Seeded entry and connected live authoring boundary                  | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-LD3`               | Ordinary live assignment, learner-work, and immutable evidence path | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-T5`                | Fixed-or-pool assignment editing and deterministic issued draws     | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-D1`                | Canonical Library discovery and evidence-backed question detail     | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-D2`                | Live curation and shared problem selection                          | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-B1`                | Revisioned Blueprints, public Alpha curricula, and shared reuse     | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-B2`                | Curriculum adoption, rollover, term shifting, and controlled update | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-R0`-`WP-R2`, `WP-PY-L1` | Accepted cross-roadmap capabilities                                 | [Release plan](active/release_completion_plan.md), [changelog](../CHANGELOG.md)                  |

## Dependency-ordered queue

The authoritative package sequence is in the [release completion plan](active/release_completion_plan.md)
and [professor capability plan](active/professor_capability_architecture_plan.md). The current
handoff is:

1. Complete `WP-PROF-T6` against its linked assignment-home, separate Questions/Policies,
   focused-mutation, and live Student-view contract.
2. Complete `WP-PROF-G1` against its automated-grading operation, exception-routing, retry,
   recalculation, and immutable-receipt contract.
3. Continue the professor plan's remaining grading-operation and final
   production-stack packages in its declared dependency order.
4. Resume the release queue at `WP-RC8`, then follow the release plan through native-family,
   learner-payload, reconciliation, LTI, upload, deployment, cost-control, and release closure
   packages.
5. Run the complete final-material-tree Validation suite before declaring the goal complete.

## Operational references

- [LIVE_DEMO_SPEC.md](../LIVE_DEMO_SPEC.md) defines the live demo behavior.
- [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) defines required Validation evidence.
- [DEVELOPMENT.md](../DEVELOPMENT.md), [INSTALL.md](../INSTALL.md), [USAGE.md](../USAGE.md), and
  [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) own operational instructions.
- The dated comparison snapshot is
  [project_status_report_2026-08-10.md](reports/project_status_report_2026-08-10.md); older status
  notes and `partial_commit_status.md` are historical references.
