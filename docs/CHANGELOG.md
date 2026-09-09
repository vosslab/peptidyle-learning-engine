# Changelog

## 2026-09-09

### Additions and New Features

- Replaced the eight-image Live Demo capture path with a 52-image, manifest-driven atlas spanning
  Public, Instructor, Student, and Sysadmin workflows. The active corpus is flat within four
  role-owned folders and uses the canonical laptop, tablet, phone, and square viewport profiles.

- Added a generated scan-oriented Screenshot Atlas, a receipt binding the manifest digest and
  exact PNG path/dimension/hash set, and a coverage ledger accounting for every current route and
  Ribbon destination as captured, covered by equivalent visual evidence, or deferred for a named
  missing product capability.

- Added typed screenshot scenarios with one registry mapping manifest checkpoints to executable
  workflows. Captures now exercise ordinary Live Demo routes and persisted mutations for Question
  publication, Course invitation claim, Assignment release and Student response progression,
  Instructor Account lifecycle, and scoped Sysadmin support.

### Fixes and Maintenance

- Made `devel/capture_screenshots.sh` the single capture and replay operator contract. Publication
  validates the complete staging corpus before promotion, retains a full recovery backup during
  the portable multi-path replacement, rolls back ordinary failures, and rejects stale recovery
  evidence instead of claiming unsupported filesystem atomicity.

- Expanded screenshot validation around semantic state, same-origin navigation and transport,
  page errors, closed privacy profiles, canonical dimensions, exact output closure, duplicate
  bytes, manifest-to-registry closure, and deterministic gallery generation. Live verification
  reports byte differences for review without using pixel equivalence as a machine gate.

- Closed the pre-merge screenshot audit findings without adding another orchestration layer. The
  publisher now rejects unknown root files, folders, and role-folder entries; its ordinary
  mid-replacement rollback is fault-tested; receipt tampering and privacy-profile selection have
  focused coverage; and the focused corpus suite is discovered by the canonical Node test lane.

- Replaced the Student grading sleep with the existing state-based Playwright polling idiom. The
  two workflows that intentionally navigate immediately after loading role data now wait for and
  drain those exact completed responses before navigation, so privacy inspection remains strict
  without racing response-body disposal.

- Added selector-contract source citations to the four scenario-family modules and documented the
  intentional same-origin scoped-support capability preparation. Updated screenshot troubleshooting
  and removed a stale direct JavaScript capture command from an older active plan.

- Recorded the Assignment Attempt time limit and unlimited-retry learning model redundantly in the
  Student guidance so an independently read role section preserves the intended practice policy.

- Archived the completed
  [archive/durable_live_demo_screenshot_corpus.md](archive/durable_live_demo_screenshot_corpus.md)
  with its final verifier summary, closeout gates, and implementation findings. The replay atlas
  and all 52 replay PNGs remain in the ignored `test-results/screenshot-corpus/verify/` review lane.

- Corrected the Assignment Attempt completion trigger's execution authority
  with forward migration `2026090901`. Authorized PLE, WeBWorK, and iMathAS
  Question Backend grading writers now apply the released Assignment
  Completion Rule through `ple_private_owner`, the Database Schema Owner Role
  for `ple_private`, without receiving direct `completed_at` update access.

- Added migration-catalog evidence for the trigger's exact owner,
  security-definer mode, search path, and closed execute ACL. The existing
  iMathAS PostgreSQL Store test continues to prove that one committed Grading
  Result records its Question Statistics Observation exactly once and now also
  reaches the completion invariant successfully.

- Renamed `source_me.sh`'s temporary repository-root variable so sourcing the
  environment inside a launcher that owns a read-only `REPO_ROOT` no longer
  emits a shell assignment error during an otherwise successful acceptance
  lane.

- Scoped the Course Appearance bucket initializer to an explicit Compose
  profile and disabled its pseudo-TTY. Its one-shot container remains disposable
  with `--rm`, while the default cross-store teardown no longer asks Compose to
  remove that already absent service container or emits interactive-terminal
  noise.

### Decisions and Failures

- Closed the screenshot-corpus plan without extending its architecture from implementation-run
  visual observations. Human review of the 52-image atlas is the next separate activity; concrete
  UI defects or coverage gaps discovered there become focused work of their own.

