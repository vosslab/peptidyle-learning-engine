# WP-W3 leave and return review

## Verdict

**ACCEPTED TO LIVE.** The hardened J3
specification starts through the same rendered local student sign-in and exact
arranged Mastery hrefs as J1. It reaches the visible `Return to assignment`
button through Tab, asserts focus, and activates the button with Space. It
then observes the assignment overview and focused main landmark before using
the visible Start or resume practice button through Tab and Space to return to
the run surface.

The M5 runner now appends the public fragment only after that final visible
run assertion. This permits the fixed real-stack J3 check; it does not claim a
completed real browser walk, persistence, saved response, score, grading, or
M5 completion.

## Scope and method

This independent read-only review covered the active walkthrough plan, the
no-mouse contract, J3 workstream record, J3 specification, fragment, focused
Node test, shared keyboard helper, live configuration, and the current Solid
route/control implementation. It did not start a stack or modify product,
test, runner, or report source.

## Findings

| Check                             | Evidence                                                                                                                                                                                                                                              | Result |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Visible student entry             | The initial `page.goto("/")` is the honest public entry. The labelled local credential field and rendered sign-in button are reached with `tabTo`; the sign-in button is focused then activated with native Enter.                                    | PASS   |
| Exact arranged selection          | The course and Mastery assignment locators use only the configured public UUID hrefs and each requires count one before Tab, focus assertion, and Enter.                                                                                              | PASS   |
| No pointer or navigation shortcut | J3 has no `.click()`, mouse action, direct post-login route navigation, browser history call, API request, storage setup, cookie setup, or route interception. It sends no Escape.                                                                    | PASS   |
| Rendered recovery control         | `getByRole("button", { name: "Return to assignment" })` resolves the native button rendered by the response controls. Its visual `(Esc)` text is aria-hidden, so the exact accessible name is durable.                                                | PASS   |
| Keyboard recovery                 | Tab reaches the return control, a focus assertion precedes Space, and the route changes to the rendered `assignmentOverview` surface. The application queues main-landmark focus after route changes; J3 observes that focus.                         | PASS   |
| Keyboard resume                   | The visible `Start or resume practice` button is reached by Tab, focused, and activated with Space. J3 then observes `runAttempt`; it does not infer run identity, saved data, or score.                                                              | PASS   |
| Public fragment shape             | The constructor creates exactly `schemaVersion`, `journey`, `status`, `elapsedMs`, `courseId`, `assignmentId`, `visibleOutcomeCodes`, and `diagnostics`. Runtime inspection confirms a normal plain object and exactly the three fixed visible codes. | PASS   |
| Identifier and timing bounds      | The UUID expression accepts lowercase RFC UUID versions 1 through 8 only. Uppercase input is rejected; elapsed time is a safe integer from 0 through 1,800,000 milliseconds.                                                                          | PASS   |
| Redaction boundary                | The fragment accepts only public course/assignment IDs and elapsed time, emits fixed visible leave/return/start codes and empty diagnostics, and imports no answer source, feedback, response, score, credential, storage, or API client.             | PASS   |
| Shared integration and ordering   | J3 imports the guarded private state appender and invokes it only after the final resumed `runAttempt` assertion. The fixed prefix accepts J3 only in the J1, J2, J3 sequence, and the renderer requires its exact public schema and fixed codes.     | PASS   |

## Offline validation

- PASS: `node --import tsx --test tests/test_student_leave_resume_evidence.mjs` - 3 passed.
- PASS: `npx playwright test tests/playwright/ui_walkthrough_keyboard_j3.spec.ts` - 1 expected
  live-only skip; no mock-browser substitution occurred.
- PASS: `npx tsc --noEmit`.
- PASS: ESLint and Prettier on the J3 spec, fragment, and Node test.
- PASS: `python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py` - 958
  passed.
- PASS: focused ASCII scan and `git diff --check`.

## Required live evidence

Run the fixed real-stack J3 selection:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

Accept the J3 portion only if all of the following are observed against the
real IPv4 gateway:

- The existing arranged student signs in and opens exactly the current Mastery
  assignment with visible keyboard controls.
- Tab visibly reaches the rendered Return to assignment button; Space, not
  Escape, browser history, or direct navigation, leads to `assignmentOverview`.
- The route-main landmark receives focus after the return route change.
- The visible Start or resume practice button is keyboard-operated and returns
  to `runAttempt` without a data, response, score, grading, or database claim.
- The report validates and includes only the bounded public J3 fragment and
  retains no browser artifact or credential.

## Review boundary

This decision does not accept J2, J4, J5, J8, canonical onboarding, all-family
work, or a release gate. A subsequent independent live review is required
after the fixed runner reports J3.

## Hardened integration re-review

The follow-up review confirms the requested integration hardening without
starting a live stack:

- The Playwright test has a total `test.setTimeout(90_000)` budget.
- The exact visible Mastery href must have count one and be visible. From the
  route-focused main landmark, `tabTo(..., "backward")` uses at most 40 native
  Shift+Tab operations before focus is asserted and native Enter activates it.
- The rendered Return to assignment action is reached through forward Tab and
  activated with Space. J3 contains no pointer action, direct focus call,
  history action, API request, storage access, answer/feedback inspection, or
  score/private-state inference.
- Both post-return `assignmentOverview` plus main-focus observations and each
  post-start `runAttempt` observation have explicit 15-second waits.
- `appendJourneyState(passedJ3LeaveReturnFragment(...))` follows the final
  resume assertion. A failure before that point cannot append a J3 PASS row.
- The appender validates a mode-0700 parent, mode-0600 regular non-symlink
  file, bounded canonical ASCII prefix, strict J1/J2/J3 order, and matching
  public IDs before its canonical rewrite. The renderer subsequently requires
  exactly the fixed J3 public keys and visible codes.

Focused validation passes: 16 Node tests, one expected offline Playwright
skip, TypeScript, ESLint, Prettier, 26 runner pytest cases, ASCII, and
`git diff --check`. The only remaining J3 gate is the independent live run
listed above.
