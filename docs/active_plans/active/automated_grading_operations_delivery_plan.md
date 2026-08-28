# Plan: automated grading operations delivery

## Status

This execution companion is governed by the binding
[automated grading operations plan](automated_grading_operations_plan.md).
That parent remains the source of truth for scope, architecture, contracts, dependency order, and
migration allocation. This document owns the detailed W5-W7 delivery sequence, evidence,
acceptance, and handoff. It preserves the approved scope, including migrations 2026081861
through 2026081865.

## Delivery work packages

### G1-W5: compose strict course-scoped operations HTTP

- **Owner/package:** expert coder, `WP-PROF-G1 / G1-W5`, operation capability
  and server route boundary.
- **Depends on:** G1-W4 stable worker handoff and its passed migration
  stabilization gate. Migrations `2026081861_instructor_grading_operation_capabilities.sql`
  through `2026081865_scoring_invalidation_source_bindings.sql` are allocated before source edits.
- **Owned artifacts:** dedicated `GradingOperationStore` contracts and safe
  values in `crates/learning-data-access/src/contracts/grading_operations.rs`
  and `grading_operations.rs`; Memory/Postgres implementations; focused
  `crates/server/src/course/grading_operations.rs`,
  `crates/server/src/course/routing.rs`, `crates/server/src/composition/router.rs`,
  narrow `crates/server/src/route_policy.rs` entries, and route tests; migrations
  `2026081861_instructor_grading_operation_capabilities.sql`,
  `2026081862_grading_operation_lifecycle_projection.sql`,
  `2026081863_scoring_invalidation_origin.sql`, and
  `2026081864_scoring_invalidation_capability.sql`, plus
  `2026081865_scoring_invalidation_source_bindings.sql`.
- **Required behavior:** the course router adds the explicit bound
  `S: Store + CourseRecordsAccessStore + SessionStore + GradingOperationStore + 'static`;
  it does not extend global `Store` or `AutomatedGradingStore`. Memory and
  PostgreSQL expose equivalent safe Instructor projections. Bounded list rows are W5's complete
  metadata detail; G2 owns protected learner-work detail. Retry and recalculation are
  body-free, `If-Match`/idempotency-key guarded actions. Each store operation
  rechecks session-derived Instructor authority, tenant, course, and assignment
  inside its transaction. Retry uses W4. One canonical scoring-invalidation capability owns
  every recalculation origin, its immutable causal record, generation, worker job, operation
  thread, replay, and supersession; it delegates enqueue to 1830. Source-specific wrappers derive
  causal identity and actor evidence from authoritative domain receipts, session authority, or
  worker completion rather than accepting caller assertions. Migration 1831 remains the sole
  current-score publisher while worker transitions close or reopen the matching operation thread.
- **Permanent offline gate:** Memory/Postgres contract parity and route tests
  cover Instructor success, Student/outsider/foreign-tenant/revoked concealment,
  in-transaction session recheck, stable ordering, cursor/revision bounds,
  body-free header actions, exact replay/conflict, no-store, and strict
  answer-free serialization. Origin tests cover Instructor, definition/content, manual-grade,
  learner-support, and accepted-completion invalidations, same-origin replay, supersession, and
  exact terminal projection. Tests use controlled IDs/revisions and no services,
  sleeps, current time, or score-bearing fixtures.
- **Connected one-time gate:** exercise list, retry, and recalculation as Elena
  against the disposable stack; deny the same calls as a Student and unrelated
  user; verify metadata-only responses and a gradebook change through the
  worker's 1830/1831 path.
- **Handoff:** send W6 exact paths, DTO/decoder shapes, empty/loading/action-
  in-progress/error states, accessible action names, and revision recovery.
  Send W7a the Instructor setup and expected safe network receipts.

### G1-W6: build the assignment-local Instructor surface

- **Owner/package:** SolidJS/TypeScript engineer, `WP-PROF-G1 / G1-W6`.
- **Depends on:** G1-W5 DTOs and accepted T6 workspace seam.
- **Owned artifacts:** `src/api/decoders/grading_operations.ts`,
  `src/api/http_client/grading_operations.ts`,
  `src/pages/assignment_workspace/assignment_workspace_operations_model.ts`,
  `src/pages/assignment_workspace/assignment_workspace_operations_page.tsx`,
  its focused stylesheet, `src/routes.ts`, `src/route_contract.ts`, workspace
  navigation, and focused Node tests.
