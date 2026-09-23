# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 18 entries are archived in [CHANGELOG-2026-09n.md](CHANGELOG-2026-09n.md).

## 2026-09-23

### Fixes and Maintenance

- Instructor Tier 2 now follows the fixed role-and-Tier 1 mappings across deeper routes. Course and
  Assessment local workflows use page links and breadcrumbs; Gradebook now has a Course actions
  link. The Assessment row stays Due Soon and Templates because current evidence does not settle a
  general collection task's name and shape; Browse All remains a future opportunity. Student Tier 2
  contents and order remain unresolved, preserving the current Student behavior.
- Removed mechanism-specific font-loading assertions; computed production font-family checks
  remain.
- RecordList now renders optional region headings on the same subgrid tracks as its rows and applies
  the same responsive priority hiding to both. The production Gradebook no longer owns a parallel
  fractional column grid. Its Chromium route check confirms matching header/region order and aligned
  edges across all four canonical viewports; visual review covered both tablet scroll positions.
- Removed the unused shared `.page` geometry and retargeted course-theme positioning to the scoped
  PageFrame root.
- Removed the duplicate reading-width cap from Assessment creation content; its default PageFrame
  owns page width while the content wrapper keeps only its task layout.
- Removed the Assessment Questions consumer override that replaced RecordList's responsive
  identity/action tracks with a separate one-column grid.
- Assessment Student View and workspace loading/error content now inherit their declared PageFrame
  width instead of narrowing only the body inside a full-width route.
- PageFrame-root `.route-error` content retains its compact 48rem cap while aligning to the frame's
  left origin; standalone error notices keep their centered placement.
- The signed-in shell's route `ErrorBoundary` fallback now uses PageFrame's title, lede, and action
  slots instead of a separate page heading and layout.
- Consolidated RecordList browser contracts into one shared Chromium fixture run and combined the
  Ribbon identity/geometry route sweep. Removed implementation-specific window pixel and DOM-node
  assertions while retaining identity/order, focus, reorder, region, and presentation behavior.
- Removed the unreferenced Ribbon design-variant screenshot sweep; its design matrix was
  implementation evidence, while the shared fixture remains in use by the durable all-theme Ribbon
  density check.
- Avatar presentation checks now protect catalog identity and selection across Gallery/List and
  viewports; layout geometry remains visual-review evidence rather than a CSS-measurement gate.
- M2 WP-B1/B2 checks pass. In a disposable copy, route-scoped tier-one behavior made WP-B1 fail
  when Instructor `courseAssessments` lost `courses`; pre-M1 conditional task-row behavior made
  WP-B2 fail with a 40px offset. The old WP-B1 schema used IDs removed by M1, so the isolated
  reproduction used current valid IDs to confirm the route-dependent behavior.
- Rotated older day blocks to [CHANGELOG-2026-09o.md](CHANGELOG-2026-09o.md), keeping the two
  newest dates active.
- The Canonical Blueprint JSON textarea now uses the shared monospace font token.

### Developer Tests and Notes

- Audit follow-up aligned the Ribbon design guides with the fixed role-and-Tier 1 rule and kept
  unresolved Student Tier 2 contents out of the permanent Task Row topology assertion.
- After the audit follow-up, the consolidated Ribbon contract passed (18/18) and the route contract
  passed (4/4).
- The fixed Instructor rows pass the consolidated Ribbon contract tests (18/18) and route-contract
  tests (4/4). The focused Chromium shell evidence and four-viewport fast UI lane pass. Matched
  laptop/phone route captures retain 40px/44px Task Row bounds across all measured transitions.
- `source ./source_me.sh && python3 local_stack.py acceptance` passed the database baseline,
  installation-data replay, and Course Appearance PostgreSQL/MinIO coherence oracles. A fresh
  canonical screenshot publication completed all scenarios and privacy checks; static manifest
  verification and the screenshot publication tests (9/9) passed.
- `source ./source_me.sh && ./launchers/run_fast_checks.sh` and `all_test.sh` each reached the
  Python suite with 9,279 passed and one failure: the local
  `ribbon_route_scope_audit_2026-09-23.md` links to a missing
  `docs/screenshots/instructor/average.png`. Targeted Markdown-link checks for the changed guidance,
  decision, model, and plan documents passed (27/27); the audit was preserved.
