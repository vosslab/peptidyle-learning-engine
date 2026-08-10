# Student keyboard accessibility audit

Status: implementation and focused acceptance complete for the evidence scope recorded below on
2026-08-09. Live WebWork browser acceptance remains pending.

This is a focused no-mouse audit of the student browser interface, not a claim of complete WCAG
conformance. The reviewer used a cognitive walkthrough over a built, mock-backed student journey,
mounted dynamic production-component fixtures, and source inspection where a browser scenario was
not present. No student participants or screen-reader users were recruited for this pass, an
automated axe scan was not added, and live WebWork browser acceptance has not yet been run. The
inside of a third-party external-tool frame remains that provider's responsibility.

The acceptance goal is direct: a student can open a course, open an assignment, begin or resume a
run, answer every currently implemented response family, submit, read feedback, continue, review a
summary, recover from an error, and return without a mouse. The completed evidence demonstrates the
full route only for the built mock journey below; the remaining response families have the fixture or
source-inspection coverage named in their rows.

## Task model

| Step             | Student goal               | Keyboard path                           | Completion evidence                                   |
| ---------------- | -------------------------- | --------------------------------------- | ----------------------------------------------------- |
| Enter content    | Bypass repeated navigation | Tab to skip link, Enter                 | Main content receives focus                           |
| Choose work      | Open course and assignment | Tab, Enter                              | Each route loads and main content receives focus      |
| Start practice   | Begin or resume a run      | Tab, Enter                              | Current question and response control appear          |
| Answer           | Enter a response           | Family-specific keys below              | Format status announces ready or explains the problem |
| Submit           | Record the response        | Enter on entry control or submit button | Feedback or recovery state appears                    |
| Continue         | Move to the next task      | Tab, Enter                              | Next question or run-complete summary appears         |
| Review and leave | Inspect results or return  | Tab, Enter                              | Assignment overview or new practice run opens         |

The built mock journey through a single-choice response is covered by `a student completes the
primary course-to-answer path without a pointer` in `tests/playwright/frontend_contract.spec.ts`.
It is not evidence that every response family has completed the entire route end to end.

## Response-family keyboard contract

| Response family | Required behavior                                                                                                                                | Accepted evidence                                                                        |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| Single choice   | Tab enters the native radio group; arrows move and select; Space selects; number keys are an additional shortcut; Enter submits a ready response | Built mock course-to-answer journey uses Arrow Down and Enter                            |
| Multiple answer | Tab enters the checkbox group; arrow keys move focus without changing selection; Space toggles; Enter submits a ready response                   | Mounted dynamic production-component fixture covers Arrow Right, Space, and Enter        |
| Numeric         | Tab reaches the native number input; browser arrow keys adjust it; Enter submits when locally valid                                              | Source inspection plus shared response-controller tests                                  |
| Short text      | Tab reaches the textarea; Enter remains text entry; Tab reaches Submit answer and Enter activates it                                             | Native textarea/button semantics and source inspection                                   |
| Ordering        | Tab reaches visible Up/Down controls; Enter activates them; Up/Down Arrow moves the focused item; a polite status announces the new position     | Mounted dynamic production-component fixture moves one item twice and submits with Enter |
| File upload     | The intentionally unavailable state exposes no fake upload field; Tab and Enter can activate Return to assignment                                | Source inspection and native button semantics                                            |
| External tool   | Tab and Enter open the same-origin broker; the frame has a title; validated readiness focuses Submit answer; Enter submits; Escape returns       | Mounted broker fixture plus source inspection; live WebWork browser acceptance pending   |

## Findings and corrections

| Severity | Baseline finding                                                                                                                                         | Correction                                                                                                                                     | Status                          |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| High     | Every response on a run-summary page had a focusable `Stay on summary` button whose action did nothing. Thirty responses meant thirty useless Tab stops. | Read-only feedback omits the advance action. The completed-run surface now has one Back to assignment action outside the response list.        | Fixed and tested                |
| High     | Repeated feedback panels reused `feedback-panel-heading`, so several regions referenced the same DOM ID.                                                 | Each panel now creates a unique heading ID and uses it for its own `aria-labelledby`.                                                          | Fixed and tested with 30 panels |
| Medium   | Multiple-answer checkboxes required Tab between every option and offered no arrow navigation.                                                            | Arrow keys move focus among checkboxes without silently toggling them; Space retains standard selection behavior.                              | Fixed and tested                |
| Medium   | Ordering had keyboard-operable buttons but did not support the owner's requested arrow-key workflow or announce a move.                                  | Focused move controls accept Up/Down Arrow, retain focus on the moved item, and announce its new position. Enter remains the visible fallback. | Fixed and tested                |

