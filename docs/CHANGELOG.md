# Changelog

## 2026-09-07

### Additions and New Features

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

## 2026-09-06

### Additions and New Features

- Completed Live Demo restoration M18. The current direct Instructor can
  download a no-store attachment of pending, unexpired Student Course
  Invitations in existing mailer JSON. The browser downloads only; the mailer
  dry run makes no send or delivery claim. Fresh route and dry-run gates passed.
  The fixed demo has one Instructor persona, so foreign-Instructor enforcement
  is procedure/catalog evidence rather than browser evidence.

- Advanced Live Demo restoration M12 with a strict decoder/cardinality
  correction and seven asset-free native render-only controls before its later
  managed Question Asset delivery completion. No Student Response, submission,
  grading, or feedback was claimed at this stage.

- Completed Live Demo restoration M11. Student Assignment Access and initial
  issuance accept only public Course and Assignment references, authorize the
  signed-in Student's exact active Student Record, calculate access from the
  released snapshot, and refuse due/late-rejected starts before issue. The
  server atomically issues or resumes a full answer-free QuestionPresentation
  pinned to the exact Question Revision, with a 53-bit OS-random public
  QuestionSeed, nonce, title, prompt, and response format. Its private
  one-to-one immutable QuestionPresentation binding retains only nonce and full
  descriptor checksum; source/S3 resolution and reproduction details remain
  private Question Attempt/source-binding facts, and resume reproduces the same
  public presentation. The final fresh fixed HTTPS stack passed
  `bash tests/e2e/e2e_live_demo_assignment_attempt.sh`. Response controls,
  response persistence, submission, grading, feedback, Student View Scenario
  evaluation, and M19 serial-browser acceptance remain unclaimed.

- Completed Live Demo restoration M10. The direct-Instructor Assignment
  Workspace now creates and saves Course-owned Unreleased Assignments with an
  exact Assignment Edit Number, selects bounded Available Published Questions,
  calculates release validation, provides an answer-free Assignment Preview,
  and atomically creates an immutable Assignment Revision. The Preview has no
  Student identity, Student work, response, answer, feedback, or delivery
  state, and is not the retained Student View Scenario contract. The final
  fresh fixed HTTPS stack passed
  `bash tests/e2e/e2e_live_demo_assignment_release.sh`, including its service
  oracle and real Chromium journey. Student delivery, Student View Scenario
  evaluation, grading, and M19 serial-browser acceptance remain unclaimed.

- Added the tracked Live Demo restoration blueprint, now retained at
  [docs/archive/restore_live_demo.md](archive/restore_live_demo.md).
  It defines the authority order, full three-persona completion contract, 22 milestone ladder,
  package ownership, security constraints, and production-browser evidence required to replace
  the former developer structural preview with the actual Live Demo.
- Completed Live Demo restoration M0 with
  [docs/active_plans/audits/live_demo_foundation_findings.md](active_plans/audits/live_demo_foundation_findings.md).
  The report records the fresh forced-RLS/default-deny baseline, classifies every retained
  Playwright specification, assigns client and decoder handoffs, and identifies the structural preview as
  a destination M19 must retire rather than a restored product path.
- Completed Live Demo restoration M1. The fixed disposable topology now starts exactly one
  internal `worker` Service Identity with a dedicated `ple_worker_login`, no HTTP listener,
  gateway route, host port, renderer/object-store credential, or direct protected-table access.
  The browser owner may stop and replace only that labelled worker through sealed,
  capability-validated actions; the real worker topology and lifecycle runner passed. This is a
  runtime foundation only: Job draining, leases, and protected claim/commit behavior remain M3.
- Completed Live Demo restoration M2. `GET /health` now performs bounded live checks of the API
  database pool and, in the disposable topology, the declared object-store bucket, private
  renderer, and worker socket. It returns only a closed unavailable category; the gateway maps a
  missing API process to that same safe 503 form. The sealed readiness runner proved every
  dependency's stopped, bounded-503, recovered, and healthy state. This does not add Job
  execution, a worker lease, or browser-task completion.
- Completed Live Demo restoration M3. The connected fresh PostgreSQL 17 runtime now has one
  milestone gate for the immutable migration baseline, forced RLS/default deny, and the
  `ple_imathas_question_backend_grading_worker` claim-and-commit boundary. It proves a worker
  login cannot read protected tables or assume another role, while stale, foreign, and duplicate
  lease paths refuse and one receipt persists. The running worker still has no generic Job
  dispatcher or Student delivery path.
