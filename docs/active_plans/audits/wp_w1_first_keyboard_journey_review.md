# WP-W1 first keyboard journey review

## Verdict

**ACCEPTED.** J1 is now a single non-retry completion path. It requires
the exact current Mastery href to be visible, reaches it only through bounded
backward Tab navigation from the route-focused main landmark, asserts focus,
and uses native Enter. After Start, it requires exactly two visible unchecked
native radios, reaches the second only through bounded backward Tab, asserts
focus, selects with Space, submits once, observes the Feedback heading, then
uses keyboard Continue and a bounded visible fresh-practice button. It neither
retries nor infers completion from a heading. The J1 platform path contains no
Playwright pointer action or grading read.

Focused offline evidence and the latest fresh real-stack acceptance support the
changed source. Earlier live evidence, including the former `Start another
practice run` control label, remains historical. This accepts WP-W1/M4 J1 only;
later journeys and release gates remain separate.

## Scope and method

This independent read-only review covered the active walkthrough plan, the
no-mouse contract, M3 arrangement evidence, the J1 specification, the fixed
Python runner/configuration, and the current route/component implementation.
It uses a keyboard cognitive walkthrough for the remote student's task: sign in,
open the arranged Mastery activity, submit visible responses, read authorized
feedback, and observe completion. This is code and offline-test evidence, not a
substitute for the required real-browser live walk.

## Findings

| Check                       | Evidence                                                                                                                                                                                                                                                                         | Result                               |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| No pointer action           | J1 contains no `.click()`, mouse, tap, or direct platform-control focus action.                                                                                                                                                                                                  | PASS                                 |
| Visible authentication      | Tab reaches the labelled password field and rendered submit button with focus assertions; native Enter activates sign-in and visible post-login heading/course-surface assertions follow.                                                                                        | PASS                                 |
| Course selection            | Exactly one visible `a[href="/courses/<courseId>"]` is required, then Tab, focus assertion, and native Enter are used.                                                                                                                                                           | PASS                                 |
| Assignment selection        | Exactly one current course-scoped Mastery href is required and visibly asserted, then bounded backward Tab from route-focused main, a focus assertion, and native Enter are used.                                                                                                | PASS                                 |
| Start and response          | Start uses Tab/Space. J1 requires exactly two visible unchecked native radios, reaches radio two with bounded backward Tab, asserts focus, selects it with Space, then uses visible Submit/Continue controls with focus or checked assertions.                                   | PASS                                 |
| Route focus                 | The spec confirms `#main-content` after both visible route changes; the application routes focus there.                                                                                                                                                                          | PASS                                 |
| Feedback focus              | The product focuses the Feedback heading, then only advances after a delay while that heading remains active. J1 finds Continue with Tab and confirms focus before Space.                                                                                                        | PASS subject to live timing evidence |
| Completion, not retry       | J1 makes one visible response, then boundedly requires the fresh-practice control after Continue. Historical evidence records its former `Start another practice run` label. J2 alone owns incorrect-then-correct retry.                                                         | PASS                                 |
| Selector durability         | Current public hrefs match course-list and course-assignment routes; exact count checks prevent retained-volume duplicate selection.                                                                                                                                             | PASS                                 |
| No browser shortcut         | No `page.request`, `addCookies`, storage state, route interception, direct post-login assignment `goto`, or programmatic focus is present in J1. Initial `page.goto("/")` is the honest entry route; `evaluate` reads active element only for a focus assertion.                 | PASS                                 |
| Evidence redaction          | The state fragment and rendered report permit only public IDs, fixed visible milestone codes, elapsed time, and bounded diagnostics. The Python runner drops child stdout/stderr and writes a mode-0600 report. J1 does not inspect feedback body, answer text, or grading data. | PASS                                 |
| Persisted browser artifacts | The fixed live runner adds `PLAYWRIGHT_NO_COPY_PROMPT=1`; the current Playwright config has no trace, screenshot, or video enablement, so normal Playwright defaults remain off.                                                                                                 | PASS for current runner boundary     |
| Fixed invocation            | The Python runner supplies only fixed smoke, arranged, and J1 specs after live validation and arrangement; live mode disables the mock preview server.                                                                                                                           | PASS                                 |