The summary cleanup follows WCAG 2.2 focus-order guidance, which recommends avoiding focusable
elements that cannot be operated or actioned. The radio behavior follows the WAI-ARIA Authoring
Practices convention for Tab into a group and arrow movement inside it
([WCAG 2.2 Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html),
[WAI-ARIA radio-group pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/)).

## Guideline ledger

| Task need                            | Guideline                          | Acceptance criterion                                                                                                                | Evidence                                                                                          | Status                            |
| ------------------------------------ | ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------------------- |
| Operate everything without a pointer | WCAG 2.2 SC 2.1.1 Keyboard         | Every listed student action has a keyboard path without timed keystrokes                                                            | One built mock journey, response-family fixtures, and source inspection as specified above        | Pass for recorded evidence scope  |
| Leave widgets and embedded work      | WCAG 2.2 SC 2.1.2 No Keyboard Trap | Tab continues past controls; Escape leaves a response widget; provider frame uses ordinary iframe navigation                        | Built mock journey, widget tests, mounted broker fixture; live WebWork browser acceptance pending | Pass for PLE-owned recorded scope |
| Encounter controls in task order     | WCAG 2.2 SC 2.4.3 Focus Order      | Tab follows reading/task order and skips no-op history controls                                                                     | Built mock journey and 30-panel summary assertion                                                 | Pass for recorded evidence scope  |
| See the focused control              | WCAG 2.2 SC 2.4.7 Focus Visible    | Every focusable PLE control uses the global visible focus treatment; response cards expose the indicator around the complete target | `src/style.css`, existing palette audit, browser focus assertions                                 | Pass for audited controls         |
| Understand composite controls        | WAI-ARIA APG keyboard conventions  | Radio arrows select; checkbox arrows move focus; order arrows move the item; visible buttons remain available                       | Built mock radio journey and mounted dynamic production-component fixtures                        | Pass for recorded evidence scope  |

WCAG 2.2 requires functionality to be keyboard operable and keyboard focus to be visible
([WCAG 2.2](https://www.w3.org/TR/WCAG22/)). The repository keeps a separate measured color record in
[PALETTE_CONTRAST_AUDIT.md](../PALETTE_CONTRAST_AUDIT.md); this pass did not repeat those color
measurements.

## Heuristic delta

Scores use 0 for a critical problem and 4 for no material issue in this keyboard-focused scope.

| Nielsen heuristic               | Before | After | Evidence for change                                                                |
| ------------------------------- | -----: | ----: | ---------------------------------------------------------------------------------- |
| Visibility of system status     |      3 |     4 | Ordered moves now have a polite position announcement                              |
| Match with the real world       |      4 |     4 | Visible Up/Down labels remain the primary ordering vocabulary                      |
| User control and freedom        |      3 |     4 | Tab, arrows, Enter, Space, and Escape cover the primary path                       |
| Consistency and standards       |      3 |     4 | Native radio/button behavior is preserved and APG arrow conventions are explicit   |
| Error prevention                |      4 |     4 | Disabled/pending controls and local format validation remain intact                |
| Recognition over recall         |      3 |     4 | Ordering instructions name both visible controls and arrow shortcuts               |
| Flexibility and efficiency      |      2 |     4 | Arrow movement avoids repeated Tab presses while visible controls remain available |
| Aesthetic and minimalist design |      2 |     4 | Per-response no-op summary actions were removed                                    |
| Error recognition and recovery  |      4 |     4 | Existing retry and preserved-response paths remain keyboard-operable               |
| Help and documentation          |      3 |     4 | The response-family contract and task evidence are now recorded here               |

## Validation

The focused acceptance gate is:

```bash
node --import tsx --test tests/test_response_widgets.mjs
./run_playwright_tests.sh --build \
  tests/playwright/frontend_contract.spec.ts \
  tests/playwright/feedback_panel.spec.ts \
  tests/playwright/run_summary_route.spec.ts \
  tests/playwright/student_keyboard_accessibility.spec.ts \
  tests/playwright/external_tool_response.spec.ts
```

The first run rebuilt the shipped artifacts and passed the component/unit lane. After correcting a
test assumption about already-restored focus, the final browser lane passed all 20 named scenarios.
This confirms the built mock journey and the mounted fixture coverage listed above; it does not turn
source-inspection rows or the broker fixture into live WebWork end-to-end acceptance. The complete
11-stage repository gate and all 1,699 repository-owned Python tests also passed. A disposable test
index made the new artifacts visible to tracked-file policy checks during the gate.

## Remaining human evaluation

Before calling the fall pilot fully accessible, run live WebWork browser acceptance through the real
broker and provider flow, then run at least one VoiceOver plus Safari session and one NVDA plus
Firefox or Chromium session with representative students. Include the institutional login provider
and any real external-tool provider because neither production identity nor provider internals exist
in this local mock-backed browser acceptance path.
