# Cohesive modules and test pruning

This ledger tracks work to simplify code and remove temporary or redundant test
scaffolding while preserving behavior through existing meaningful checks. The
ten advisory-band files below are an investigation baseline, not a required
file inventory for extraction. The source limit and gate stay unchanged.

## Accepted direction

**D1 - Optimize for simplicity, not file count.** Remove unnecessary code,
tests, and abstractions first. Proposed extraction names are options; keep a new
module only when its cohesion and clarity exceed its interface and navigation
costs. Advisory warnings trigger investigation, not mechanical splitting. Keep
the existing under-1,000-line rule and add no under-900 acceptance target.
Implementation may use this simpler direction without another planning or human
gate.

Implementers confirmed D1 for JSON, policy views, validation/submission,
composition, and SQL. Their cohesive capability boundaries remain justified;
the policy page specifically keeps delivery fields and page state together,
extracting only feedback controls and release/readiness views.

## Baseline

The initial worktree already contained unrelated staged and unstaged edits. An
ignored snapshot in `output_cohesive_test_pruning/` records the initial short
status, staged and unstaged diff summaries, and changed paths. The snapshot has
576 status lines, 347 unstaged diff-stat lines, and 242 staged diff-stat lines.

| Scoped file | Initial lines |
| --- | ---: |
| `crates/domain/src/validation.rs` | 935 |
| `crates/server/src/assessment_delivery/submission.rs` | 918 |
| `crates/server/src/composition.rs` | 909 |
| `schemas/base_schema/50_functions/question_authoring_operations.sql` | 947 |
| `src/features/ple_question_json_authoring/question_json_editor_page.tsx` | 931 |
| `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` | 939 |
| `tests/playwright/test_student_assessment_presentation.mjs` | 957 |
| `tests/support/record_list_harness.tsx` | 969 |
| `tests/support/ribbon_shell_harness.tsx` | 976 |
| `tests/test_ribbon_contract.mjs` | 980 |

The ten files total 9,461 lines. The focused source-line-limit suite passed
2,020 tests with the expected ten advisory warnings (`source ./source_me.sh &&
pytest -q tests/test_source_file_line_limit.py`). Previously verified baseline
results supplied with this task are 47 Node authoring checks passed and both
TypeScript configurations passed; their exact invocations will be recorded with
integration evidence.

## Milestones