- After the test-surface consolidation, `source source_me.sh && ./launchers/run_fast_ui_checks.sh`
  and the focused browser-scenario contract tests passed; `git diff --check` was clean.
- In the later test-review snapshot, the focused UI lane, five browser-scenario pytest tests,
  formatting, and `git diff --check` passed. The aggregate `run_fast_checks.sh` attempt exited 1
  because sandbox permissions blocked Chromium startup for the unrelated
  `course_instance_assessment_refresh.mjs` test; that test passed when run alone with browser
  launch permission. No aggregate fast-check pass is claimed for this later snapshot.
- The Assessment creation CSS change passed its focused Prettier and whitespace checks; the shared
  PageFrame reading-width rule remains the page-level width source.
- `source ./source_me.sh && ./launchers/run_fast_ui_checks.sh` passed the RecordList four-viewport,
  state, presentation, reorder, window, and production-route Chromium checks after removing the
  Assessment Questions track override.
- `source ./source_me.sh && ./launchers/run_fast_ui_checks.sh` passed the shared RecordList and
  production-route Chromium checks. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed
  9,289 Python tests and 456 Node tests plus Rust, TypeScript, lint, and formatting. The default
  sandbox's first attempt failed only when Chromium could not bootstrap; the same command passed
  with browser-launch permission.
- A production-shell Gradebook capture compared the synthetic PageFrame error-content state at
  1280x800: the centered 291px wrapper at x=495 now remains 768px wide at x=32, aligned with the
  PageFrame title. Fresh visual review found no overflow or clipping. The focused Chromium UI lane
  and fast compliance gate passed; this is not live route-failure acceptance.
- After the PageFrame error-content alignment and shell ErrorBoundary changes, the focused Chromium
  UI lane and `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,287 pytest tests
  plus Rust, TypeScript, Node, lint, and format checks).
- The Student Attempt PageFrame migration no longer carries the old empty title spacer or nested
  title style in its timer row. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed
  9,289 pytest tests, 456 Node tests, Rust, TypeScript, lint, and formatting checks.
- Student Course landing and Grades now use the Course long name as the PageFrame title; the
  optional Course banner remains in content and Grades stays a subordinate task heading. Updated
  the Student Grades screenshot-corpus selector. The isolated landing browser check and four-size
  visual review passed; no full-stack Grades capture or full-suite run was repeated.
- The shared Course breadcrumb now uses the long name when the trail fits and the Instructor-defined
  short name when space is constrained. The production-shell Chromium fixture and fresh visual
  review passed at 1280px, 320x640/200% root text, and 393x852/200% root text; the PageFrame heading
  retains the long Course name. Fixtures reuse the production shell and components with synthetic
  route/API/content data, so this is not full-stack acceptance. The 320px/200% Instructor stress
  capture also showed crowded Ribbon controls, outside this plan's Instructor screenshot policy.
- Keep the descriptive Course identity sample local to the production-shell harness; shared route and
  model fixtures retain generic Course labels. The fit-based breadcrumb fallback measures actual
  content width through the shell's existing `ResizeObserver`, avoiding a viewport breakpoint.
- After the Course identity fixture and breadcrumb changes,
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,287 pytest tests plus Rust,
  TypeScript, Node, lint, and formatting checks. The command was rerun with Chromium launch access
  after the sandbox denied the first browser-backed attempt. No `all_test.sh` rerun was made.
- WP-F formatter reuse now passes a focused four-test suite: date helpers accept a prebuilt
  formatter, and component/page scopes reuse it across repeated rows and recovery entries. An
  independent source review approved the change. On 2026-09-23, the focused presentation test,
  `npx tsc --noEmit`, six-file Prettier check, and `git diff --check` passed. The final
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,268 pytest tests plus Rust,
  TypeScript, Node (456 passed), lint, and formatting checks after both final code changes.
- SUI-03's fresh headless review used the production ApplicationShell, AppRibbon,
  BreadcrumbPrelude, PageFrame, and shared styles with synthetic route data and body content. At
  393x852 and 200% root text, Home and full Biochemistry are readable at rest; keyboard focus
  brings each full breadcrumb label into view. There was no document horizontal overflow or page/
  console error. A fresh source reviewer approved the scrollport and image_evaluator passed the
  documented gate. The visible leaf label is partial at rest; this is minor. These artifacts are
  bounded shell evidence, not a full Assessment-page screenshot. The aggregate 9,268-test fast
  check above ran after the final source edits; it does not replace or update earlier full-suite or
  Live Demo replay receipts.
- Current source has no new permanent mutation test. Headless `./launchers/run_fast_ui_checks.sh`
  passed after narrow Chromium sandbox escalation; `source ./source_me.sh &&
