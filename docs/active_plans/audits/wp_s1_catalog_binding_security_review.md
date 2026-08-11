# WP-S1 catalog binding security review

## Verdict

ACCEPTED TO RERUN.

This read-only review covers the retained-catalog binding repair at the J13-to-J1
runner boundary. It does not make a Podman or final walkthrough acceptance claim.

## Boundary findings

- The arranger output accepts the catalog title only in the one-record instructor
  arrangement. The runner requires the exact lower-case title shape
  `Pilot retry corpus pilotref` followed by 32 lower-case hexadecimal characters.
- The title is copied into `instructor_catalog_search_title`, then the retained
  arrangement contains only `label`, `problemId`, and `versionId`. It therefore
  cannot enter `PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON` or a report arrangement.
- J13 receives the title only through `PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE`.
  `hand_off_instructor_setup()` removes that environment variable and the
  instructor-only marker before J1. Student children receive only the validated
  course, Mastery assignment, and corpus problem public UUIDs.
- The descriptor-anchored state reader requires a mode-0700 parent, mode-0600
  regular `journeys.json`, bounded ASCII canonical JSON, and exact J11/J12/J13
  records. It rejects replacement, reordered, private, inherited, hidden,
  accessor, Symbol, duplicate-key, upper-case-ID, oversized, and mismatched
  records.
- The J13 `problemId` and `versionId` must equal the retained arranged corpus IDs
  before student children can start. Its course and Mastery assignment IDs are
  also the sole values accepted by the student append state.
- `instructor_setup_handoff` is a distinct failure stage between the instructor
  browser child and J1. Child stdout/stderr are not retained in the failure
  report; reports contain only status, seed, stage, and already-redacted public
  arrangements when applicable.
- `--student-repeat-only` is the only successful partial mode. It records the
  explicit `student_repeat_only` marker after J1-J4. The default route raises a
  fail-closed error after J4 until WP-S2/WP-E1 provide the gradebook and final
  schema-v2 report; the instructor-only and student-repeat-only flags are
  mutually exclusive.

No answer, credential, or catalog title is persisted in the journey state,
arrangement state, or failure receipt. Credentials are passed to the arranger or
rendered local-login child only through protected local files.

## Validation

- `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_harness_independence.py`
  - 46 passed.
- `npx playwright test tests/playwright/simulator/ui_walkthrough_arrange.spec.ts tests/playwright/simulator/instructor_setup_state.spec.ts tests/playwright/simulator/student_repeat_state.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts`
  - 19 passed.
- `npx tsc --noEmit`
  - passed.
- `npx prettier --check` on the reviewed TypeScript files, `python3 -m py_compile`
  on the reviewed Python files, and `git diff --check`
  - passed.

## Repair recheck

The post-review catalog binding repair remains accepted. The J13 specification now
uses `exactCatalogResult()` instead of a direct first-result selection. The helper
uses target-compatible Playwright locators and requires exactly one visible article
with the arranged title. Its hostile fixture proves two matching retained results
throw instead of selecting either result; its single-result fixture passes.

- `npx tsc --noEmit`
  - passed.
- Focused live-config/arranger Playwright checks
  - 10 passed, 1 explicit live-only specification skipped.
- `npx playwright test tests/playwright/simulator/instructor_catalog_binding.spec.ts`
  - 2 passed.
- Focused runner and harness checks
  - 46 passed.

## Scope note

No Podman command was run. This review authorizes the next retained-stack rerun
of the WP-S1 partial slice; final student scoring, gradebook, and schema-v2
visible-outcome acceptance remain owned by WP-S2 and WP-E1.
