# WP-O1 Python runner independent review

## Verdict

ACCEPTED.

The revised Python lifecycle fails closed when cleanup or report creation
fails, and its real-stack handoff is now independently evidenced. The manager
and independent elevated macOS runs completed the exact public runner command;
WP-O2 independently reviewed the resulting browser smoke and cleanup evidence.

## Findings

No remaining findings at the WP-O1 offline boundary.

## Confirmed behavior

- The Bash script is a thin, strict-mode entrypoint that sources `source_me.sh`
  and `exec`s the typed Python CLI.
- Argparse requires the decimal uint32 seed and rejects conflicting build
  choices. AUTO uses both safe dist outputs, `--build` omits launcher
  `--skip-build`, and explicit reuse fails before report or Podman work.
- The runner uses argv-array subprocesses, exposes progress stages, does not
  read `local-demo.json` or credential contents, and sends walkthrough values
  only to the Playwright child environment.
- The exact stopped-and-running Podman label checks occur before report setup
  and again before launch. The selected compose project, port, report basename,
  env file, and existing sibling credential boundaries are validated.
- Cleanup is gated on a launcher attempt, uses the chosen provider and env file,
  honors `--keep`, and does not request volume deletion. A nonzero or `OSError`
  cleanup downgrades a would-be success to `FAIL` at `cleanup`, preserves an
  earlier failure stage, and still writes the redacted report when possible.
- The atomic report has exactly `status`, `masterSeed`, and `stage`, with a
  mode-0700 parent and mode-0600 result. It revalidates and recreates the
  private directory after Playwright artifact cleanup, opens that directory
  with a dirfd and `O_NOFOLLOW` when available, creates a random exclusive
  temporary result, and atomically replaces the report basename. A report-write
  failure or directory-symlink replacement returns a concise nonzero result
  without claiming success or writing outside the report directory.
- `main()` handles narrow operational `OSError` and `UnicodeError` cases after
  runner creation without masking programming errors, then performs eligible
  cleanup and avoids a traceback.

## Validation

Passed without Podman or a browser:

```bash
source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py
bash -n tests/e2e/e2e_ui_walkthrough.sh
bash tests/e2e/e2e_ui_walkthrough.sh --help
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed nope --skip-build
source source_me.sh && python3 -m py_compile tests/e2e/e2e_ui_walkthrough.py \
  tests/test_ui_walkthrough_runner.py
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i tests/e2e/e2e_ui_walkthrough.sh
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i tests/e2e/e2e_ui_walkthrough.py
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i tests/test_ui_walkthrough_runner.py
git diff --check -- tests/e2e/e2e_ui_walkthrough.sh \
  tests/e2e/e2e_ui_walkthrough.py tests/test_ui_walkthrough_runner.py
```

The focused pytest suite passed 18 tests, including report-directory recreation
after Playwright removes `test-results` and symlink-replacement refusal. ASCII,
line-length, py-compile, shell-syntax, and diff-whitespace checks passed.

The ordinary sandbox first ran the exact public command and reached launcher
check, start, and the Playwright boundary. macOS denied Chromium's Mach-port
startup before a browser context opened; the runner failed closed and recorded
only `{"status":"FAIL","masterSeed":42,"stage":"playwright_smoke"}`. This
was an execution-sandbox browser permission limit, not an application result.

The manager and independent reviewer then ran the same command with approved
elevated macOS Chromium and Podman access. It exited 0; the browser verified
the public IPv4 `/health` origin, HTTP 200, and exact `{"status":"ready"}`
body. The runner wrote exact
`{"status":"PASS","masterSeed":42,"stage":"complete"}` JSON in a mode-0700
directory with a regular mode-0600 report. Playwright's `.last-run.json` had no
failed tests, and `podman ps --all --quiet` was empty after runner cleanup.
These observations are independently recorded in
`docs/active_plans/audits/wp_o2_live_playwright_review.md`.
