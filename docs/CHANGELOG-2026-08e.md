## 2026-08-27

### Decisions and Failures

- Approved the WP-INST-G1 automated-grading operations binding plan. Source and dependency review
  found that learner grading could begin before the accepted response had a durable retry owner, so
  G1 now makes one immutable server-private `submission` authoritative before grading and separates
  mutable execution, evaluation, and Instructor-operation projections from append-only receipts.
- Allocated forward migration `2026081849` for accepted-submission execution, automated-grading
  operations, receipts, RLS, broker privilege, retention fences, and purge order. G1 reuses the
  existing assignment recalculation and current-score publication authorities in migrations
  `2026081830` and `2026081831` rather than creating a competing score path.
- Bound one existing-worker execution path with an exact-job lease for both the synchronous fast
  path and background recovery. Deterministic grader exceptions become assignment-local,
  metadata-only Instructor work; retry reuses accepted private input, recalculation remains
  generation-fenced, and automated capabilities stay structurally separate from human grading.
- Accepted `WP-INST-G1 / G1-W2` for the typed pending contracts, answer-free metadata parents and
  private-response child, canonical UTF-8 response identity, Memory/PostgreSQL parity, dedicated
  worker-only execution store, separate API/worker process logins, and migrations 1849/1850. G1
  remains incomplete; G1-W3 then became the next dependency-ordered stage.
- W4 owns forward migrations `2026081851` through `2026081860` and their split claim, verified-read,
  load, completion-lock, commit, and fail capabilities; W5 owns `2026081861` and its dedicated
  Instructor operation capability. W7b
  owns executable API-denial/worker-lease/RLS proof, and final `all_test.sh` remains required.
- Accepted `WP-INST-G1 / G1-W3` for exhaustive typed pending reads, the answer-free no-store
  `accepted_pending` replay projection, closed deterministic grader categories, Native/WebWork/QTI/
  composite classification, the preserved opaque iMathAS broker boundary, and Memory learner-read
  projection parity. `G1-W4` is now current; it owns migrations `2026081851` through `2026081860` and consumes
  W3's sealed contract-paired dispatch dependency before its first learner effect.

### Developer Tests and Notes

