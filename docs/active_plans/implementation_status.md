# Implementation status and handoff

Last updated: 2026-08-27

This is the sole mutable registry for the global current-package handoff and shared migration
allocations. The [implementation plan](implementation_plan.md), active
[professor capability plan](active/professor_capability_architecture_plan.md), and active
[release completion plan](active/release_completion_plan.md) own architecture, scope, dependency
order, validation, and acceptance. Durable product decisions remain in
[Human Guidance](../HUMAN_GUIDANCE.md); package history and detailed receipts remain in the
[changelog](../CHANGELOG.md).

## Current handoff

- **Current package:** `WP-PROF-G1` - the automated-grading operation boundary. W1, W2, and W3
  are accepted below. `G1-W5` is the current dependency-ordered implementation stage: it owns
  the course-scoped Instructor operation list, retry, and generation-fenced recalculation
  capability and canonical scoring-invalidation ownership in allocated migrations 1861 through
  1865. W4's source and migration handoff is stable: the API
  exact fast path and background recovery use distinct eagerly attested private pools, one common
  tuple-fenced handler, canonical completed evidence, and the route-bound learner status reader
  across migrations 1851 through 1860. W4 acceptance remains open for W7b's contract-named
  executable PostgreSQL oracle and final Validation; that later joint evidence does not block W5
  from consuming the stable W4 operation boundary. The
  [G1 binding plan](active/automated_grading_operations_plan.md) makes immutable accepted
  `submission`/`submission_idempotency` metadata parents authoritative before grading, keeps their
  payload marker answer-free, and stores the canonical response only in a composite-FK private
  child. G1 remains incomplete until W4-W7 and final Validation pass. Existing `WP-P2` owns the
  later consumer-by-consumer replacement of legacy broad reads and corresponding grant reductions.
- **Current acceptance predecessor:** `WP-PROF-B2` accepted 2026-08-26. Its focused adoption boundary
  now owns preview-before-save fork and instantiation, rollover, term shifting, immutable provenance,
  controlled fast-forward, divergence recovery, and answer-free import inspection over ordinary
  teaching courses. Its final receipt is in the professor plan and changelog: all 77 migrations and
  Store/RLS oracles; all 15 production HTTPS journeys with independent Elena Instructor and Morgan
  Sysadmin passkeys; its then-current 75-artifact privacy-validated corpus; exact cleanup; and final
  Validation passed.
- **Accepted prerequisites:** `WP-PROF-S1` through `S7`, `T1` through `T3`, `BS1`, `LD1` through
  `LD3`, `T5`, `D1`, `D2`, `B1`, and `B2` are accepted. Their scopes and evidence are retained in
  the owning plans and changelog.
- **Release handoff:** `WP-RC8` remains parked and acceptance-open. It owns provider/mailbox,
  unrelated passkey, multi-replica, security, HCI, and release gates. Professor live-demo work does
  not imply production onboarding, deployment, or release acceptance.

## G1-W2 accepted evidence

`WP-PROF-G1 / G1-W2` is accepted on 2026-08-27 for its static/offline implementation and fresh
schema evidence. This acceptance kept `WP-PROF-G1` incomplete while W3 stabilized the typed
pending/read boundary. W4 owns 1851 through 1860, W5 owns 1861 through 1865, W7b owns executable PostgreSQL authority proof,
and final `all_test.sh` remains required.

- **Accepted artifacts:** typed `SubmissionPreparation::AcceptedPending` and
  `SubmissionReceiptRead` contracts; answer-free `submission` and `submission_idempotency` parents;
  the composite-FK `accepted_submission_private_response` child; canonical UTF-8 response identity;
  equivalent Memory/PostgreSQL behavior; the dedicated worker-only execution store; separate API and
  worker process logins; and migrations 1849/1850.
- **Rust and focused evidence:** the learning-data-access full suite main target passed 308 tests
  with 1 intentionally ignored test, and auxiliary targets were green. Strict format, check, and
  Clippy gates passed. The focused policy/process/documentation/source set passed 2,008 tests.
- **Repository evidence:** `./check_codebase.sh` passed all 5 gates, including 356 Node tests.
- **Database evidence:** fresh PostgreSQL 17 applied all 80 migrations; the second migration pass was
  a no-op; database verification returned `database verify: compatible`. The repaired database-baseline
  Rust selector now resolves to exactly one intended test.
