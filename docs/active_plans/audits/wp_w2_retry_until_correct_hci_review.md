# WP-W2 retry-until-correct review

## Verdict

**ACCEPTED.** The repaired J2 vertical
preserves the accepted J1 platform path and starts its retry exercise through
visible server-owned state. After Start/resume, J2 waits only for a rendered
inline error, visible radio, or visible `Start another practice run` button.
It proceeds on the radio surface, or focuses and Space-activates that actual
button before proceeding. J2 then selects the first and second visible radios.
It observes Feedback headings, visible retry, and visible completion without
inspecting feedback body, correctness, score, answer, source, API response,
cookie, storage, or browser history.

## Scope and method

This independent review covered WP-W2, the active walkthrough plan, the
no-mouse contract, J1 and J2 specifications, the shared Tab helper, the fixed
serial Python runner, and the public-only report boundary. It applies a
keyboard cognitive walkthrough for a remote student: sign in, open the exact
arranged Mastery activity, make the two visible selections, read authorized
feedback, observe a visible retry, and observe completion.

## Findings

| Check                       | Evidence                                                                                                                                                                                                                                                                           | Result                               |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Shared J1 path              | J1 now imports only `tabTo`; its sign-in, current href, route-focus, Space, feedback, and completion behavior is otherwise retained.                                                                                                                                               | PASS                                 |
| Whole-browser keyboard path | J2 uses initial root navigation only, then Tab/Shift+Tab, native Enter, and Space. It contains no `.click()`, mouse/tap, direct focus, API request/route, cookie, storage-state, back/forward, or direct post-login route action.                                                  | PASS                                 |
| Visible sign-in             | Tab reaches the labelled credential input and visible submit button, with focus assertions before fill and native Enter; the visible courses surface follows.                                                                                                                      | PASS                                 |
| Exact Mastery route         | J2 requires exactly one visible current href `/courses/<courseId>/assignments/<masteryAssignmentId>`, reaches it by Tab, and activates it with native Enter.                                                                                                                       | PASS                                 |
| Existing run recovery       | After Start/resume, a bounded structural poll accepts only a rendered inline error, a visible radio, or the actual visible `Start another practice run` button. Only the latter is Tabbed to, focused, and Space-activated before J2 expects radios.                               | PASS                                 |
| Visible first response      | Only after the visible fresh run appears, the first rendered radio is reached by Tab, focused, selected by Space, visibly checked, and followed by visible format readiness and keyboard Submit.                                                                                   | PASS                                 |
| Feedback boundary           | Each submission requires only the `Feedback` heading. J2 neither queries feedback text nor tests correctness, score, correct response, or answer content.                                                                                                                          | PASS                                 |
| Visible retry               | After the first keyboard Continue, J2 applies the same 15-second bounded structural poll and proceeds only when visible radios return `run`; a visible inline error or fresh-practice button fails. This is the retry observation; no grading result is inferred or reconstructed. | PASS                                 |
| Visible completion          | After the second keyboard Continue, J2 requires the actual visible `Start another practice run` button. It no longer relies on asynchronously changing summary wording.                                                                                                            | PASS                                 |
| Focus semantics             | Every exercised control is located through `tabTo`, asserted focused, and activated with native Enter or Space. The product's delayed feedback focus moves to Continue only while the Feedback heading remains active; J2 still uses Tab and a focus assertion before activation.  | PASS subject to live timing evidence |
| Report boundary             | J1 writes one bounded state row; J2 validates it with descriptor-level private-file checks and appends only a bounded J2 row. The renderer requires exact ordered J1/J2 fragments for the same public course and assignment, with J2's fixed `visible_retry` milestone.            | PASS                                 |
| Fixed serial runner         | The Python runner invokes fixed smoke/arranged/J1 specs, then a separate fixed J2 invocation before the sole renderer. Separate invocations make the cross-spec order explicit despite Playwright's normal parallel setting.                                                       | PASS                                 |

## HCI acceptance ledger

| Student step   | Need                                                          | Observable acceptance criterion                                                                               | Required live evidence                               |
| -------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Enter Mastery  | Reach the prepared activity without a pointer                 | Tab/Enter uses the exact current Mastery href; route focus returns to main                                    | Focused link and focused `#main-content`             |
| Fresh practice | Start retry practice from visible server state when needed    | A direct visible radio proceeds; otherwise actual `Start another practice run` is focused and Space-activated | Visible radio, or focused button and new run surface |
| First try      | Select and submit without a shortcut                          | First rendered radio is focused/checked with Space; Submit is focused/activated with Space                    | Checked radio, ready status, Feedback heading        |
| Retry          | Recover using visible system state rather than hidden grading | Continue is focused/activated with Space; only visible radios may satisfy the bounded retry poll              | Visible radios after first Continue                  |
| Second try     | Complete with the same accessible controls                    | Second rendered radio uses the same Tab/Space/submit path                                                     | Checked radio, ready status, Feedback heading        |
| Completion     | Know the next useful action                                   | Actual fresh-practice button is visible after second Continue                                                 | `Start another practice run`                         |

## Offline validation

- PASS: focused Playwright list/config run - 5 passed, 2 honest live-only skips.
- PASS: `npx tsc --noEmit -p tsconfig.lint.json`.
- PASS: TypeScript ESLint and Prettier on J1/J2, shared keyboard helper, report
  renderer, and report child; Python compile and Pyflakes on the fixed runner.
- PASS: `node --import tsx --test tests/test_visible_outcome_report.mjs` - 12 passed.
- PASS: `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py tests/test_markdown_links.py tests/test_ascii_compliance.py tests/test_source_file_line_limit.py -q` - 1800 passed.
- PASS: `git diff --check`.