- **Required behavior:** use the W5 production route and strict decoder; show
  question-first groups with a learner alternate, safe states, named retry and
  recalculate controls, receipts, focus recovery, and no protected learner
  material. The page is production-shaped and usable at the Instructor-only
  1280x800 desktop profile.
- **Permanent offline gate:** TypeScript typecheck/lint and pure decoder/model
  tests cover unknown or answer/response/evaluation/feedback/score fields,
  path/navigation, cursor reset, action transitions, keyboard names, and error
  recovery. Keep these tests deterministic and network-free.
- **Connected one-time gate:** use the built `dist/` app on the real HTTPS stack,
  visible route navigation, and visible controls; capture the Instructor-only
  1280x800 screenshot and provenance record. A mock-backed browser result does
  not satisfy this gate.
- **Handoff:** send W7a stable selectors, accessible names, route entry, state
  transitions, and screenshot provenance contract.

### G1-W7a: prove the visible recovery journey

- **Owner/package:** Playwright/integration engineer, `WP-PROF-G1 / G1-W7a`.
- **Depends on:** G1-W6 and W4's deterministic fault seam.
- **Owned artifacts:** `tests/playwright/e2e/automated_grading_recovery.spec.ts`,
  `tests/e2e/e2e_browser_scenarios_failure.py`,
  `tests/e2e/e2e_browser_scenario_contract.py`,
  `tests/e2e/e2e_browser_scenario_execution.py`,
  `tests/e2e/e2e_browser_fault_orchestrator.py`,
  `tests/e2e/e2e_browser_suite_owner.py`, scenario registration, and
  `tests/e2e/browser_screenshot_corpus.json` plus its provenance source.
- **Required behavior:** extend the closed fault orchestrator with the
  `deterministic_grader_exception` transition. Elena visibly creates or selects
  ordinary live course/assignment/question state; the designated Student
  visibly submits; accepted work visibly enters `acceptedPending`, clears its
  answer buffer, and uses **Check grading status** without another answer POST.
  The acceptance-only worker commits one deterministic exception and the
  Student status reaches `instructorAttention`. Elena then opens Grading
  operations and performs exactly one visible retry. The ordinary worker
  completes the new execution generation, the Student status reaches
  `completed`, and Elena observes the current Gradebook total. Browser traffic
  and operation views remain answer-free.
- **Permanent offline gate:** validate scenario/fault schema, selector and
  accessible-name invariants, and screenshot provenance schema with deterministic
  checks. Keep browser execution and screenshots out of the offline pytest lane;
  use no sleeps, pixel equivalence, or incidental collection counts.
- **Connected one-time gate:** run
  `source source_me.sh && python3 local_stack.py acceptance` against the real
  built HTTPS stack, with the visible Student submission and Instructor recovery
  flow. Publish Instructor-only 1280x800 evidence and exact provenance.
  Acceptance uses the built `dist/`, activates the deterministic fault, and
  observes the score produced by the ordinary grading worker.
- **Handoff:** send W7b the public-safe scenario correlation and browser/network
  receipts. W7b resolves its internal submission, job, lease, execution, and
  score identities inside the PostgreSQL oracle. Send closeout the exact
  command, screenshot provenance, and semantic review.

### G1-W7b: prove PostgreSQL and worker recovery

- **Owner/package:** expert coder, `WP-PROF-G1 / G1-W7b`, PostgreSQL and
  persistence-evidence boundary.
- **Depends on:** G1-W4 exact claim/outcome contract, G1-W5 route policy, and
  W2 migrations. W7a supplies an independent visible journey for closeout.
- **Owned artifacts:**
  `crates/learning-data-access/tests/postgres_automated_grading_operations_live.rs`,
  `tests/e2e/e2e_database_baseline.sh`, and the migration catalog assertions
  for 1849/1850 and the ordered W4 migrations 1851 through 1860. W7b registers the oracle and owns its receipt.
- **Required behavior:** on a fresh database migrated twice, prove worker-only
  claim/load/commit/fail, role and forced-RLS closure, exact lease and tenant
  fences, retention and issued-evidence checks, duplicate/stale/superseded
  rejection with no lost-claim mutation, append-only receipts, and API denial
  of private reads and execution-role assumption. For every W4 evidence source
  text/projection pair, verify SHA-256 over the stored UTF-8 source text and
  structural equality with its JSONB projection; cover normal, manual, and
  automated receipt writers. Compare ordinary synchronous completion and
  accepted-worker completion for equal run, enrollment, and scalar summary
  results. Drive the normal scoring worker to show 1830 enqueues and 1831 alone
  publishes assignment/course current scores.
  Use explicit IDs, tokens, workers, generations, and controlled lease state.
  Execute the binding contract's focused connected proof.
  Its final-acknowledgement proof separates a decoded successful function result
  from a forced final `transaction.commit()` acknowledgement failure; known
  function/statement failures retain their exact `Known(StoreError)` result.