- **Independent approvals:** `sql_correctness_post_repair_review.report.md` approved the repaired
  SQL and `w2_security_post_repair_review.report.md` approved the repaired W2 source boundary. Their
  scope explicitly leaves W4 outcome behavior, W7b executable API-denial/worker-lease/RLS proof,
  browser behavior, WP-P2 grant reduction, and final G1 Validation open.

## G1-W3 accepted evidence

`WP-PROF-G1 / G1-W3` is accepted on 2026-08-27 for the typed pending/read stabilization and
post-validation outcome classification. This acceptance advances the current stage to G1-W4; it
does not accept `WP-PROF-G1`, whose W4-W7 work and final Validation remain required.

- **Accepted artifacts:** exhaustive `SubmissionReceiptRead` pending/read handling; the minimal
  answer-free, no-store `accepted_pending` 202 replay projection; closed deterministic grader
  failure categories and operation-reason mapping; Native, WebWork, QTI, and composite
  post-validation classification; the preserved opaque iMathAS broker boundary; and aligned Memory
  learner-attempt projection for accepted-pending detail reads.
- **Rust evidence:** `server_core` passed 384 tests with 3 intentional connected ignores, and all
  server integration and doctest targets were green. `learning-data-access` passed 308 tests with
  1 intentional ignore in its main target, with auxiliary targets green. Strict Clippy passed for
  both affected crates.
- **Repository evidence:** 3,643 documentation and source-policy checks passed. The permanent
  local route tests cover answer-free submitted projections and the no-store provider-free pending
  replay without services, timing, or fixture data.
- **Independent approvals:** the architecture and security/privacy reviews both approved the final
  W3 boundary. They confirm that W3 preserves answer-free learner data, generic deterministic
  failure handling, and the separate iMathAS broker while creating no acceptance, claim, outcome,
  job, or learner-client effect.
- **Handoff:** W4 consumes the sealed W3 pending/read and deterministic-category contracts before
  dispatching its paired first-effect, worker, and learner-status work. It owns allocated migration
  1851 schema/roles layer plus integrity, public-function authority, table authority, acquisition, read, load, completion-lock, commit, and fail capabilities through 1860. The aggregate `all_test.sh` remains the
  manager-owned final gate; a subagent aggregate invocation has no retained terminal result and is
  intentionally unverified.

## G1-W4 stable implementation handoff

`WP-PROF-G1 / G1-W4` reached its stable implementation handoff on 2026-08-27. This advances source
work to W5 while keeping W4 acceptance open for W7b's full executable PostgreSQL oracle and the
final material-tree Validation gate.

- **Implemented boundary:** one immutable accepted-submission effect; split exact-fast-path and
  generic-recovery claims; type-distinct eagerly connected pools and service logins; one shared
  leased grading handler; canonical source/digest/projection evidence; atomic tuple-fenced
  load/lock/commit/fail; route-bound verified completed reads; and answer-free pending, attention,
  and completed learner projections.
- **Focused evidence:** `learning-data-access` passed 332 tests with 1 intentional connected ignore;
  `server_core` passed 413 tests with 3 intentional connected ignores; the five process-login tests
  and 1,754 source-length checks passed; strict Clippy, formatting, and diff hygiene were green.
- **Connected stabilization:** a fresh PostgreSQL 17 baseline applied all 90 tracked migrations,
  repeated migration as a no-op, passed compatibility and every registered connected phase, and
  left no disposable resources. The production-shaped headless stack then started the API, worker,
  and HTTPS gateway through the eager private-pool login, membership, and function-surface
  preflights; API and gateway were healthy, the worker remained running, and exact stop cleanup left
  no labelled container, network, or volume.
- **Independent review:** the initial review found lazy private-pool startup; the durable repair made
  typed factories eagerly connect and preflight their exact allowed and denied function surfaces.
  Re-review approved the resulting fail-closed composition with no remaining blocker in the W4
  source handoff.
- **Open acceptance evidence:** W7b still owns
  `postgres_automated_grading_operations_live`, its database-baseline registration, exhaustive
  role/RLS/function proof, outcome and immutable-evidence behavior, ordinary-versus-worker parity,
  and the 1830-to-1831 score-publication sequence. Final `all_test.sh` remains required after W5-W7.

## T6 accepted evidence

`WP-PROF-T6` was accepted on 2026-08-27. Its binding plan remains the acceptance authority, and
the ledger advances to `WP-PROF-G1` after the exact final tracked-tree Validation gate passed.