- Six independent Plan, Test, Style, Documentation, Legacy, and Comment audit passes found no
  blocker or high-severity issue. Live verification then exposed a navigation race in the privacy
  monitor: completed JSON response bodies could be discarded if a workflow immediately left the
  page. Starting reads for every response was rejected after canceled traffic could wait
  indefinitely; exact request-completion boundaries fixed the demonstrated cases without weakening
  privacy checks.

- Diagnosed the 2026-09-08 aggregate failure as an execution-owner mismatch:
  the accepted completion trigger ran as `ple_api_owner`, whose deliberately
  narrow Assignment Attempt privilege permits a row lock but not mutation of
  `completed_at`. The repair preserves that least-privilege boundary in a
  forward migration instead of changing accepted migration history or
  widening API and worker table grants.

### Developer Tests and Notes

- `node --import tsx --test tests/test_screenshot_corpus.mjs` passed all nine focused corpus tests.

- `./check_codebase.sh` passed strict TypeScript, ESLint, Prettier, and all 359 Node tests after the
  screenshot-corpus integration.

- `./devel/run_playwright_tests.sh` passed the complete serial production-browser suite against a
  fresh stack: authorization, Instructor authoring, native Student recovery, Assignment release,
  WeBWorK rendering, Instructor Account lifecycle, scoped support, invitation export, and Course
  seed journeys. An initial restricted-shell launch was denied by macOS Mach-port sandboxing before
  the first browser action; the unrestricted rerun passed without an application failure.

- `./devel/capture_screenshots.sh` published all 52 captures from a clean Live Demo with role counts
  `3/19/23/7`, no unmanaged or byte-identical active PNGs, a bound receipt and atlas, and clean
  stack teardown.

- `./devel/capture_screenshots.sh --verify` passed static validation and all 52 live replay
  checkpoints from another clean stack, preserved the tracked corpus, reported five byte-level
  differences for human review, retained the temporary atlas, and proved teardown.

- `source source_me.sh && ./launchers/all_test.sh` passed the final closeout tree: 416 generated
  Rust-owned TypeScript contracts, three fixture contracts, Rust formatting/checks/strict Clippy/
  tests/doctests, the browser Wasm target, 359 Node tests, 6,004 Python tests, PostgreSQL migration/
  authority/persistence acceptance, and PostgreSQL-plus-MinIO Course Appearance coherence. Both
  real-service lanes removed their disposable resources, and the aggregate ended with
  `PASS: complete live acceptance is green.`

- The final post-audit `./devel/capture_screenshots.sh --verify` replayed all 52 captures, reported
  five non-gating byte differences, preserved the tracked corpus, retained the temporary atlas,
  and ended with a clean owned stack.

- The focused Markdown-link, ASCII, and whitespace suite passed 1,886 checks, and
  `git diff HEAD --check` passed on the mixed staged and unstaged working tree.

- `source source_me.sh && python3 local_stack.py acceptance` passed the fresh
  74-migration apply, no-op replay, PostgreSQL 17 catalog and restricted-login
  probes, all three iMathAS Question Backend PostgreSQL Store tests, cleanup,
  and the PostgreSQL-plus-MinIO Course Appearance coherence oracle.

- `source source_me.sh && ./launchers/all_test.sh` passed on the final material
  tree: 416 generated Rust-owned TypeScript contracts, three tracked fixture
  contracts, Rust formatting/checks/strict Clippy/tests/doctests, 350 Node
  tests, 5,969 pytest cases, the disposable PostgreSQL 17 schema/authority/
  persistence lane, and PostgreSQL-plus-MinIO Course Appearance coherence.

## 2026-09-08

### Behavior or Interface Changes

- Compacted authenticated Application Shell chrome by moving the single
  Peptidyle home identity into the Ribbon Context Row and removing the
  redundant authenticated site-header band. Context and Tab Rows remain
  persistent, while the Task Row now appears exactly when the declared route
  topology contains a task group.

- Preserved admission-independent Ribbon geometry. A task-capable route keeps
  its Task Row even when every Task is unavailable, while loading, deferred
  labels, content errors, relationship checks, and authorization outcomes do
  not add or remove rows.

- Restored an accepted Student response's answer-free grading-status panel
  directly after reload. The existing private browser marker now triggers the
  server-authorized status read without starting a new Assignment Attempt or
  exposing a response, answer, correctness, or score.

- Connected the Instructor Course Instance page to the ordinary authorized
  Assignment-list contract. It now shows every persisted Assignment with its
  real lifecycle status and workspace link, keeps roster and creation actions
  available, and removes obsolete restoration-lane copy from the product UI.