- Completed Live Demo restoration M4. The fixed disposable baseline now installs five exact
  ordinary Accounts and four private-source Published Questions with first immutable Question
  Revisions, source bindings, publication events, and Object Records. Its sealed inventory
  receipt exposes aggregate counts only, and the real-stack replay gate proves a second lifecycle
  start preserves that baseline without duplicate Accounts or Published Questions. This is a
  foundation for later Question Library and Student delivery work, not browser acceptance.
- Completed Live Demo restoration M5. The fixed stack now resolves the Instructor-only Question
  Library through PostgreSQL session authorization, typed immutable private-source bindings, and
  server-side PLE compilation. Search, stable Question ID resolution, and answer-free Question
  Details serialize no source, checksum, response, answer, or feedback fields; Student and
  anonymous requests receive the same concealment. The real browser runner enters the enabled
  Ribbon destination, searches the seeded baseline, and opens a Question Details route. This does
  not claim authoring, Course, Student delivery, grading, or Sysadmin workflows.
- Completed Live Demo restoration M6. An Instructor can now enter My Question Drafts, create and
  save a private canonical PLE Question JSON Draft Question through an opaque Draft Question
  Reference and exact Edit Number, review publication changes, and publish an immutable first
  Question Revision. PostgreSQL procedures keep Authoring Workspace UUIDs, Draft Question UUIDs,
  private source Object Records, addresses, and checksums server-only; published DTOs carry no
  draft identity or source path. The real Chromium gate returns through Question Library and opens
  the newly published Question. This does not claim Blueprint Course, Course, Assignment, Student
  delivery, grading, or Sysadmin workflows.
- Completed Live Demo restoration M7. An Instructor can create and publish a reusable,
  answer-free Blueprint Course whose immutable Blueprint Revision holds exact Question Revision
  References. PostgreSQL procedures preserve immutable successor revisions; the Blueprint Course
  Owner controls lifecycle writes, and another Active Instructor receives closed Blueprint Course
  Read Access. The disposable service and real Chromium commands both passed against the fixed
  HTTPS stack on 2026-09-06. This does not claim Course Instance creation, roster, Assignment
  delivery, Student work, grading, Sysadmin workflow, or M19 serial browser acceptance.
- Completed Live Demo restoration M8. An Active Instructor can create a Course Instance from one
  exact Available published Blueprint Revision and Course Term through the Courses Ribbon, then
  enter its initial Teaching Team. The atomic PostgreSQL boundary retains immutable Course Origin,
  Course Schedule Revision 1, initial Assigned Instructor Course Membership/event, and creation
  audit evidence; a Sysadmin creator has no ambient Course access. The disposable authority and
  real Chromium commands passed against the fixed HTTPS stack on 2026-09-06. This does not claim
  roster, Student Records, invitations, Assignment delivery, Student work, grading, or M19 serial
  browser acceptance.
- Completed Live Demo restoration M9. A direct current Instructor can import a bounded reviewed
  Course Roster: PostgreSQL resolves or creates each Student Account by immutable Student
  Authentication Email and records a pending Course Invitation with course-scoped roster metadata.
  The authenticated target claim creates the exact Student Record and active Student Course
  Membership; revocation immediately ends access without deleting protected educational records.
  The disposable authority and real Chromium commands passed against the fixed HTTPS stack on
  2026-09-06. This does not claim email delivery, export, Assignment delivery, Student work,
  grading, or M19 serial browser acceptance.

### Fixes and Maintenance

- Moved the retained Playwright browser-test wrapper from the repository root to
  [`devel/run_playwright_tests.sh`](../devel/run_playwright_tests.sh), matching its developer-only
  role and private real-stack input boundary. Its build check, argument forwarding, Playwright
  invocation, Git-root resolution, and exit behavior remain unchanged. Current operational
  references use the new path; historical changelog entries, completed plans, and recorded command
  evidence retain the path that was current when written.

## 2026-09-05

### Additions and New Features

- Restored one developer screenshot command at `devel/capture_screenshots.sh`. It delegates the
  fresh fixed Live Demo and Playwright installation to their existing launchers, enters through the
  visible Elena Instructor Account choice, and regenerates the then-current desktop, selected-state,
  invitation-email, tablet, and phone images. The capture is
  one-time visual evidence rather than a permanent test or a revival of the retired teaching-workflow
  corpus.