- **Focused architecture and contracts: passed.** Migration `2026081848`, persisted incomplete
  Drafts, focused Questions and Policies commands, strict shared revisions, publication readiness,
  answer-free Student view, generic unexpected-error mapping, and the fixed-slot replacement route
  pass focused Rust, TypeScript, Node, lint, format, source-size, and static policy gates. The
  focused suite includes 19 Node tests and 375 runnable server tests; future-run replacement
  preserves issued snapshots while changing the authoritative question.
- **Connected live-demo journey: passed.** The production-shaped HTTPS owner passed the complete
  visible scenario selection, including independent Instructor and Sysadmin passkeys, assignment
  workspace authoring, same-assignment Student submission and Instructor gradebook observation,
  fixed-slot replacement, recovery, item pools, discovery, curation, reusable curricula, and
  curriculum adoption. Complete local-stack acceptance passed all 15 browser scenarios, the
  78-migration/DB oracle, WebWork oracle, replica restart, and exact cleanup.
- **Screenshot publication: passed.** The current 61-artifact corpus passed PNG, privacy, provenance,
  single-origin, atomic-publication, exact-cleanup, and human visual review. Instructor and Sysadmin
  evidence remains 1280 by 800 desktop-only; Student evidence retains its declared variable
  profiles.
- **Independent review: passed.** Final architecture/security and HCI/accessibility reviews return
  ACCEPT with no unresolved P0, P1, or P2 finding. The shared browser client enforces `no-store`
  for editor responses, and Questions provides title-bound controls and an accessible replacement
  summary.
- **Final Validation: passed.** `source source_me.sh && ./all_test.sh` passed on the exact final
  tracked tree, including Rust checks/tests/doctests/Wasm, frontend/codebase/Node, 7,428 pytest
  cases, all 15 production browser scenarios, all 78 migrations and database oracles, isolated
  WebWork, and replica restart/durable replay. The six durable closure paths were tracked as part
  of that material tree:
  `crates/server/src/course/tests/assignment_revision/replacement.rs`,
  `docs/screenshots/instructor/assignment_workspace/01_assignment_policies.png`,
  `docs/screenshots/instructor/assignment_workspace/02_student_view.png`,
  `src/pages/assignment_workspace/assignment_workspace_authoring.css`,
  `tests/test_assignment_workspace_policy_summary.mjs`, and
  `tests/test_assignment_workspace_replacement_client.mjs`.

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
- **Screenshot publication: passed.** At B2 acceptance, all 75 declared real-stack artifacts passed
  PNG integrity, privacy, provenance, atomic publication, and human visual review. Instructor and
  Sysadmin evidence used only the 1280 by 800 desktop profile; Student evidence retained the
  declared variable profiles.
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
| `2026080908`              | `WP-P2`               | Allocated secure question-grading payloads and the post-G1-W2 legacy-consumer/grant-reduction transition          |
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
| `2026081849`              | `WP-PROF-G1`          | Accepted W2 operation/evaluation/execution schema prerequisite and receipts                                     |
| `2026081850`              | `WP-PROF-G1`          | Accepted W2 private accepted-response, acceptance/replay, retention/RLS, and lease-fenced execution boundary    |
| `2026081851`              | `WP-PROF-G1 / G1-W4` | Schema and roles; proof: fresh schema/role shape query                                                            |
| `2026081852`              | `WP-PROF-G1 / G1-W4` | Integrity guards and triggers; proof: immutable-write rejection                                                   |
| `2026081853`              | `WP-PROF-G1 / G1-W4` | Public function authority; proof: effective catalog closes PUBLIC/default EXECUTE and legacy v1 load                 |
| `2026081854`              | `WP-PROF-G1 / G1-W4` | Witness/RLS/table authority and receipt version SELECT; proof: exact authority and ACL matrix                      |
| `2026081855`              | `WP-PROF-G1 / G1-W4` | Split generic/exact claim and ready/max convergence; proof: one winner and sibling denial                          |
| `2026081856`              | `WP-PROF-G1 / G1-W4` | Four-key structural verified read; proof: entitled route succeeds and changed key fails                            |
| `2026081857`              | `WP-PROF-G1 / G1-W4` | Exact private execution load; proof: exact claim loads once and rejects mismatches                                |
| `2026081858`              | `WP-PROF-G1 / G1-W4` | Completion lock; proof: lock fences stale or duplicate completion                                                  |
| `2026081859`              | `WP-PROF-G1 / G1-W4` | Commit-v2; proof: full 36-input signature commits one immutable aggregate                                          |
| `2026081860`              | `WP-PROF-G1 / G1-W4` | Fail; proof: NULL-safe closed failure validation preserves invalid-call state                                      |
| `2026081861`              | `WP-PROF-G1 / G1-W5` | W5 Instructor grading-operation capability and broker surface                                                      |
| `2026081862`              | `WP-PROF-G1 / G1-W5` | W5 worker-authoritative grading-operation lifecycle projection                                                     |
| `2026081863`              | `WP-PROF-G1 / G1-W5` | W5 immutable scoring-invalidation origin evidence                                                                  |
| `2026081864`              | `WP-PROF-G1 / G1-W5` | W5 canonical generation, job, operation, and supersession capability                                               |
| `2026081865`              | `WP-PROF-G1 / G1-W5` | W5 source-specific invalidation witnesses and least-privilege adapters                                             |