- Made `/` the one Product Role-aware Course index promised by the route
  contract. Students now reach their real current Course Instances through the
  shared Courses Ribbon destination, and the redundant Student-only index route
  was removed instead of adding role-specific Ribbon navigation logic.

### Fixes and Maintenance

- Rotated the complete 2026-09-06 through 2026-09-04 day blocks into
  `docs/CHANGELOG-2026-09e.md` after the active changelog crossed its
  repository line threshold; the two newest day blocks remain active.

- Centralized the viewport-height floor in `.ple-shell-frame`, replacing
  per-shell and course-theme height subtraction with structural grid tracks.
  Short themed and unthemed pages now fill the viewport without chrome-induced
  document overflow, and tall pages continue to grow normally.

- Removed global `nav` and `nav a` presentation leakage from the shared style
  sheet. Ribbon navigation is component-owned, while the Course Instance page
  now owns the only retained course-action navigation treatment. Shared shell
  top padding was also tightened for the compact frame.

- Expanded offline and Chromium evidence for taskful and taskless topology,
  admission-stable geometry, live topology transitions, branded narrow-screen
  behavior, shell-height ownership, course-canvas extent, focus, and local CSS
  ownership.

- Reconciled connected-browser navigation and seeded-state oracles with the
  current used-Course baseline and direct Course Instance Assignment action.
  The canonical fresh-stack production-browser suite now passes its complete
  authorization, authoring, recovery, release, render, administration,
  support, invitation, and seeded-Course journey set.

- Rebuilt all eight manifest-owned current screenshots with the compact
  authenticated chrome and passed the screenshot manifest verifier. Capture
  privacy continues to reject protected Student fields while recognizing the
  exact assignment-landing endpoint's contract-approved self-only aggregate
  points as safe response data.

- Corrected the Instructor Gradebook route to validate and use its exact Course
  Instance Reference directly, matching the ordinary Course roster and
  Assignment surfaces. The browser now reaches the existing server-authorized,
  answer-free Gradebook HTTP contract without depending on the unrelated legacy
  route-scope lookup.

- Corrected the foundational seed-inventory receipt to validate its current
  named aggregate fields and recognize the real Pending-to-Available Question
  Asset delivery transition. The two-start replay now proves the unchanged
  `5|4|4|4|4|1|1|1|1` inventory after complete startup.

- Anchored the Live Demo launcher and its TypeScript prerequisite helper to
  their own filesystem locations, removing Git from the runtime launch path.

- Removed the rootless-only Podman gate from the disposable Local Stack and
  Developer Browser Suite. The local migration and API-login verification now
  run in a profile-only Compose job, so the active rootful or rootless
  connection can run the Live Demo without relying on host PostgreSQL port
  forwarding; containment remains in the Compose topology.

- Bound the Local Stack's `podman-compose` provider to the Python 3.12
  interpreter selected by `source_me.sh`, rather than Homebrew's independent
  wrapper interpreter. The declared runtime dependency now installs that
  provider alongside the controller.

- Made the Podman lifecycle recover from ordinary host variation without
  enforcing rootless or rootful execution. A stopped default machine gets one
  bounded start retry, Compose selection falls back from Podman's dispatcher to
  the selected Python module and then a standalone `podman-compose`, incomplete
  diagnostic metadata remains informational, and `doctor` distinguishes the VM
  provider from its guest operating system. The macOS guide now records the
  AppleHV rootful initialization used by the second development machine, and
  the one-shot database migrator no longer inherits the API health check.

- Split Student activity convergence, Browser Suite external operations, and
  lifecycle database identity construction into focused modules. Their former
  owners are now below the repository's source-size ceiling without overrides
  or compressed logic, and the two runnable Course acceptance scripts have the
  required executable mode.

- Reconciled the Live Demo specification, local-stack operations, API and
  database maps, design decisions, and cookbook with the shipped used-Course
  baseline and its real Instructor and Student launch-readiness workflows.

### Decisions and Failures

- An additional `./launchers/all_test.sh` run passed generated contracts,
  Rust formatting/checks/Clippy/unit tests/doctests, codebase checks, 5,949
  pytest cases, and the preceding PostgreSQL migration/authority probes, then
  stopped in the isolated iMathAS database oracle because
  `postgres_store_commits_statistics_from_the_stored_grade_exactly_once`
  received `Forbidden`. This isolated database authorization failure was
  untriaged at that checkpoint; the narrower Ribbon evidence and complete fresh-stack
  production-browser suite are green and are not represented as aggregate acceptance.