## HCI acceptance ledger

| Student step      | Need                                                           | Contract criterion                                                                 | Evidence required live                                            |
| ----------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Sign in           | Complete the same journey without a pointer                    | Tab reaches visible local sign-in button; native Enter or Space activates it       | Visible focused button and post-login course heading              |
| Choose work       | Recognize and open the correct prepared activity               | Current visible course and assignment hrefs are unique and keyboard activated      | Focus, route-main focus, and visible Mastery title/action         |
| Answer and submit | Preserve agency without requiring a shortcut                   | Native radio plus explicit Submit answer use Space                                 | Checked control, ready status, submit focus, and Feedback heading |
| Read feedback     | Learn the authorized result without disorienting focus changes | Feedback heading is announced/focused and delayed advance never steals moved focus | Feedback heading then Continue focus only after the delay         |
| Complete          | Know the task is done and what follows                         | Actual fresh-practice button is visible after Continue                             | Former label: `Start another practice run`                        |

## Offline validation

- PASS: `npx playwright test tests/playwright/ui_walkthrough_live_config.spec.ts tests/playwright/simulator/visible_outcome_report.spec.ts tests/playwright/ui_walkthrough_keyboard_j1.spec.ts --reporter=line` - 5 passed, 1 J1 live-only skip.
- PASS: ESLint and Prettier on J1, live configuration, visible report, and Playwright config.
- PASS: `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py tests/test_markdown_links.py tests/test_ascii_compliance.py tests/test_source_file_line_limit.py -q` - 1799 passed.
- PASS: `git diff --check`.
- PASS: `npx tsc --noEmit -p tsconfig.lint.json`.

## Required live evidence after repair

From a clean selected Compose project, run:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

Verify all of the following before accepting WP-W1:

- The live child has `PLAYWRIGHT_NO_COPY_PROMPT=1`, mock preview is disabled,
  and the public IPv4 gateway origin is used.
- After password-field fill, Tab reaches `Sign in locally`; a focused assertion
  precedes native Enter or Space. No platform `.click()` or `.focus()` occurs.
- The student uses only Tab/Shift+Tab, Space, and native Enter through current
  exact course and Mastery hrefs, Start, response, Submit, Feedback, Continue,
  and completion.
- After the course route focuses main, the exact visible Mastery href is reached
  by bounded backward Tab, is focused, and is activated with native Enter. No
  direct focus, route navigation, pointer action, or widget shortcut substitutes
  for that retained-volume replay path.
- Start produces exactly two visible unchecked native radios. Bounded backward
  Tab focuses radio two, Space visibly checks it, the explicit Submit button is
  focused and Space-activated once, and only the Feedback heading is observed.
  Continue is focused and Space-activated; a bounded visible fresh-practice
  button is the sole completion evidence. The former `Start another practice
run` label is historical evidence only. J1 must not retry or use a completion
  heading as an outcome proxy.
- Focus visibly reaches `#main-content` after both route changes, Feedback is
  visibly announced/focused before Continue, and focus never returns after the
  student moves it.
- The retry path bases each action only on rendered controls/status/feedback or
  completion, never answer order, feedback body, score, or a reconstructed
  grading result.
- The report has only the allowed public fields and no answer, feedback body,
  credential, cookie, source, trace, screenshot, video, or copied page context.
- The report directory is mode 0700, its report is mode 0600, and no selected
  Podman containers remain after normal no-volume cleanup.

## Re-review decision

A new independent offline inspection confirms the `.click()` repair, the
shared TypeScript helper's bounded native forward/backward Tab semantics, the
retained-volume replay correction, and J1's narrowed one-response scope. The
source guard requires the exact visible assignment href and
`tabTo(page, assignmentLink, "backward")`, exactly two unchecked radios, the
backward second-radio entry, visible fresh-practice completion, and no retry
loop or completion-heading inference. Focused offline checks pass. A new live
run is still required for this changed replay path. This review does not accept
J2 retry-until-correct, later
student/instructor journeys, canonical onboarding, all-family coverage, or a
release gate.