- Added a temporary deployment-gated `/live-demo/ribbon` developer structural preview as the seeded
  sign-in destination. It rendered the real production `AppRibbon` from an
  explicitly labelled populated Instructor structural model; activating a
  control changes fixture selection without navigating to or admitting an
  unbacked teaching route. The later restoration removed that retired preview;
  the same page gave the operator the existing dry-run and bounded attended
  `launchers/send_invitations.py` commands while
  keeping Mail.app delivery outside the browser.
- Added the temporary attended macOS invitation mailer. It reads one private JSON export,
  defaults to dry run, sends through visible Mail.app composition, throttles the batch, and keeps
  atomic owner-private current status so reruns suppress confirmed or indeterminate recipients.
  A deliberate resend requires `--send`, `--only`, and `--force-resend` together.
- Completed Ribbon Application Shell M12: the generated 24-destination
  `docs/ux/RIBBON_DESTINATION_LEDGER.md` now derives canonical label, route identity, client method,
  backing evidence, and per-Product-Role Ribbon Availability from the executable catalog and
  capability registry. Its exact-one generated-section markers are fail-closed and covered by a
  stale-document, ordering, editorial-preservation, invalid-argument, and openable-evidence test.
  The generator emits a Prettier-stable machine section without changing the editorial prose; a
  permanent formatting regression proves that boundary.
  The companion `docs/ux/RIBBON_TASK_MODEL.md` records per-role teaching tasks and a
  heuristic/accessibility evidence ledger; `docs/ux/FRONTEND_CAPABILITY_INTEGRATION.md` gives future
  complete-path work its ordered integration contract.

### Behavior or Interface Changes

- The Ribbon now implements the precision-field-console visual philosophy in production CSS. Its
  Context, Tab, and Task Rows occupy distinct neutral planes, so truthfully empty rows remain
  deliberate structure; compact identity dividers, stronger selected keys, and local pending-state
  motion sharpen hierarchy without changing fixed geometry or adding decorative accent placements.
  Narrow-phone task labels are more readable while retaining the established reachable-control
  profile.
- PLE's general UI language now carries the Application Shell's restrained-surface, proximity-first,
  deliberate-density, stable-spatial-memory, point-of-interaction-feedback, discrete-responsive, and
  geometry-native accessibility principles beyond the Ribbon. The durable ownership decision gives
  every authenticated Product Role one shell-owned Ribbon while route pages retain headings, local
  content, and Page Actions.

### Fixes and Maintenance

- Moved the Live Demo front door from the repository root to
  [`launchers/run_live_demo.sh`](../launchers/run_live_demo.sh). The launcher now resolves the
  checkout through Git before sourcing `source_me.sh`, while its start, open, stop, default-headless,
  TypeScript-setup, and fixed-owner behavior remains unchanged. Current commands and the screenshot
  launcher now use the new path; historical changelogs and archived plans retain the path that was
  current when they were written.
- Repaired the disposable Live Demo's first migration by creating its exact temporary
  `ple_migrator` principal and default-deny bootstrap grants before Cargo applies the immutable
  principal-baseline migration. The lifecycle now verifies that migrated schema through the actual
  least-privilege API login after it is created, including its required database `CONNECT` grant.
  Before declaring the browser stack ready, it initializes the fixed disposable demo Accounts and
  proves the same-origin selector can mint an ordinary Authenticated Session; a healthy process
  without usable demo entry can no longer be reported as a ready Live Demo.
  Its generated HTTPS target now supplies that exact browser origin to the production API, and
  both the API and gateway health checks preserve the canonical public `Host` authority instead
  of sending their internal listener addresses.
  [Brewfile](../Brewfile) now declares the system-wide Python prerequisite. `launchers/run_live_demo.sh`
  always delegates TypeScript dependency setup to its existing helper without managing Python.
- The Live Demo launcher now prints its ready HTTPS URL by default so an operator can choose the
  browser that opens it. `open` reads and opens the authenticated URL of an already-running suite,
  while `start --open` creates and opens a fresh one. `--open` remains the compatible `open`
  shorthand; `--headless` remains an explicit spelling of the default behavior.
