# Changelog

## 2026-08-26

### Additions and New Features

- Added the WP-PROF-T6 W2 Instructor assignment workspace shell: canonical Overview, Questions,
  Policies, and Student view routes share one course-authorized assignment load, assignment titles
  open the Instructor Overview, and local navigation preserves the surrounding course management
  context.
- Extracted the answer-free learner assignment landing into a reusable presentation component;
  ordinary learner overview retains query, progress, and start-or-resume control while Instructor
  Student view can supply an informational action, context cue, and return link.
- Implemented the Instructor Student-view landing: the current answer-free live assignment is
  loaded through the exact course/assignment authority, rendered by the shared learner
  presentation, and paired with stable-identity, no-mutation guidance to explicit Student entry.
- Implemented the WP-PROF-B2 curriculum-adoption API and Instructor browser composition across
  preview-before-save adoption, rollover, term shifting, provenance receipts, controlled
  fast-forward, and divergence recovery.

### Fixes and Maintenance

- Reconciled the real-stack browser plan with the complete live-demo baseline, including the two
  ordinary Chapter 1 teaching courses, five persisted learner observations, and the current
  75-artifact screenshot corpus. Updated operator examples to the canonical root launcher and
  repaired small documentation, ASCII, and Python-readability issues found by the six-pass audit.
- Rebuilt `docs/RELATED_PROJECTS.md` as an evidence-first visitor guide using the current
  relationship taxonomy and confidence tiers. Every retained or added destination now states its
  shared audience outcome and authoritative evidence; current integrations and explicit prior art
  remain separate from adjacent alternatives and the planned LTI Advantage reference.
- Removed the healthy browser-Wasm implementation banner from the live product shell. The
  production UI now stays focused on teaching work while an inert diagnostic preserves connected
  browser evidence; visible fallback copy accurately explains that slower checks use the server.
  Republished all 75 declared real-stack screenshots against the corrected shell.
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
- Published all 75 declared live-demo screenshots from the production-shaped HTTPS stack. PNG,
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

### Decisions and Failures

- Added the owner-directed `WP-PROF-T6` assignment-workspace binding plan and advanced the current
  professor handoff to it before `WP-PROF-G1`. The assignment title becomes the canonical entry to
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

### Developer Tests and Notes

- Passed the focused disposable PostgreSQL/RLS curriculum-adoption oracle and the connected
  production-shaped HTTPS four-context curriculum-adoption browser journey, including visible
  creation, DST correction, controlled update, divergence recovery, destination evidence, and
  rollover.
- Passed focused TypeScript compilation, ESLint, Node browser-contract/workflow checks, Python
  scenario-contract checks, 23 question-model tests, 62 learning-data-access tests, and the
  repository ASCII, source-size, Markdown-link, and diff-hygiene gates.
- Passed the complete repository codebase gate with all five checks and 322 Node tests, plus all
  7,361 pytest checks. The complete Rust gate passed generated contracts and fixtures, formatting,
  both compile graphs, all three strict Clippy graphs, both test/doctest graphs, and the browser Wasm
  target. Independent post-fix review returned ACCEPT with no unresolved P0, P1, or P2 finding.
- Passed all 15 connected production-browser journeys, the 77-migration PostgreSQL/RLS/persistence
  baseline, the isolated WebWork scoring and outage oracle, and the API replica restart/replay
  oracle. Each disposable owner returned an exact cleanup receipt.
- Accepted WP-PROF-B2 and advanced the professor handoff to WP-PROF-G1. Final
  `source source_me.sh && ./all_test.sh` passed the complete Rust, 322-case Node, 7,361-case pytest,
  15-journey production-browser, 77-migration PostgreSQL, WebWork, replica-restart, and exact-cleanup
  gates on the published material tree.
- Passed 26 focused developer-lease, exact-reset, and CLI contracts. Two consecutive real
  `./run_live_demo.sh --headless` launches proved active-owner replacement; the replacement exposed
  only its loopback HTTPS gateway, returned HTTP 200, reached six running long-lived services, and
  printed the matching root stop command for operator handoff.

## 2026-08-25

### Fixes and Maintenance

- Made API schema startup topology-aware: disposable local PostgreSQL now fails before serving
  when schema verification is unavailable, while AWS workload retains its degraded-start diagnostic;
  incompatible schemas remain fatal for both topologies.
- Established the audited `WP-PROF-B2` domain and browser-contract foundation for explicit
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
- Accepted WP-PROF-B1 reusable curricula and advanced the professor queue to WP-PROF-B2 curriculum
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
  weakened it. Professor/Instructor and Sysadmin visual evidence now uses only the canonical 1280 by
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
- Accepted WP-PROF-D2 live problem curation and advanced the professor queue to WP-PROF-B1 reusable
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
- Accepted WP-PROF-D1 question discovery on the canonical live product path and advanced the
  professor queue to WP-PROF-D2 collections, Favorites, saved searches, bulk curation, and reusable
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

- Accepted WP-PROF-T5 item pools on the canonical live-demo product path. Elena creates an ordered
  mixed fixed/pool assignment using public Question IDs, previews a server-generated no-store draw,
  and the normal Student path issues, grades, resumes, and exposes immutable evidence for the
  selected items. Pre-issue structural edits serialize against first run; issued work stays
  immutable and later structural editing presents a visible new-assignment recovery path.
  Production HTTPS acceptance, refreshed screenshot provenance/privacy publication, independent
  visual review, and complete final Validation passed; the professor handoff advances to
  WP-PROF-D1 discovery.
- Hardened WP-PROF-T5 assignment authoring: create and update now establish the session,
  Instructor course authority, and update-route assignment binding before bounded JSON decoding;
  every refusal remains `no-store`. Shared Rust/TypeScript cardinality limits now cap ordered
  entries at 1,024, candidates per pool at 1,024, and total candidates at 8,192 before any catalog
  resolution; accessible authoring recovery feedback names the applicable correction.
- Accepted `WP-PROF-LD3` on the canonical live product path and advanced the professor queue to
  `WP-PROF-T5` item pools. The production HTTPS browser completed all ten visible role, passkey,
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
  `WP-PROF-T4` parallel execution sidecar before production. New current package `WP-PROF-LD3`
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