./launchers/run_fast_checks.sh` passed 9,267 pytest tests plus Rust, TypeScript, Node, lint, and
  format checks. Screenshot corpus tests passed 9/9, static screenshot verification passed, and
  the 139-capture WP-G1 visual review found no in-scope visual fix. The blank Student WebWork
  preview remains an unconfirmed separate-owner candidate; the crowded Sysadmin account form is
  deferred low-priority cleanup. Screenshots do not establish keyboard or runtime behavior.
- The latest clean semantic replay passed workflow, manifest closure, privacy, dimensions, and
  artifact integrity with 135 byte differences; the warm loop passed in 378 seconds with 112
  observational byte differences. Byte differences are not failure gates; no pixel or byte
  thresholds were added.
- PageFrame evaluates allowed content classes reactively, and the fast UI route check verifies that
  entering and leaving Question Pool review updates the production Library presentation while
  retaining its windowed rows, order, regions, and spacer checks. The headed registry can open
  PageFrame reading and Student Coursework production fixtures optionally, with headless completion
  autonomous. `./launchers/run_fast_ui_checks.sh` exited 0; the Student helper headless smoke had no
  page or console errors, headed selected cases reached ready state, and Node syntax, Prettier, and
  `git diff --check` passed.
- The official pre-change Instructor capture showed the Assessment Student View body ending before
  the full-width PageFrame header. After removing the inner cap, a one-time stack-free production-
  shell probe rendered the real fullWidth PageFrame with the existing
  `assessment-workspace-student-view` content class and measured frame, header, and content widths
  equal at 1216px. This probes the production shell/Frame with feature content, not the full
  Assessment API route or a post-change Live Demo screenshot. The isolated Chromium lane and current
  fast compliance gate passed separately.

## 2026-09-22

### Behavior or Interface Changes

- The Student course short name no longer appears in the top Ribbon row; the full course title
  remains the page heading.
- PageFrame keeps a fixed production root, content origin, and route-owned width mode outside Ribbon
  navigation. Caller-specific styles are confined to the content region; the redundant private
  `.auth-page`/`.roster-page` width is removed, and SignInPage no longer passes the unused
  `auth-page` marker. The shared action slot is limited to recurring page-level collection actions
  admitted by WP-C1.

### Fixes and Maintenance

- Tightened the Student responsive fixes after code review: phone-height Ribbon tokens no longer
  leak onto coarse-pointer tablets, compact breadcrumb behavior covers the 393px phone corpus,
  the Assessment Type history adapter parses its closed value once, and the Attempt context
  comments describe the UUID-bearing browser contract accurately.
- M1 Shell contract: tier-one Ribbon destinations now follow Product Role, while signed-in shell
  rows retain stable height across route scope.
- M2 Shell checks: the role-only tier-one and declared task-row topology are covered by focused
  contract checks.
- M3 Shared components: `PageFrame`, `RecordList`, and the named date-formatting boundary provide
  the shared UI composition layer.
- M4 List options: presentation, reorder, and windowing compose outside `RecordList`; the
  six-site reorder comparison keeps shared mechanics separate from caller-owned workflow policy.
- M5 Seven-page proof: the proof set covers scan, dense, windowed, reorderable, action-bearing,
  Student, and gallery/list record patterns.
- M6 Page sweep: page frames and named date formatting replace per-page heading and date layout
  duplication across the swept page groups.

### Developer Tests and Notes

- 2026-09-22: Visual review of the synthetic M1 WP-A5 shell fixtures passed for the planned roles
  and viewports; corrected Student overview captures show the Course and Assessment breadcrumb.
  The focused Ribbon contract test passed (19/19). Attempt ancestry in the initial structural
  fixture was incomplete because resolved relationship scope was absent; source and contract
  evidence confirm production supplies it. No CSS change was warranted. Evidence is in
  `test-results/m1-wpa5-fixture-shell-20260922/` and
  `test-results/m1-wpa5-student-overview-20260922/`; these are not Live Demo proof. The M1 full
  suite remains pending.
- Plan execution now treats headed Chromium as an optional debugging mode. Required visual review
  consumes saved production-shell and screenshot-corpus artifacts through a fresh image-evaluator
  subagent; plan checkpoints and follow-up decisions are manager/agent-owned.
- Four-size visual inspection passed; `./launchers/run_fast_ui_checks.sh` and
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,266 tests).
- 2026-09-22: After the PageFrame formatting-only normalization,
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,266 Python tests in 7.70 seconds.
- Implemented the M1--M6 UI foundations: role-based tier-one Ribbon destinations and stable shell
  rows; focused shell checks; shared `PageFrame`, `RecordList`, and date formatting; caller-composed
  list presentation, reorder, and windowing; seven record-pattern proofs; and the page-frame/date
  formatting sweep.
- Fast UI developer lane passed: the shared production browser environment source is consumed by
  production and full-environment harnesses; isolated Chromium (`./launchers/run_fast_ui_checks.sh`
  and its documented headed single-case form) exercises the production shell/`PageFrame`,
  `RecordList` primitive states/options, avatar Gallery/List, Gradebook/Library route compositions,
  and Student Coursework. After the final phone action inset, the full isolated Chromium lane passed
  with its canonical viewport checks. An earlier four-size Coursework visual review found no
  responsive overflow; a follow-up 393px visual review confirmed the inset and hidden fixture route
  diagnostic. Overview-unavailable evidence is concise and state-oriented,
  and computed font-family checks retain the Atkinson Next/Mono contract while font response/loading
  assertions were removed; production build-order checks no longer inspect raw font-family output.
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,266
  Python tests plus Rust/TypeScript/lint/format gates). No backend stack, Live Demo, or screenshot
  publication ran; the compact full-stack WP-5 parity sample remains pending.
- A fresh screenshot publication after the final UI code provides direct Student laptop, tablet,
  phone, and square captures plus direct Instructor and Sysadmin laptop captures; Public scope is
  unchanged. `--verify-static`, the screenshot-corpus Node tests (13/13), and the atlas Markdown-link
  test passed. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed its offline aggregate
  checks, including 9,201 Python tests.
- One clean `source ./source_me.sh && ./devel/capture_screenshots.sh --fresh --verify` replay passed
  semantic workflow, manifest closure, scenario privacy, dimensions, and published-artifact
  integrity. It retained 130 byte-different replay PNGs in `test-results/screenshot-corpus/verify/`;
  visible IDs and dates can vary across clean resets, so those byte differences are observational,
  not a screenshot-count or failure gate. The full `all_test.sh` integration receipt is intentionally
  unrun and incomplete after an exit-130 interruption during its offline typecheck/lint phase, before
  `local_stack.py acceptance`; the warm-loop runner is also unrun. The clean M7 semantic replay is
  passed, but full plan acceptance remains pending.
- After the PageFrame/shell boundary tightening, the isolated Chromium lane passed and the offline
  aggregate fast checks passed (9,266 Python tests plus Rust, TypeScript, lint, and format gates).
  Visual review of the current-production Instructor harness confirmed one Ribbon, a reading-width
  CourseList, and full-width Gradebook. Student narrow captures came from the route-only M6 fixture,
  so they do not establish Student shell parity; the full-stack parity sample remains pending.
- M6 ownership/CSS audits confirmed WP-E4 and WP-E7 boundaries; legacy workspace rule names are
  reported as cleanup candidates and were not deleted. Avatar picker CSS had no safe cleanup.
- WP-E5 course-instance assessment refresh browser assertions live under `tests/playwright/`;
  the Node test-discovery entry imports that browser test without importing Playwright.
