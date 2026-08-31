# Student keyboard accessibility audit

> **Historical evidence notice - superseded for current browser acceptance.** This dated audit
> records the focused fixture evidence available when it was written. Current browser product
> behavior uses the production `dist/` disposable real-stack path defined in
> [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md); this audit's visual fixtures do not establish
> canonical screenshot provenance before V1.

Status: implementation and focused acceptance complete for the evidence scope recorded below. The
live PLE-owned WebWork keyboard path passed on 2026-08-10; representative screen-reader and real
third-party-provider evaluation remains pending.

This is a focused no-mouse audit of the student browser interface, not a claim of complete WCAG
conformance. The reviewer used a cognitive walkthrough over a built, mock-backed student journey,
mounted dynamic production-component fixtures, the live PLE-owned WebWork path, an automated axe
scan of the student question and feedback states, and source inspection where a browser scenario was
not present. No student participants or screen-reader users were recruited for this pass. The inside
of a third-party external-tool frame remains that provider's responsibility.

The durable required behavior is now separated from this dated evidence in
`docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`. New Question Formats must satisfy that contract as part
of their own acceptance package.

The acceptance goal is direct: a student can open a course, open an assignment, begin or resume a
run, answer every currently implemented Question Format, submit, read feedback, continue, review a
summary, recover from an error, and return without a mouse. The completed evidence demonstrates the
full route only for the built mock journey below; the remaining Question Formats have the fixture or
source-inspection coverage named in their rows.

## Evidence layers

The simulator separates the keyboard contract into two independently failing lanes:

| Evidence layer           | Keys and behavior                                                                                   | Failure classification                       |
| ------------------------ | --------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| Primary platform journey | Tab and Shift+Tab move focus; Space selects choices and activates buttons; Enter activates links    | Browser/platform accessibility regression    |
| Widget extensions        | Enter-to-submit from a response input, Arrows, digits 1-9, and Escape operate their bounded widgets | PLE shortcut or composite-control regression |

The primary journey never uses an Arrow key, digit shortcut, response-input Enter, or Escape. A
student can always reach the visible Submit answer action. Extension scenarios use production
response components, but their convenience does not become a prerequisite for answering.

## Task model

| Step             | Student goal               | Primary platform path            | Completion evidence                                   |
| ---------------- | -------------------------- | -------------------------------- | ----------------------------------------------------- |
| Enter content    | Bypass repeated navigation | Tab to skip link, Enter          | Main content receives focus                           |
| Choose work      | Open course and assignment | Tab to each native link, Enter   | Each route loads and main content receives focus      |
| Start practice   | Begin or resume a run      | Tab to the button, Space         | Current question and response control appear          |
| Answer           | Enter a response           | Tab to the choice, Space         | Format status announces ready or explains the problem |
| Submit           | Record the response        | Tab to Submit answer, Space      | Feedback or recovery state appears                    |
| Continue         | Move to the next task      | Tab to the visible action, Space | Next question or run-complete summary appears         |
| Review and leave | Inspect results or return  | Tab to the visible action, Space | Assignment overview or new practice run opens         |

The built mock journey through a single-choice response is covered by `a student completes the
primary platform-key course-to-answer path without a pointer` in
`tests/playwright/frontend_contract.spec.ts`. It also proves Shift+Tab can reverse from Submit answer
to the selected response and return. It is not evidence that every Question Format has completed the
entire route end to end.

## Question Format keyboard contract

| Question Format | Primary platform path                                                               | Separately tested or documented extensions                                     | Accepted evidence                                                                         |
| --------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Single choice   | Tab enters the native radio group; Space selects; Tab and Space submit explicitly   | Native radio Arrows select; digits 1-9 select a visible ordinal; Enter submits | Built mock platform journey, mounted extension scenarios, and live PLE-owned WebWork path |
| Multiple answer | Tab moves through checkboxes; Space toggles; Tab and Space submit explicitly        | Arrows move focus without changing selection; digits toggle; Enter submits     | Mounted production-component platform and extension fixtures                              |
| Numeric         | Tab reaches input and Submit answer; typing enters data; Space activates submission | Browser number adjustment Arrows and ready-input Enter-to-submit               | Source inspection plus shared response-controller tests                                   |
| Short text      | Tab reaches textarea and Submit answer; Space activates submission                  | Escape returns; Enter remains ordinary multiline text entry                    | Native textarea/button semantics and source inspection                                    |
| Ordering        | Tab reaches visible move buttons; Space moves and submits                           | Up/Down Arrow moves the item and announces its new position                    | Mounted production-component platform and extension fixtures                              |
| File upload     | Unavailable state exposes no fake field; Tab and Space activate Return              | Escape returns when it is safe                                                 | Source inspection and native button semantics                                             |
| External tool   | Tab reaches broker, submit, return, and retry buttons; Space activates them         | Escape returns; native frame internals retain their own contract               | Mounted broker fixture plus source inspection; real provider internals remain unevaluated |

## Findings and corrections

