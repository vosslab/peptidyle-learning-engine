# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-12

### Behavior or Interface Changes

- Renamed PLE's QPM WASM consumer contract to the project-owned `qpm-wasm` package and
  `qpm_wasm` generated artifacts. The importer and vendor tool now load the producer's QPM
  namespace; QTI remains the name of the assessment interoperability standard. Removed the unused
  QTI runtime environment setting from the browser-test composition.
- Rotated the completed October 6 and October 7 change blocks to
  [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).

## 2026-10-11

### Additions and New Features

- Vendored QPM's browser WASM distribution with its license, source commit, and artifact hashes.
  The refresh script accepts `-q/--qpm-root`; the browser build ships the distribution at a
  content-addressed URL and loads it from Import.
- Added Import beside Draft creation. QPM converts readable question banks locally to Native JSON;
  the existing Draft API saves questions sequentially with ordinary metadata defaults. The results
  list preserves successful Draft links, server rejection messages, image skips, and individual
  request failures. Shared media only skips questions that reference images in their display content.

### Fixes and Maintenance

- Removed 1,664 named tests and 231 test-bearing files after restoring 36 low-cost behavior checks; conftest remains unchanged.
  The [pruning report](TEST_PRUNING_REPORT.md) records the clean rebuild, PostgreSQL/browser acceptance, and regeneration of all 257 screenshots.
- Pruned temporary and duplicate duration, Student presentation, Ribbon, and RecordList fixture
  checks. Retained Student wording/privacy tests, MATCH pointer/keyboard/touch behavior, real App
  workflows with a stub API, and RecordList selection, metadata retention, and reorder outcomes.
  Retired fixture geometry, font, structural, and inventory proofs are labeled historical evidence.
- Removed the stale standalone Bloom fixture from the teaching-browser launcher and retained the
  Course-to-Create-Assessment journey. Made the RecordList image fixture independent of avatar
  catalog order and replaced Ribbon's complete Student shortcut inventory assertion with required
  destination and unavailable Attempt behavior checks. Removed ignored Ribbon fixture arguments
  and eager whole-catalog validation while preserving validation of rendered controls.
- Removed the nine-test Ribbon SSR/CSS and fixture-inventory script from the full E2E launcher.
  Its Instructor fixture exercised only available controls; the disabled branch did not establish
  disabled Instructor behavior. Marked that checklist evidence retired and unsupported.
- Corrected the Student Course entry fixture's bonus denominator and asserted score values without
  binding them to incidental wording. Removed row-position and inventory checks while retaining
  keyboard, resume/review/start, unavailable action, score, and viewport usability behavior.
- Aligned the release browser fixture with the API-required `poolIssues` array and typed response.
  Retained alert, correction-category, and recovery behavior checks without exact heading or prose
  assertions. Both final browser corrections change test fixtures and assertions only.
- Simplified the scoped authoring and delivery code before extracting cohesive owners. Native JSON
  authoring now separates answer mapping and workflow coordination from the page; Assessment
  policies retain page state and delivery fields while feedback and release views own presentation.
  Domain validation separates authored and issued-presentation rules behind the existing public API.
  Server composition separates route assembly, environment configuration, and worker entry points.
  Draft SQL separates access, lifecycle, and fork operations in the same installation slot, preserving
  all function bodies, creation roles, security settings, and grants. Removed redundant submission
  fakes, duplicate receipt checks, composition route reconstruction, and the email adapter indirection.
- Removed unused upstream browser and Node demos from the vendored QPM package. Refreshes now
  copy only the wrapper, generated runtime, types, and license. Updated artifact provenance and
  removed the demo ZIP library's line-limit exception; all eight retained artifact hashes match
  the previously accepted bytes.
- Archived 215 historical planning files and repaired their references, retaining nine active
  files with an explicit [current-work index](active_plans/active/current_work.md). Preserved open
  findings and deferred proposals without claiming implementation. Replaced human-dependent
  plan gates with agent-owned local setup, behavior tests, independent assessment, and repair
  paths; assigned remaining release and Human Guidance evidence work explicit execution owners.
  See the [cleanup report](archive/reports/active_plan_cleanup_2026_10_11.md).
