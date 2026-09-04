# PLE no-mouse accessibility contract

## Status and authority

This is the durable interaction contract for every PLE-owned student browser surface. It applies to
the course, assignment, Assignment Attempt, response, Student Feedback, summary, continued-practice, recovery,
asset, and PLE-owned iMathAS Question Backend boundary. `HUMAN_GUIDANCE.md` is the owner decision: every student action
must be possible with the keyboard alone. The primary path uses the browser platform contract: Tab
and Shift+Tab move focus, and Space selects choices or activates focused buttons. Arrow keys,
digits 1-9, Enter-to-submit from a response input, and Escape are documented Question Response Control extensions that
may improve efficiency but are never required to complete the task.

This document defines required behavior. The dated implementation evidence, findings, limitations,
and human-evaluation backlog remain in
[STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md). Passing an
automated scanner is evidence for semantics, not proof of this contract or complete WCAG
conformance.

## Student and context

The primary student may be working remotely on a laptop, may have limited dexterity, may use a
keyboard because a pointer is unavailable or tiring, or may combine the keyboard with a screen
reader. The critical task is to open assigned work, understand the question, answer it, submit it,
read the authorized result, recover from a failure, and continue mastery practice without touching a
mouse or trackpad.

Failure has educational consequences: an inaccessible control can prevent a student from answering,
can consume limited time, or can make a saved response appear lost. No student action may therefore
depend on hover, drag, pointer coordinates, a specific timed key sequence, or discovering an
undocumented shortcut.

## Platform path and Question Response Control extensions

PLE keeps two keyboard evidence layers so a failure names the right owner:

1. **Primary platform path.** The complete course-to-mastery journey works with Tab and Shift+Tab
   for focus, Space for native choice selection and button activation, and Enter for native link
   activation. Submission is always available through the visible Submit answer button. This path
   does not require an arrow key, digit shortcut, response-input Enter, or Escape.
2. **Question Response Control extensions.** Enter-to-submit from an eligible ready response input, composite-control
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
   links and buttons. PLE additionally permits Enter-to-submit from an eligible, locally ready
   single-line or choice response input. Enter inside a multiline text area inserts text.
6. **Arrows, digits, and Escape are scoped extensions.** Arrows may operate a response composite,
   digits 1-9 may select a visible choice ordinal while a choice input has focus, and Escape may
   return from a Question Response Control when no work is discarded. Native dialogs and input-method editors retain
   their own key handling first.
7. **Focus is always visible and never trapped.** A student can see the focused target, move away
   with ordinary keyboard commands, and return without losing the current response.
8. **Dynamic changes are announced selectively.** Validation, ordering moves, submission state,
   Student Feedback Release, errors, and recovery outcomes use concise status or alert semantics. PLE does
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

| Step                  | Required keyboard behavior                                                                                     | Completion evidence                                                            |
| --------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Enter the application | The first useful Tab stop exposes Skip to learning content; Enter moves focus to main content                  | Main content is focused and named                                              |
| Choose a course       | Tab reaches the course link; Enter opens it                                                                    | Route content loads and main receives focus                                    |
| Choose an assignment  | Tab reaches Start assignment; Enter opens it                                                                   | Assignment title and action are available                                      |
| Begin or resume       | Tab reaches Start or continue practice; Space activates it                                                     | Question heading and Question Response Control appear                          |
| Read the question     | Reading order follows prompt, assets, instructions, response, status, then submit                              | No interactive content is skipped or inserted out of order                     |
| Answer                | The Question-Type-specific contract below works without a pointer                                              | The selected or entered response is visibly represented                        |
| Validate              | Format state is announced without grading or disclosing an answer                                              | Ready or actionable validation text is available                               |
| Submit                | Tab reaches Submit answer; Space sends exactly one logical response                                            | Pending state prevents a duplicate submission                                  |
| Read Student Feedback | Authorized Student Feedback receives a heading and sensible focus; unreleased Student Feedback is not inferred | Student can read result and next action                                        |
| Continue              | Tab and Space operate Continue, Back to assignment, or Start another practice Assignment Attempt               | The next question, assignment, or fresh-seed practice Assignment Attempt opens |
| Recover               | Error, offline, stale state, and reauthentication retain the response and expose a keyboard action             | Retry uses the same logical submission identity where required                 |

Route changes focus the main content rather than leaving focus on a removed navigation element.
Student Feedback may focus its heading and later its primary advance control only when the student has not
moved focus elsewhere. A delayed focus helper never steals focus back from the student.

## Question Type contract

### Single choice and WeBWorK RadioButtons

- Tab enters the native radio group at the checked option or the browser's initial native option.
- Space selects the focused option; Tab then reaches the explicit Submit answer button.
- As separately tested extensions, native radio arrows move focus and selection, number keys 1-9
  may select a visible ordinal while a choice has focus, and Enter may submit a locally ready
  response from that input.