- Stopping an in-progress Live Demo launch now waits for the fixed lease-owning supervisor to publish
  its authenticated stop endpoint, rather than misreporting the protected startup lease as an
  irrecoverable "already running" failure.
- Moved the four-command aggregate wrapper from the repository root to
  [`launchers/all_test.sh`](../launchers/all_test.sh). Its Rust, frontend, Python, and connected
  acceptance calls remain unchanged and in the same fail-fast order.
- Moved the temporary invitation-mailer front door from `tools/` to
  [`launchers/send_invitations.py`](../launchers/send_invitations.py). The launcher now reads
  as an entry point for the importable [`invitation_mailer/`](../invitation_mailer/) package
  instead of as a standalone repository utility. `source_me.sh` now applies the repository's
  canonical Git-root `PYTHONPATH` extension so subdirectory launchers can import root packages.
- Refreshed the README; architecture and file map; installation and usage; FAQ and input formats;
  roadmap and TODO; development, troubleshooting, and cookbook; and related-projects documentation
  from current contracts and executable boundaries. The seeded Live Demo still proves only
  server-owned ordinary-session entry; retained teaching and browser workflows remain unavailable.
- Corrected generated Ribbon destination-ledger evidence links so the anchor text names the linked
  repository path and the exact `::symbol` remains adjacent evidence outside the anchor. The
  generator contract now prevents path-like labels from drifting away from their GitHub targets;
  the regenerated ledger passes all 207 Markdown-link checks.
- Corrected the generated evidence for Teaching Operations to its current page component and hardened
  ledger generation so missing, duplicate, or reordered machine-section markers fail rather than
  silently rewriting documentation. The generator formatting repair preserves editorial prose while
  making the machine section stable under Prettier.
- Six independent close-out audit passes corrected the authenticated malformed-scope boundary: a
  matched, signed-in scoped URL now retains its declared, data-free Ribbon schema while malformed,
  public, and signed-out states continue to withhold the shell. The repair preserves the distinction
  between URL syntax, display structure, and authorization.
- Moved build-, CLI-, and browser-condition checks out of the fast Node lane into
  `tests/e2e/` and `e2e_run_all`; removed brittle one-time implementation inventories while retaining
  durable behavioral contracts. The canonical Git-root ledger-generator helper now owns its path
  resolution, and the current documentation and comments describe the same test and acceptance
  boundary.
- Rotated the complete 2026-09-03 changelog day block to `docs/CHANGELOG-2026-09d.md`, retaining
  exactly the two newest date blocks in this active changelog.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Removals and Deprecations

- Archived the completed invitation-mailer plan at
  `docs/archive/student_activation_mailer.md`. The mailer remains intentionally small and includes
  disposal instructions in [USAGE.md](USAGE.md) for removing it after the trial.
- Archived the completed Ribbon Application Shell plan at
  `docs/archive/ribbon_application_shell.md`. The task-owned index and worktree now agree that the
  archive path is present and the superseded active-plan path is absent. The three superseded velvet
  plans remain only in `docs/archive/`; the legacy course and
  assignment-workspace navigation retirement recorded by M11 remains the current interface
  ownership.

### Decisions and Failures

- The mailer accepts signup URLs created elsewhere and claims only its local dispatch observation;
  it does not create an Account, Course Invitation, Course Enrollment, or signup-completion fact.
  Real Mail.app delivery remains an attended operator check. A one-time native script-compilation
  probe in the automated environment reached an unavailable macOS High-level Services connection
  and aborted before executing the script; no message was composed or sent.
- Screenshot capture was skipped because Playwright and the canonical production-browser owner were
  unavailable. Existing historical README embeds remain preserved as design reference, not current
  browser, accessibility, visual, privacy, or teaching-workflow acceptance. No duplicate NEWS or
  release-history block was added because `v26.09` already has one.
- Current Ribbon Availability remains truthfully all unbacked and therefore unavailable: no backend,
  Server Route, Service, or Browser Surface is claimed by this close-out. `./run_playwright_tests.sh`
  remains unclaimed because it requires documented human-owned `PLE_*` real-stack inputs. Focused
  Chromium fixtures prove only their stated current shell and content-boundary invariants; they are
  not live-stack acceptance.
- The final `./all_test.sh` aggregate passed on the coherent task-owned tree. This does not change
  the separate human-owned production-browser input requirement.