- **Permanent offline gate:** no new connected behavior is promoted into the
  fast lane. W2-W5 Memory, contract, route, and readiness tests remain the
  permanent deterministic boundary; migration checks are deterministic
  schema-self-verification and do not mock production privileges.
- **Connected one-time gate:** register and run the exact live oracle through
  the canonical fresh-database baseline, including role/catalog denial,
  worker success, claim competition, retention, retry/exhaustion, and score
  publication. A zero-test or skipped test is failure.
- **Handoff:** send W7 SQL/test receipt, role matrix, correlated operation/
  execution/score IDs, and residual risks. W7 runs complete Validation on the
  final material tree.

### G1-W7: close G1

- **Owner:** integrator.
- **Outcome:** independent review and final material-tree evidence establish the entire G1 loop.
- **Owned files/modules:** [implementation_status.md](../implementation_status.md), this plan,
  [CHANGELOG.md](../../CHANGELOG.md), final evidence records, and warranted operational docs.
- **Dependencies:** W7a and W7b.
- **Implementation steps:** consume the browser and PostgreSQL handoffs; run independent
  architecture/security and HCI reviews; reconcile documentation; run final Validation on the exact
  material tree; and record commands, results, and remaining evidence honestly.
- **Permanent tests:** none; this package closes evidence already owned by W2-W7b.
- **One-time/connected evidence:** independent reviews and final `all_test.sh`.
- **Success criteria:** Instructor-visible action routes safe recovery to a current total without
  human scoring, every required final gate is green, and the ledger points to reproducible evidence.
- **Handoff:** G2 consumes operation/receipt links; G3 consumes independent analysis publication;
  G5 consumes only actionability-qualified operations.


## Validation and evidence matrix

- **Accepted input survives grade failure.** Permanent: Memory lifecycle, replay, and retention
  contract tests. Connected: `postgres_automated_grading_operations_live` private-payload/RLS
  oracle.
- **Response identity is cross-store exact.** Permanent: canonical helper variants and Memory digest
  regression. Connected: the PostgreSQL broker returns the same digest for the same typed response.
- **Private response and worker authority are separated.** Permanent: inline Memory, typed
  login-profile/role/store composition, canonical serialization/digest, and dedicated-store
  capability tests. One-time: fresh migrate-twice/verifier; migration `DO` catalog assertions are
  schema self-verification. Connected: W7b PostgreSQL catalog plus executable API-denial and
  worker-success oracle.
- **Immutable evidence remains verifiable.** Permanent: the versioned Rust
  encoder and reader tests prove source-text digest and typed/projection
  coherence. Connected: W7b verifies the stored source-text/hash and JSONB
  projection on the freshly migrated real database for normal, manual, and
  automated writers.
- **Operations are typed and answer-free.** Permanent: domain and decoder rejection tests.
  One-time: independent architecture review.
- **Actions are immutable and replay-safe.** Permanent: Memory receipt immutability, exact replay,
  and changed conflict. Connected: concurrent duplicate-action PostgreSQL oracle.
- **Tenant/course/role isolation holds.** Permanent: Store and route foreign, learner, outsider, and
  revoked-member cases. Connected: canonical unauthorized probe.
- **Exception routing is exact.** Permanent: submission classification matrix. Connected: the
  controlled deterministic failure.
- **Acknowledged learner work is recoverable.** Permanent: W4 tagged-union decoder and client-state
  tests cover 202 buffer clearing, `acceptedPending`, and status-read recovery without answer POST
  replay. Connected: W7a visibly completes pending-to-completed recovery on the HTTPS stack.
- **Commit outcomes are diagnosable.** Permanent: W4 proves known error
  propagation, injected acknowledgement ambiguity, and cancellation-safe
  timeout-to-one-`TimedOut`. Connected: W7b proves known function failure and
  the final PostgreSQL acknowledgement fault separately.
