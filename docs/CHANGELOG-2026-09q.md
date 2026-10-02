## 2026-09-22

> September 21 entries are archived in [CHANGELOG-2026-09o.md](CHANGELOG-2026-09o.md).

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
- M1 Shell contract: tier-one Ribbon destinations now follow User Role, while signed-in shell
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