`2026081803` (`S5`), `2026081804` (`S3`), and `2026081805` (`S4`) reflect the accepted
pre-file allocation reorder. Allocations `2026081811`, `1813`, `1815`, `1821`, and `1822` retain
their numeric identities. T6 owns `2026081848`; G1 accepted `2026081849` and `2026081850` in
addition to reserved enqueue/publication capabilities `2026081830` and `2026081831`. G3 retains
`2026081832`. G1-W4 owns ordered forward allocations `2026081851` through `2026081860`: schema/roles,
integrity, public-function authority, table authority, claim, read, load, completion lock, commit,
then fail. G1-W5 owns `2026081861` through `2026081865`: Instructor operations, lifecycle
projection, immutable invalidation origins, the canonical invalidation capability, and
source-specific least-privilege witnesses.
None changes W2's accepted status or rewrites an accepted
migration. The professor plan owns dependencies among reserved capabilities.

## Accepted package pointers

| Package                     | Current durable result                                                    | Owning evidence                                                                                  |
| --------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `WP-PROF-LD1`               | Base Course installation lifecycle and retained-state rules               | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-LD2`               | Seeded entry and connected live authoring boundary                        | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-LD3`               | Ordinary live assignment, learner-work, and immutable evidence path       | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-T5`                | Fixed-or-pool assignment editing and deterministic issued draws           | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-T6`                | Accepted assignment workspace, focused replacement, and live Student view | [T6 plan](active/instructor_assignment_workspace_plan.md), [changelog](../CHANGELOG.md)          |
| `WP-PROF-D1`                | Canonical Library discovery and evidence-backed question detail           | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-D2`                | Live curation and shared problem selection                                | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-B1`                | Revisioned Blueprints, public Alpha curricula, and shared reuse           | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-PROF-B2`                | Curriculum adoption, rollover, term shifting, and controlled update       | [Professor plan](active/professor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-R0`-`WP-R2`, `WP-PY-L1` | Accepted cross-roadmap capabilities                                       | [Release plan](active/release_completion_plan.md), [changelog](../CHANGELOG.md)                  |

## Dependency-ordered queue

The authoritative package sequence is in the [release completion plan](active/release_completion_plan.md)
and [professor capability plan](active/professor_capability_architecture_plan.md). The current
handoff is:

1. Implement `WP-PROF-G1` from its approved binding plan. W2 and W3 are accepted and W4 has a
   stable implementation handoff; continue current stage G1-W5, then follow W6 and W7 in
   dependency order. Prove deterministic replay, current-total
   recalculation, and visible exception recovery.
2. Continue the professor plan's remaining grading-operation and final
   production-stack packages in its declared dependency order.
3. Resume the release queue at `WP-RC8`, then follow the release plan through native-family,
   learner-payload, reconciliation, LTI, upload, deployment, cost-control, and release closure
   packages.
4. Run the complete final-material-tree Validation suite before declaring the goal complete.

## Operational references

- [LIVE_DEMO_SPEC.md](../LIVE_DEMO_SPEC.md) defines the live demo behavior.
- [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) defines required Validation evidence.
- [DEVELOPMENT.md](../DEVELOPMENT.md), [INSTALL.md](../INSTALL.md), [USAGE.md](../USAGE.md), and
  [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) own operational instructions.
- The dated comparison snapshot is
  [project_status_report_2026-08-10.md](reports/project_status_report_2026-08-10.md); older status
  notes and `partial_commit_status.md` are historical references.