- Retired the former rule that reserved an empty Task Row on every Ribbon.
  Route topology, not capability admission, now owns whether that row exists:
  this removes unused chrome without making authorization or asynchronous state
  observable through geometry.

- Retired viewport-height calculations distributed across the ordinary shell
  and course-theme canvas. They encoded chrome knowledge in multiple owners and
  could overflow a short page after chrome changed; the shell frame now owns
  the one structural viewport floor.

- Kept current screenshot ownership with `docs/SCREENSHOT_CONTRACT.md` and its
  capture manifest. The older UI design review remains historical and was not
  repurposed as an attachment ledger.

## 2026-09-07

### Additions and New Features

- Added disposable Live Demo Course acceptance for the complete used-teaching
  baseline. Its four modes prove exact Course, roster, released Question set,
  Student work, and Gradebook state; realistic anonymous, Student,
  cross-Student, and Sysadmin concealment; repeat convergence without duplicate
  Course objects; and recovery after every provisioning stage. The fixed stack
  accepts a bounded `start --stop-after <stage>` debug flag solely to
  manufacture those fresh interruption checkpoints; ordinary startup still
  converges the full baseline.

- Wired complete Live Demo Course provisioning into the fixed TLS lifecycle
  after whole-stack readiness and before browser opening. A fresh
  `run_live_demo.sh start --headless` now reports success only after the
  realistic teaching baseline and its private zero-outstanding-stage report
  exist; non-TLS and non-owned targets retain their prior lifecycle.

- Completed representative Live Demo Student activity through the same
  convergent provisioner. Mary now has four genuinely graded Question
  Submissions with a pinned mixed `2/4` result, Jack has two genuinely graded
  submissions in one open four-Question Assignment Attempt, and Avery remains
  enrolled without an Assignment Attempt. Response references are derived only
  from each answer-free Question Presentation, and interrupted grading resumes
  from nonce-bound status plus real Gradebook facts.

- Added the standalone convergent Live Demo Course provisioner and its
  `provision-course` debug command. It resolves Blueprint Course, Course
  Instance, roster, Assignment, selection, and release state through ordinary
  product HTTP contracts; supports bounded `--stop-after` and read-only
  `--report` planning; keeps sessions in transient private cookie jars; and
  writes a mode-0600 product-fact report whose machine references make retries
  restart-safe.

- Added the ordinary Instructor Assignment list for one exact Course Instance.
  The new product-named database function, Store operation, and HTTP route
  return only Assignment Reference, title, status, and Edit Number; active
  direct Course Membership is rechecked in PostgreSQL, while anonymous,
  Student, Sysadmin, and foreign-Instructor requests receive the same `404`
  concealment.

- Added fixed-origin Live Demo gateway builders for seeded persona sessions
  and authenticated JSON product requests. Session credentials remain in
  private cookie-jar files rather than command arguments, while API paths,
  methods, bodies, and Assignment Edit Number preconditions are validated and
  encoded at one controller boundary.

- Defined the required Live Demo teaching-data baseline in the authoritative
  specification and added one I/O-free declarative Course seed with an ordered
  convergence planner. The baseline derives its roster emails and four
  Question IDs from the foundational seed and describes Mary, Jack, and Avery
  through real Assignment Attempt and Question Submission facts rather than
  display-only demo states.

- Completed Live Demo restoration M19. The final fresh controller-managed fixed
  HTTPS stack passed `./devel/run_playwright_tests.sh --build`; its serial owner
  exercised connected authentication, Instructor authoring, Student recovery,
  Assignment release, WeBWorK render, Sysadmin Account, scoped-support, and
  invitation-export journeys against the production bundle. This is connected
  browser evidence, separate from the narrower focused milestone and service receipts.

- Completed Live Demo restoration M20. `./devel/capture_screenshots.sh`
  created eight safe, manifest-listed connected captures through visible PLE
  navigation, then stopped its owned stack. The one-time artifacts live in
  `public/`, `instructor/`, `student/`, and `sysadmin/` screen folders; the
  dedicated `--verify` mode validates their PNG manifest without starting a
  stack. The current corpus includes one normal desktop Ribbon capture for
  each Product Role and Student tablet, phone, and square responsive evidence.

