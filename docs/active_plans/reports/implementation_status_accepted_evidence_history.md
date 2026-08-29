# Accepted evidence history

This report preserves closed accepted evidence from
[implementation_status.md](../implementation_status.md). It is historical evidence only and is
never current package or migration-allocation authority.

## G1-W2 accepted evidence

`WP-INST-G1 / G1-W2` is accepted on 2026-08-27 for its static/offline implementation and fresh
schema evidence. This acceptance kept `WP-INST-G1` incomplete while W3 stabilized the typed
pending/read boundary. W4 owns 1851 through 1860, W5 owns 1861 through 1865,
W7b owns executable PostgreSQL authority proof, and final `all_test.sh` remains required.

- **Accepted artifacts:** typed `SubmissionPreparation::AcceptedPending` and
  `SubmissionReceiptRead` contracts; answer-free `submission` and `submission_idempotency` parents;
  the composite-FK `accepted_submission_private_response` child; canonical UTF-8 response identity;
  equivalent Memory/PostgreSQL behavior; the dedicated worker-only execution store; separate API and
  worker process logins; and migrations 1849/1850.
- **Rust and focused evidence:** the learning-data-access full suite main target passed 308 tests
  with 1 intentionally ignored test, and auxiliary targets were green. Strict format, check, and
  Clippy gates passed. The focused policy/process/documentation/source set passed 2,008 tests.
- **Repository evidence:** `./check_codebase.sh` passed all 5 gates, including 356 Node tests.
- **Historical database evidence:** fresh PostgreSQL 17 applied all 80 migrations; the second migration pass
  was a no-op; database verification returned `database verify: compatible`. The repaired
  database-baseline
  Rust selector now resolves to exactly one intended test.
- **Independent approvals:** `sql_correctness_post_repair_review.report.md` approved the repaired
  SQL and `w2_security_post_repair_review.report.md` approved the repaired W2 source boundary. Their
  scope explicitly leaves W4 outcome behavior, W7b executable API-denial/worker-lease/RLS proof,
  browser behavior, WP-P2 grant reduction, and final G1 Validation open.

## G1-W3 accepted evidence

`WP-INST-G1 / G1-W3` is accepted on 2026-08-27 for the typed pending/read stabilization and
post-validation outcome classification. This acceptance advances the current stage to G1-W4; it
does not accept `WP-INST-G1`, whose W4-W7 work and final Validation remain required.

- **Accepted artifacts:** exhaustive `SubmissionReceiptRead` pending/read handling; the minimal
  answer-free, no-store `accepted_pending` 202 replay projection; closed deterministic grader
  failure categories and operation-reason mapping; Native, WebWork, QTI, and composite
  post-validation classification; the preserved opaque iMathAS broker boundary; and aligned Memory
  Student-attempt projection for accepted-pending detail reads.
- **Rust evidence:** `server_core` passed 384 tests with 3 intentional connected ignores, and all
  server integration and doctest targets were green. `learning-data-access` passed 308 tests with
  1 intentional ignore in its main target, with auxiliary targets green. Strict Clippy passed for
  both affected crates.
- **Repository evidence:** 3,643 documentation and source-policy checks passed. The permanent
  local route tests cover answer-free submitted projections and the no-store provider-free pending
  replay without services, timing, or fixture data.
- **Independent approvals:** the architecture and security/privacy reviews both approved the final
  W3 boundary. They confirm that W3 preserves answer-free Student data, generic deterministic
  failure handling, and the separate iMathAS broker while creating no acceptance, claim, outcome,
  job, or Student-client effect.
- **Handoff:** W4 consumes the sealed W3 pending/read and deterministic-category contracts before
  dispatching its paired first-effect, worker, and Student-status work. It owns allocated migration
  1851 schema/roles layer plus integrity, public-function authority, table authority, acquisition,
  read, load, completion-lock, commit, and fail capabilities through 1860. The aggregate
  `all_test.sh` remains the manager-owned final gate; a subagent aggregate invocation has no
  retained terminal result and is intentionally unverified.