## Independent live re-review

### Command and result

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

PASS. The real launcher, gateway, arrangement child, and fixed Playwright child
completed successfully. The terminal wrapper returned after the browser phase
while the Python runner was still performing its own no-volume cleanup and
report write; a read-only process check confirmed that state, and inspection
continued only after the runner exited. No manual cleanup was issued.

### Redacted evidence

- The private report directory is mode 0700 and
  `ui_walkthrough_seed_42.json` is mode 0600.
- Its compact schema is version 1 with `status: PASS`, `masterSeed: 42`, and
  `stage: complete`; it contains the five labelled public arrangement records.
- Its sole J1 row is `PASS` for the arranged Mastery assignment with no
  diagnostics and exactly `visible_start`, `visible_response`, `visible_submit`,
  `visible_feedback`, and `visible_completion`.
- `.last-run.json` records `status: passed` with an empty `failedTests` list.
- `test-results` contains only `.last-run.json` and the private report file;
  no trace, screenshot, video, or failure artifact is present.
- No `ple-ui-walkthrough-*` runner private-state directory remains under the
  active temporary directory, and `podman ps --all --quiet` is empty.

### Historical live verdict

The preceding independent live evidence confirmed the prior keyboard-only J1
path, redacted outcome boundary, and runner-owned cleanup. It does not cover
the later retained-volume backward-Tab replay correction.

## Retained-volume and single-completion re-review

The current J1 source is **ACCEPTED TO LIVE**. After the course link is
keyboard-activated, the application visibly focuses `#main-content`. J1 first
requires exactly one visible current Mastery href, then calls the shared
`tabTo` helper with `"backward"`. The helper sends only bounded `Shift+Tab`
events and reads `document.activeElement` only to verify the visible target; it
does not call focus, click, route, request, cookie, storage, or history APIs.
J1 asserts target focus and activates the native link with Enter.

The same primary-path semantics apply to the unselected two-radio group:
bounded backward Tab through the browser's native order reaches radio two, then
Space performs the native selection. The test verifies exact radio count, unchecked
state, focus, checked state, ready status, focused explicit Submit, Feedback
heading, focused Continue, and bounded actual fresh-practice completion. It
does not inspect answer text, answer keys, correctness, score, feedback body,
or a completion heading, and has no retry loop; J2 remains the sole retry
journey. This is ordinary reverse platform traversal, not a direct-navigation
or widget-shortcut workaround. Focused Playwright configuration tests passed (5
pass, 1 honest live-only skip), as did strict TypeScript, ESLint, Prettier,
public-report/source guards (13 pass), and `git diff --check`. No live stack,
browser artifact, or cleanup action was used for this review. Repeat the exact
required live command before restoring a final live acceptance for WP-W1.

## Latest independent live acceptance

On 2026-08-11, the exact fresh-build command completed without manual
intervention:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
```

The runner finished its own cleanup before inspection. Its private report
directory was mode 0700 and `ui_walkthrough_seed_42.json` was mode 0600. The
schema-v1 PASS report has `masterSeed: 42`, `stage: complete`, five public
arrangement records, and one J1 PASS row for course
`c7a11b65-a521-5b98-9bcd-0217e3e98d1c` and Mastery assignment
`019ff026-36d6-77f2-848d-417d9003cbf3`. J1 records exactly
`visible_start`, `visible_response`, `visible_submit`, `visible_feedback`, and
`visible_completion`, with empty diagnostics. `.last-run.json` records passed
with no failed tests. Only that report and `.last-run.json` remained in
`test-results`; no trace, screenshot, video, error-context, credential, or
other artifact remained. No runner private-state directory remained under
`/private/tmp`, and `podman ps --all --quiet` was empty.

**Final verdict: ACCEPTED.** This live run covers the retained-volume and
single-completion J1 source currently reviewed. It does not extend acceptance
past WP-W1/M4 J1.