### Developer Tests and Notes

- Permanent offline invitation-mailer tests cover config and export boundaries, domain and URL
  validation, duplicate suppression, private atomic status, template safety, per-recipient failure,
  interruption recovery, dry-run sender isolation, and the narrow targeted-resend argument guard.
  The disposable E2E executes the real launcher in default dry-run mode with a batch limit, then
  uses a fake sender to prove initial send, no-op rerun, incremental send, and deliberate resend
  without importing Mail.app. The focused tests, registered E2E runner, Python hygiene,
  support-directory boundary, Bandit scan, shell syntax, and Markdown-link checks passed.
  A fresh six-pass Plan, Test, Style, Documentation, Legacy, and Comment audit moved the operator
  contract into [USAGE.md](USAGE.md), aligned root discovery with the Git-root rule, removed
  redundant whole-workflow pytest coverage, and corrected stale claims in the completed plan.
  The fresh full aggregate is not green: its Rust lane passed, then the frontend lane stopped
  because `node_modules` is absent. The full Python lane was run independently and all 5,296
  tests passed. This mailer adds no root script.
- A fresh six-pass audit of the precision-field-console refinement found no plan or documentation
  drift. Its concrete test, style, legacy, and comment findings were repaired and independently
  re-reviewed: the all-theme browser oracle now locks three distinct row planes and two-part focus
  in standard and forced-colors presentations; the narrow profile uses explicit cascade order; and
  the CSS comments name their actual layout and grouping responsibilities.
- Six fresh Plan, Test, Style, Documentation, Legacy, and Comment audit passes informed the close-out
  repairs; independent targeted technical, UX, and generator-security re-reviews accepted them.
  Fresh focused temporary Chromium fixture captures and visual inspection passed their stated shell
  and content-boundary checks; they do not establish a current production screenshot corpus or
  live-stack acceptance. Committed `docs/screenshots/` images are historical reference only. Fresh
  screenshot publication and visual acceptance await the restored human-input production-browser owner.
  The exact-one ledger generator check passed. The fast Node gate now passes with 360 tests, and
  `bash tests/e2e/e2e_run_all.sh` passes all 15 non-browser E2E checks; end-to-end build, CLI, and
  browser-condition checks are intentionally outside the fast Node lane. `git diff --check` passed.
  The aggregate `./all_test.sh` passed its Rust, fast frontend, full Python, and two real-service
  acceptance lanes on the coherent final tree.

## 2026-09-04

### Fixes and Maintenance

- Adopted the authority-aligned Account Creation Security Hardening active plan and removed its
  ignored superseded root drafts. `TERMINOLOGY_CONTRACT.md` and `USER_ROLES.md` are read-and-follow,
  read-only authority documents for this work.
- Completed M1 of Account Creation Security Hardening: Human Guidance now records the approved
  robustness rule, and Failure Recovery defines salvage, clean retry, irrecoverable-item handling,
  affected-boundary refusal, and data-retention limits. The committed, rejected, retryable, and
  indeterminate outcome model remains unchanged.
- Completed M2 of Account Creation Security Hardening: Create Instructor Account now writes
  immutable, role-qualified Active Sysadmin actor evidence in the same transaction as the Account,
  Authentication Email, and initial Account State. Forced RLS, revoked runtime table access, a
  narrow writer, and update/delete refusal protect the event; connected PostgreSQL acceptance and
  independent security review passed.
- Deferred M3 of Account Creation Security Hardening after two clean disposable `webauthn-rs`
  start/finish attempts reached the same persistent Store-contract blocker: PLE cannot durably
  create, retrieve, or atomically consume discoverable-ceremony and validated credential state.
  No passkey route, Browser Surface, setup credential, installation command, session issuance, or
  completion claim was retained. Seeded demo entry, health, ordinary session handling, and logout
  remain independent of this deferred capability.
- Completed M4 of Account Creation Security Hardening for the deferred-passkey outcome. Seeded
  entry now retains each unambiguous configured persona, reports a bounded unavailable count for
  omitted records, and isolates zero-valid-persona configuration to seeded entry while health,
  ordinary sessions, and logout remain available. The Browser Surface retains valid choices and the
  focused retained-persona journey proves ordinary session resolution. The browser registry now
  selects the baseline seeded-entry/session/logout/course-boundary scenario; this is not a claim of
  complete real-stack browser acceptance, which remains M5 evidence.