- Completed Live Demo restoration M15. Fresh controller-managed fixed HTTPS
  stacks passed `bash tests/e2e/e2e_live_demo_gradebook.sh --api` and
  `--browser`: the current Course Instructor receives answer-free immutable
  Gradebook evidence, foreign-Course access receives 404 concealment, and the
  visible Gradebook task completes. Individual Student Work remains
  unavailable; M19 serial-browser acceptance remains unclaimed. This is
  whole-system plan acceptance/live browser evidence, not a new pytest.

- Completed Live Demo restoration M16. A fresh controller-managed fixed HTTPS
  stack passed `bash tests/e2e/e2e_live_demo_instructor_accounts.sh`: the
  service proves the Sysadmin-only Instructor Account lifecycle, concealment,
  and session revocation, while the visible Sysadmin Ribbon task creates an
  Instructor Account and deactivates then reactivates it. The disposable M16
  migration correction grants only `ple_private_owner` the Instructor-only
  lock policy needed by the existing `SELECT ... FOR UPDATE` boundary; no
  direct application/API table access was granted. This is plan
  acceptance/live browser evidence, not a new pytest. Course or Student Record
  access, passkey feature work, and M19
  serial-browser acceptance remain unclaimed.

- Completed Live Demo restoration M17. A fresh controller-managed fixed HTTPS
  stack passed `bash tests/e2e/e2e_live_demo_support_capability.sh --issue` and
  `--browser`: an Instructor issues and revokes exact-Course registered
  roster-support capability while foreign or unregistered state remains
  concealed, and the visible Sysadmin scoped roster task completes. It grants
  no ambient Sysadmin Course or Student Record access. This is whole-system
  plan acceptance/live browser evidence, not a new pytest; M19 serial-browser
  acceptance remains unclaimed.

- Completed Live Demo restoration M13. A fresh controller-managed fixed HTTPS
  stack passed `bash tests/e2e/e2e_live_demo_submission_recovery.sh`: one
  format-valid Student Response is accepted once with closed pending grading
  state, and an interrupted leased native PLE evaluation recovers one terminal
  result and receipt. This is whole-system plan acceptance evidence, not a new
  pytest. M15 Gradebook/Student Work and M19 serial-browser acceptance remain
  unclaimed.

- Completed Live Demo restoration M12. The final fresh controller-managed fixed
  HTTPS stack passed `bash tests/e2e/e2e_live_demo_native_controls.sh`:
  authorized Question Asset retrieval returns an immutable redirect while
  anonymous, foreign-Student, absent, and malformed references have
  indistinguishable concealment; all eight issued native PLE response formats
  strictly decode; and all eight controls, including the fixed HOTSPOT, reach
  valid local states by keyboard without submission. This is plan
  acceptance/live browser evidence, not a new pytest. Student Response
  persistence, submission, grading, feedback, recovery, and M19
  serial-browser acceptance remain unclaimed.

- Revalidated M11 on the same fresh controller-managed fixed HTTPS stack with
  `bash tests/e2e/e2e_live_demo_assignment_attempt.sh`. The format-only M12
  presentation now omits M13 submission, grading, and feedback controls; this
  does not implement any M13 behavior.

- Completed Live Demo restoration M14. WP-M14-1's answer-free WeBWorK
  render-issuance boundary previously passed with `--render`; on a fresh
  controller-managed stack, `bash tests/e2e/e2e_live_demo_webwork.sh --grade`
  passed: `WeBWorK grade authority: deterministic renderer grade commit and
bounded renderer failure complete`; `Live Demo WeBWorK grade: PASS`. The
  renderer fault is a bounded terminal local outcome. This service/browser
  cadence does not claim M19 serial production-browser acceptance.

### Behavior or Interface Changes

- Replaced role-suffixed Live Demo persona labels with the complete fictional
  identities Elena Rivera, Mary Okafor, Jack Nguyen, Avery Thompson, and Morgan
  Delgado. The seeded Student Authentication Emails now use matching
  non-routable `.invalid` addresses while preserving the closed persona keys
  and ordinary Account, Authenticated Session, Course Roster Import, and Course
  Membership authority paths.

- Reconciled M21 current-boundary documentation with the Human Guidance and
  Terminology Contract authority order. The Live Demo is now documented as the
  connected PLE application, and M19's nonce-bound submission status continues
  to expose only grading state while Student Feedback remains separately
  policy-evaluated.

