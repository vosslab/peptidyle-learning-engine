# WP-S1 student take-and-repeat HCI review

## Acceptance addendum

The retained-stack full run and independent replay subsequently passed the
corrected local no-email pilot on 2026-08-11. The two keyboard-only student
runs now have integrated J5 visible-score and schema-v2 report evidence. This
does not accept email/canonical onboarding, J6/J7, all-family, multi-student,
or release work.

## Scope and verdict

Independent read-only review of the in-progress schema-v2 J1--J4 student
journey against the historical walkthrough plan and the no-mouse contract. No
product code, runner, Podman machine, live stack,
or retained data was changed.

**ACCEPTED TO LIVE.** The repaired path now fails closed unless J2 resumes the
active J1 retry, and its visible interaction sequence constructs exactly two
completed runs without activating a third. This is a partial student gate only:
it does not claim the required live run, gradebook outcome, J8, final report, or
any email-related behavior.

## Visible task sequence

- J1 starts from the allowed root entry, uses the rendered local credential
  form, and opens the exact arranged course and assignment through one visible
  href each. Native Tab/Enter then reaches Start, two unchecked radios, Submit,
  Feedback, and Continue. It passes only when it returns to `runAttempt` with
  two cleared radios and no fresh-practice control: the visible retry state.
- J2 repeats that visible course/assignment selection and requires a `run`
  surface. A rendered fresh-practice control is now an error, rather than an
  action to activate. It selects the second visible radio, submits, continues,
  and explicitly observes `Start another practice run`; this is completion of
  run 1.
- J3 reaches and activates that exact fresh-practice control with keyboard
  focus, proves both new response controls are clear, uses the rendered Return
  to assignment control, and visibly resumes the same run. The two controls
  are still clear after resume.
- J4 opens that active second run through the same visible course/assignment
  path, proves the controls are clear, completes it by keyboard, and observes
  both the fresh-practice heading/control and Back to assignment. It only
  focuses Back to assignment; it never activates fresh practice. Thus no third
  run is created.

The state append order is fixed after the atomic J11/J12/J13 prefix as J1, J2,
J3, J4 and cross-binds the public course and assignment IDs. This is consistent
with the later J5 requirement to see exactly two completed histories.

## Keyboard and shortcut boundary

The four specs use only the allowed root `page.goto("/")`, rendered visible
links/controls, `tabTo` or visible pagination, and literal Tab, Shift+Tab,
Enter, and Space. The shared helper's only `evaluate` checks whether the
rendered target is `document.activeElement`; it neither focuses nor selects it.
No pointer action, direct focus, non-root route, history, request/API,
cookie/storage, Arrow/digit/Escape key, answer text, or feedback-body read was
found. The local credential value is supplied only through its labelled,
rendered form boundary, which the contract intentionally allows.

## Focused validation

| Check | Result |
| --- | --- |
| `python -m pytest tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py` | PASS: 44 tests. This includes the permanent forbidden-shortcut scanner and runner ordering/failure-stage checks. |
| `npx tsc --noEmit` | PASS. |
| `python3 -m py_compile tests/e2e/e2e_ui_walkthrough.py tests/test_ui_walkthrough_runner.py` | PASS. |
| Focused Playwright `student_repeat_state` and keyboard-helper fixtures | State fixture PASS (3 tests); four browser-body fixture tests could not launch because this sandbox denies Chromium's macOS Mach-port registration. This is an environment limitation, not a test assertion failure; no Podman stack was started. |

## Historical next evidence

At the time of this review, the required next step was the retained-stack
partial command and independent replay before accepting the student slice:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build --student-repeat-only
```

Confirm the produced protected state contains the ordered J11, J12, J13, J1,
J2, J3, J4 PASS prefix with empty diagnostics. Keep the later J5/J8 and
schema-v2 public-report acceptance separate.