- Removed dormant passkey configuration requirements and virtual-WebAuthn-only test scaffolding
  after M3 deferral. Current local-stack, Compose, and deployment configuration no longer requests
  WebAuthn secrets, while durable checks preserve the absence of that unavailable capability.
- Post-audit remediation removed unused deferred-WebAuthn Cargo dependencies and dormant passkey
  CSS, pruned brittle source-snapshot tests while retaining generated-environment and fail-closed
  registry coverage, and corrected current Live Demo documentation and comment wording. Passkeys
  remain deferred; this does not claim passkey implementation or acceptance.
- Removed permanent-document dependencies on archived implementation, release, status, and
  wire-naming plans. Durable architecture, contract, roadmap, database, evidence, TODO, and
  changelog documents now own those references; dated reports and audits link to `docs/archive/`.
  Refreshed `CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, and the agent orientation to describe the
  durable/working/archive boundary. The focused Markdown-link gate passes all 196 documents.
- Completed the documentation-set refresh: rewrote the README and Cookbook around the executable
  seeded-session boundary; aligned Install, Usage, FAQ, input-format, roadmap, TODO, development,
  and troubleshooting guidance with the absence of current teaching routes; refreshed release,
  news, and related-project records; and retained a concise `AGENTS.md` that points only to durable
  authorities. Rotated the older 2026-09-02 changelog block to
  [CHANGELOG-2026-09c.md](CHANGELOG-2026-09c.md). Historical screenshots remain managed design
  reference because no live Compose project was running and the local Podman machine reported a
  lockfile-permission warning; no unsupported browser acceptance claim was added.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Developer Tests and Notes

- Adopted the Ribbon Application Shell plan and completed M0: Product Role Solid fixtures,
  production `createApplicationApi` counted-fake-transport proof, one-mount transition,
  deferred-resolution, scroll, and routing helpers, plus forced-colors/reduced-motion context
  options are available for later milestones. Focused gates and full `./check_codebase.sh` pass
  with 298 Node tests. The browser harness is construction and transport evidence only, not
  authorization evidence; no Ribbon UI is claimed by this milestone.
- Completed Ribbon Application Shell M1: declared-route parameter zipper, exhaustive `RouteId`
  scope map, and six branded parsers, including Blueprint Course, preserve an explicit invalid
  state and declared scope for malformed scoped references; there is no prefix or Product fallback.
  All 24 routes are covered. Syntax validation is not authorization. Focused 7 and full
  `./check_codebase.sh` (305 Node tests) pass.
- Completed Ribbon Application Shell M2: all 24 routes now own declared scope, tab, task-group,
  and content-layout metadata, including the exact eight-row `fullWidth` translation. The two
  Context Control routes have no selected tab. The canonical 11-item tab tuple derives the type
  and retains the unselected, unbacked Instructor Accounts position without route or authorization
  changes. Focused 11 and full `./check_codebase.sh` (309 Node tests) pass.
- Completed Ribbon Application Shell M3: the exact 11-tab/13-task catalog keeps role, priority,
  and presentation independent; nine immutable role/scope schemas include the Sysadmin Instructor
  Accounts append point; and a total 24-entry registry joins those controls. Every current teaching
  destination is truthfully unbacked because no complete production handler exists; backed proof
  requires runtime validation. Capability, Product Role, then relationship determine availability;
  `Checking` is withheld, and Ribbon visibility is not authorization. Focused 17 and full
  `./check_codebase.sh` (326 Node tests) pass. This is data-only M3, not a UI or M4 claim.
- Consolidated Ribbon scope authority: `routeScopeKey` now derives from
  `RouteContract.ribbon.scope`, removing the duplicate 24-row table. A mutation-and-restore
  regression proves that authority for valid and malformed paths; URL syntax behavior and the
  authorization boundary are unchanged. Focused 9 and full `./check_codebase.sh` (328 Node tests)
  pass.
- Completed Ribbon Application Shell M4: pure synchronous `deriveRibbonModel` keeps typed route,
  Product Role, and display-label inputs separate; fail-closed `buildRoutePath` canonically proves
  every one of 24 routes. The immutable designed topology is UI admission, not authorization:
  current controls remain all `Unavailable` with no `Checking`; deferred HTTP-client proof covers
  all three roles and scopes. Relationship `Checking` and a missing-course-reference Back guard
  retain slots without inventing URLs; compile-only negative cases pass. Full
  `./check_codebase.sh` passes with 341 Node tests. This does not claim a rendered Ribbon UI or a
  shipped capability.
- Completed Ribbon Application Shell M5: two branded public-reference-keyed queries resolve Course
  and Assignment Attempt identities, while the reactive `RouteScopeProvider` owns resolution in an
  effect and projects cached data purely. Exact-kind, malformed, and wrong-kind inputs fail closed;
  C and R key families remain stable and distinct; keyed entries contain late inactive results.
  The compiled `ApplicationApiProvider > RouteScopeProvider > consumer` tree proves reuse without
  a permanent raw request-count contract. This is presentation data, not authorization; production
  mounting remains deferred to M10. Full `./check_codebase.sh` passes with 347 Node tests.
- Completed Ribbon Application Shell M6: model-only `AppRibbon` renders exactly the permanent
  Context, Tabs, and Tasks navigation rows, including an empty Task row. Named per-profile rem
  tokens drive the Ribbon block and shell grid, and the production CSS artifact carries that
  contract before mounting. Fixture geometry covers all model states; Chromium confirms 320px and
  200% text focus reachability without document overflow. Canonical fixture admission rejects a
  missing route build instead of silently downgrading it. The Ribbon remains unmounted; M7 design
  fixture work is next. Full `./check_codebase.sh` passes with 347 Node tests.
- Completed Ribbon Application Shell M7: the static, full-width real-`AppRibbon` design laboratory
  covers nine role/scope schemas, every availability, selection, Task Row, title, and Assignment
  Attempt Progress state, plus all 15 real course themes. Two complete treatments were built;
  Fieldstation is selected and Atlas retained because the former reads as one continuous surface,
  while Atlas's cell-like Tabs and ambiguous selected-tab slash weaken that hierarchy. Production
  carries forward six non-negotiables: one continuous instrument-panel surface; context,
  destination, then task-area pre-reading order; state without geometry change; label-first compact
  working-set visibility; restrained biome accent projection; and two-part focus with explicit
  forced-colors behavior. Corrected phone wrapping/clipping and the full-width lab; Chromium at
  1280px, 320px, and 320px/200% plus full `./check_codebase.sh` (350 Node tests) pass. This selects
  direction only: no application, server, session, router, M8, M9, M9a, M9b, or production mount is
  claimed.
- Completed Ribbon Application Shell M8: Ribbon-owned pending navigation keeps the exact destination
  and receives an injected in-flight signal; only the initiating exact control exposes `aria-busy`, and
  its state clears on settle, redirect, no-start, and disposal. Selected Tabs reveal only when clipped
  with nearest-inline scrolling, using reduced-motion `auto` rather than `smooth`; rapid selection and
  unmount safety are covered. Real Solid and Chromium evidence passes alongside full
  `./check_codebase.sh` with 358 Node tests. This remains unmounted and makes no M9+ claim.
- Completed Ribbon Application Shell M9: true 320px device-viewport handling corrects the former
  inflated layout viewport, and explicit single-line `nowrap` projection holds against the legacy `nav`
  rule. Context, Tabs, and Tasks retain three paint frames with one labelled row and two inert pinned
  direction cues; the Context row blends its author color without becoming a navigation control.
  Selected Tabs have clearance and every Tab remains reachable by horizontal scroll. Coarse controls
  reach 44px; portrait, phone, and 200% profiles preserve their within-profile geometry, while observer
  disposal remains safe. Real M6--M9 Chromium evidence and full `./check_codebase.sh` pass with 359
  Node tests. No icons/M9a+, production mount, or old-navigation retirement is claimed.
- Completed Ribbon Application Shell M9a: a closed model-gated glyph map and deterministic,
  same-origin 16-glyph SVG sprite/build now supply only catalog-declared Font Awesome SVG artwork,
  with exact artwork attribution. Icons pair with visible labels; only explicitly safe conventional
  controls may become icon-only. Selection epochs, resize re-reveal, and disposal preserve the
  selected Tab or Task without unrelated geometry movement; static and compiled 320px/200% evidence
  keeps the selected Task fully visible. The independent visual jury accepted the result. Full
  `./check_codebase.sh` (376 Node tests) and M7/M8/M9 Chromium evidence pass. This remains unmounted;
  M9b production density, M10 application mounting, and old-navigation retirement are not claimed.
- Completed Ribbon Application Shell M9b: the selected Fieldstation treatment now gives the
  unmounted Ribbon one continuous, three-row instrument-panel surface with a single bottom edge;
  Context answers where, Tabs establish the strongest destination rhythm, and grouped Tasks provide
  the lighter work-entry strip. `standard` and `compact` presentation classes, token-derived
  proximity spacing, flat-to-soft control states, fixed selected-Tab geometry, and the exactly three
  semantic course-accent placements (course marker, selected-Tab underline, selected-Task
  background) preserve the planned visual system without role- or priority-driven sizing. The Tundra
  Task-Area separator now uses the derived neutral after the all-theme oracle measured the original
  border at 1.63:1. Static and Chromium oracles cover all 15 themes, forced colors, reduced motion,
  Instructor 1280px direct visibility, selected-control geometry, and the canonical unmutated
  Fieldstation screenshot. Independent visual and technical re-reviews accepted the result. Focused
  static checks, M7/M8/M9/M9b Chromium scripts, the icon-sprite check, `git diff --check`, and full
  `./check_codebase.sh` (377 Node tests) pass. This is still a model/fixture visual system: M10
  application mounting, live teaching workflow, and legacy-navigation retirement are not claimed.
- Completed Ribbon Application Shell M10: the production-owned `ApplicationShell` now composes one
  `RouteScopeProvider`, the theme-variable bridge, a stable `AppRibbon`, and the content shell.
  Its keyed, content-only `ErrorBoundary` preserves navigation while content recovers, and the
  legacy primary navigation no longer mounts. Current production truthfully admits no Tabs or Tasks
  because no destination has a complete handler; an explicitly labelled populated structural fixture
  uses that same production shell to prove visible controls and error recovery without pretending to
  be a live workflow. Deferred Course C1 scope updates reactively; public, malformed, and signed-out
  states withhold the Ribbon and recover correctly. Browser evidence proves accessible Tab and Task
  activation after content error, stable Ribbon DOM identity across route and scope changes, sign-out
  bubbling, and skip-link focus. Focused M10 Chromium evidence, M7/M8/M9/M9b browser regressions,
  sprite and diff checks, and `./check_codebase.sh` (377 Node tests) pass. `run_playwright_tests.sh`
  is not claimed because it requires human-owned `PLE_*` real-stack inputs.
- Completed Ribbon Application Shell M11: a durable responsibility inventory records the replacement
  owner and acceptance check for every retired course-management concern. The superseded
  `CourseManagementFrame`, course and assignment-workspace navigation, old course-theme scope,
  route context, hook, and classifier are retired. `useRouteScopeData` now returns an Accessor and
  all 12 consumers react to scope changes; five eager page families use content-local keyed deferred
  boundaries under one persistent shell, provider, and Ribbon. Page-owned instructor eyebrow, `h1`,
  and New assignment Page Action, student course identity, and theme variables/live preview remain
  intact. The route-surface layout exception and dead sysadmin branch are removed; the literal
  `/workspace` placeholder is retired while canonical `myQuestionDrafts` remains future, unbacked,
  and omitted. The focus-selector repair and regenerated isolated-course capture passed independent
  visual acceptance. `npx tsc --noEmit`, 29 focused checks, M10 Chromium evidence, M11 deferred
  evidence twice, M7--M9b regressions, dead-export and diff scans, and `./check_codebase.sh` (384
  Node tests) pass. `run_playwright_tests.sh` is not claimed because documented human-owned `PLE_*`
  real-stack inputs are required.
- Completed M5 implementation and verification for Account Creation Security Hardening. The final
  aggregate `source source_me.sh && ./all_test.sh` passed Rust, 292 JavaScript checks, 4,930 Python
  tests, connected PostgreSQL acceptance, and PostgreSQL-plus-MinIO Live Demo acceptance. Required
  account-creation hardening is complete; the isolated passkey capability remains plan-authorized
  deferred with no passkey route, setup credential, installation command, or completion claim. The
  plan was archived with the required history-preserving `git mv` to
  `docs/archive/account_creation_security_hardening.md`.
- Declared the existing Graphify XML and tree-sitter development-tool requirements so the complete
  Python suite can exercise its tracked utilities without an undeclared-import failure.