### Fixes and Maintenance

- Clarified the API-contract evidence boundary: current route claims specify
  the behavior exercised by the separately recorded completed M19 connected
  production-browser acceptance; they do not substitute for that receipt.

- Removed the unreferenced legacy roster-delivery display helper. Its retained
  transport contract remains a separately owned API-retirement decision.

- Reconciled current implementation, evidence, and accessibility documentation
  after an independent whole-codebase audit. Retired unreferenced browser-E2E
  support now leaves the registered serial owner as the only active path;
  permanent source comments describe current capability boundaries rather than
  temporary execution labels; and screenshot capture uses the shared Git-root
  anchor. The audit kept the fast-suite CLI-scan and selected-E2E-runner policy
  questions explicit rather than adding speculative test machinery.

- Restored the Student-owned Assignments Ribbon tab during Assignment Access
  and Question Presentation, and retained that selected tab at the existing
  Student Course landing route. Server and route authorization are unchanged.
  The one-time screenshot rebuild now verifies a selected normal Ribbon tab
  before writing each authenticated artifact.

- Corrected M14 WeBWorK issuance to return the first atomic Assignment Attempt
  projection instead of a second call that had already resumed that attempt.
  The one-time render proof now verifies the true initial and resume states,
  its private replay join, and the outcome-free Student surface.

- Corrected the disposable M8 Course Instance helper to select its fixed Elena
  Instructor persona from a multi-Instructor active list. M16 may create
  additional active Instructor Accounts; global Instructor cardinality is not
  a Course Instance requirement.

- Synchronized shared style guides, tests, and repository support files from the starter template.

- Repaired the one-time screenshot rebuild boundary: it now waits for the
  visible seeded sign-in surface after navigation commit rather than unrelated
  document completion, then begins workflow-response privacy inspection. The
  generated WebAssembly bridge now uses its current object-shaped initializer.
  A fresh `./devel/capture_screenshots.sh` rebuild produced the eight declared
  artifacts, stopped its owned stack, and passed `--verify`.

- Restored the M8 Course Instance acceptance assertion that the newly Assigned
  Instructor can retrieve the exact closed teaching-team projection. It now
  accepts one or more active Instructors rather than encoding the retired
  one-Instructor fixture cardinality. A fresh authority run passed and stopped
  its owned stack.

- Audited the restoration's current browser inventory. Retired unregistered
  legacy scenario providers, specifications, helpers, and an obsolete
  provider-only pytest rather than reviving unavailable routes for their sake.
  The registered serial owner, focused scenario partition, and the current
  M10/M14/M16/M17/M18 browser journeys remain the connected acceptance set.

- Removed a brittle complete-inventory Ribbon assertion, replaced a capture
  CSS selector with a role-scoped control, and corrected permanent comments to
  use established product terminology. Current role guides and historical
  visual references now distinguish the functional capture workflow from their
  retained images.

- Bound the disposable WeBWorK browser proof to the Assignment reference it is
  given, preserving the accessible link interaction while avoiding an
  accidental first-card selection. Closed the Ribbon fixture parameter maps
  over the current declared route parameters, so a future route parameter
  cannot silently leave shared test support untyped.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Removals and Deprecations

- Removed the retired structural Live Demo presentation route and its separate
  screenshot directory. Current rendered evidence is role-owned rather than a
  parallel demo gallery.

### Developer Tests and Notes

- M21 used a one-time manual authority reconciliation rather than adding the
  absent planned authority-ledger script as a fragile permanent source-inventory
  test. On the formatter-final material tree, the final aggregate passed 5,844
  offline tests and both disposable live-service acceptance oracles; the
  GUI-capable serial production-browser owner also passed.

- Audited the restoration test changes under the repository's permanent-test
  admission rules. Removed unused Ribbon fixture inventory and a fixed seeded
  Account UUID assertion; retained current authorization, route/Ribbon, and
  answer-free recovery boundaries. Disposable browser acceptance and screenshot
  capture remain one-time evidence, not permanent fast-suite behavior tests.
  `source source_me.sh && ./launchers/all_test.sh` passed after the audit.

- After audit remediation, `source source_me.sh && ./launchers/all_test.sh`
  and `./devel/run_playwright_tests.sh --build` completed their current gates.
  The serial browser owner rebuilt its fixed stack, completed its registered
  scenarios and visible milestone journeys, and its owned stack was stopped
  after the receipt check.