## Required live evidence

From a clean selected Compose project, run:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

Before accepting WP-W2, independently verify:

- The fixed runner invokes the real IPv4 gateway with mock preview disabled and
  `PLAYWRIGHT_NO_COPY_PROMPT=1`.
- J1 completes first and J2 then begins in a fresh browser path, preserving the
  exact fixed serial runner order and private state handoff.
- J2 visibly Tabs to and focuses the credential input, local sign-in submit,
  exact Mastery link, Start, completed-summary fresh-practice button, first
  radio, Submit, first Continue, second radio, second Submit, and second
  Continue; only native Enter or Space activates each control.
- After Start/resume, the bounded structural observation reaches exactly one of:
  a visible inline error (which fails), visible radios (which proceed directly),
  or actual visible `Start another practice run` (which is Tabbed to, focused,
  and Space-activated before radios). It must not infer a retry path from a
  stale/removed response control or asynchronous heading wording.
- First selection produces only the observed Feedback heading. After Continue,
  the same 15-second structural poll must reach visible radios (`run`); a
  visible inline error or fresh-practice button fails. No body text,
  correctness, score, or answer material is inspected or retained.
- Second selection produces a visible `Start another practice run` button after
  Continue, rather than relying on summary heading timing.
- The private report directory is mode 0700 and report file mode 0600; it holds
  exactly ordered J1 and J2 PASS rows, five arrangement records, and only public
  IDs, fixed milestone codes, timing, and empty diagnostics.
- No trace, screenshot, video, browser error context, private temporary state
  or private sibling artifact directory, selected Podman container, credential,
  cookie, feedback body, source, or answer-bearing material remains after
  normal runner cleanup.

## Failure and repair re-review

Two real-stack attempts exposed an honest stale-run problem: J2's original
Start/resume path reached the completed summary from existing student state, so
the test correctly could not find a new response control. The repair does not
clear state, inject a run, or infer an outcome. It requires the visible completed
summary and then uses the ordinary keyboard-only `Start another practice run`
control to obtain the server-owned fresh variation before beginning the first
and second visible-radio sequence.

The repair also moves Playwright matcher artifacts to a validated private sibling
of the runner-owned temporary state rather than `test-results`; the parent and
state file retain private mode checks, and the runner deletes that exact state
root after the fixed report is rendered. Offline tests now cover unsafe state
replacement and artifact-directory derivation. The source remains accepted to
live under the checklist above.

## Third-live repair re-review

Two further live attempts showed that the original summary-heading checks could
race the asynchronous summary request. The latest repair uses the real visible
`Start another practice run` control as both intermediate and final completion
evidence. The intermediate control is reached by Tab, asserted focused, and
Space-activated; the final control is required visibly after the second Continue.
This strengthens the keyboard evidence without reading a private state or
reconstructing a grade. Source guards and focused offline gates pass again.

## Final post-start repair re-review

The final repair replaces timing-sensitive summary assumptions with the pure
`classifyPostStartSurface` helper. Its only inputs are rendered counts for
visible `.inline-error`, visible radios, and the visible fresh-practice button;
it returns `error`, `run`, `fresh-practice`, or `pending`. A bounded Playwright
poll waits only until that structural classifier is no longer pending. J2 fails
generically on `error`, proceeds on `run`, and uses Tab, focus assertion, and
Space only on `fresh-practice` before requiring radios. It reads no heading
wording, response body, feedback body, score, correctness, answer, source, or
private state in this decision.

The retained live diagnosis was two visible radios, zero inline errors, and no
fresh-practice button; that is correctly the `run` branch. The focused source
guards, TypeScript check, lint/format checks, report test (12 passing), and
diff check passed. This is a source-only re-review: no new live run was made.

## Delayed retry-controls repair re-review

The first Continue no longer assumes that retry controls are already rendered.
It invokes the same pure classifier with an explicit 15-second bounded poll.
Only its `run` result permits the second radio sequence. `error` and
`fresh-practice` both fail through the generic rendered-retry-controls error;
`pending` times out. This uses no route marker, heading wording, private
browser/server state, feedback body, or grading data. The visible retry proof
is consequently the rendered radio control itself, not an inference about why
it appeared. Focused TypeScript, ESLint, Prettier, and the 12-test report and
source guard suite passed; no live run was made.

## Limits

This verdict accepts the current J2 retry vertical only. It does not accept M5
as a whole, J3-J5/J8, canonical onboarding, all-family coverage, or release
completion.

## Latest independent live acceptance

On 2026-08-11, the exact fresh-build command completed without manual
intervention:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
```

The runner finished its own cleanup before inspection. Its private report
directory was mode 0700 and `ui_walkthrough_seed_42.json` was mode 0600. The
schema-v1 PASS report has `masterSeed: 42`, `stage: complete`, and five public
arrangement records. Its J2 PASS row has the same public course
`c7a11b65-a521-5b98-9bcd-0217e3e98d1c` and Mastery assignment
`019ff026-36d6-77f2-848d-417d9003cbf3` as J1, records exactly
`visible_start`, `visible_response`, `visible_submit`, `visible_feedback`,
`visible_retry`, and `visible_completion`, and has empty diagnostics.
`.last-run.json` records passed with no failed tests. Only the private report
and `.last-run.json` remained in `test-results`; no trace, screenshot, video,
error-context, credential, feedback body, answer material, or other artifact
remained. No runner private-state directory remained under `/private/tmp`, and
`podman ps --all --quiet` was empty.

**Final verdict: ACCEPTED.** This live run covers the current J2 visible
retry-until-correct path. It does not accept M5 as a whole, J3-J5/J8, canonical
onboarding, all-family coverage, or release completion.
