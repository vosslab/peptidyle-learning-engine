# Whole-interface theme progress

This report records completion evidence for the original scope and acceptance criteria in
[`whole_interface_theme_plan.md`](../../archive/whole_interface_theme_plan.md).

## Baseline and comparison

- The pre-change Light comparison is [before.png](../../theme_comparison/before.png). It combines
  the 15 existing Instructor Course workspace captures at `384 x 240` pixels, in the stable
  registry order: tundra, forest, desert, grass, arctic, ocean, tropical, coral-reef, swamp,
  underground, salt-marsh, wetland, sea-floor, magma, and beach.
- The source captures are Light-only. There is no Dark before image because the current screenshot
  runtime has no Dark display mode.
- The canonical after comparisons are [after_light.png](../../theme_comparison/after_light.png) and
  [after_dark.png](../../theme_comparison/after_dark.png). Each is a labeled `1240 x 1472` contact
  sheet made from the final published 15-theme Course-workspace captures in the same registry order
  as the before image.
- Before implementation began, the baseline fast gate exited `2` during the frontend typecheck.
  Rust workspace checks and tests, including the browser WebAssembly target check, passed. The
  existing TypeScript error is
  `tests/playwright/screenshot_corpus/filenames.ts(6,8): TS2724`: `RibbonTabId` is imported from
  `src/ribbon/ribbon_catalog`, which exports `RibbonTaskId` instead. See
  `/private/tmp/ple-theme-baseline.log`.
- The supplemental Node baseline passed all 479 tests with zero failures. See
  `/private/tmp/ple-theme-node-baseline.log`.
- The supplemental pytest baseline collected 9,620 tests: 9,611 passed and 9 failed. Eight
  existing Markdown-link audit files failed:
  `docs/active_plans/audits/UI_UX_USABILITY_AUDIT.md`,
  `docs/active_plans/audits/record_list_caller_investigation_2026-09-25.md`,
  `docs/active_plans/audits/record_list_page_frame_standardization_audit_2026-09-25.md`,
  `docs/active_plans/audits/ribbon_route_scope_audit_2026-09-23.md`,
  `docs/active_plans/audits/student_screenshot_breakdown_appendix_2026-09-21.md`,
  `docs/active_plans/audits/student_ui_stability_and_density_audit_2026-09-21.md`,
  `docs/archive/audits/record_list_candidate_inventory_audit_2026-09-23.md`, and
  `docs/archive/reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md`.
  `tests/test_podman_disk_budget.py::test_podman_disk_usage_stays_under_20_gb` also failed because
  the Podman socket at `127.0.0.1:54395` refused the connection. See
  `/private/tmp/ple-theme-pytest-baseline.log`.
- No source implementation edits had begun when these baseline results were collected.

## Milestone ledger