- **One derived-score path remains.** Permanent: pure handler classification, generation-fence, and
  lease-loss disposition tests. Connected: PostgreSQL lease/replay and competing-generation oracle,
  including controlled expired/superseded claims whose committer cannot mutate execution,
  evaluation, or score state.
- **Automated and human grading stay separate.** Permanent: route and Store capability tests.
  One-time: independent security call-path review.
- **The Instructor task is learnable.** Permanent: client/page model, action, and focus tests.
  Connected: `automated_grading_recovery` HTTPS journey and 1280 by 800 semantic review.
- **The current total is trustworthy.** Permanent: scoring conformance plus G1 receipt/generation
  assertions. Connected: Instructor action through worker to Gradebook observation.

Permanent tests follow the repository admission rule: stable regression-prone behavior,
deterministic offline execution, no service calls/sleeps/current-time dependence, and test-owned
state. [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) owns the complete evidence classes.
Graphify reports, migration snapshots, source inventories, query plans, screenshots, and
provenance are one-time evidence rather than permanent tests.

## Risks

- **Ephemeral retry input:** W1/W2 make persisted `submission` the private authority before grading.
- **Unclear accepted-response retention:** W1 assigns the relationship; W2 enforces it in schema and
  Store policy.
- **Receipt/state becomes a competing grader:** W2/W4 keep it metadata/evidence only and retain the
  1830/1831 derived-score path.
- **Concurrent actions double-advance generation:** W2 serializes action/enqueue with idempotency and
  revision; W4 preserves both execution and scoring generation fences.
- **Human scoring leaks into recovery:** W5 supplies only typed automated commands and proves
  structural separation.
- **Dense UI obscures action:** W6 uses question-first hierarchy, named action, status band, and
  focus recovery.
- **Scope grows into G2/G3/G5:** W7 records typed downstream handoffs instead of copying those
  capabilities.

## Success criteria

G1 is accepted on the final material tree when:

1. Shape-valid accepted learner input becomes an immutable server-private `submission` before
   grading; invalid input remains pre-persistence and successful fast grading retains ordinary
   learner behavior.
2. A `202 Accepted` learner outcome clears the response buffer, enters `acceptedPending`, and gives
   the learner visible **Check grading status** recovery without another answer POST; the bound
   status GET reaches the same `completed` receipt projection after execution commits.
3. A deterministic grader exception creates one answer-free operation visible to the current
   Instructor by question and learner, while invalid input and outages retain their own behavior.
4. Exact action replay returns the original receipt; changed replay conflicts; duplicates create
   neither a second receipt nor an extra generation.
5. Retry uses accepted immutable input, recalculation uses 1830, and visible totals publish only
   through the existing private-stage, lease-checked, generation-fenced 1831 path.
6. Original learner receipts remain unchanged; stale generations cannot publish; foreign, learner,
   and revoked callers receive no operation fact or mutation.
7. The canonical HTTPS stack shows the learner pending recovery, Instructor recovery loop, and
   current Gradebook total through
   visible UI actions at the required 1280 by 800 desktop acceptance viewport.
8. Independent architecture/security and HCI review accept the implementation, and
   `source source_me.sh && ./all_test.sh` is green on the exact final material tree.

## Documentation closeout

G1-W7 records migration identities, implemented routes, permanent-test ownership, connected
commands/results, screenshot provenance, review outcomes, and final Validation in implementation
status, this plan, and the changelog. Operational docs change only for a shipped Instructor-visible
live-demo step. G2/G3/G5 link to accepted contracts rather than duplicate them.

## Dispatch-ready sequence

1. Assign W1 to the architect and release-integrator collaboration; approve the accepted-input
   lifecycle and record migrations 1849/1850 in the status ledger.
2. Assign W2 to the persistence owner after W1 freezes types and allocations.
3. Assign W3 after W2 exposes the accepted-input/operation contract, then W4 after W3 publishes its
   closed exception/retry classification.
4. Assign W5 after W2-W4 stabilize action outcomes and authorization data.
5. Assign W6 after W5 publishes Instructor DTOs and paths; keep Instructor browser ownership
   confined to its modules while W4 retains learner delivery recovery.
6. Assign W7a after the visible action path exists and W7b after the worker/HTTP boundaries exist.
   Use the production-shaped stack and visible UI actions for product state, with the deterministic
   fault as the bounded harness setup.
7. Assign W7 after both connected-evidence owners deliver their artifacts and results; have the
   integrator run independent reviews and final Validation on that exact material tree.
