# WP-S1 student repeat state review

## Verdict

ACCEPTED TO LIVE. The security, privacy, state, runner, and focused behavior
gates below pass, including the originally pending TypeScript formatting check.

## Verified contract

- Schema-v2 state is an exact ordered prefix: J11, J12, J13, then append-only
  J1, J2, J3, J4. All student fragments bind the J13 course and assignment
  IDs, expose only fixed visible outcome codes, use canonical lower-case UUIDs,
  have empty diagnostics, and bound elapsed time to 30 minutes.
- The instructor parser rejects unknown, hidden, symbol, inherited, and
  accessor fields, duplicate or noncanonical JSON, malformed modes, oversized
  files, reordered fragments, and elapsed times outside the bounded window.
- The student append boundary applies the same exact-own-enumerable-data-field
  rule before reading an input fragment. It rejects reordered journeys, foreign
  assignment IDs, unsafe file metadata, accessors, hidden fields, symbols, and
  inherited fields.
- The Python handoff uses an `O_DIRECTORY|O_NOFOLLOW` parent descriptor and
  descriptor-relative child open, then rechecks the named parent mode/device/
  inode. Its hostile parent-replacement test passes. It checks exact J11-J13
  codes, empty diagnostics, bounded elapsed values, canonical UUIDs, shared
  course ID, and J13 corpus IDs against the sole corpus-publication arrangement.
- J2 now fails if it encounters fresh practice rather than J1's active retry;
  it cannot silently begin another run. J3 visibly starts exactly the second
  run, leaves it, and resumes it with cleared controls. J4 completes it and
  observes, but does not activate, fresh practice. This preserves exactly two
  completions for WP-S2 gradebook proof.
- `--student-repeat-only` is explicit and mutually exclusive with instructor
  setup only. It reports the clearly partial `student_repeat_complete` stage
  and `student_repeat_only` mode. Default execution fails closed after J4,
  awaiting WP-S2 and WP-E1; no schema-v2 final-report claim is made. Failure
  reports remain redacted, private mode 0600, and cleanup remains one
  no-volume compose-down plus private-state removal.

## Evidence

Passed locally without starting Podman:

```text
python3 -m py_compile tests/e2e/e2e_ui_walkthrough.py tests/test_ui_walkthrough_runner.py
python3 -m pytest -q tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_harness_independence.py
# 44 passed, 5 subtests passed

npx playwright test tests/playwright/simulator/instructor_setup_state.spec.ts \
  tests/playwright/simulator/student_repeat_state.spec.ts \
  tests/playwright/ui_walkthrough_live_config.spec.ts --reporter=line
# 15 passed

npx tsc --noEmit -p tsconfig.lint.json
npx eslint --max-warnings 0 <WP-S1 TypeScript paths>
npx prettier --check <the six WP-S1 TypeScript paths>
git diff --check
```

This review does not claim the later WP-S2 gradebook, J8 cross-actor, or
WP-E1 final-report work. It authorizes only WP-S1's retained-stack live
student-repeat validation.