| Milestone                     | Required result                                                                                                                                                                                         | Acceptance evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Status    |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| M1 shared theme model         | Rename the Course-only contract; define five Light and five Dark palette colors per Theme; derive all remaining tokens through one shared rule; implement the Theme ownership and Display mode rules.   | `Theme` spans schema, Rust, generated TypeScript, and the browser registry. The stable all-30 contrast contract passes.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Complete. |
| M2 document-level application | One owner writes the active look and `color-scheme` to `<html>`; remove inner Theme scope, Course section theming, and rail; retain Course Appearance preview through the owner.                        | `AppearanceOwner` is the sole HTML controller, and preview state is supplied to it. The browser probe confirmed that the same Ribbon stays mounted across Product-to-Course navigation and that an unsaved Course preview restores the saved state.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Complete. |
| M3 surface migration          | Paint Canvas, Surface, Secondary, Accent, and Highlight across page surfaces; replace light-only literals; define warning and Dark-safe status and User Role colors; check Question content in Dark. | Shared surface migration and Dark-safe semantic colors are implemented. The accepted corpus includes full Course workspaces in every Light/Dark look and a Dark native Question detail capture. The WeBWorK bridge probe recorded the expected Dark Magma background (`rgb(40, 21, 20)`), white foreground, and no page errors. The matching native author-content receiver was reviewed with the bridge change.                                                                                                                                                                                                                                                                                                                                                           | Complete. |
| M4 palettes                   | Pilot Forest, Arctic, Magma, and Desert in both modes on the Course workspace and Assessment Question Editor; settle the shared derivation; author the other 11 looks; add all-30 contrast coverage.    | The shared derivation and all 15 Light/Dark source palettes are current; the all-30 contrast contract passes. Independent review accepted all 16 workspace/editor pilot captures and the final 30-look Course-workspace corpus.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Complete. |
| M5 user preferences           | Store nullable Light/Dark preference for every Account and personal Theme for each Instructor; provide the Ribbon toggle and Profile controls; test persistence and page-load appearance.               | Schema and self-only API paths, nullable preference, personal Theme, Ribbon control, and Profile controls are implemented. The personal Theme update was repaired at the RLS boundary; the embedded appearance bridge now sends a validated prefixed JSON string. The browser probe confirmed live browser following, explicit Light/Dark persistence, clearing back to browser following, account isolation, personal/Course Theme independence, preview restoration, and saved-state restoration. The final 90-sample load trace has zero wrong mounted looks, the Dark WeBWorK bridge probe passed, and the connected Account oracle passed against a disposable PostgreSQL schema. A readiness latch prevents a later route load from remounting initial shell chrome. | Complete. |
| M6 screenshots and docs       | Capture both modes for every Course theme, a personal theme on filtered Question Library, and this before/after comparison; update authority documents and changelog entries.                           | The final published corpus contains the 30 Course workspace looks and three personal-theme captures. The dedicated Instructor WeBWorK replay captured all eight checkpoints without browser errors after the bridge repair. The final Light and Dark contact sheets use those published captures. Canonical `--verify` passed manifest closure, privacy, and published-artifact integrity.                                                                                                                                                                                                                                                                                                                                                                                 | Complete. |

## Current source checks

- The schema source generator and style check pass. The narrow role-Foreign-Key exception remains
  source behavior without a dedicated implementation-fixture test.
- The all-30 contrast contract, nullable display-mode request unit, and existing screenshot corpus
  definition checks provide the retained permanent coverage. The temporary account-scope mock and
  schema-exception fixture files were removed. The M4 palette-pilot runner and its staging-only
  scenario branch were also removed after the accepted captures were retained as one-time evidence.
  Browser-launcher and Live Demo gateway test files returned to their prior coverage rather than
  retaining Theme-specific infrastructure assertions.
- Appearance persistence assertions now live in the existing connected Account oracle's single seed,
  rather than a second seed or a separate baseline-owner invocation. This avoids duplicating
  disposable-account fixtures while retaining the behavior-level persistence contract.
- An earlier fast-gate receipt predates the final test-surface reduction and runtime repairs. Its
  test totals are historical only. The recorded `all_test.sh`, browser, and connected acceptance
  receipts also predate the later User Role wire-name alignment and audit corrections. They remain
  dated Theme-completion evidence, not validation of the combined current tree.
- The post-correction focused closeout restored generated prerequisites without changing lockfiles.
  `./check_codebase.sh` passed both TypeScript projects, ESLint, Prettier, and `483` Node tests;
  the focused Live Demo auth test passed `5`, the existing Ribbon route-scope command passed `8`,
  and the existing Course Appearance accessibility evidence command passed. These results close
  the User Role contract's local verification gap. They do not repeat or replace the historical
  full-suite, connected PostgreSQL/MinIO, browser-probe, or screenshot-corpus receipts.
- The final `./launchers/all_test.sh` receipt passed schema, Rust, WebAssembly, TypeScript, ESLint,
  Prettier, and Node (`484/484`). Pytest reported `9,645` passed, eight failed, and three existing
  line warnings. The launcher exited `1` only for the same eight baseline Markdown-link audits
  listed above; it found no new failure. The three separate acceptance lanes passed: the connected
  Account persistence oracle, ordinary installation-data provision and replay including the Live
  Demo Course seed, and the Course Appearance PostgreSQL/MinIO coherence oracle.
