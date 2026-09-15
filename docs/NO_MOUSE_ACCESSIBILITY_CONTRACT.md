# PLE no-mouse accessibility contract

## Status and authority

This is the durable interaction contract for every PLE-owned student browser surface. It applies to
the Course, Coursework, Assessment Attempt, response, Student Feedback, summary, continued practice,
auto-submission,
asset, and PLE-owned iMathAS Question Backend boundary. For a backend-owned document, PLE owns its
generic Student UI framing, keyboard reachability, bridge and lifecycle, and baseline theme; the
Question Backend owns the document and its control semantics. `HUMAN_GUIDANCE.md` is the owner
decision: every PLE-owned student action must be possible with the keyboard alone. The primary
path uses the browser platform contract: Tab
and Shift+Tab move focus, and Space selects choices or activates focused buttons. Arrow keys,
digits 1-9, Enter-to-activate from a response input, and Escape are documented Question Response Control extensions that
may improve efficiency but are never required to complete the task.

This document defines required behavior. The dated implementation evidence, findings, limitations,
and human-evaluation backlog remain in
[STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md). Passing an
automated scanner is evidence for semantics, not proof of this contract or complete WCAG
conformance.

## Student and context

The primary student may be working remotely on a laptop, may have limited dexterity, may use a
keyboard because a pointer is unavailable or tiring, or may combine the keyboard with a screen
reader. The critical task is to open Coursework, understand each Question, save responses, submit
the Assessment Attempt, read the authorized result, handle a failure, and continue practice without touching a
mouse or trackpad. Saving changes the working response; the critical completion
action submits the whole Assessment Attempt and finalizes all saved responses.

Failure has educational consequences: an inaccessible control can prevent a student from answering,
can consume limited time, or can make a saved response appear lost. No student action may therefore
depend on hover, drag, pointer coordinates, a specific timed key sequence, or discovering an
undocumented shortcut.

## Platform path and Question Response Control extensions

PLE keeps two keyboard evidence layers so a failure names the right owner:

1. **Primary platform path.** The complete Course-to-Assessment journey works with Tab and Shift+Tab
   for focus, Space for native choice selection and button activation, and Enter for native link
   activation. Response persistence is available through the visible Save response button, and
   whole-Attempt submission is available through the visible Submit Assessment action. This path
   does not require an arrow key, digit shortcut, response-input Enter, or Escape.
2. **Question Response Control extensions.** Enter-to-activate from an eligible ready response input, composite-control
   arrows, visible-choice digits 1-9, and Escape provide efficient Question Response Control behavior. Each
   extension is scoped to its Question Response Control, documented beside the control when discoverability matters,
   and tested separately from the primary journey.

An extension never replaces the visible control, changes the saved domain action, overrides text
editing or input-method composition, or becomes the only recovery path. A primary-path failure is a
platform keyboard accessibility regression. An extension failure is a PLE shortcut regression.

## Core interaction rules

1. **Every pointer action has a keyboard path.** Links, buttons, fields, choices, retries, dialogs,
   downloads, continued-practice actions, and return actions are reachable and operable without a
   pointer.
2. **Use native HTML first.** Native links, buttons, inputs, fieldsets, legends, labels, selects, and
   text controls own their standard keyboard behavior. ARIA supplements semantics; it does not
   replace a native control without a demonstrated need.
3. **Tab follows the learning task.** Tab and Shift+Tab enter, leave, and traverse logical controls
   in reading order. Native radio groups retain one ordinary Tab entry point.
4. **Space is the primary response action.** Space selects a focused radio, toggles a focused
   checkbox, and activates a focused button. PLE does not override it with a hidden global shortcut.
5. **Enter preserves native controls and offers one bounded extension.** Enter activates focused
   links and buttons. PLE additionally permits Enter to activate the visible response action from an eligible, locally ready
   single-line or choice response input. Enter inside a multiline text area inserts text.
6. **Arrows, digits, and Escape are scoped extensions.** Arrows may operate a response composite,
   digits 1-9 may select a visible choice ordinal while a choice input has focus, and Escape may
   return from a Question Response Control when no work is discarded. Native dialogs and input-method editors retain
   their own key handling first.