| Severity | Baseline finding                                                                                                                                         | Correction                                                                                                                                 | Status                          |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- |
| High     | Every response on a run-summary page had a focusable `Stay on summary` button whose action did nothing. Thirty responses meant thirty useless Tab stops. | Read-only feedback omits the advance action. The completed-run surface now has one Back to assignment action outside the response list.    | Fixed and tested                |
| High     | Repeated feedback panels reused `feedback-panel-heading`, so several regions referenced the same DOM ID.                                                 | Each panel now creates a unique heading ID and uses it for its own `aria-labelledby`.                                                      | Fixed and tested with 30 panels |
| Medium   | Multiple-answer checkboxes required Tab between every option and offered no arrow navigation.                                                            | Arrow keys move focus among checkboxes without silently toggling them; Space retains standard selection behavior.                          | Fixed and tested                |
| Medium   | Ordering had keyboard-operable buttons but did not support the owner's requested arrow-key workflow or announce a move.                                  | Visible controls retain their Tab-and-Space path; focused controls also accept Up/Down Arrow, retain focus, and announce the new position. | Fixed and tested                |
| Medium   | The built primary journey mixed native platform operation with Arrow selection and response-input Enter, so one failure could not identify its owner.    | The primary journey now uses Tab, Shift+Tab, Space, explicit submission, and native links; each widget extension has an isolated scenario. | Fixed and tested                |

The summary cleanup follows WCAG 2.2 focus-order guidance, which recommends avoiding focusable
elements that cannot be operated or actioned. The radio behavior follows the WAI-ARIA Authoring
Practices convention for Tab into a group and arrow movement inside it
([WCAG 2.2 Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html),
[WAI-ARIA radio-group pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/)).

## Guideline ledger

| Task need                            | Guideline                          | Acceptance criterion                                                                                                                | Evidence                                                                   | Status                            |
| ------------------------------------ | ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------- |
| Operate everything without a pointer | WCAG 2.2 SC 2.1.1 Keyboard         | The complete primary journey works through visible platform controls without requiring a widget shortcut                            | Built mock platform journey, live WebWork, and source inspection           | Pass for recorded evidence scope  |
| Use efficient optional controls      | WCAG 2.2 SC 2.1.1 Keyboard         | Arrow, digit, Enter-to-submit, and Escape behavior is scoped and fails independently of the primary path                            | Mounted production-component extension scenarios                           | Pass for recorded evidence scope  |
| Leave widgets and embedded work      | WCAG 2.2 SC 2.1.2 No Keyboard Trap | Tab continues past controls; Escape leaves a question response control; provider frame uses ordinary iframe navigation              | Built mock journey, widget tests, live WebWork, and mounted broker fixture | Pass for PLE-owned recorded scope |
| Encounter controls in task order     | WCAG 2.2 SC 2.4.3 Focus Order      | Tab follows reading/task order and skips no-op history controls                                                                     | Built mock journey and 30-panel summary assertion                          | Pass for recorded evidence scope  |
| See the focused control              | WCAG 2.2 SC 2.4.7 Focus Visible    | Every focusable PLE control uses the global visible focus treatment; response cards expose the indicator around the complete target | `src/style.css`, existing palette audit, browser focus assertions          | Pass for audited controls         |
| Understand composite controls        | WAI-ARIA APG keyboard conventions  | Radio arrows select; checkbox arrows move focus; order arrows move the item; visible buttons remain available                       | Built mock radio journey and mounted dynamic production-component fixtures | Pass for recorded evidence scope  |

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
| User control and freedom        |      3 |     4 | Platform keys complete the primary path; shortcuts remain optional                 |
| Consistency and standards       |      3 |     4 | Native radio/button behavior is preserved and APG arrow conventions are explicit   |
| Error prevention                |      4 |     4 | Disabled/pending controls and local format validation remain intact                |
| Recognition over recall         |      3 |     4 | Instructions state the platform action before optional widget shortcuts            |
| Flexibility and efficiency      |      2 |     4 | Arrow movement avoids repeated Tab presses while visible controls remain available |
| Aesthetic and minimalist design |      2 |     4 | Per-response no-op summary actions were removed                                    |
| Error recognition and recovery  |      4 |     4 | Existing retry and preserved-response paths remain keyboard-operable               |
| Help and documentation          |      3 |     4 | The response-widget and Question Type contract evidence is now recorded here       |

## Validation

The focused acceptance gate is:

```bash
node --import tsx --test tests/test_question_response_controls.mjs
./run_playwright_tests.sh --build \
  tests/playwright/frontend_contract.spec.ts \
  tests/playwright/feedback_panel.spec.ts \
  tests/playwright/run_summary_route.spec.ts \
  tests/playwright/student_keyboard_accessibility.spec.ts \
  tests/playwright/external_tool_response.spec.ts
```

The focused run rebuilds the shipped artifacts and checks the response controller, the primary
platform journey, independently named widget-extension scenarios, and the student question/feedback
axe surface. This confirms the built mock journey and mounted fixture coverage listed above; it does
not promote source-inspection rows into full-route evidence. The scenarios assert durable student
outcomes rather than exact Tab counts or DOM layout, so they qualify as permanent behavior tests
under `docs/PYTEST_STYLE.md` rather than one-time implementation probes.

The current production-browser gate owns the WebWork interaction path through
`run_playwright_tests.sh --build`. It verifies keyboard-only selection and submission through the
PLE-owned radio projection while the browser contacts PLE only and receives no upstream source,
credential, hidden field, or answer mapping.

## Remaining human evaluation

Before calling the fall pilot fully accessible, run at least one VoiceOver plus Safari session and
one NVDA plus Firefox or Chromium session with representative students. Include the institutional
login provider and any real external-tool provider because neither production identity nor provider
internals exist in this local PLE acceptance path.
