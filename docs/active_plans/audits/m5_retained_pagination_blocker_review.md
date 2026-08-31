# M5 retained-pagination blocker review

## Verdict

**RESOLVED: ACCEPTED.** The earlier retained-pagination blocker was real, but
the current retained-data acceptance exposes the arranged current assignment and
gradebook targets through ordinary visible keyboard controls. It completed M5
without a volume reset, fresh project, direct route, API shortcut, or manual
cleanup.

## Evidence

- Supported authenticated API checks retained only safe booleans: assignments
  page has `count: 50`, `hasNext: true`, and `currentTarget: false`; gradebook
  has the same `count: 50`, `hasNext: true`, and `currentTarget: false`.
- [CourseAssignmentsPage](../../../src/pages/course_assignments_page.tsx) renders
  only `runtime.queries.assignments(courseId)` items. It has no cursor state or
  visible next-page control.
- [GradebookPage](../../../src/pages/gradebook_page.tsx) loads one summary page
  with `loadGradebookPage` and renders only `page.items`. Its cursor control is
  limited to an already opened student run history, not the gradebook summary.
- The active M5 integration record already identifies the first-page retained
  volume limit as a product-page limitation, not a report or selector
  workaround: [M5 shared integration](../workstreams/wp_m5_shared_integration.md).
- The exact live attempt reached `playwright_arranged` and failed closed. Its
  redacted report directory and report file remained mode 0700 and 0600;
  runner cleanup left no private temporary state and no Podman containers.

## Why this is BLOCKED

The active walkthrough plan requires a visible next target and keyboard route.
J3 and J4 cannot reach their exact arranged assignment href when it is absent
from the rendered first assignments page. J5 cannot reach its exact arranged
gradebook row when it is absent from the rendered first summary page, and J8
depends on the J5/J4 outcomes. The tests correctly fail closed rather than
selecting a stale first row, inferring a target, using a direct route, or using
an API/browser-state shortcut.

The launcher preserves volumes for realistic retained-stack behavior. Resetting
volumes or selecting a fresh project would remove the condition being tested and
would not prove the product can expose the arranged work after ordinary retained
use. Neither a volume reset nor a direct-navigation shortcut is authorized for
M5 acceptance.

## Required product follow-up

Implement keyboard-complete, visible cursor pagination (or an equivalent
visible bounded next-page control) for both the course assignment list and the
gradebook summary. It must consume the server cursor/`hasNext` state, append or
deduplicate rows safely, preserve logical focus and reading order, announce
loading, added rows, errors, and end-of-list state, and keep an actionable
visible control reachable by Tab and operable with Space or native Enter.

Then add focused product tests with more than 50 retained rows and rerun the
arranged M5 journey on the retained stack. Only a visible exact assignment link
and gradebook row can unblock J3-J5/J8.

## Scope

This was the original finding on the retained stack before its 2026-08-11
resolution. It applied to M5 J3, J4, J5, and J8; it did not revoke accepted J1
or J2 evidence, diagnose a grading defect, or authorize a workaround.

## Resolution evidence

On 2026-08-11, the exact retained-data command completed:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
```

The redacted schema-v1 report is PASS with `masterSeed: 42` and `stage:
complete`. Its private directory and report file were mode 0700 and 0600. J1,
J2, J3, J4, J5, and J8 are each PASS with empty diagnostics. Their exact visible
codes are respectively:

- J1: `visible_start`, `visible_response`, `visible_submit`,
  `visible_feedback`, `visible_completion`.
- J2: the J1 codes plus `visible_retry`.
- J3: `visible_start`, `visible_leave`, `visible_return`.
- J4: `visible_mastery_completion`, `visible_mastery_fresh_practice`,
  `visible_exam_completion`, `visible_exam_closed`, `visible_back_action`.
- J5: `visible_gradebook`, `visible_run_history`.
- J8: `visible_learner_completion`, `visible_instructor_gradebook`.

`.last-run.json` records `passed` with no failed tests. Only it and the private
report remained in `test-results`; no trace, screenshot, video, or other
artifact remained. No `ple-ui-walkthrough-*` private temporary root remained.
The first read-only Podman check immediately after the Python runner exit saw a
short cleanup tail; a second read-only check seconds later was empty, with no
manual intervention. The final retained stack is therefore clean.