| ID | Scope and keep/delete rationale | Status | Evidence |
| --- | --- | --- | --- |
| M1 | Record baseline counts and worktree snapshot before edits. | Done | 2,020 passed; 10 warnings; 10 files at 909-980 lines. |
| M2 | Remove the duration scanner when its fragile source-shape checks are redundant with meaningful behavior coverage. | Done; review pending | `tests/test_assessment_duration.mjs`: 91 to 74 lines; five semantic tests remain, with five scanner `includes`/import checks removed and no replacement. Baseline and post-change focused command passed 15 tests: `node --import tsx --test tests/test_assessment_duration.mjs tests/test_base_assignment_policy_autosave_model.mjs tests/test_assignment_workspace_policy_model.mjs tests/test_assessment_summary_policy_decoder.mjs`. |
| M3 | Prune redundant Ribbon Node checks where durable browser coverage owns the behavior. | Done; review pending | `test_ribbon_contract.mjs` fell 980 to 368 lines and 20 to 8 retained tests. Targeted Student Tier 2 destination and unavailable-context `activeAttempt` assertions replaced broad source inventories. Both TypeScript configs, the Node contract (8/8), scoped shell Chromium and QuestionPool checks, ESLint, Prettier, and diff check passed. |
| M4 | Keep semantic student-assessment Node checks and the `MATCH` browser path; preserve meaningful student behavior coverage. | Done; review pending | Focused Node checks passed; the 3-modality browser suite passed 5/5. The replacement is currently 257 lines versus 957 at baseline; final formatting remains. |
| M5 | Prune redundant RecordList checks while retaining behavior-level coverage. | Done; review pending | D1 removed duplicate outline matrices and low-level demos without replacement; retained state refresh, accessible native selection, image fallback, and real reorder/disabled/rejected-save outcomes. `record_list_harness.tsx` 969 to 256; browser contract 705 to 212; helper 89 to 77; reorder helper (306 lines) deleted. The image fixture uses an inline Amber Arch record and existing SVG, independent of avatar catalog order. Focused browser, both TypeScript configs, Prettier, ESLint, and `git diff --check` passed. |
| M6 | Purge obsolete Ribbon fixtures and API inventories; retain a fake-data owner helper where its responsibility justifies it. | Done; review pending | Across the Ribbon shell/browser set, touched lines fell 3,709 to 1,761 (1,948 fewer): browser contract 342 to 283; synthetic evidence 824 deleted; shell helper 149 to 31; model fixtures 438 to 291; harness 976 to 180; retained API helper 608. Removed source/CSS/SSR/order/catalog inventories, route matrices, synthetic screenshot/geometry/sign-out fixtures, and unused globals/counters. Retained fail-closed route/role admission, canonical breadcrumbs, selected continuity, real-App deferred labels and roster decisions, read-only Student View, vocabulary writes, and Profile sign-out/protected recovery. The API helper owns fake data; root keeps stable App mount/globals/disposal. A one-time deep-equality probe confirmed all 12 emitted fixtures remained unchanged; the probe was removed. The 9-check direct component E2E and its runner invocation were deleted without replacement: its disabled-Instructor assertion was vacuous, and its two failures were stale route-param/CSS-arithmetic assumptions. Existing browser navigation, keyboard, Profile, responsive, route/capability, production-style, and sprite-build coverage remains. Final fixture formatting, ESLint, both TS configs, shell syntax, and reference absence checks passed. |
| M7 | Simplify JSON answer behavior; extract a helper only if it clarifies ownership enough to justify its interface. | Done; review pending | `question_json_editor_page.tsx`: 931 to 255 lines; answer helper: 64 lines. No permanent tests were added; temporary proof code removed 27 workflow bodies. |
| M8 | Simplify JSON editor controller behavior; extract a module only if it clarifies ownership enough to justify navigation cost. | Done; review pending | Controller module: 768 lines. Both TypeScript configs and all 47 Node authoring tests passed at baseline and each stage; ESLint and Prettier reported zero findings. |
| M9 | Keep delivery fields and page state together; extract only feedback controls and release/readiness views. | Done; review pending | `assessment_workspace_policies_page.tsx`: 939 to 756 lines; feedback controls: 69 lines; release/readiness views: 193 lines. D1 rejected delivery-fields extraction (12 state props and 14 callbacks would add interface cost). Both TypeScript configs passed with zero diagnostics; four scoped Prettier checks and ESLint passed. Fast UI release fixture gained its required `poolIssues` array and explicit assessment-release validation type; retained correction behavior checks role alert, categories, and Questions recovery link while removing exact-heading/full-prose checks. Delivery Chromium passed 3/3; fixture/browser lines moved 704 to 696 (harness 542, browser 154); scoped lint-TS and Prettier passed; no production change or permanent test was added. |
| M10 | Simplify domain validation; split responsibilities only where the module boundary improves clarity. | Done; review pending | `validation.rs` 935 to 157 lines; authored validation 426; presentation validation 340 (923 total). Focused validation checks passed 11 cases. |
| M11 | Remove the submission fake test when existing meaningful checks cover its contract. | Done; review pending | `submission.rs` 918 to 357 lines. Production behavior is unchanged; the non-leak test is retained. Focused matching and Native WASM checks passed 2 and 1 cases; submission check passed 1 case. Combined validation/submission source totals fell from 1,853 to 1,280 lines. |
| M12 | Simplify server API assembly; split by responsibility only where ownership becomes clearer. | Done; review pending | `composition.rs` and `live_demo.rs` fell from 1,080 combined lines to 958 across the 20-line facade, API (380), environment (193), workers (152), and demo (213); `object_storage.rs` remains 95 lines. Ten public function definitions and the TOTP re-export remain; 19 production bodies compare unchanged after namespace/format normalization. Scoped `rustfmt --check`/diff and `cargo check -p server_core -p project-tools --all-targets --features server_core/local-disposable-storage` passed. `cargo test -p server_core --lib --features local-disposable-storage composition::` passed 1 test (202 filtered). |
| M13 | Simplify worker and environment responsibilities; introduce modules only where cohesive boundaries justify them. | Done; review pending | Worker and environment modules are 152 and 193 lines. D1 removed the fake SessionStore/manual Router test and redundant optional-email helper/test; three durable missing, malformed, and duplicate mapping scenarios are consolidated in one owner-side test. The test module fell from 192 to 46 lines, with no permanent extraction test or framework. |
| M14 | Simplify the SQL question-authoring fork; split it only where cohesive boundaries justify them. | Done; review pending | Question-authoring fork module is 369 lines. SQL structural evidence is recorded under M15. |
| M15 | Simplify SQL access and lifecycle code; split responsibilities only where ownership becomes clearer. | Done; review pending | SQL access and lifecycle modules are 141 and 439 lines; 18 statements, roles, and grants are identical; schema checker reported zero findings. |
| M16 | Integrate, review, and verify all scoped changes and retained behavior. | In progress | Direct teaching-browser cleanup removed a missing Bloom fixture and incidental editor-layout assertions while retaining course to visible Create Assessment to title entry; the fast UI launcher no longer invokes the orphan Bloom test. Student entry evidence now checks actual `3 / 8 points` rendering and preserves keyboard course/resume/review/start/unavailable journeys, scores/pending states, due consistency, axe, and four-viewport usability; stale positional/layout/tag/date/probe assertions were removed. Focused browser, Prettier, and TypeScript passed. A separate 9-check direct Ribbon component E2E was removed as vacuous/stale, with real browser and capability coverage retained; docs corrected the unsupported disabled-rendering checklist claim and marked it unsupported. Backend/frontend spec and quality reviews and fresh test-spec re-review passed. Offline `all_test` passed Rust, 584 Node checks, TypeScript/lint/format, and 11,109 pytest checks with no warnings; stale generated output resolved a target-cache budget failure. Local-stack acceptance and post-runtime docs checks remain pending. `SOLID_MODEL.md` now names `createQuestionJsonEditorController` as state owner beneath the page (called at line 69); changelog records teaching launcher, RecordList, Ribbon, release, and student-entry corrections. Final scoped metrics at `output_cohesive_test_pruning/final_scoped_metrics.md`: 13,976 to 9,319 lines (4,657 fewer) across 12 named cohorts: 12,576 to 7,945 core, plus student-entry (18 fewer) and release (8 fewer); teaching and deleted-SSR cohorts have unknown baselines and are omitted. Fifteen new source/test files are below 1,000 lines, maximum 768. No repo-wide net change is claimed. Source/contracts regeneration showed no changes. |

Use one-time checks as implementation evidence, not as permanent replacement
tests. Keep the existing line-limit threshold and avoid reducing line counts by
compressed formatting. Add exact outcomes and remaining concerns as milestone
owners report them, including the rationale for each extraction or deletion.
