# WP-W6 policy contrast review

## Verdict

**ACCEPTED.** The integrated repair splits entry behavior by policy
posture. Mastery alone may reach an existing completed summary and use the
focused `Start another practice run` control to enter a fresh run. Exam waits
only for a visible active run, forbidden fresh action/heading, or closed result;
it immediately rejects either Mastery-only affordance, permits an already closed
Exam only with Back available, and otherwise completes one visible response to
its neutral closed summary.

## Scope and method

This independent offline review covered WP-W6/J4, the plan, the no-mouse
contract, the paired browser spec, the production completion UI, and the
isolated public fragment. It uses a keyboard cognitive walkthrough: open exact
Mastery/Exam assignment links, complete visible work through native controls,
and distinguish the available next action without a score or policy-engine
assertion.

## Findings

| Check                         | Evidence                                                                                                                                                                                                                                                                                         | Result                |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- |
| Exact visible routes          | Each separate context signs in through the rendered form, requires one exact visible current course and Mastery/Exam href, uses bounded backward Tab from route-focused main, asserts focus, and activates native Enter.                                                                         | PASS                  |
| Keyboard-only operation       | J4 uses root entry plus bounded Tab/Shift+Tab, native Enter, and Space. It has no pointer action, direct focus, API/route interception, cookie, storage, history, Arrow, digit, or tabindex shortcut.                                                                                            | PASS                  |
| Mastery path                  | It handles existing completed state only through focused/Space-activated `Start another practice run`; each response requires exactly two visible unchecked native radios, with radio two reached only through backward Tab, focus, and Space. Final paired heading/action and Back are visible. | PASS                  |
| Closed Exam path              | One visible response ends at `This run is complete`; final Mastery heading and fresh action have count zero, while Back is visible.                                                                                                                                                              | PASS at final summary |
| Initial Exam contrast         | `startVisibleExamRun()` waits for bounded active/fresh/Mastery/closed visible state, immediately requires fresh action and Mastery heading counts of zero, permits already-closed only with Back, otherwise requires the active run.                                                             | PASS                  |
| Feedback and scoring boundary | The test observes the Feedback heading only. It reads no feedback body, correctness, score, answer, source, or private response.                                                                                                                                                                 | PASS                  |
| Context separation            | Mastery and Exam use separate fresh browser contexts and each signs in through its rendered local form; no state is copied between them.                                                                                                                                                         | PASS                  |
| Public fragment               | The pure fragment permits only three public UUIDs, bounded elapsed time, five fixed visible codes, and empty diagnostics. It has no answer or feedback field.                                                                                                                                    | PASS                  |
| Terminal-state hardening      | A bounded 30-second visible-only classifier treats pending, Feedback, and neutral completion as transient; it terminates on paired Mastery action/heading, exact closed Exam, mismatched Mastery paired controls, or inline error. Policy-specific guards fail observed nonexpected terminals.   | PASS                  |
| Evidence ordering             | The J4 fragment is constructed only after both policy assertions pass and is appended only after both browser contexts close in `finally`.                                                                                                                                                       | PASS                  |
| Scope isolation               | J4 does not change the shared runner or shared J1/J2 visible-report contract.                                                                                                                                                                                                                    | PASS                  |

## Required live checklist

From a clean selected Compose project, run the isolated J4 live invocation.
Verify:

- Mastery can visibly reach an existing completed summary, then Tabs/focuses and
  Space-activates `Start another practice run` before the first radio appears.
- Mastery performs the first/second visible-radio transitions through Feedback
  headings only, then exposes the final fresh-practice action and Back control.
- Exam Start/resume reaches a visible run directly and fails closed if either
  `Start another practice run` or `Keep practicing with a fresh variation`
  appears before its response.
- Exam completes one visible response with a Feedback heading, then shows only
  `This run is complete`, has no fresh action, and retains Back to assignment.
- Every operated control is reached by Tab, asserted focused, and activated with
  native Enter or Space; no pointer, direct focus, API, storage, cookie, or
  history shortcut is introduced.