- Completed six independent QPM audit passes and reconciled the plan's unmet, deferred QTI reader
  acceptance with implemented browser imports. Clarified deferred image binding in the import
  specs and Design Decisions, removed hidden refresh environment configuration, and pruned retired
  QTI dependencies from the dated audit snapshot. Kept the required Playwright selector contract
  with source-line references. See the [audit report](archive/audits/vendor_qpm_audit_2026_10_11.md)
  for findings, focused checks, and unrelated pre-existing dependency-snapshot drift.
- Removed the unused QTI adapter and imported-source helper after confirming that their consumers
  were confined to the removed code and tests. Removed their exclusive fixtures, constructors,
  and Cargo dependencies; the remaining workspace builds.
- Documented browser conversion ownership, refresh and usage steps, per-question media handling,
  individual failures, and saved-Draft versus publication validation. Design Decisions records
  HOTSPOT as PLE-only authoring. TODO records inline images, HTML rendering, and QPM's missing
  Canvas/Blackboard QTI readers. Updated obsolete current references and repaired historical source
  links using immutable Git revisions.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Developer Tests and Notes

- Student presentation passed two Node and three MATCH browser tests, both TypeScript
  configurations, and scoped formatting/lint checks. RecordList's retained Chromium contract,
  both TypeScript configurations, and scoped formatting checks passed. These source-fixture
  checks do not establish production-stack or assistive-technology acceptance.
- Scoped module work passed both TypeScript configurations, 47 Native JSON authoring tests,
  15 Assessment-policy tests, and scoped ESLint/format checks. Rust validation passed 11 domain,
  two matching, one native-Wasm response-format, and one submission test; server/project-tools
  all-target checks and the consolidated live-demo mapping test passed. SQL schema style and a
  one-time comparison confirmed unchanged 18 function statements and grants. These focused checks
  precede independent review and final repository acceptance; no permanent extraction tests were added.
- Runtime-only QPM packaging passed a refresh into a temporary directory, the browser build and
  typecheck, mixed-bank Native JSON conversion from the built package, and 2,748 focused hygiene
  checks. The built package excludes upstream demos. Added no permanent tests or fixtures.
- Documentation cleanup passed 557 Markdown-link checks, both 1,248-bullet checklist checks,
  relocation and visual-artifact integrity checks, and 6,032 repository hygiene checks. Fresh
  independent review accepted retained-plan ownership and autonomous completion. An actual-WASM
  probe verified all 18 vendor hashes and unsupported reader results for real Canvas/Blackboard
  QTI packages. These checks do not renew earlier runtime acceptance.
- QPM vendor refresh, artifact-hash verification, a real WASM BBQ-to-Native-JSON conversion, and
  the complete PLE debug build passed for the vendor milestone.
- `cargo check --workspace` passed after removing the unused QTI adapter, its server/workspace
  dependencies, and the exclusive imported-source constructor and fixture support.
- The permanent seven-kind BBQ browser journey passes through saved Draft previews. Focused live
  checks published all seven kinds through ordinary controls, verified mixed/shared-image packages,
  continued after HTTP and network failures without retries, displayed converter warnings, and
  confirmed that ordinary pages request no QPM wrapper or WASM.
- Captured and visually checked the import screen, results, and a published imported Question.
  Removed one-time probes and capture registration; retained only the seven-kind browser journey.
  Documentation link and guidance-format checks passed (554 checks).
- The final `all_test.sh` passed, including 11,051 Python tests, all 19 PostgreSQL acceptance
  groups, installation-data provision/replay, and Course Appearance PostgreSQL/MinIO coherence.
- Renamed PLE's integration paths to `assets/qpm_wasm/` and `devel/vendor_qpm_wasm.py` after the
  naming clarification. Upstream filenames and all artifact hashes remain unchanged; the rebuilt
  browser bundle is identical to the one used for live verification. Design Decisions reserves
  QPM for software/integration names and QTI for the standard and its specific functionality.
  Final fast checks passed with 11,063 Python tests after targeted Rust artifact cleanup restored
  the required disk budget.