- Hardened migration \`2026081861\`'s immutable grading-operation receipt check:
  retry and recalculation now require every subtype-owned result field to be non-NULL and
  reject all opposite-subtype fields. The connected PostgreSQL oracle rollback-probes malformed
  rows for both actions; its focused Rust compile, formatting, and diff checks pass, while the
  ignored live run still requires the canonical acceptance runtime.
- Reconciled the G1 Instructor operations pagination boundary: the Rust adapter now forwards the
  validated public page size (`1..=100`), while migration 1861 performs its bounded one-row
  overfetch internally. The Memory max-page assertion passes; the disposable PostgreSQL oracle
  now owns the corresponding maximum-page assertion, but its current run is blocked earlier by
  the existing auth-session role fixture constraint.
- Split the automated-grading operations plan into a binding architecture/dependency plan and the
  linked [automated_grading_operations_delivery_plan.md](active_plans/active/automated_grading_operations_delivery_plan.md)
  companion. The focused companion owns W5-W7 delivery, evidence, acceptance, risks, closeout,
  and dispatch while retaining migrations 2026081861 through 2026081865.
- Regenerated the complete live-demo screenshot corpus through the canonical
  `./capture_screenshots.sh` entry point. The disposable production-shaped HTTPS stack completed
  every registered browser scenario, validated all 61 declared PNG artifacts and their privacy
  boundary, published the corpus atomically, and left the stack cleanup inventory empty.
- Repaired the accepted-submission execution boundary exposed by the connected screenshot journey:
  a question-attempt payload remains immutable issuance evidence while relational lifecycle state
  records submission, and the browser identifies an active attempt from that lifecycle status. The
  shared browser workflow now follows visible asynchronous-grading status and successor issuance
  instead of assuming immediate feedback or reusing an answer-free completed attempt.
- Aligned the connected receipt, manual-grading, issued-attempt, and WebWork service oracles with
  that immutable answer-free contract. The WebWork oracle now proves accepted-pending submission,
  server-worker completion, status-route disclosure, and answer-free idempotent POST replay; its
  focused live service gate, the 81-migration PostgreSQL baseline, and replica-restart gate pass
  with empty disposable cleanup inventories.
- Updated the isolated WebWork service oracle to treat `completed` with
  `scoringStatus: recalculating` as the privacy-preserving intermediate between worker completion
  and current-score publication. It validates redaction while polling the route-bound status GET,
  then asserts the exact authorized current result; the real service child passed full/zero scoring,
  outage/restart, redaction, answer-secrecy, and exact-cleanup rerun checks with host
  process-inspection permissions. The manager-owned `source source_me.sh && ./all_test.sh` then
  passed the complete Rust, frontend/Node, pytest, production-browser, 81-migration PostgreSQL,
  WebWork, replica-restart, and exact-cleanup aggregate. That receipt predates the bounded audit
  maintenance edits below, so final-tree Validation remains required. G1 remains incomplete because
  W4 acceptance and W5-W7 remain.
- Ran six independent plan, style, test-quality, legacy, documentation, and comment/readability
  audit passes over the current G1-W4 tree. The review keeps W4 acceptance open on two design
  findings: completed PostgreSQL reads still need a least-privilege proof of the worker-owned
  canonical evaluation source/digest relationship, and learner acceptance still needs the planned
  exact-job synchronous claim through the same leased handler used by background recovery.
- Applied the audit's bounded maintenance repairs: moved flat-run HTTP support into its focused test
  module, removed the accepted-worker compatibility alias, made the browser pending-status helper
  tolerate a legitimate worker-completion race, corrected the 36-value completion-capability
  contract, refreshed async learner and recovery documentation, closed accepted T6 evidence ledger
  entries, removed an incidental screenshot-count claim, and cleared generated Python bytecode.
  Focused native/QTI lifecycle and accepted-worker tests, strict server Clippy,
  `./check_codebase.sh`, 3,700 documentation/source-policy tests, and `git diff --check` pass.
- Repeated all six independent audit passes on the resulting G1-W4 material tree. The fresh review
  independently reconfirmed the exact-claim fast-path and canonical completed-read blockers. Its
  bounded follow-ons moved automated-grading browser behavior into focused learner-workflow
  support, replaced title-based successor detection with a safe attempt identity, closed three
  family-specific pending-response assertions, clarified learner attention and async demo guidance,
  and qualified the earlier aggregate receipt. The three affected Rust lifecycle tests, strict
  server Clippy, all five `./check_codebase.sh` gates with 359 Node tests, 1,993 source/link checks,
  and `git diff --check` pass. Final-tree aggregate Validation remains open.
- Completed the G1-W4 stable implementation handoff. The API exact fast path and background
  recovery now use type-distinct private logins and eagerly connected pools, one validated
  lease/deadline contract, and the same tuple-fenced grading handler. Canonical completed reads use
  the four-key verifier; learner POST/status remain answer-free. A fresh PostgreSQL baseline applied
  all 90 migrations, repeated as a no-op, passed every registered connected phase, and cleaned up
  exactly. The production-shaped headless stack then started API, worker, and HTTPS gateway through
  login/membership/function preflight and stopped with no labelled resources. Focused Rust,
  process-login, source-length, strict Clippy, format, and diff gates passed; independent re-review
  approved the fail-closed repair. W5 may consume this stable handoff, while W4 acceptance remains
  open for W7b's dedicated executable PostgreSQL oracle and final material-tree Validation.
- Aggressively restored `docs/HUMAN_GUIDANCE.md` to terse owner-level statements. Moved uncertain
  technical interpretation, rationale, compliance mechanics, demo evidence, content-format detail,
  authentication policy, and local-stack behavior into structured `docs/DESIGN_DECISIONS.md`
  entries rather than treating polished agent prose as the human's words.
- Completed independent architecture and repository-rules/test-policy reviews of the G1 plan. Both
  approved the final state ownership, package dependencies, one-owner artifact map, public
  `GO-<positive>` references, answer-free browser contract, UI-first recovery journey, and split
  permanent-versus-connected evidence without unresolved findings.
- Defined atomic connected-evidence packages: W7a owns the visible production-stack Instructor
  journey and screenshot provenance, W7b owns the disposable PostgreSQL/RLS/worker oracle, and W7
  owns only independent review, documentation reconciliation, and final material-tree Validation.
- `./check_codebase.sh` passed on the G1-W1 plan tree: TypeScript typecheck, lint-typecheck, ESLint,
  Prettier verification, and all 356 deterministic Node tests were green. G1 final acceptance still
  requires the full evidence model after every remaining G1 package is implemented.
- G1-W3 evidence passed 384 `server_core` tests with 3 intentional connected ignores, all server
  integration and doctest targets, and the learning-data-access main target (308 passed, 1 ignored)
  with its auxiliary targets. Strict Clippy passed for both affected crates, 3,643 documentation and
  source-policy checks passed, and independent architecture and security/privacy reviews approved.
  At W3 acceptance, the final aggregate remained manager-owned because a scoped subagent invocation
  retained no terminal result; the later successful manager run is recorded above.
- W2 evidence passed the learning-data-access full suite main target (308 passed, 1 ignored) with
  auxiliary targets green; strict format, check, and Clippy gates; 2,008 focused policy, process,
  documentation, and source tests; and `./check_codebase.sh` 5/5 with 356 Node tests. Fresh
  PostgreSQL 17 applied all 80 migrations, the second migration pass was a no-op, and database
  verification returned `database verify: compatible`; the repaired database-baseline Rust
  selector now resolves to exactly one intended test.
- Independent `sql_correctness_post_repair_review.report.md` and
  `w2_security_post_repair_review.report.md` approvals close the W2 SQL and source blockers. Their
  remaining evidence boundary is explicit: W4 worker outcomes, W7b executable authority, WP-P2
  grant reduction, browser behavior, and final G1 Validation remain open.

## 2026-08-26

### Additions and New Features

- Added the WP-INST-T6 W2 Instructor assignment workspace shell: canonical Overview, Questions,
  Policies, and Student view routes share one course-authorized assignment load, assignment titles
  open the Instructor Overview, and local navigation preserves the surrounding course management
  context.
- Extracted the answer-free learner assignment landing into a reusable presentation component;
  ordinary learner overview retains query, progress, and start-or-resume control while Instructor
  Student view can supply an informational action, context cue, and return link.
- Implemented the Instructor Student-view landing: the current answer-free live assignment is
  loaded through the exact course/assignment authority, rendered by the shared learner
  presentation, and paired with stable-identity, no-mutation guidance to explicit Student entry.
- Completed the WP-INST-T6 assignment workspace: Questions owns ordered fixed and pooled content,
  Policies owns delivery and lifecycle decisions, Overview is the assignment home, and Student view
  uses the same answer-free learner presentation while retaining Instructor identity. Empty Drafts
  persist before content exists, and publication readiness remains server-owned.
- Implemented the WP-INST-B2 curriculum-adoption API and Instructor browser composition across
  preview-before-save adoption, rollover, term shifting, provenance receipts, controlled
  fast-forward, and divergence recovery.

### Fixes and Maintenance

- Reconciled the real-stack browser plan with the complete B2 live-demo baseline, including the two
  ordinary Chapter 1 teaching courses, five persisted learner observations, and the current
  75-artifact corpus at B2 acceptance. Updated operator examples to the canonical root launcher and
  repaired small documentation, ASCII, and Python-readability issues found by the six-pass audit.
- Rebuilt `docs/RELATED_PROJECTS.md` as an evidence-first visitor guide using the current
  relationship taxonomy and confidence tiers. Every retained or added destination now states its
  shared audience outcome and authoritative evidence; current integrations and explicit prior art
  remain separate from adjacent alternatives and the planned LTI Advantage reference.
- Removed the healthy browser-Wasm implementation banner from the live product shell. The
  production UI now stays focused on teaching work while an inert diagnostic preserves connected
  browser evidence; visible fallback copy accurately explains that slower checks use the server.
  At B2 acceptance, republished its 75 declared real-stack screenshots against the corrected shell.
- Made the live-demo Student-to-Instructor relationship explicit: seeded students and their
  Instructor share ordinary course relationships, and the connected learner journey now verifies
  Mary's persisted best score, latest score, and completed-run count in Elena's authorized
  gradebook projection.
- Made `run_live_demo.sh` a complete fresh-clone launch path: when `node_modules` is absent it
  visibly invokes the propagated `devel/setup_typescript.sh` before the production build, while
  the propagated `devel/setup_playwright.sh` remains the single optional browser installer. Each
  start now completes fixed-owner cleanup before creating a fresh seeded demo, so rerunning the
  root command replaces the prior session instead of failing on its single-flight lease. The
  browser-free operator mode is exposed consistently as `--headless` by both the root launcher and
  typed controller.
- Moved the live-demo lease, workspace, and authenticated control receipts from Cargo's disposable
  `target/` tree to the dedicated mode-0700 `local_runtime/live_demo_browser/` boundary, preserving
  operator control when build cleanup runs while a demo is active.
- Separated the developer parent's clean-build handoff allowance from the child lifecycle's bounded
  service-readiness waits, so a distribution-clean launch is not terminated while a healthy build
  is still progressing.
- Refreshed the owned README, development, installation, usage, and cookbook surfaces for the
  propagated setup paths and current curriculum-adoption ownership. The current README and role
  guides now describe the freshly published live-demo evidence.
- Recorded the owner-approved pre-production baseline decision: the reviewed clean-cluster v1
  baseline reissues `2026081818` with `Biochemistry: Protein Structure and Function` as the final
  visible installed teaching-course title, regenerates disposable live-demo volumes, and records
  the resulting canonical immutable checksum. The same ordinary-course topology names
  `Genetics: Foundations of Inheritance` and `Biochemistry: Molecular Foundations`, while Morgan
  and Avery retain their separate ordinary authorization course. Blueprint and Alpha vocabulary
  remains aggregate-specific; five deterministic Chapter 1 observations appear through ordinary
  item-analysis and discovery surfaces. Role-visible courses derive from active relationship
  authority. Focused relationship tests are permanent evidence, while the fresh live-stack
  database and visual walkthrough are one-time package evidence.
- At B2 acceptance, published all 75 declared live-demo screenshots from the production-shaped
  HTTPS stack. PNG,
  privacy, provenance, atomic-publication, and cleanup checks passed; human review covered the
  Instructor, Student, Sysadmin, discovery, curation, reusable-curriculum, and rollover surfaces.
  Instructor and Sysadmin artifacts remain 1280 by 800 desktop-only, while Student artifacts retain
  the declared variable viewport mix.
- Added the human-facing assignment title to the answer-free curriculum import-inspection contract.
  Instructor evidence now leads with meaningful assignment names while retaining compact `A-N`
  references as secondary audit evidence. The rollover journey also uses a coherent Spring 2028
  term, learner questions use biology misconceptions instead of placeholder alternatives, and
  institution collection access is presented in user-facing language.
- Split the visible curriculum-adoption page into focused workflow and proposal-panel modules so
  each authored TypeScript source remains within the repository source-size boundary.
- Covered B1/B2 migrations `2026081837` through `2026081847`: accepted `1837` remains immutable;
  `1842` carries the forward relative-time-validator repair. The database guide now treats the
  migration directory and shared status ledger as the physical and allocation authorities.
- Bound term-shift client refusals to the real course-scoped apply path and removed the fabricated
  generic apply-path diagnostic.
- Bound the curriculum-adoption browser journey to the Alpha source's visible title and public
  reference, keeping source identity under test while allowing presentation punctuation to evolve.
- Repaired live-demo role entry after the full connected suite exposed a shared-persona rate-limit
  lockout. The public selector now applies caller-network and deployment-service budgets, preserving
  bounded request cost while allowing independent visitors to choose the same seeded role.
- Replaced the browser-catalog source-text assertion with the boundaries that carry behavior value:
  strict owner-input decoding, registered selection, mandatory Instructor and Sysadmin passkey
  journeys, and focused real-stack execution. Scenario namespaces remain available for identities
  that need collision resistance while visible records use meaningful teaching language.
- Routed PostgreSQL login synchronization and restricted-grader provisioning failures through the
  existing bounded private-environment redactor, preserving actionable child diagnostics while
  protecting generated live-stack credentials.
- Made the idempotent PostgreSQL login update part of bounded semantic readiness, so a fresh stack
  tolerates the official image's temporary-to-final server handoff before migrations begin.
- Unified the issued-learner-work boundary for Memory and PostgreSQL assignment editing. Fixed-item
  identity, order, points, delivery, and scoring plus pool draw, points, order, algorithm, and
  candidate structure remain immutable after issuance; presentation-only assignment-title changes
  remain available. The visible Questions recovery preserves local structural edits and directs the
  Instructor to create a new assignment.
- Made Policies save and reload one accessible transaction: the complete editor locks while a
  request is in flight, stale and publication-repair paths recover keyboard focus, and the compact
  summary distinguishes current saved delivery from unsaved lifecycle, schedule, disclosure,
  audience, variation, grade, and continued-practice decisions.
- Replaced horizontally scrollable Instructor task navigation with visible wrapping navigation.
  The connected Student-view capture now proves the Course management row and all Overview,
  Questions, Policies, and Student view links remain in the canonical 1280 by 800 viewport.
- Made the Questions-page Check and Add Question ID operations one visible busy transaction: the
  existing fieldset publishes progress and `aria-busy`, then restores its idle state after every
  outcome. Connected journeys now wait for the semantic Add completion before saving, which keeps
  client ordering aligned with the revision-checked assignment contract.

### Decisions and Failures

- Corrected the WP-INST-T6 fixed-slot replacement boundary after the live journey showed that
  selection alone did not commit a future-run replacement. The focused route-policy command now
  resolves the public Question server-side, preserves the assignment item identity, advances the
  shared revision, returns authoritative no-store editor detail, and leaves issued snapshots
  unchanged. The route-policy exposure also fixes the underlying 404 rather than adding a caller
  workaround. Title-bound accessible controls and the replacement summary make the two-step action
  clear, while the shared browser client enforces no-store on every editor response.
- Completed the corrected T6 evidence pass: the focused suite has 19 Node tests and 375 runnable
  server tests; fixed-slot replacement and learner delivery pass on the production-shaped live
  stack; complete local-stack acceptance passes all 15 browser scenarios, the 78-migration/DB
  oracle, WebWork oracle, replica restart, and exact cleanup; and independent security and HCI
  re-reviews are clean. The six durable T6 closure paths are now tracked; final package acceptance
  remains tied to `source source_me.sh && ./all_test.sh` passing on that exact final tracked tree.
- Accepted WP-INST-T6 on 2026-08-27. The exact tracked-tree `source source_me.sh && ./all_test.sh`
  gate passed Rust checks/tests/doctests/Wasm, frontend/codebase/Node, 7,428 pytest cases, all 15
  production browser scenarios, all 78 migrations and database oracles, isolated WebWork, and
  replica restart/durable replay. The Instructor handoff advances to WP-INST-G1.

- Added the owner-directed `WP-INST-T6` assignment-workspace binding plan and advanced the current
  Instructor handoff to it before `WP-INST-G1`. The assignment title becomes the canonical entry to
  one Instructor assignment home; Questions and Policies become focused revision-checked pages; and
  Student view renders the current answer-free learner landing while retaining the Instructor
  session. ADAPT supplied comparison evidence for title navigation, child routes, and the familiar
  view control; Peptidyle retains ordinary identity and enrolled-Student grading rather than
  changing the Instructor into a generated student account. Permanent tests protect semantic
  behavior and authority, while the ADAPT comparison, Graphify impact map, wire inspection, and
  1280 by 800 walkthrough remain one-time package evidence.
- A distribution clean exposed that Cargo's `target/` directory could not safely own a long-lived
  demo lease or control receipt. The interrupted legacy run also exposed that passing every
  dependency-related container ID to one `podman rm --depend` command can partially succeed and then
  fail on an ID removed by that same command. The durable repair separates runtime state from build
  output and re-inventories after each dependency-aware removal.
- A first screenshot capture preview failure did not reproduce in the focused preview path. A
  second complete capture identified the durable cause as client Add/Save ordering, rather than an
  API failure; the visible busy transaction and semantic completion wait repair that boundary.

### Developer Tests and Notes

- Passed the focused disposable PostgreSQL/RLS curriculum-adoption oracle and the connected
  production-shaped HTTPS four-context curriculum-adoption browser journey, including visible
  creation, DST correction, controlled update, divergence recovery, destination evidence, and
  rollover.
- Passed focused TypeScript compilation, ESLint, Node browser-contract/workflow checks, Python
  scenario-contract checks, 23 question-model tests, 62 learning-data-access tests, and the
  repository ASCII, source-size, Markdown-link, and diff-hygiene gates.
- Passed the integrated WP-INST-T6 production-shaped HTTPS capture with all live scenarios, one
  origin, independent Elena Instructor and Morgan Sysadmin passkey paths, same-assignment Mary
  Student submission and Elena gradebook observation, all 61 current screenshot artifacts, privacy
  validation, atomic publication, and exact cleanup. Independent architecture/security and
  HCI/accessibility reviews returned ACCEPT with no unresolved P0, P1, or P2 finding. Final T6
  Validation remains open until the new files are tracked and the exact complete gate passes.
- Passed the complete repository codebase gate with all five checks and 322 Node tests, plus all
  7,361 pytest checks. The complete Rust gate passed generated contracts and fixtures, formatting,
  both compile graphs, all three strict Clippy graphs, both test/doctest graphs, and the browser Wasm
  target. Independent post-fix review returned ACCEPT with no unresolved P0, P1, or P2 finding.
- Passed all 15 connected production-browser journeys, the 77-migration PostgreSQL/RLS/persistence
  baseline, the isolated WebWork scoring and outage oracle, and the API replica restart/replay
  oracle. Each disposable owner returned an exact cleanup receipt.
- Accepted WP-INST-B2 and advanced the Instructor handoff to WP-INST-G1. Final
  `source source_me.sh && ./all_test.sh` passed the complete Rust, 322-case Node, 7,361-case pytest,
  15-journey production-browser, 77-migration PostgreSQL, WebWork, replica-restart, and exact-cleanup
  gates on the published material tree.
- Passed 26 focused developer-lease, exact-reset, and CLI contracts. Two consecutive real
  `./run_live_demo.sh --headless` launches proved active-owner replacement; the replacement exposed
  only its loopback HTTPS gateway, returned HTTP 200, reached six running long-lived services, and
  printed the matching root stop command for operator handoff.
- Regenerated the 61-artifact production HTTPS screenshot corpus. One origin, PNG, privacy,
  provenance, atomic-publication, exact-cleanup, offline verification, the 87-case publisher suite,
  and seven capture/manifest Node checks passed. Prettier, ESLint, both TypeScript configurations,
  14 focused Node tests, and the production learner-delivery journey also passed.
- An independent human-interaction review inspected all 61 artifacts at original detail. It found no
  P0 issue and recorded grounded P1--P3 follow-up opportunities: persistent context on dense or
  scrolled Instructor pages, consequence copy beside high-impact actions, visible state proof for
  artifacts 47, 58, and 61, and an above-fold Student start action on iPhone and square viewports.
  The detailed review is one-time visual evidence, not a permanent test.

## 2026-08-25

### Fixes and Maintenance

- Made API schema startup topology-aware: disposable local PostgreSQL now fails before serving
  when schema verification is unavailable, while AWS workload retains its degraded-start diagnostic;
  incompatible schemas remain fatal for both topologies.
- Established the audited `WP-INST-B2` domain and browser-contract foundation for explicit
  curriculum adoption. Validated server-only assignment and course semantic baselines bind exact
  publication pins, authored order, pools, defaults, and relative schedules; structural equality
  governs equivalence while a canonical digest supplies immutable evidence. Target-term resolution
  uses calendar-day offsets and existing course-zone authority. Operation-specific answer-free
  previews and results bind truthful destinations; every write command derives from its exact
  preview, fast-forward apply accepts only an eligible preview, and outcome-specific recovery types
  make incompatible actions unrepresentable. Bounded streaming decoders protect assignment
  witnesses and replacement choices. The six-pass audit and independent approval resolved every
  contract, documentation, style, legacy, readability, and permanent-test finding. All 146
  `question_model` tests, two doctests, TypeScript compilation, formatting, strict Clippy, source
  limits, Markdown links, and diff hygiene pass.
- Accepted WP-INST-B1 reusable curricula and advanced the Instructor queue to WP-INST-B2 curriculum
  adoption, rollover, term shifting, provenance, and controlled updates. Approved Instructors can
  create revisioned private assignment Blueprints, inspect non-enrollable public Alpha curricula,
  and reuse ordered Alpha questions through the same answer-free `ProblemPicker` used by ordinary
  assignment authoring. Creator-only updates, stale-draft recovery, exact immutable publication pins,
  and semantic no-op revisions preserve the aggregate boundary.
- Completed B1 production evidence: the 67-migration PostgreSQL baseline passed forced-RLS, broker,
  creator-versus-reader, Student-refusal, revision, rollback, and cleanup checks; the visible HTTPS
  journey covered Elena's creator workflow, Avery's reader workflow, Alpha picker reuse, and the
  independent Elena Instructor and Morgan Sysadmin passkeys. The privacy-validated corpus records
  canonical 1280 by 800 desktop creator, picker, and reader states. Architecture, security/privacy,
  HCI/accessibility, and documentation/evidence reviews approved with no unresolved P0--P3 finding.
- Restored the owner-defined viewport allocation after a generalized responsive-corpus paraphrase
  weakened it. Instructor/Instructor and Sysadmin visual evidence now uses only the canonical 1280 by
  800 16:10 desktop profile; Student evidence retains the 40/30/20/10 laptop, portrait-tablet,
  iPhone Pro, and square planning mix. The manifest validator enforces the teaching-role boundary,
  and screenshot publication retires the prior non-laptop teaching-role variants.
- Updated the Rust 1.98 quality boundary with fixed-width slice projection, removed the obsolete
  IMathAS test import, and replaced route-local large-error suppressions with one boxed
  `HttpRefusal` transport that preserves exact Axum responses. Split teaching-preview projection
  logic into its focused module while retaining the route's authority and no-store behavior.
- Kept permanent pytest focused on durable product and security behavior. The vendored checkout disk
  budget is now documented as a one-time operational cleanup check, while the full suite retains its
  deterministic offline fixture policy.
- Rotated the 2026-08-18 through 2026-08-23 day blocks into
  `docs/CHANGELOG-2026-08d.md` under the repository's two-current-day changelog policy.
- The full live-stack checkpoint before the B2 contract audit passed `source source_me.sh &&
./all_test.sh`: the Rust workspace, all five codebase gates, 315 Node tests, 7,067 pytest cases,
  the complete production HTTPS browser suite, all 67 database migrations, the WebWork and
  replica-restart service oracles, and exact cleanup. B2 final Validation remains open until its
  Store, PostgreSQL, browser, and evidence slices are complete on one final material tree.
- Accepted WP-INST-D2 live problem curation and advanced the Instructor queue to WP-INST-B1 reusable
  curricula. Favorites, private and institution collections, canonical saved searches,
  revision-checked bulk actions, and the shared `ProblemPicker` now compose the ordinary Library and
  assignment-authoring paths. Elena Instructor and Morgan Sysadmin retain their independent passkey
  journeys, and browser contracts expose safe catalog projections plus public Question IDs.
- Completed D2 production evidence: the 66-migration PostgreSQL baseline and isolated curation
  oracle passed forced-RLS, broker, canonical-filter, immutable-membership, revision, and cleanup
  checks; the visible HTTPS journey passed curation-to-live-assignment reuse; and the privacy-checked
  corpus includes Elena's three canonical desktop D2 views and Morgan's desktop institution view.
  Architecture, security/privacy,
  HCI/accessibility, and documentation/evidence reviews reported zero P0--P3 findings.
- Made the canonical acceptance lifecycle resilient to interrupted owners. Fixed-project recovery
  now gives Podman the complete verified container set with dependency-aware removal. Schema startup
  checks retain existing Base Course freshness policies when the exact catalog graph is compatible,
  reconcile only observed drift, and verify the repaired graph; shared current-schema live oracles
  use read-only compatibility checks while isolated upgrade oracles retain migration authority.
- Final `source source_me.sh && ./all_test.sh` passed the Rust workspace, all five codebase gates
  including 297 Node tests, 6,982 pytest checks, and the complete connected acceptance path with
  exact cleanup.
- Accepted WP-INST-D1 question discovery on the canonical live product path and advanced the
  Instructor queue to WP-INST-D2 collections, Favorites, saved searches, bulk curation, and reusable
  problem selection. Library search now combines public metadata and response-family facets with
  validity-governed cross-course evidence and actor-authorized own-course usage. Generated
  questions show one deterministic server-materialized example through the shared semantic prompt
  renderer; the answer-free browser contract excludes seed, randomization, response, grading,
  source, and answer material.
- Completed D1 production evidence: the disposable PostgreSQL baseline passed all 65 migrations and
  Store/RLS oracles; the visible HTTPS journey produced five independent observations across two
  courses while retaining Elena Instructor and Morgan Sysadmin passkey entry; and the canonical
  desktop discovery evidence passed screenshot provenance, privacy, keyboard, and independent
  visual review with exact resource cleanup. Architecture, security/privacy, and HCI reviews
  reported no P0--P3 findings.
- Restored the repository quality gates after the discovery expansion by splitting the Memory
  catalog search implementation and its statistics/detail tests into focused modules, moving
  problem-detail presentation into a page-owned stylesheet, registering the discovery scenario in
  the canonical local-import inventory, and clearing regeneratable stale Rust build output before
  final Validation.
- Final `source source_me.sh && ./all_test.sh` passed the Rust workspace, all five codebase gates
  including 275 Node tests, 6,727 pytest checks, and the aggregate connected acceptance path.
  `source source_me.sh && ./capture_screenshots.sh` separately passed all production visual
  scenarios and committed the privacy-validated 77-artifact corpus.

## 2026-08-24

### Fixes and Maintenance

- Accepted WP-INST-T5 item pools on the canonical live-demo product path. Elena creates an ordered
  mixed fixed/pool assignment using public Question IDs, previews a server-generated no-store draw,
  and the normal Student path issues, grades, resumes, and exposes immutable evidence for the
  selected items. Pre-issue structural edits serialize against first run; issued work stays
  immutable and later structural editing presents a visible new-assignment recovery path.
  Production HTTPS acceptance, refreshed screenshot provenance/privacy publication, independent
  visual review, and complete final Validation passed; the Instructor handoff advances to
  WP-INST-D1 discovery.
- Hardened WP-INST-T5 assignment authoring: create and update now establish the session,
  Instructor course authority, and update-route assignment binding before bounded JSON decoding;
  every refusal remains `no-store`. Shared Rust/TypeScript cardinality limits now cap ordered
  entries at 1,024, candidates per pool at 1,024, and total candidates at 8,192 before any catalog
  resolution; accessible authoring recovery feedback names the applicable correction.
- Accepted `WP-INST-LD3` on the canonical live product path and advanced the Instructor queue to
  `WP-INST-T5` item pools. The production HTTPS browser completed all ten visible role, passkey,
  authoring, preview, replacement, grading, learner-delivery, recovery, WebWork, and QTI-import
  scenarios; the isolated WebWork and replica-restart oracles also passed with exact durable replay
  and empty final resource, process, private-artifact, and workspace inventories. The 63-artifact
  production screenshot corpus passed privacy checks and independent visual review. Reserved
  `2026081826` for T5's one complete revision-checked pre-issue assignment-definition replacement,
  closed v1 pool-draw authority, and database serialization against first run creation.
- Initialized the browser owner's ordered execution contracts for ordinary and screenshot selections
  after optional screenshot replacement, preventing ordinary acceptance from raising an unbound-local
  failure; added a focused behavioral regression for the ordinary child-execution path.
- Selected the live demo as PLE's single product and acceptance path and retired the unaccepted
  `WP-INST-T4` parallel execution sidecar before production. New current package `WP-INST-LD3`
  converges ordinary assignment mutation, learner delivery, deterministic server-owned grading,
  immutable issued evidence and receipt replay, course grade control, and audited Instructor
  inspection. Removed the sidecar domain, Store, server composition, routes, generated contracts,
  dedicated tests, E2E hook, and migrations 1811, 1813, 1815, 1821, and 1822; recomposed the
  surviving 1812, 1814, and 1816--1820/1823 authority chain around live course state. The active
  source and migration inventories contain no retired execution references; focused Rust suites and
  the fresh PostgreSQL assignment-authority oracle are green.
- Restored whole-course grade-settings mutation after assignment writes moved behind their dedicated
  broker. PostgreSQL now performs the exact assignment-set validation and serialization inside the
  grade-control capability; the application adapter validates only the closed request shape. Hand-
  raised T4 expected-revision and immutable-witness conflicts no longer masquerade as retryable
  database serialization failures, so stale grade drafts reach the browser as the intended preserved
  412 workflow. The settings page also keeps a successful save acknowledged when its subsequent
  totals refresh fails and names the visible recovery action.
- Bound browser HTTP requests and Solid Router query entries to one abortable session generation.
  Confirmed logout and subsequent session establishment now retire pending work before another
  account can load, while the production-browser course picker waits for visible course identity
  instead of treating the persistent application shell as route readiness.
- Kept learner assignment discovery and explicit entitlement inspection on the read-only S5 path.
  These projections no longer request mutation locks that the application role intentionally lacks;
  later learner writes continue to reauthorize through their server-owned transactional capability.
- Split learner activity projection hydration from mutation locking. Assignment-overview progress,
  enrollments, runs, attempts, summaries, and instructor history now use plain snapshot reads and the
  read-only entitlement evaluator; brokered activity transitions remain the sole owners of write
  serialization. This restores the live learner assignment entry without broadening application-role
  privileges.
- Kept post-start run items, attempt lists, prefetched-question lookups, and pending-submission
  projections on that same read boundary. Server-owned attempt issuance continues through the 1817
  learner-work broker witness, while ordinary run navigation no longer tries to lock teaching source
  rows through the application role.
- Made public course, assignment, run, and workspace reference resolution uniformly projection-only.
  Learner run-reference authorization now evaluates current entitlement without mutation locks, so a
  newly issued run can open through its opaque public route under the least-privilege application role.
- Separated future-content replacement from active-attempt timing-policy mutation. A revision-checked
  fixed-item replacement now updates the assignment definition for future runs while leaving issued
  question evidence and effective-policy receipts untouched, matching the immutable issued-work
  contract and avoiding a false assignment conflict when learner work is active.
- Completed the focused assignment-mutation boundary around that replacement. The consolidated
  broker trigger now admits only its transaction-scoped operation marker, Rust verifies the exact
  revision advance returned by every focused mutation, wrong course/tenant routes are concealed
  before broker entry and revalidated under its lock, and repeated ordered positions may reference
  the same immutable publication without weakening reference validation.
- Bound successor-receipt reads and finalization to the explicit learner course/assignment route.
  Receipt replay, issued-presentation validation, and completed-run summaries are lock-free
  projections; successor mutation remains owned by the learner-work preparation broker. Completed
  public receipts now always suppress successor delivery state, including idempotent replay, while
  wrong routes remain concealed and cannot alter immutable receipt evidence.
- Kept assignment delivery preview source reads on the same least-privilege projection boundary.
  Derived previews now bind their internal assignment and membership evidence inside one consistent
  repeatable-read snapshot and append the private audit as the final statement, without acquiring
  teaching-data mutation locks through the application role.
- Restored teaching-operations projections under the same least-privilege rule: instructor checks
  and course-group policy reads now remain plain application-role reads, while the dedicated
  server-owned mutation brokers retain locking and current-authority revalidation. The live browser
  journey now waits for the visible group editor before attempting its first mutation.
- Closed co-instructor invitation mutation authority with session-derived database capabilities for
  create, revoke, and decline, completed the existing acceptance broker's lock privileges, and
  revoked direct application-role invitation writes and sequence access. Memory and PostgreSQL now
  reject an actor that disagrees with the presented active session.
