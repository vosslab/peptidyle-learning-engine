# WP-W4 instructor gradebook review

## Verdict

**ACCEPTED OFFLINE.** The repaired J5 implementation is a sound separate-context,
keyboard-first instructor walk. It binds the rendered Gradebook link to the
arranged current course. Live evidence and shared-report integration remain
separate M5 obligations.

## Scope and method

This read-only review covered the active walkthrough plan, W4 workstream,
J5 spec and public fragment, live-input credential reader, keyboard helper,
and the current course/gradebook components. It did not start a Podman stack,
run a browser against a live service, alter implementation files, inspect
scores, dates, learner identities, cookies, storage, APIs, or private records.

## Original finding

| Severity | Finding | Evidence | Required correction |
| --- | --- | --- | --- |
| P1 | The Gradebook target was not course-scoped. | The earlier revision selected only `getByRole("link", { name: "Gradebook", exact: true })`. The product route is `/instructor/courses/<courseId>/gradebook`. | RESOLVED: the repair owns an exact href selector, requires count one, then uses Tab, focused assertion, and native Enter. A focused unit test pins the selector. |

## Accepted offline boundaries

| Check | Evidence | Result |
| --- | --- | --- |
| Fresh context | J5 creates `browser.newContext({ baseURL: inputs.baseUrl })`, opens its own page, and closes the context in `finally`. | PASS |
| Visible local instructor sign-in | The exact labelled credential input is reached by `tabTo`, focus is asserted, the instructor value is filled, then the rendered sign-in button is reached by Tab and activated with native Enter. | PASS |
| Keyboard route | Course, exact current Gradebook href, and run-history controls use Tab plus focused assertions; Enter activates each. No pointer, direct focus, navigation shortcut, route interception, request context, storage state, cookie, database, or session injection occurs. | PASS |
| Gradebook outcome | The walk waits for the rendered gradebook surface, a visible `View run history` button, `aria-expanded="true"`, and a visible named run-history region. It neither reads score, date, nor the learner identifier suffix. | PASS |
| Credential boundary | `instructorCredentialFromValidatedFile` verifies a regular non-symlink file and exact mode 0600 on non-Windows hosts, selects exactly one `instructor=` line, and emits only a generic unsafe-metadata error. The J5 fragment/report boundary receives no credential. | PASS |
| Public fragment | The J5 fragment has fixed schema version, `J5`, `PASS`, fixed visible codes, empty diagnostics, and only supplied course/assignment IDs plus elapsed time. The focused test rejects answer-like vocabulary from serialized output. It is intentionally not wired to the shared report yet. | PASS for deferred M5 integration |
| Hygiene | J5 focused Playwright/config tests, strict TypeScript, ESLint, Prettier, and tracked-diff whitespace checks pass. The focused re-review used `PW_PORT=4276` because another agent owned the default mock-server port. | PASS |

## Offline validation

- PASS: `PW_PORT=4276 npx playwright test tests/playwright/simulator/instructor_gradebook_j5.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts tests/playwright/ui_walkthrough_keyboard_j5.spec.ts --reporter=line` - 6 passed, 1 J5 live-only skip. This includes the exact-course Gradebook-selector test.
- PASS: `npx prettier --check` on the five J5/config TypeScript files.
- PASS: `npx eslint --max-warnings 0` on the same files.
- PASS: `npx tsc --noEmit`.
- PASS: `git diff --check`; targeted ASCII and forbidden-shortcut scans produced no findings.

## Required re-review and live checklist

Verify that the J5 runner integration records only public J5 evidence. Then,
after J1/J2/J3 learner activity is present in the selected shared arrangement,
run the fixed walkthrough command and independently verify:

- J5 creates a new context with the configured public loopback base URL.
- The instructor uses the rendered credential form, Tab, and native Enter only.
- The exact current course href and exact current Gradebook href are unique,
  focused through Tab, and activated with native Enter.
- The run-history button is visibly focused and Enter produces
  `aria-expanded="true"` and the named region without reading gradebook score,
  date, or learner identity text.
- Shared report/state integration carries only the exact public course and
  assignment IDs, the two J5 visible codes, elapsed time, and permitted
  diagnostics; it contains no credential, answer, score, date, learner ID,
  cookie, trace, screenshot, video, or copied page context.
- Normal no-volume cleanup leaves no selected Podman containers or temporary
  runner state.

This review accepts the offline W4/J5 slice only. It accepts no live J5
evidence, shared-report integration, cross-actor conclusion, or M5 exit claim
until the stated live evidence is independently checked.

## Integrated J5/J8 re-review

### Verdict

**ACCEPTED TO LIVE.** The formatting and private-state reader repairs are now
present. The added browser-side J5 and J8 bindings satisfy the reviewed
behavior, public-only evidence, and focused offline gate contracts. This is
not live acceptance.

### Accepted integration boundaries

| Check | Evidence | Result |
| --- | --- | --- |
| Deterministic visible title | Assignment arrangement derives `Peptide mastery retry <full-problem-UUID>` from the public corpus problem ID. The runner passes that strictly validated public UUID as `PLE_UI_WALKTHROUGH_LIVE_MASTERY_PROBLEM_ID`; J5 derives the same title locally. | PASS |
| Exact visible gradebook row | J5 filters rendered rows by an exact visible rowheader title, requires exactly one matching row and exactly one scoped `View run history` button, then reaches that button by Tab and activates it with Enter. There is no `.first()` fallback. | PASS |
| History target | After keyboard activation, J5 requires `aria-expanded="true"`, reads its public DOM `aria-controls` reference, and requires that exact target to be visible. It reads no score, date, learner-ID suffix, or run text. | PASS |
| Fail-closed absence | No pagination or alternate-row fallback exists. If the first gradebook page lacks the expected row, the exact row count assertion fails before a J5 fragment is made. | PASS |
| Public runner/report boundary | Config and runner pass validated public UUIDs only. The J5 state fragment and renderer schema retain course/assignment IDs, elapsed time, and fixed codes only; the deterministic title does not enter the fragment or report. | PASS |
| J8 consistency | The fixed child parses the private canonical prefix, accepts exactly J4 and J5 at positions four and five, and derives J8 only when their public course and Mastery assignment IDs match. It does not inspect a title, score, learner, credential, API, or private record. | PASS |
| No browser shortcut | Targeted scans found no `.first()`, pointer action, API request context, request call, storage state, cookie operation, storage inspection, or `page.evaluate` in the J5/J8 binding path. | PASS |

### Focused validation

- PASS: `PW_PORT=4277 npx playwright test tests/playwright/simulator/instructor_gradebook_j5.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts tests/playwright/ui_walkthrough_keyboard_j4.spec.ts tests/playwright/ui_walkthrough_keyboard_j5.spec.ts --reporter=line` - 7 passed, 2 expected live-only skips.
- PASS: `node --import tsx --test tests/test_visible_outcome_report.mjs` - 14 passed,
  including noncanonical-state and unsafe-parent rejection by the J8 reader.
- PASS: `python3 -m pytest -q tests/test_ui_walkthrough_runner.py` - 26 passed.
- PASS: targeted ESLint, `npx tsc --noEmit`, and `git diff --check`.
- PASS: targeted `npx prettier --check` after the repair.

No live stack was started for this re-review. The required live checklist above
still applies.