7. **Focus is always visible and never trapped.** A student can see the focused target, move away
   with ordinary keyboard commands, and return without losing the current response.
8. **Dynamic changes are announced selectively.** Validation, ordering moves, save and finalization
   state, Student Feedback Release, errors, and auto-submission outcomes use concise status or alert semantics. PLE does
   not announce every keystroke or repeat the whole question.
9. **Keyboard and pointer produce the same domain action.** The input method never changes the saved
   response, seed, grading backend, points, disclosure policy, or server-owned result.
10. **Server authority is unchanged.** Keyboard helpers perform browser-side response entry and
    format validation only. Answers, grading rules, partial credit, and correctness remain
    server-only.

These rules implement the intent of [WCAG 2.2 Keyboard and No Keyboard
Trap](https://www.w3.org/TR/WCAG22/#keyboard-accessible), focus-order and focus-visible requirements,
and the [WAI-ARIA keyboard-interface guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/).
PLE uses the [radio-group keyboard pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/) where
native radio controls supply the behavior.

## Whole student journey

| Step                  | Required keyboard behavior                                                                                                                       | Completion evidence                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
| Enter the application | The first useful Tab stop exposes Skip to learning content; Enter moves focus to main content                                                    | Main content is focused and named                                              |
| Choose a course       | Tab reaches the course link; Enter opens it                                                                                                      | Route content loads and main receives focus                                    |
| Choose an Assessment  | Tab reaches the Assessment under Coursework; Enter opens it                                                                                      | Assessment title, Type, and action are available                               |
| Begin or resume       | Tab reaches Start or continue practice; Space activates it                                                                                       | Question heading and Question Response Control appear                          |
| Read the question     | Reading order follows prompt, assets, instructions, response, status, then save                                                                  | No interactive content is skipped or inserted out of order                     |
| Answer                | A PLE-native Question Response Control follows its Question-Type-specific contract; a backend-owned document follows its backend-owned semantics | The selected or entered response is visibly represented                        |
| Validate              | Format state is announced without grading or disclosing an answer                                                                                | Ready or actionable validation text is available                               |
| Save                  | Tab reaches Save response; Space persists the current position                                                                                   | Pending state prevents a duplicate save                                        |
| Finish                | Tab reaches Submit Assessment; Space submits the whole Assessment Attempt                                                                         | Submitted state prevents duplicate submission                                  |
| Read Student Feedback | Authorized Student Feedback receives a heading and sensible focus; unreleased Student Feedback is not inferred                                   | Student can read result and next action                                        |
| Continue              | Tab and Space operate Continue, Back to Coursework, or Start another Assessment Attempt                                                          | The next Question, Assessment, or allowed new Attempt opens                    |
| Reconnect or expire   | Before expiry, reauthentication resumes the same Attempt; at expiry, the server submits the whole Attempt and finalizes saved responses         | The clock is unchanged and no grading action is offered                        |

Route changes focus the main content rather than leaving focus on a removed navigation element.
Student Feedback may focus its heading and later its primary advance control only when the student has not
moved focus elsewhere. A delayed focus helper never steals focus back from the student.

## PLE-native Question Type contract

### Single choice

- Tab enters the native radio group at the checked option or the browser's initial native option.
- Space selects the focused option; Tab then reaches the explicit Save response button.
- As separately tested extensions, native radio arrows move focus and selection, number keys 1-9
  may select a visible ordinal while a choice has focus, and Enter may activate Save response for a
  locally ready response from that input.
- Choice labels are readable text; visual letters such as A or B are not the response identity.

### Multiple answer

- Tab and Shift+Tab move through the checkbox set; Space toggles only the focused choice; the
  explicit Save response button completes the primary path.
- As separately tested extensions, arrow keys move focus among choices without changing selection,
  digits 1-9 toggle a visible choice while a checkbox has focus, and Enter activates Save response
  only when the selection-count rule is satisfied.
- A visible label and programmatic checked state identify every choice.

### Fill in the blank and multi-blank

- Tab reaches blanks in prompt reading order and Shift+Tab reverses that order.
- Every blank has a stable visible or programmatic label that identifies its context; placeholder
  text is not the only label.
- Tab reaches the explicit Save response button and Space activates it. Enter-to-activate from a
  single-line blank is an extension only when the entire response is ready; multi-blank forms do not
  let Enter in one blank bypass unfinished fields.
- Validation identifies the blank requiring attention without moving focus unexpectedly.

### Numerical entry

- Tab reaches a native numeric or text entry with an appropriate input mode.
- Typing is always available; browser increment/decrement arrows may remain available but are not
  required for scientific notation or high-precision values.
- Units, tolerance instructions, and required format are associated with the control.
- Tab reaches the explicit Save response button and Space activates it. Enter-to-activate is an
  extension for a finite, locally valid response; an empty field never becomes zero.

### Matching

- Each left-side item has labeled match choices in reading order. The primary path exposes each
  available pairing through focusable native controls that Tab can reach and Space can select.
- A native select or tested composite may additionally provide its documented arrow behavior, but
  it does not remove the Tab-and-Space path.
- The current pairings are available as text and programmatic values, not color or line geometry
  alone.
- Dragging lines or cards may be offered as an additional pointer interaction, never as the only
  method.

### Ordering

- Tab reaches visible Move earlier and Move later controls; Space activates them.
- As an extension, Up and Down Arrow on a focused move control move the item in the corresponding
  direction.
- Focus follows the moved item, and a polite status announces its new position.
- The response is not communicated by visual position alone; each item exposes its current ordinal.
- Drag and drop may be added for pointer users without replacing the button and arrow path.

### Hotspot

- A hotspot question must not require a pointer-only click or a path-dependent gesture.
- The image has a text alternative and explicit instructions for keyboard use.
- The current PLE Question JSON HOTSPOT contract requires named public Hotspot Regions. Its primary control
  is an equivalent labeled radio or checkbox list that Tab reaches and Space selects. Region labels
  describe the diagram without revealing correctness.
- A later pointer overlay or coordinate cursor may be added as an extension, but it must preserve the
  labeled list and may not make the image the only operable response surface.
- The current selection is programmatically available without exposing the private correct-region
  set.
- If a pedagogically equivalent keyboard interaction cannot be provided, the item is not eligible
  for a graded PLE Assessment.

### Short text and iMathAS Question Backend controls

- A multiline short-text field retains ordinary text-entry keys; Tab reaches Save response and Space
  activates it.
- Students have no file-upload control. A Question Backend does not widen that product boundary.
- PLE-owned iMathAS Question Backend launch, readiness, response capture, return, and error recovery expose native
  buttons reachable with Tab and activated with Space. The iframe has a title and cannot trap focus.
  A third-party tool's internal interface is separately evaluated; PLE does not call the whole task
  accessible merely because its launch button is accessible.

### Backend-owned documents

- A backend-owned document supplies its own HTML, controls, labels, keyboard behavior, and response
  interpretation. PLE does not infer its educational Question Type from those controls or convert
  them into a PLE-native Question Response Control.
- PLE provides a titled, keyboard-reachable document frame, ordinary focus movement into and out of
  that frame, its generic baseline theme, response saving, and whole-Assessment submission.
- The document bridge captures the complete backend form payload as a bounded opaque canonical ordered-pair
  Student Response. It preserves repeated names, form order, and legitimate backend hidden fields;
  it never carries renderer credentials to the document.
- The Question Backend remains responsible for accessibility of its document and controls. PLE
  connected evidence establishes the framing and lifecycle boundary, not accessibility breadth for
  untested backend content.

## Timing, continued practice, and auto-submission

- No keyboard operation requires a key to be pressed within a shorter interval than pointer use.
- A server deadline is announced, preserves the last valid saved responses, and submits the whole
  Attempt at most once according to the Assessment policy.
- Before Attempt expiry, reauthentication keeps the current response available for the Student to
  save or edit without resetting the server clock. At expiry, the server automatically submits the
  whole Attempt, finalizes its saved responses, and leaves other Questions unanswered.
- When Assessment policy allows another Attempt, its action is reachable through
  ordinary Tab and Space. A new Attempt receives its own server-owned
  randomization state; resuming the current Attempt preserves its state.
- A student can leave a Question Response Control with Escape or a visible return action without committing an
  answer. If leaving would discard local work, PLE asks for confirmation through a keyboard-complete
  dialog.

## Visual and assistive-technology requirements

- Focus indicators meet the repository's measured non-text contrast rule in every course theme and
  remain visible in forced-colors mode.
- Correctness never relies on color alone; visible text and semantic state convey the outcome.
- Response targets remain usable at 320 CSS pixels without horizontal scrolling of the whole page.
- Reduced-motion preferences remove nonessential focus or transition animation without hiding state.
- Question images, math, tables, and code retain reading-order semantics and useful alternatives.
- Status regions are polite for ordinary progress; blocking errors use alert semantics and name the
  next recovery action.

## Permanent evidence contract

Permanent tests protect stable user behavior, not today's component layout:

- current connected routes complete the implemented Course-to-Assessment-to-Question workflow
  through explicit whole-Assessment submission and recovery with the primary platform keys;
- Question Type evidence separately identifies arrow, digit, Enter-to-save, and Escape extension
  regressions while operating real production components;
- the student question and Student Feedback surfaces have no serious or critical axe findings;
- focus management tests cover Student Feedback, summaries, route changes, recovery, and avoidance of
  keyboard traps;
- connected WeBWorK evidence proves PLE can present a representative backend-owned document, save
  its opaque response, and complete the backend-owned lifecycle through the PLE-only network
  boundary; and
- each new PLE-native Question Type adds its Question-Type-specific no-mouse behavior before
  acceptance.

Tests assert outcomes such as focused control, changed selection, preserved response, announcement,
and completed action. They do not freeze exact Tab counts, DOM ancestry, private helper names, or the
current number of controls. A bounded `tabTo` helper is acceptable because it demonstrates that a
target is reachable; it does not assert its precise position in the tab sequence.

Automated scanning is a permanent semantic regression check because it exercises the shipped student
surface offline and can detect plausible labeling, relationship, role, and structural failures. It
does not replace the keyboard walkthrough.

## One-time and human evidence

The following are valuable implementation or release evidence but do not become permanent tests:

- exploratory accessibility-tree inspection;
- one-time screenshots or focus-ring recordings;
- manual VoiceOver plus Safari and NVDA plus Firefox or Chromium walkthroughs;
- canonical PLE email sign-in, invitation claim, account-security, and third-party-tool evaluations; and
- temporary probes used to diagnose a browser or assistive-technology combination.

Record those results in the audit or release evidence, then remove disposable scripts and fixtures.
Human evaluation should include representative students before the fall pilot; automated success
does not establish screen-reader comprehension, shortcut discoverability, or confidence.

## Acceptance checklist for a new student action

- [ ] The action has a visible, labeled native control or a justified tested composite.
- [ ] Tab and Shift+Tab reach and leave it in task order.
- [ ] Space completes the primary selection or button action without requiring a shortcut.
- [ ] Arrow, digit, Enter-to-save, or Escape extensions are scoped, documented, and tested apart
      from the primary platform journey.
- [ ] Focus remains visible, is restored after content reload, and is never trapped.
- [ ] Dynamic state is announced once with an actionable message.
- [ ] Keyboard and pointer operation produce the same server command and exact operation result.
- [ ] Failure preserves Student input and offers the applicable keyboard action.
- [ ] The behavior has the smallest durable test that would catch a real regression.
- [ ] Any temporary inspection or assistive-technology probe is recorded separately and removed.

## Current evidence boundary

M12 accepted all eight native response controls, including HOTSPOT, reaching valid local states by
keyboard on an issued Question Presentation. The M19 serial production-browser owner separately
exercised connected reload and expiry auto-submission workflows. Focused component and browser evidence
continues to cover platform-key extensions such as arrows, digits, Enter, and Escape; it does not
replace the remaining human assistive-technology acceptance below.

The remaining human boundary is deliberate. Before claiming accessibility for the local Fall pilot,
run representative VoiceOver/Safari and NVDA/browser walkthroughs through the visible local sign-in,
Instructor Course/roster/Assessment setup, and Student take/Student Feedback/repeated-Attempt path. Canonical PLE
email-code sign-in, Course Invitation claim, future Account credential settings, and any real
third-party provider remain separate production-account accessibility evidence. Optional SSO account
linking, if introduced later, needs its own focused accessibility evaluation and does not replace
either boundary.
