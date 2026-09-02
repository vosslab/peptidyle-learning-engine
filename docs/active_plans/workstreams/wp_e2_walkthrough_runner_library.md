# WP-E2 walkthrough runner library

## Status

ACCEPTED. The no-email pilot behavior remains unchanged behind the dedicated
walkthrough package and stable compatibility entry points.

## Contract

- Make `bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42` the
  canonical command.
- Keep `tests/e2e/e2e_ui_walkthrough.sh` and its Python partner as directly
  runnable compatibility facades.
- Own reusable Python behavior under `tests/walkthrough/walklib/`.
- Own fixed TypeScript child processes under `tests/walkthrough/children/`.
- Keep Playwright journeys under `tests/playwright/` as visible user-path
  evidence.
- Preserve fail-closed state, report, subprocess, Podman ownership, redaction,
  cleanup, and no-volume behavior exactly.
- Default AUTO reuses readable, non-symlink `dist/index.html` and `dist/main.js`
  and builds only when either is absent. `--build` is the sole public build
  override; the runner keeps the launcher's internal `--skip-build` detail private.
- On instructor-child failure only, permit one closed redacted checkpoint value:
  `login_visible`, `signed_in`, `course_created`, `course_opened`,
  `student_active`, `assignment_editor_opened`, `question_search_result_selected`, or
  `assignment_created`. `login_visible` means only that the visible local-login
  control rendered; it is written before any credential value is entered.
- Do not import another E2E runner or E2E test-support module.

## Package boundary

- `configuration.py`: CLI values, local files, gateway, and build selection.
- `models.py`: typed runner inputs, process results, and error contract.
- `process.py`: shell-free captured subprocess boundary.
- `arrangement_contract.py`: strict bounded arrangement parser.
- `v2_report_contract.py`: strict redacted schema-v2 parser.
- `runner.py`: the sole composition root and lifecycle orchestrator.

The package `__init__.py` contains only a package docstring. Callers import the
owning submodule directly; it is not a re-export facade.

## Verification

- Focused Python runner, cleanup, report-contract, and harness-policy suites.
- Python import, pyflakes, typing/import/naming/source-line policy checks.
- CLI help and shell syntax.
- Node and TypeScript walkthrough source gates.
- Independent architecture and security review before retained-stack reuse.

## Acceptance evidence

- The host-bound canonical retained-stack command
  `bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42` passed after
  the package move. The earlier sandboxed attempt failed before Chromium entered
  the test body and was not treated as product evidence.
- The sole retained public report is canonical schema v2 with seed 42, the
  exact ordered J11/J12/J13/J1/J2/J3/J4/J5/J8 PASS rows, empty diagnostics,
  and one label-only corpus arrangement. Its aggregate 4676 ms equals the row
  sum; the directory/file modes are 0700/0600.
- Cleanup left no Podman containers or private walkthrough directory. It did
  not remove volumes. Independent retained-live and final closeout reviews
  found no P1/P2 issue.
- Permanent checks now follow `docs/PYTEST_STYLE.md`: visible behavior, strict
  parsers, redaction, filesystem integrity, cleanup ownership, and failure
  containment remain; source-text, exact-path, argv, help, timeout, count, and
  migration-wiring probes were removed after serving as implementation checks.
