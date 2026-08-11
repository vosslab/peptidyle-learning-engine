# WP-E2 walkthrough library security review

## Verdict

ACCEPTED TO LIVE. The dedicated `tests/walkthrough/` extraction preserves the
runner's fail-closed boundaries and does not import another E2E runner or a
test module. The legacy entry points are compatibility facades only; both
canonical and legacy Python paths invoke `walklib.runner.main`, while the
legacy shell path execs the canonical shell path.

## Security and behavior boundary

- `walklib.runner` remains the sole lifecycle owner. It imports only sibling
  `walklib` modules; neither it nor the compatibility facades import an E2E
  runner, test module, or test-support helper.
- The fixed children now live under `tests/walkthrough/children/`. Their
  imports point only to the established Playwright simulator helpers. The
  recursively expanded harness-policy gate scans every Python, shell, and
  TypeScript source in the dedicated directory, both compatibility facades,
  and the browser journeys; it still rejects product-internal imports,
  database-shaped setup, private browser/API paths, non-keyboard interaction,
  answer-bearing assertions, and hidden failure-to-PASS conversion.
- Private report and handoff-state defenses are preserved: symlink refusal,
  directory-descriptor/no-follow operations, parent identity checks, regular
  file checks, `0700` report/state directories, and `0600` report/state files.
  The report is atomically replaced only after its compact redacted payload is
  serialized.
- Arrangement output remains accepted only as one canonical ASCII JSON line at
  most 2,048 bytes; visible outcome output remains one canonical ASCII JSON
  line at most 4,096 bytes. Renderer stderr and cross-actor stdout/stderr cause
  failure, and child output is not copied to the report or normal diagnostics.
- Podman cleanup remains guarded by exact `containers` project-label ownership,
  occurs only after a runner launch, honors `--keep`, uses `down
  --remove-orphans`, and never requests volume removal. Cleanup or report-write
  failure downgrades the run to a redacted failure result.

## J3/J4 terminology repair

The former source-contract test files are now behavior-named
`tests/test_student_leave_resume_evidence.mjs` and
`tests/test_student_completion_policy_evidence.mjs`; neither contains
milestone terminology. Their paired simulator modules retain `"J3"` and
`"J4"` solely as serialized `journey` values needed by the public report
schema. `walklib.v2_report_contract.EXPECTED_JOURNEYS` likewise retains those
schema identifiers, not plan/milestone names. The permanent naming gate rejects
new `test_j*`, `test_m*`, and `test_w*` filenames.

## Verification

No Podman command was run and no retained report was read or changed.

```text
source source_me.sh && python3 -m pytest -q
  tests/test_ui_walkthrough_runner.py
  tests/test_ui_walkthrough_runner_cleanup.py
  tests/test_ui_walkthrough_v2_report_contract.py
  tests/test_ui_walkthrough_harness_independence.py
  tests/test_test_naming_conventions.py
  tests/test_import_dot.py
  tests/test_function_typing.py
  tests/test_pyflakes_code_lint.py
  tests/test_shebangs.py
1269 passed, 11 subtests passed

npx tsc --noEmit
npx tsc --noEmit -p tsconfig.lint.json
npx eslint --max-warnings 0 relevant walkthrough sources
all passed

node --import tsx --test tests/test_student_leave_resume_evidence.mjs
  tests/test_student_completion_policy_evidence.mjs
8 passed, 0 failed

git diff --check
passed
```