## G1-W4 stable implementation handoff

`WP-INST-G1 / G1-W4` reached its stable implementation handoff on 2026-08-27. It advanced source
work to W5 while W7b prepared the executable PostgreSQL oracle and W7 prepared final
material-tree Validation.

- **Implemented boundary:** one immutable accepted-submission effect; split exact-fast-path and
  generic-recovery claims; type-distinct eagerly connected pools and service logins; one shared
  leased grading handler; canonical source/digest/projection evidence; atomic tuple-fenced
  load/lock/commit/fail; route-bound verified completed reads; and answer-free pending, attention,
  and completed Student projections.
- **Focused evidence:** `learning-data-access` passed 332 tests with 1 intentional connected ignore;
  `server_core` passed 413 tests with 3 intentional connected ignores; the five process-login tests
  and 1,754 source-length checks passed; strict Clippy, formatting, and diff hygiene were green.
- **Connected stabilization:** a fresh PostgreSQL 17 baseline applied all 90 tracked migrations,
  repeated migration as a no-op, passed compatibility and every registered connected phase, and
  left no disposable resources. The production-shaped headless stack then started the API, worker,
  and HTTPS gateway through the eager private-pool login, membership, and function-surface
  preflights; API and gateway were healthy, the worker remained running, and exact stop cleanup left
  no labelled container, network, or volume.
- **Independent review:** the initial review found lazy private-pool startup; the durable repair
  made typed factories eagerly connect and preflight their exact allowed and denied function
  surfaces.
  Re-review approved the resulting fail-closed composition with no remaining blocker in the W4
  source handoff.
- **Follow-on evidence:** W7b supplied
  `postgres_automated_grading_operations_live`, its database-baseline registration, exhaustive
  role/RLS/function proof, outcome and immutable-evidence behavior, ordinary-versus-worker parity,
  and the 1830-to-1831 score-publication sequence. G1-W7 completed the fresh HCI review and final
  `all_test.sh` material-tree Validation during G1 closeout.

## G1 accepted evidence

`WP-INST-G1` was accepted on 2026-08-28 after W5 through W7b, forward reconciliation, independent
review, and final material-tree Validation completed.

- **Implemented operation boundary:** the course-scoped Instructor list, retry, and recalculation
  routes use revision and idempotency fences. The immutable operation receipts and canonical
  scoring-invalidation capability keep the original Student receipt stable while the ordinary
  worker publishes only the current generation's total.
- **Student and Instructor journey:** the production HTTPS scenario submits Student work once,
  clears the browser answer buffer on `acceptedPending`, exposes **Check grading status**, routes a
  deterministic grader exception to Instructor attention, completes one visible retry, and shows
  the resulting total in the Instructor Gradebook. The focused
  `automated_grading_recovery` browser journey passed against the real stack.
- **Historical pre-reconciliation connected evidence:**
  `source source_me.sh && .venv/bin/python local_stack.py acceptance`
  passed against the historical pre-reconciliation 95-migration material tree, with the
  production browser suite, PostgreSQL baseline and oracles, isolated WebWork grading, API-replica
  restart and durable replay, and exact disposable resource cleanup.
- **Screenshot publication:** `source source_me.sh && ./capture_screenshots.sh` atomically
  published the current 63-artifact corpus after PNG, privacy, provenance, single-origin, and
  cleanup checks. The two G1 Instructor artifacts use the required 1280 by 800 desktop viewport;
  the operation artifact visibly confirms the canonical Question ID copy action.
- **Independent review:** architecture, security, and fresh G1 HCI rereviews returned ACCEPT. The
  HCI closeout found no P0/P1/P2 issue in the one-submit Student status flow, title-first copyable
  Question ID, target-specific retry, focused accepted confirmation, Student completion, or
  Gradebook propagation.