- The J4 fragment remains bounded to public IDs, fixed visible codes, elapsed
  time, and empty diagnostics; no feedback body, correctness, score, answer, or
  artifact is retained. This isolated package still must not claim shared report
  or runner integration.

## Offline validation

- PASS: J4 Playwright listing found one test; its normal offline invocation
  skipped honestly without explicit live configuration.
- PASS: strict TypeScript, targeted ESLint, and Prettier.
- PASS: `node --import tsx --test tests/test_student_completion_policy_evidence.mjs` -
  4 passed.
- PASS: `source source_me.sh && python3 -m pytest tests/test_ascii_compliance.py
tests/test_markdown_links.py tests/test_source_file_line_limit.py -q` - 1774
  passed.
- PASS: `git diff --check`.

## Repair re-review

The initial blocker was real: a generic entry helper normalized the forbidden
Exam fresh-practice state. The repaired source has separate Mastery and Exam
helpers. The Exam helper observes a bounded set of visible states, fails closed
for either forbidden Mastery affordance before a response is submitted, and does
not use private policy data to choose its branch. If the server already reports
the neutral closed Exam result, the browser still requires Back; otherwise it
executes one ordinary visible response, Feedback heading, and Continue sequence.

Focused source gates now pass: J4 Playwright listing, strict TypeScript,
targeted ESLint/Prettier, the current four public-fragment/classifier/source
guard Node tests, focused Python hygiene checks, ASCII, and `git diff --check`.
This accepts the source to live under the checklist above; no live J4 claim has
been made.

## Integrated hardening re-review

The current source is **ACCEPTED TO LIVE**. The shared sign-in helper uses only
exact visible current hrefs, bounded backward Tab, focused assertions, and
native Enter. Mastery and Exam retain distinct entry helpers. Every operated
response requires exactly two visible unchecked native radios; radio two uses
only bounded backward Tab, focus, and Space. No Arrow/digit/tabindex extension
is used.

After Continue, `j4_terminal_surface` classifies only rendered headings,
fresh-practice control, and inline-error presence. Pending, Feedback, and
neutral completion deliberately remain bounded transients; error and mismatched
Mastery heading/action are terminal failures. Mastery requires the paired
heading/action then visibly requires Back; Exam accepts only the exact closed
heading, requires absent Mastery/fresh controls, and visibly requires Back.
The fragment is created only after both paths pass and appended only after both
contexts close. The source reads no feedback body, correctness, score, answer,
policy engine, browser private state, or transport response.

Focused J4 listing, strict TypeScript, targeted ESLint/Prettier, the four Node
fragment/classifier/source-guard tests, markdown/ASCII checks (958 passed), and
`git diff --check` all pass. No live run was made.

## Limits

This review accepts J4's current paired live contrast. It does not by itself
accept canonical onboarding, all-family coverage, or release completion.

## Retained-data M5 live acceptance

On 2026-08-11, the full retained-data command completed without a volume reset,
fresh project, direct navigation, or manual cleanup:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
```

The private report directory and report file were mode 0700 and 0600. Its
schema-v1 PASS payload records J4 PASS with empty diagnostics, the public course
ID `c7a11b65-a521-5b98-9bcd-0217e3e98d1c`, Mastery assignment ID
`019ff0a3-46e4-7b12-9630-a0c9f8dda51d`, Exam assignment ID
`019ff0a3-46ed-73b2-8022-7fb2a6153442`, and exactly
`visible_mastery_completion`, `visible_mastery_fresh_practice`,
`visible_exam_completion`, `visible_exam_closed`, and `visible_back_action`.
The same report records PASS empty-diagnostic J1, J2, J3, J5, and J8 rows.
`.last-run.json` has passed with no failed tests; only it and the private report
remained in `test-results`. No private temporary root remained. A short normal
Podman cleanup tail cleared without intervention; the final read-only
`podman ps --all --quiet` check was empty.

**Final verdict: ACCEPTED.**
