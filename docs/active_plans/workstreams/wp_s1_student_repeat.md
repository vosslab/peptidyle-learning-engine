# WP-S1 student keyboard take and repeat

## Status

Implementation is ready for independent HCI and security review. It has not
received a Podman live-run claim. The command is deliberately partial until
WP-S2 and WP-E1 add visible instructor scoring and the final schema-v2 report.

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
  Continue, and the real fresh-practice control after completion one.
- J3 activates that fresh-practice control, proves the new controls are clear,
  leaves through the visible Return control, and resumes the same active second
  run. J4 proves controls are still clear, completes it, and observes the fresh
  practice and Back controls without starting a third run.

## State boundary

`student_repeat_state.ts` appends only schema-v2 public fragments after the
atomic J11/J12/J13 prefix. It rejects reordered, cross-assignment, inherited,
hidden, symbol, and accessor-shaped data before serialization. Historical
schema-v1 state/report modules remain intact for later WP-E1 migration work.

## Offline evidence

- `python3 -m py_compile tests/e2e/e2e_ui_walkthrough.py tests/test_ui_walkthrough_runner.py`
- `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_harness_independence.py -q`
- `npx tsc --noEmit`
- Focused Playwright state/config tests pass; live J1-J4 specs skip outside the
  explicit runner invocation.

## Remaining gate

Independent HCI/security review, then a real retained-stack run of:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build --student-repeat-only
```

That partial run does not accept gradebook cells, run-history counts, the final
report, email onboarding, or production identity.
