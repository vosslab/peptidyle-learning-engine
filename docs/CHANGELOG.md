# Changelog

## 2026-09-05

### Additions and New Features

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

### Removals and Deprecations

- Archived the completed Ribbon Application Shell plan at
  `docs/archive/ribbon_application_shell.md`. The task-owned index and worktree now agree that the
  archive path is present and the superseded active-plan path is absent. The three superseded velvet
  plans remain only in `docs/archive/`; the legacy course and
  assignment-workspace navigation retirement recorded by M11 remains the current interface
  ownership.

### Decisions and Failures

- Current Ribbon Availability remains truthfully all unbacked and therefore unavailable: no backend,
  Server Route, Service, or Browser Surface is claimed by this close-out. `./run_playwright_tests.sh`
  remains unclaimed because it requires documented human-owned `PLE_*` real-stack inputs. Focused
  Chromium fixtures prove only their stated current shell and content-boundary invariants; they are
  not live-stack acceptance.
- The final `./all_test.sh` aggregate passed on the coherent task-owned tree. This does not change
  the separate human-owned production-browser input requirement.

### Developer Tests and Notes

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
