# WP-S1 retained catalog-binding HCI review

## Scope and verdict

Independent read-only review of the repair for the retained-catalog binding
failure. The review covered the retry-corpus arranger, Python runner handoff,
instructor J13 Playwright child, PostgreSQL catalog-search query, and focused
offline gates. It did not start Podman, a stack, or a live browser journey.

**BLOCKED.** The visible binding design is sound, but the current primary
TypeScript and targeted ESLint gates fail. In addition, there is no retained
hostile ambiguity fixture proving that an additional matching rendered catalog
heading/result makes J13 fail rather than selecting one by order. Do not rerun
the retained stack until both repairs pass their focused gates.

## Accepted interaction and data boundary

- The private arrangement mints one new public title from its fresh workspace
  UUID: `Pilot retry corpus pilotref` followed by the lowercase hyphenless UUID.
  It is answer-free, bounded, unique per arrangement, and contains only ASCII
  letters, digits, and spaces. It is not derived from the selected variant,
  choices, Answer Key, Question Feedback, Question Answer Explanation, or Question Grading Input.
- J13 receives that title only through the private runner environment handoff.
  The runner validates its exact bounded shape, strips it from arrangements and
  reports, and removes it before later student child handoff. The public J13
  state retains only the public problem/version IDs after rendered selection.
- The instructor begins at `/`, fills the labelled `Search published problems`
  control, reaches `Search catalog` with `tabTo`, verifies focus, and activates
  it with Enter. It scopes a row to the exact rendered title, requires that
  title-bearing row count to be one, then reaches the row's `Add published
version` button by Tab and activates it with Enter. Static scanner inspection
  found no pointer operation, direct focus/route/history, request/API,
  cookie/storage, answer, or feedback-body shortcut in the child.
- PostgreSQL catalog search uses
  `websearch_to_tsquery('simple', $1)` against the metadata search vector in
  `crates/learning-data-access/src/postgres/catalog.rs`. The generated compact
  `pilotref` token intentionally avoids UUID hyphens and other web-search
  operators, so it is an ordinary simple-dictionary term rather than syntax.

This satisfies the cognitive-walkthrough requirement for recognition over
ordering: an instructor sees and searches for the fresh visible title, verifies
the exact displayed heading, and invokes its own rendered Add control by
keyboard. The static review cannot substitute for the required retained-stack
replay.

## Blockers

### B1 - primary TypeScript and lint gates are red

`retryCorpusCatalogSearchTitle` uses `workspace.replaceAll("-", "")`.
The repository's primary compiler target does not provide `String.replaceAll`.

Observed results:

- `npx tsc --noEmit` fails with TS2550 at
  `tests/playwright/simulator/retry_corpus.ts:203`.
- `npx tsc --noEmit -p tsconfig.lint.json` fails with the same TS2550.
- Targeted ESLint reports the resulting unresolved call and unsafe assignment,
  call, and member-access errors at the same line.

Use a target-compatible deterministic hyphen removal and rerun both TypeScript
and ESLint gates. Do not change the title contract or expand the configured
TypeScript library merely for this test helper.

### B2 - ambiguity rejection lacks a retained executable fixture

The J13 child currently asserts one exact title-bearing row with
`toHaveCount(1)`, which correctly fails on an additional row with the same
heading. However, no focused retained test creates that duplicate matching
heading/result and demonstrates the assertion refuses it. Add an executable
fixture at the J13 selector boundary that renders the fresh title twice and
expects failure; retain the normal one-result success fixture as its contrast.
It must prove no first-row fallback is introduced.

## Focused evidence

| Check                                                                                                                                                                               | Result                     |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `python3 -m pytest -q tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py`                                                                        | PASS: 46 tests, 5 subtests |
| `npx playwright test tests/playwright/simulator/retry_corpus.spec.ts tests/playwright/simulator/ui_walkthrough_arrange.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts` | PASS: 17 tests             |
| `npx prettier --check` on arranger/J13/config files                                                                                                                                 | PASS                       |
| `git diff --check` on reviewed files                                                                                                                                                | PASS                       |
| `npx tsc --noEmit`                                                                                                                                                                  | FAIL: B1                   |
| `npx tsc --noEmit -p tsconfig.lint.json`                                                                                                                                            | FAIL: B1                   |
| targeted ESLint                                                                                                                                                                     | FAIL: B1                   |

The initial Playwright invocation named a nonexistent `chromium` project; the
same focused tests passed when run through this repository's configured default
project. No browser body or Podman stack was used for the review beyond those
unit-style Playwright tests.

## Rerun checklist

1. Replace the target-incompatible title normalization and retain the exact
   public-title regular expression.
2. Add the duplicate rendered-heading/result rejection fixture and prove the
   normal exact-one path remains keyboard-only.
3. Rerun the two TypeScript commands, targeted ESLint, the focused Python
   scanner/runner suite, and the focused Playwright suite above.
4. Request re-review; only then run the retained-stack student-repeat command.

## Re-review - 2026-08-11

**ACCEPTED TO RERUN.** Both blockers are repaired without weakening the
catalog-binding or no-shortcut boundary.

- The UUID normalization now uses the target-compatible global hyphen regular
  expression, preserving the exact `pilotref` public-title contract. Both root
  and lint TypeScript configurations and targeted ESLint pass.
- `exactCatalogResult` is now the shared J13 selector boundary. It scopes the
  visible catalog result to an exact public heading, requires exactly one row,
  requires visibility, and returns that row only after those checks. J13 still
  reaches its own Add button with `tabTo` and Enter; it has no first-row
  fallback.
- The retained browser fixture renders two exact title-bearing articles and
  expects rejection, then renders one and expects success. This is the
  required direct contrast for a retained-catalog ambiguity rather than a
  source-only assertion.

### Re-review evidence

| Check                                                                                                        | Result                     |
| ------------------------------------------------------------------------------------------------------------ | -------------------------- |
| `npx tsc --noEmit`                                                                                           | PASS                       |
| `npx tsc --noEmit -p tsconfig.lint.json`                                                                     | PASS                       |
| targeted ESLint                                                                                              | PASS                       |
| focused Prettier and `git diff --check`                                                                      | PASS                       |
| `python3 -m pytest -q tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py` | PASS: 46 tests, 6 subtests |
| owner focused Playwright suite including `instructor_catalog_binding.spec.ts`                                | PASS: 19 tests             |

This reviewer's repeat of the 19-test command passed the 17 non-browser-body
tests but could not launch the two new fixture bodies because this sandbox
denies Chromium macOS Mach-port registration. The failure occurred before test
body execution and is the same known sandbox limitation; it does not indicate
a selector failure. No Podman stack was started.

The retained-stack student-repeat smoke may now be rerun. It remains a later,
separate proof of the real instructor-to-student path; this verdict does not
claim that live result.
