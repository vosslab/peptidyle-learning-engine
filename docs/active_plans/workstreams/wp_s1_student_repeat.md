# WP-S1 student keyboard take and repeat

## Status

**ACCEPTED AS PART OF THE CORRECTED LOCAL NO-EMAIL PILOT.** Two same-seed
retained-stack `--build` runs exercised J1--J4 after visible J11/J12/J13
setup; the student completed two runs using the keyboard platform path. The
partial flag remains useful for failure isolation, but the default full command
now supplies the accepted integrated evidence through WP-S2 and WP-E1.

## Delivered path

- `--student-repeat-only` runs corpus publication, J11/J12/J13, then J1/J2/J3/J4.
  It writes a redacted partial receipt with stage `student_repeat_complete` and
  mode `student_repeat_only`; the default command fails closed rather than
  claiming complete before WP-S2/WP-E1.
- The runner reads the protected J11/J12/J13 state through descriptor-anchored
  no-follow checks. It exports only the validated public course, assignment,
  problem, and version IDs after confirming exact visible outcomes, bounded
  elapsed time, and the arranged corpus reference.
- J1 opens the exact instructor-created course and Mastery assignment, submits a
  visible first response, observes Feedback and Continue, and stops only at an
  active retry screen with two unchecked controls.
- J2 resumes that retry, submits the visible second response, observes Feedback,
  Continue, and the real fresh-practice control after completion one, then
  activates it to begin the blank second run.
- J3 resumes that active second run, selects a visible control, leaves through
  the visible Return control, and resumes it with both controls cleared. J4
  proves controls are still clear, completes it, and observes the fresh practice
  and Back controls without starting a third run.

## State boundary

`student_repeat_state.ts` appends only schema-v2 public fragments after the
atomic J11/J12/J13 prefix. It rejects reordered, cross-assignment, inherited,
hidden, symbol, and accessor-shaped data before serialization. Historical
schema-v1 state/report modules remain intact for later WP-E1 migration work.

## Offline evidence

- `python3 -m py_compile tests/e2e/e2e_ui_walkthrough.py tests/test_ui_walkthrough_runner.py`
- `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py -q`
- `npx tsc --noEmit`
- Focused Playwright state/config tests pass; live J1-J4 specs skip outside the
  explicit runner invocation.

## Acceptance boundary

The integrated full command completed the later visible gradebook and schema-v2
report gates. This workstream still does not accept email onboarding,
production identity, J6/J7, all-family, multi-learner, or release work.
