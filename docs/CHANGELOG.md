# Changelog

## 2026-08-29

### Fixes and Maintenance

- Accepted `WP-INST-WN1-SR3-student-run-store-capability`. Run and Store capabilities, Memory and
  PostgreSQL modules, routing bindings, submission-status projections, assignment behavior, and
  external-tool handoff now use canonical Student vocabulary without aliases. The generated
  run-screen contracts and the complete Gradebook row use Serde-owned `snake_case`, including
  `student_name`. Existing run issuance, authorization, prefetch, replay, answer-free recovery,
  assignment, and provider behavior remain the permanent evidence. Two independent reviews accept
  the boundary, and the full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-SR2-student-assignment-projection`. Assignment landing, progress,
  delivery, detail, late-status, score-state, private snapshot, and inactive-course identities now
  use canonical Student vocabulary. Their Serde-owned browser contracts and generated TypeScript
  use direct `snake_case`; strict decoders and UI adapters preserve score withholding,
  class-statistics disclosure, answer-free detail, and Instructor Student view. The ledger now
  states the separate `QM-ACTIVITY` ownership of the retained internal
  `StudentAssignmentSummary` aggregate. Independent review accepts the clarified boundary, and the
  full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-SR1-disclosure-statistics`. Disclosure and Student class-statistics types,
  Store inputs, PostgreSQL modules, generated TypeScript, reusable-curriculum defaults, and strict
  browser decoders now use one canonical Student vocabulary and direct Serde-owned `snake_case`
  contract. Existing timing, stale-score redaction, k-anonymity, and answer-free projection tests
  remain the permanent evidence; the full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-OPS10-e2e-orchestrators`. Private shell state now follows the naming
  policy, and the non-browser aggregate includes all eight maintained lanes. Full execution also
  hardened generated MinIO credentials against CLI parsing and made the multi-database live-demo
  lifecycle migrate every schema before issuing cluster-wide service-role memberships. The final
  aggregate reports 8 passed and 0 failed with exact disposable cleanup.

- Accepted `WP-INST-WN1-OPS9-e2e-database-baseline`. Private shell state now follows the naming
  policy while explicit immutable fixture constants retain uppercase spelling. The fixed leased
  PostgreSQL owner passed all 109 migrations, idempotency and verification, registered live
  service and RLS oracles, and exact cleanup of its container, volume, and network.

- Accepted `WP-INST-WN1-OPS8-e2e-course-appearance`. Private shell state now follows the naming
  policy, and the course-appearance service oracle runs as a closed profile under the fixed leased
  acceptance owner. Typed mode-0600 runtime files replace ambient object-store credentials; exact
  Compose authority starts PostgreSQL and MinIO, and the real cross-store cleanup gate passes with
  empty final state. The source-size gate also drove focused live-test and item-analysis reducer
  module splits instead of exemptions.

- Accepted `WP-INST-WN1-OPS7-wasm-runner-setup`. The version-matched Wasm test-runner setup uses
  lowercase `snake_case` for private state and derives the repository from its physical script
  path. Shell syntax, a fresh pinned installation, and the subsequent matched-runner reuse path
  pass.

- Accepted `WP-INST-WN1-OPS6-python-setup`. The Python setup script uses lowercase `snake_case`
  for its private root, environment, interpreter, and receipt values, and derives the repository
  from its own physical path instead of repository metadata. The current receipt reuse and PyYAML
  verification path passes.

- Accepted `WP-INST-WN1-OPS5-wasm-build`. The Wasm build uses lowercase `snake_case` for its four
  private path/profile values while preserving argument and output behavior. The debug target
  built both bindgen flavors, and the Node consumer verified format, timer, capability, and
  presentation results.

- Accepted `WP-INST-WN1-OPS4-rust-front-door`. The ordinary Rust gate uses lowercase
  `snake_case` for its private repository path while retaining all eleven stages, argument
  handling, and the visible help contract. Shell syntax and help pass.

- Accepted `WP-INST-WN1-OPS3-browser-front-doors`. The screenshot and Playwright root scripts use
  lowercase `snake_case` for their private repository path while retaining the shared
  production-browser owner, argument forwarding, and visible help contracts. Shell syntax and
  both help paths pass.

- Accepted `WP-INST-WN1-OPS2-root-aggregate`. The root Validation front door now uses lowercase
  `snake_case` for its sole script-private path while retaining its exported process boundary and
  complete gate order. Shell syntax and focused source inspection pass; the aggregate execution
  remains owned by final WN1 acceptance.

- Accepted `WP-INST-WN1-GO1-orphaned-generated-output-retirement`. The two unconsumed `ts-rs`
  bindings are removed, leaving project-tools and `generated/api` as the single browser-contract
  generator. Graphify plus direct consumer inspection found no live dependency; regeneration
  produced 482 declarations, all 63 generator tests pass, both TypeScript configurations compile,
  and strict project-tools Clippy is green.

- Accepted `WP-INST-WN1-MG1D-automated-scoring-persistence-retirement` and the parent automated-only
  grading closure after six independent review passes. The runtime now has one deterministic
  evaluation owner with bounded retry/recalculation, immutable evidence, calculated Gradebook
  totals, and roster score export. Migration `2026081883` closes the parallel manual receipt,
  binder, policy, table, and catalog values while exact catalog rewrites preserve mature function
  identity and authority. Focused Rust, TypeScript, SQL-source, contactless-Student export, and
  fresh 109-migration PostgreSQL/RLS gates pass; retirement inventories remain one-time evidence.

- Accepted `WP-INST-WN1-MG1C-automated-item-analysis-state` after independent review and the full
  registered disposable database baseline. Memory and PostgreSQL now share one closed automated
  evaluation truth table: pending and exception work is visibly unscored, completed grades require
  immutable completion-receipt evidence plus current-generation scores, and contradictions fail
  closed. The Instructor report remains aggregate-only, same-tenant Students are denied, and the
  clean stack passed all 108 tracked migrations, RLS/privacy checks, generation fencing, and exact
  cleanup without widening access to worker-private result material.

- Accepted `WP-INST-WN1-MG1B3-evaluation-status-contracts` after independent review and fresh
  manager gates. The automated evaluation contract now has exactly four direct `snake_case`
  values, generated TypeScript matches Serde, and the answer-free status aggregate rejects
  contradictory durable state. Architecture review split the next automated-only boundary into
  truthful item-analysis state followed by persistence retirement and migration `2026081883`.

- Accepted `WP-INST-WN1-MG1B2-attempt-status` after an independent `ACCEPT` and fresh manager
  gates. Attempt lifecycle now has five direct `snake_case` values; Instructor force-submit
  atomically closes active work as answer-free `AutoSubmitted` in Memory and PostgreSQL, preserves
  exact replay, timing cleanup, and audit evidence, and creates no response or grade. The separate
  transitional manual-evaluation bridge remains allocated to its successors. Rotated complete
  older changelog day blocks under the repository's documented 800-line policy.

## 2026-08-28

### Fixes and Maintenance

- Accepted `WP-INST-WN1-A` after two `REVISE` rounds and a fresh `ACCEPT`. Its ledger binds
  automated-only grading, item analysis, exact Student roles, naming, C6 routing, authority, and
  migrations `2026081879` through `2026081888`. B1 adds the pure Serde-only browser contract crate;
  B2 splits the generator into focused owners; B3 adds strict Serde names and safe TypeScript;
  B4/B5 secure output and roots; OPS1 aligns private shell variables. A follow-up allocates orphaned
  `ts-rs` output and remaining PLE shell families. MG1A retires manual HTTP; MG1B1 removes manual outcome, key, and
  disposition variants and gives graded file uploads a typed deterministic-grader refusal. Six
  inventory-only Markdown failures remain open; final gates remain behavioral checks, TypeScript
  compilation, and final validation.

- Moved the Instructor course ribbon into one authorized course-route frame. Course identity and
  all eight course-management tabs now retain one desktop position while Assignments, authoring,
  roster, teaching operations, curriculum changes, Gradebook, grade settings, appearance, and
  their nested task pages replace only the content below the ribbon.

- Regenerated and visually reviewed the 64-artifact production-stack screenshot corpus. The
  automated-grading recovery journey now includes audited Student work as its third declared
  Instructor state, and the fresh 1280 by 800 evidence confirms the shared course title and ribbon
  remain spatially stable across course-management workspaces.

- Separated browser course-total rows from export-only roster identity. Server-calculated totals
  now remain available for ordinary connected Students whose optional institutional roster ID and
  email were never collected; the browser receives only display labels and outcomes, while the
  audited CSV represents absent optional roster fields as blank cells.

- Repaired answer-free audited Student-work inspection for identifier-bearing responses. Accepted
  browser responses are now validated directly against the reconstructed public presentation and
  retain the exact rendered identifiers the Student saw; the inspection boundary no longer tries
  to recover durable identifiers from an intentionally durable-ID-free public snapshot.

- Added an execute-only Instructor broker for resolving grading-operation Gradebook navigation.
  The PostgreSQL adapter no longer depends on direct application-table access that the existing
  least-authority migration had correctly revoked, and submission operations resolve through the
  enrollment's exact course-membership binding.

- Completed synchronous Base Course score convergence through the ordinary worker contract. The
  accepted-submission outcome carries its exact recalculation job, PostgreSQL can claim that exact
  typed job through the queue broker, and host-only installation executes the normal scoring
  handler before checking the installed completion witness.

- Bound the installed Blueprint Course's deterministic completed answer to its issued presentation
  before sending it through the ordinary accepted-submission service. The production seed now uses
  the same rendered response contract as a live Student browser and still persists the canonical
  durable response after server-side translation.

- Made the learning-data-access `test-support` feature self-contained by enabling the
  server-owned Question identifier generation its focused test builders require. The crate's
  isolated all-target gate now exercises the same explicit test capability as the workspace gate.

- Preserved durable submission acceptance when the optional grading fast path is unavailable.
  The browser now receives the stable pending state while the queued worker retains recovery
  ownership, instead of receiving a post-commit service failure.

- Aligned the retained calculated-Gradebook route coverage with its real same-origin browser
  contract, so Fetch Metadata concealment remains meaningful instead of making cookie-only test
  requests look like Gradebook failures. Student access now asserts the route's established
  non-enumerating response alongside Sysadmin and outsider access.

- Hardened G2-W4B Gradebook async sessions: retries retire obsolete chooser continuation gates,
  continuation pages reject visible or intra-page duplicate identities atomically, and focused
  deferred tests cover stale completion fencing, retained-row recovery, liveness, and disposal.

- Initial Gradebook, operation-selection, and submitted-run chooser pages now reject duplicate
  public identities before ready publication and route invalid responses through their existing
  visible error/retry states.

- Added the current validated Student display label and assignment title to the one audited
  Student-work detail response. The registered route continues to return the same immutable,
  solution-free evidence and closed return context with no-store delivery.

- Refactored the registered Gradebook route completion fixture behind a small borrowed harness
  and explicit completion identity, removing the need for a Clippy suppression while preserving
  route behavior and focused assertions.

- Implemented the G2-W4B Instructor Gradebook selection surface. A closed one-filter route now
  survives Gradebook continuation and reload; grading-operation context gives an Instructor a
  bounded named-Student choice; and a semantic submitted-run chooser requires one exact immutable
  run before inspected Student work opens. The chooser restores focus to its invoking Gradebook
  control, retains the optional public operation reference, and keeps the server's total-first
  Gradebook table and failure-routing action visible.

- Completed the G2-W4B audited Student-work return flow. One inspected-work request now carries
  the verified Gradebook or grading-operation return context; operation rows visibly enter their
  bounded Gradebook selection and regain focus on return. The same audited response supplies
  bounded server-owned Student and assignment labels after direct navigation or reload.

- Added the pure G2 Gradebook navigation owner for closed filters, public focus IDs, and
  context-preserving Gradebook, grading-operation, and audited Student-work URLs.

- Closed the G2-W4B calculated-Gradebook browser boundary. Strict decoders and same-origin,
  no-store clients now cover grading-operation filters, named-Student selection, submitted-run
  choices, and audited Student-work inspection with exact Gradebook or grading-operation return
  identity and focus binding. Malformed references and cursors, extra fields, cache drift, and
  echoed request-identity drift fail closed.

- Added permanent offline client evidence for nested calculated-page decoding, closed extra-field
  rejection, and canonical assignmentRef and membershipRef cursor/page-size URLs.

- Closed the G2-W4A registered Gradebook route boundary with offline Memory-backed HTTP coverage
  for closed selection projection, Fetch Metadata concealment before inspection audits, and exact
  operation-origin return/focus context.

- Retired the obsolete ignored PostgreSQL Base Course installation oracle frozen at migration
  `2026081808` and removed its now-unused `base_course_installation` test dependency. Current
  full-schema database authority and canonical live-demo lifecycle acceptance remain the owners.

- Repaired the calculated Gradebook route test to assert the parser's stable
  `GradebookFilterRequest` contract before server normalization.

- Split accepted-submission service, replay, and fast-path projection tests into a focused module;
  route submission coverage remains in the route-owned test module and both stay below the source
  line limit.

- Split G2-W4A Gradebook route tests into focused selection and inspection modules. Shared route
  execution helpers, deterministic backend support, and Fetch Metadata helpers now have one support
  owner; legacy Gradebook tests remain in their original module with behavior unchanged.

- Split the Memory and PostgreSQL Gradebook selection and submitted-run chooser responsibilities
  into focused backend modules; parent stores remain small trait coordinators and all authored Rust
  files stay below the repository's 999-line source limit.

- Added the G2 Gradebook server selection surface. Calculated pages normalize public grading
  operation filters before Store reads, while direct-Instructor, Fetch-Metadata-gated `no-store`
  routes provide bounded answer-free named-Student and submitted-run choices with concealed
  unavailable responses.

- Implemented the G2 Gradebook Store selection and submitted-run chooser in both Memory and
  PostgreSQL. Instructor-authorized selection is active-roster ordered, operation-bound, and
  cursor-bounded; submitted runs retain a stable completion order, mark the current score-selected
  run, and conceal stale, foreign, unavailable, or mismatched continuation state.

- Completed the Base Course canonical accepted-submission seed composition: its child-only lifecycle
  now carries distinct installer, application, and exact-fast-path PostgreSQL capabilities; the
  deterministic Mary submission uses the shared server acceptance/execution boundary, and focused
  route coverage proves durable first effect, idempotent answer-free replay, and one exact execution.

- Completed the PostgreSQL half of the G2 calculated-Gradebook and audited Student-work boundary.
  Worker failure now preserves tenant context through the queue capability, the connected fixture
  creates immutable accepted work through the production submission and scoring path, and a forward
  migration aligns the inspection broker's transient JSON rowset with exact PostgreSQL field names.
  The disposable 105-migration database baseline passed typed inspection, paired audit writes,
  broker-only private-response access, RLS, and representative role denial. The raw broker probe
  used to localize the rowset mismatch was removed after diagnosis.

- Completed the deterministic G2-W2 calculated-Gradebook and audited Student-work contracts.
  Gradebook pages are roster-first, structurally continued, and calculated from current
  server-owned scores; inspected work is immutable-evidence-bound, solution-free, retention-aware,
  and paired with internal audit witnesses. Large conformance modules were split by ownership while
  retaining one shared contract suite for Memory and future PostgreSQL implementations.

- Distinguished feedback release from score freshness throughout the Student run and history
  surfaces. Current, recalculating, and failed scores now have truthful visible and live-region
  messages, completed recalculation refreshes through the status read without resubmitting the
  answer, and the fresh production Instructor-authoring journey observes the resulting score.

- Replaced an implementation-spelling blacklist in the iMathAS launch test with durable shell
  capability assertions and exact non-disclosure of the concrete provider fixture credentials.
  Removed the similarly vocabulary-based teaching-preview key blacklist. Broad credential and
  private-field name searches remain one-time implementation audit evidence rather than permanent
  test contracts.

- Corrected the planned secure file-upload capability to use canonical Student terminology: the
  active plan is now `secure_student_file_upload_plan.md`, its proposed `StudentUploadId`,
  `student_upload`, `secure_student_uploads`, and `CON-STUDENT-UPLOAD` names are aligned across
  current contracts, and historical dated reports retain their original wording as evidence.

- Standardized the active Instructor roadmap on the temporary `WP-INST-*` package namespace and
  renamed its capability plan accordingly. These labels are disposable plan coordinates that retire
  with the planning layer. Product and source vocabulary now treats Student,
  Instructor, and Sysadmin as the sole human-role terms; new G2 contracts use
  `StudentWorkInspectionStore` and `InspectedStudentResponseV1`. Accepted migration files retain
  their historical package comments while current planning identifiers and future capabilities use
  the canonical namespace.

- Strengthened the approved `WP-INST-G2` binding plan for one roster-first calculated Gradebook and
  one explicit audited Student-work inspection read. It now binds operation filters and the
  `singleStudent`/`studentSelection` choice into structural cursor continuation; gives each later
  page its own live scoring witness; assigns safe response rendering to
  `question_model::presentation`; and specifies Fetch Metadata, server-owned audit facts,
  parameter-bound SQL, secure errors, and separate security telemetry. The four migrations now
  close authority in order: foundation (`1870`), private immutable witness (`1871`), the only
  app-executable broker with atomic audits (`1872`), and demonstrated indexes (`1873`).

- Closed the final G2 architecture findings with an exact typed Student-selection row that reuses
  the Gradebook run-choice union and a Fetch Metadata decision table covering same-origin requests
  plus explicit user-initiated top-level navigation. Independent HCI and security rereviews accept
  the interaction, evidence, and privacy boundaries.

- Reconciled the repository documentation set with current live-demo ownership, exact role
  viewports, state-derived grading recovery, project-scoped Podman cleanup, release notes, and the
  active G1 evidence boundary. The root `AGENTS.md` now routes agents to canonical documents rather
  than restating their content.
- Improved the screenshot evidence states: explicit grade-setting reloads announce the latest
  server settings, the shared assignment picker is captured while open with a selected candidate,
  and curriculum-recovery and authorized-usage captures frame their complete actionable regions.
- Tightened the root `AGENTS.md` to bare-path pointers while retaining active-plan authority,
  package identity reservations, ownership boundaries, dependency-order workflow, and the
  final-tree Validation completion rule.
- Refreshed `docs/TROUBLESHOOTING.md` with the fixed Python environment and
  owner-scoped browser, screenshot, Podman, cleanup, and migration recovery
  paths; corrected the destructive scope of `./run_live_demo.sh stop`.
- Added a typed real-stack route-surface readiness helper that uses the configured Playwright
  action timeout for assignment-overview and practice-entry waits in catalog discovery evidence.
- Reordered the shared learner assignment presentation so the single primary Start/continue action
  follows assignment identity before instructions and progress/details. The action region adapts
  across widths without overlays, and answer-free Instructor Student view omits it when no primary
  action is supplied.
- Made the live-demo Python runtime self-contained: `run_live_demo.sh` now creates or refreshes a
  fixed Python 3.12 `.venv` through `devel/setup_python.sh`, installs the declared manifests, and
  executes the controller through that environment for both start and stop. `all_test.sh` reuses
  that same owner before its pytest and connected acceptance gates. The pinned `PyYAML==6.0.3`
  runtime requirement now belongs in `pip_requirements.txt`, while developer tools extend it through
  `pip_requirements-dev.txt`.
- Reused the shared copyable Question ID control in Instructor grading operations. The operation
  keeps the question title as its heading, exposes the stable public ID with accessible copy success
  and manual-copy fallback status, and keeps the retry action bound to the exact title and ID.
- Aligned current operator documentation with the repo-owned Python runtime: `./run_live_demo.sh`
  remains the ordinary live-demo entry, while direct controller and pytest work use the prepared
  `.venv/bin/python` after sourcing repository settings.
- Classified `.venv` consistently as installed dependency state in Git, ESLint, Prettier, and
  hygiene discovery after the aggregate gate exposed ESLint traversing pip-vendored JavaScript.
- Reconciled the remaining live controller entry points with the repo-owned `.venv`, added negative
  capability-receipt coverage, removed duplicate readiness coverage, isolated the completed-receipt
  privacy test from an unrelated fixture, and clarified the response-redaction and selector-owner
  contracts found during audit.
- Updated the connected PostgreSQL G1 oracle to exercise the five-UUID retry V2 denial with SQLSTATE
  `42501` and to verify Instructor retry receipt category, actor provenance, and worker exclusivity.

### Decisions and Failures

- Advanced WP-INST-G1's accepted-input boundary: immutable server-private submissions remain the
  grading authority, replay returns the original receipt, and answer-free learner and Instructor
  projections keep response material private. Source/projection digests, receipt immutability,
  integrity-failure routing, worker readiness, and the existing generation-fenced score path remain
  explicit contracts rather than compatibility behavior.
- Proved the learner terminal path through the canonical production browser: a successful accepted
  response clears the answer buffer, reaches `acceptedPending`, exposes status-only recovery, moves
  through deterministic Instructor attention, and reaches completed feedback after one Instructor
  retry without another learner answer POST. The answer-free audit covers every submitted,
  completed, operation-list, and retry response variant.
- Accepted the connected G1 evidence package for the canonical production-browser journey, the
  fresh pre-reconciliation 95-migration PostgreSQL/RLS and worker oracle, WebWork service, and
  replica-restart acceptance. The package also atomically published and verified the 63-artifact
  screenshot corpus; HCI review repairs the dense operation and learner layouts, and independent
  architecture and security/privacy reviews approve the resulting boundaries.
- Completed the approved G1-W7 forward reconciliation: accepted migrations `2026081849`, `1850`,
  `1855`, `1859`, `1860`, `1861`, and `1865` are restored byte-for-byte, and the closeout source
  is implemented across `2026081866` through `2026081869`. The four atomic owners are receipt
  schema/preflight, execution writers, the 36-input commit-v2 writer, and Instructor writers with
  retry V2 and public V1 retirement. The affected live evidence is green on the 99-migration tree.
- Accepted `WP-INST-G1` after the final material-tree aggregate passed every required gate. G2 now
  owns audited learner-work inspection and the grade-scheme-aware calculated Gradebook.

### Developer Tests and Notes

- Regenerated all 63 screenshots through the canonical production-stack owner after the evidence
  fixes. Every ordinary scenario and the isolated deterministic grader-exception profile passed,
  publication completed atomically, cleanup was exact, the 97-case publisher suite passed, and the
  offline corpus verifier accepted the current production-dist provenance.
- Passed focused TypeScript, ESLint, Prettier, learner-presentation Node, Student-view contract, and
  `git diff --check` gates. The focused production G1 browser journey then passed clipboard
  confirmation, retry-focus, learner-completion, Gradebook, answer-free network, single-origin, and
  exact-cleanup assertions.
- Regenerated the canonical live-stack screenshot corpus: 63 PNGs, including 54 desktop artifacts
  at 1280 by 800 plus three Student artifacts at each of tablet 800 by 1280, iPhone Pro 393 by
  852, and square 800 by 800. The 97-case screenshot publisher suite passed, and the independent
  offline corpus verifier passed after its receipt contract aligned per-artifact origin and
  generation-digest validation with the publisher.
- The first exact aggregate Validation attempt passed the complete Rust gate, then exposed and
  stopped at the `.venv` ESLint ownership defect. After the ownership repair, all five frontend
  gates passed with 369 Node tests, and the permanent Python suite outside the known
  tracking-dependent Markdown-link module passed 7,654 tests.
- The audit repair gate passed 65 focused Python tests, 7 focused Node tests, shell syntax,
  Prettier, controller help, and `git diff --check`.
- An intermediate aggregate passed the complete Rust and codebase gates, then reported 7,912
  Python checks before local-link scope stopped that run. Complete live acceptance separately passed
  every production-browser scenario, all 99 migrations and connected PostgreSQL/RLS/worker oracles,
  isolated WebWork, replica restart and durable replay, and exact cleanup.
- A one-time shadow-index diagnostic added exactly the 13 intended durable artifacts to an isolated
  temporary Git index while preserving the real repository index. The unchanged aggregate then
  passed Rust/Wasm, 369 Node tests, 7,978 pytest checks, every production-browser scenario, all 99
  migrations and connected PostgreSQL/RLS/worker oracles, isolated WebWork, replica restart/durable
  replay, and exact cleanup. This one-time implementation probe established the remaining gate path.
- Final `source source_me.sh && ./all_test.sh` Validation passed on the material tree: Rust checks,
  tests, doctests, strict Clippy, and browser Wasm; 369 Node tests; 7,978 pytest checks; every
  canonical production-browser scenario; all 99 migrations and connected PostgreSQL/RLS/worker
  oracles; isolated WebWork; replica restart and durable replay; and exact disposable cleanup.