- **Forward reconciliation evidence:** accepted migration restoration and implementation of the
  four allocated forward migrations `2026081866` through `2026081869` are complete in order,
  beginning with the clean-volume fail-closed receipt preflight and ending with the V2 retry
  transition, public V1 retirement, and `DROP ... RESTRICT`. The fresh/no-op/checksum run applied
  and verified all 99 migrations; the connected G1 PostgreSQL oracle, forced-RLS inventory and role
  denials, deterministic browser recovery, isolated WebWork grading, and replica restart/durable
  replay passed with exact cleanup.
- **Final Validation:** `source source_me.sh && ./all_test.sh` passed on the final material tree.
  The exact aggregate passed Rust checks, tests, doctests, strict Clippy, and browser Wasm; all five
  frontend gates with 369 Node tests; 7,978 pytest checks; every canonical production-browser
  scenario; all 99 migrations and connected PostgreSQL/RLS/worker oracles; isolated WebWork;
  replica restart and durable replay; and exact disposable cleanup.

## T6 accepted evidence

`WP-INST-T6` was accepted on 2026-08-27. Its binding plan remains the acceptance authority, and
the completed handoff advanced to the accepted `WP-INST-G1` package.

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
  material tree, including Rust checks/tests/doctests/Wasm, frontend/codebase/Node, 7,428 pytest
  cases, all 15 production browser scenarios, all 78 migrations and database oracles, isolated
  WebWork, and replica restart/durable replay. The six durable closure paths formed part of that
  material tree:
  `crates/server/src/course/tests/assignment_revision/replacement.rs`,
  `docs/screenshots/instructor/assignment_workspace/01_assignment_policies.png`,
  `docs/screenshots/instructor/assignment_workspace/02_student_view.png`,
  `src/pages/assignment_workspace/assignment_workspace_authoring.css`,
  `tests/test_assignment_workspace_policy_summary.mjs`, and
  `tests/test_assignment_workspace_replacement_client.mjs`.

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
  item pools, grading conflicts, Student delivery, discovery evidence, curation, reusable curricula,
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
memberships and Student work: `Biochemistry: Protein Structure and Function`, `Genetics: Foundations of Inheritance`,
and `Biochemistry: Molecular Foundations`. Installer diagnostics retain an internal recipe identity, while product
surfaces use the teaching-course title. Morgan and Avery retain their separate ordinary authorization course.

The accepted B2 seed receipt records the pre-SD1 split between non-enrollable Blueprints and shared Alpha curricula.
The SD1 target consolidates reusable course content into one `BlueprintCourse` aggregate and delivery into
`CourseInstance`. ADAPT Alpha terminology remains comparison history; PLE uses `BlueprintCourse` and
`CourseInstance` for the durable model. Every CourseInstance binds to one BlueprintCourse parent, while the global
question corpus supplies its published question versions.

The corrected seed distributes five deterministic Student observations across meaningful ordinary Chapter 1
assignments titled `Molecular Foundations: Charged Functional Groups` in the Genetics and Biochemistry teaching
courses. Existing item-analysis and discovery surfaces present those observations in context through the ordinary course
evidence surfaces. Course navigation presents recognizable teaching courses from active server-owned relationships:
Instructor teaching membership, Student membership, and the Sysadmin's direct teaching membership or audited
support relation under ASVS 8.2.2 and 8.3.1. Seeded memberships provide representative course context.

Before first production deployment, the reviewed clean-cluster baseline reissues `2026081818` with the final visible
Biochemistry teaching title, and disposable live-demo volumes are regenerated from it. The resulting checksum is the
canonical immutable v1 baseline. This is the first shipped baseline, so its coherent title and topology belong in v1;
the general accepted-migration immutability rule governs the forward-only ledger after that reset and after v1 ships.

Validation classification for this correction is explicit: focused permanent relationship tests protect course,
membership, reusable-aggregate, observation, and navigation relationships; a fresh live-stack database and visual
walkthrough supplies one-time package evidence. Screenshot publication and complete Validation are green; B2 was
accepted on 2026-08-26.