- Choice labels are readable text; visual letters such as A or B are not the response identity.
- The shipped PLE-native radio Question Response Control converts the reviewed WeBWorK radio interaction.
  The browser never focuses an upstream field, renderer page, or hidden WebWork control.

### Multiple answer

- Tab and Shift+Tab move through the checkbox set; Space toggles only the focused choice; the
  explicit Submit answer button completes the primary path.
- As separately tested extensions, arrow keys move focus among choices without changing selection,
  digits 1-9 toggle a visible choice while a checkbox has focus, and Enter submits only when the
  selection-count rule is satisfied.
- A visible label and programmatic checked state identify every choice.

### Fill in the blank and multi-blank

- Tab reaches blanks in prompt reading order and Shift+Tab reverses that order.
- Every blank has a stable visible or programmatic label that identifies its context; placeholder
  text is not the only label.
- Tab reaches the explicit Submit answer button and Space activates it. Enter-to-submit from a
  single-line blank is an extension only when the entire response is ready; multi-blank forms do not
  let Enter in one blank bypass unfinished fields.
- Validation identifies the blank requiring attention without moving focus unexpectedly.

### Numerical entry

- Tab reaches a native numeric or text entry with an appropriate input mode.
- Typing is always available; browser increment/decrement arrows may remain available but are not
  required for scientific notation or high-precision values.
- Units, tolerance instructions, and required format are associated with the control.
- Tab reaches the explicit Submit answer button and Space activates it. Enter-to-submit is an
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
  for a graded PLE assignment.

### Short text, file, and iMathAS Question Backend controls

- A multiline short-text field retains ordinary text-entry keys; Tab reaches Submit answer and Space
  activates it.
- PLE-owned iMathAS Question Backend launch, readiness, submit, return, and error recovery expose native
  buttons reachable with Tab and activated with Space. The iframe has a title and cannot trap focus.
  A third-party tool's internal interface is separately evaluated; PLE does not call the whole task
  accessible merely because its launch button is accessible.

## Timing, mastery, and failure recovery

- No keyboard operation requires a key to be pressed within a shorter interval than pointer use.
- A server deadline is announced, preserves the last valid controlled response, and submits it at
  most once according to the assignment policy.
- Offline or expired-session recovery retains the response until
  the student explicitly retries or edits it.
- A failed prefetch does not block the current question. Continue falls back to the server-issued
  next state and moves focus predictably.
- Mastery completion exposes Start another practice through ordinary Tab and Space. A fresh practice
  receives fresh server-owned seeds; resuming the current attempt preserves its seed.
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

- the built mock route completes course to assignment to question to explicit submission to
  continued practice with the primary platform keys and no Question Response Control extension;
- Question Type fixtures separately identify arrow, digit, Enter-to-submit, and Escape extension
  regressions while operating real production components;
- the student question and Student Feedback surfaces have no serious or critical axe findings;
- focus management tests cover Student Feedback, summaries, route changes, recovery, and avoidance of
  keyboard traps;
- the live WebWork gate proves a keyboard-operated PLE-owned radio path and PLE-only network
  boundary; and
- each new Question Type adds its Question-Type-specific no-mouse behavior before acceptance.

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
- [ ] Arrow, digit, Enter-to-submit, or Escape extensions are scoped, documented, and tested apart
      from the primary platform journey.
- [ ] Focus remains visible, is restored after recovery, and is never trapped.
- [ ] Dynamic state is announced once with an actionable message.
- [ ] Keyboard and pointer operation produce the same server command and exact operation result.
- [ ] Failure preserves student input and offers a keyboard recovery action.
- [ ] The behavior has the smallest durable test that would catch a real regression.
- [ ] Any temporary inspection or assistive-technology probe is recorded separately and removed.

## Current evidence boundary

The built mock single-choice journey proves the primary platform path. Rendered response fixtures
prove the same Tab-and-Space path for multiple answer and ordering, then independently cover their
arrows, native radio arrows, choice digits, Enter-to-submit, and Escape. Student Feedback and summary focus
tests, the iMathAS Question Backend browser fixture, and the live WeBWorK browser gate are also implemented; the
live gate exercises an extension path and does not replace the platform-key journey.
The Chapter 1 release gate now exercises static and WeBWorK MATCH through visible keyboard controls.
Numeric, short-text, FIB, MULTI-FIB, HOTSPOT, and unavailable-file behavior still rely partly on
focused component/source evidence rather than a full route and must satisfy this contract as part of
their Question Type acceptance rather than being deferred to a later generic accessibility pass.

The remaining human boundary is deliberate. Before claiming accessibility for the local Fall pilot,
run representative VoiceOver/Safari and NVDA/browser walkthroughs through the visible local sign-in,
instructor course/roster/assignment setup, and student take/Student Feedback/retry/repeat path. Canonical PLE
email-code sign-in, Course Invitation claim, future Account credential settings, and any real
third-party provider remain separate production-account accessibility evidence. Optional SSO account
linking, if introduced later, needs its own focused accessibility evaluation and does not replace
either boundary.