- Fresh corpus setup exposed one stale `course.theme_id` projection in the Assessment Attempt context
  query, which returned a `503` after an Attempt `POST` succeeded. The query now selects the renamed
  Course Theme field; the rebuilt/reseeded canonical run completed the connected Attempt path.

Focused checks and source review do not establish browser, live-catalog, screenshot-publication,
or complete-suite acceptance.

## Current browser evidence

- Independent visual review accepted the M4 pilot after directly inspecting `before.png` and all 16
  staged captures at `test-results/screenshot-corpus/staging/instructor/theme-pilot-{forest,arctic,magma,desert}-{light,dark}-{workspace,editor}.png`.
  The accepted scope covers the shared surface derivation for those four themes.
- Independent visual review accepted all 30 final Course-workspace captures in Light and Dark,
  preserved at `test-results/whole-interface-theme-final-33/`, together with the three personal
  Theme captures. The review found visibly distinct, biome-coherent major surfaces and readable
  Dark Question content; it does not replace the remaining browser-behavior or corpus-verification
  receipts.
- `test-results/appearance_initial_load_trace_final.json` records 90 animation-frame samples for a
  saved Magma/Light preference after the readiness-latch correction. Of its 89 mounted samples, all
  have Magma/Light; `wrongMounted` is zero. The one startup Grass/Dark sample occurs before mounted
  application content and is static document state, not an application flash.
- The supervised cold-runtime startup completed after its 300-second quiet migration condition was
  corrected to observe the known-live child heartbeat and incremental output. The retained 19
  existing controller checks and independent review pass; no Theme-specific launcher tests were added.
- The dedicated `instructor_webwork` screenshot replay completed all eight checkpoints without
  browser errors after the bridge switched from an object message to a validated prefixed JSON
  string. This is a regression check for the reported `e.data.startsWith is not a function` failure,
  not a substitute for canonical corpus verification.
- `test-results/dark_webwork_bridge_probe.json` confirms the embedded WeBWorK Dark Magma computed
  colors (`rgb(40, 21, 20)` background and white foreground) with no page errors. The matching
  native author-content receiver was reviewed with the bridge change.
- The canonical `./devel/capture_screenshots.sh --verify` replay passed in 265.4 seconds. It checked
  246 captures, including all 30 Course-theme looks and three personal-theme views, for manifest
  closure, privacy, and published-artifact integrity. Its 243 screenshot byte differences are
  informational; the corpus contract does not require replayed PNG bytes to match.
- `test-results/appearance_runtime_probe.json` records a passing browser probe: browser preference
  changes apply live while the preference is unset; explicit Light and Dark persist and override
  browser changes; clearing restores browser following; Elena and Priya remain independent; the
  Ribbon retains its DOM identity across Product-to-Course navigation; and a personal Magma Theme,
  Course Grass Theme, and Forest preview restore their saved states correctly. The final load trace
  and Dark embedded WeBWorK bridge evidence are recorded above.

## Final acceptance ledger

The archived plan's requirements are established by the recorded browser, corpus, source, and
three separate acceptance-lane receipts:

- All 15 themes are recognizable whole-interface looks in Light and Dark against the before image.
- Theme ownership and Display mode rules hold in the browser.
- The all-30-look contrast test passes.
- `all_test.sh` has no failure beyond the recorded baseline. It exits nonzero because it still
  reports the eight pre-existing Markdown-link audits, while the three required acceptance lanes
  each exit zero independently.
- The documentation close-out requirements in the active plan match the implemented model.

## Artifact check

- `before.png` is a `1240 x 1472` RGB PNG. Visual inspection confirms one labeled thumbnail for
  every stable-registry theme and makes the current near-white, thin-accent baseline comparable
  with the published [after_light.png](../../theme_comparison/after_light.png) and
  [after_dark.png](../../theme_comparison/after_dark.png) whole-interface contact sheets. All three
  sheets use the same labeled 15-theme registry order.
