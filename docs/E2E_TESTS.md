# E2E_TESTS.md

End-to-end (E2E) testing conventions for this repo.

## Three E2E homes

This repo keeps three distinct opt-in execution owners:

- `tests/playwright/` (and optional `tests/playwright/e2e/` sub-grouping) - **browser-based E2E**: full Playwright walkthroughs and browser-driven tests. TypeScript repos include `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder.
- `tests/e2e/` - **generic non-browser E2E**: disposable database, service,
  build, and multi-suite runners.
- `tests/walkthrough/` - **teaching-loop walkthrough**: its canonical runner,
  fixed child processes, and importable `walklib/` orchestration package.

All three are excluded from `pytest tests/` by `tests/conftest.py`.

## Test layout overview

This repo organizes tests in four tiers, all under the `tests/` umbrella:

- `tests/test_*.py` - fast pytest unit and integration tests. Run with `pytest tests/`.
- `tests/test_*.mjs` - pure Node tests, if any (rare; not browser-driven).
- `tests/playwright/` (with optional `tests/playwright/e2e/` subfolder) - browser-driven Playwright tests. TypeScript repos include `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder.
- `tests/e2e/` - non-browser whole-system E2E. Run the direct `e2e_*.sh` and
  `e2e_*.py` entry points directly.
- `tests/walkthrough/` - dedicated instructor-to-student orchestration. Run its
  named shell entry point directly; reusable Python lives in `walklib/`.

## Why tests/e2e/ is excluded from pytest

Pytest is the fast lane. Tests under `tests/` should run in seconds so the
suite stays useful during development. End-to-end tests are by nature slow:
they invoke real scripts, read and write real files, and may hit the network
or external tools. Mixing them into `pytest tests/` makes the fast lane slow
and discourages running it.

Pytest's `collect_ignore` actively excludes `tests/e2e/`,
`tests/playwright/`, and `tests/walkthrough/` from collection regardless of
filenames inside them. This is the primary safety mechanism. Additionally,
`.mjs` and `.sh`
files are invisible to pytest by extension, and Python orchestration scripts use
the `e2e_*` prefix as a secondary, human-readable convention.

## Where non-browser E2E tests live

- Folder: `tests/e2e/` under `tests/` at the repo root.
- Pytest is configured to ignore the opt-in subtrees via
  `collect_ignore = ["e2e", "playwright", "walkthrough"]` in
  `tests/conftest.py`, so file naming inside them cannot accidentally pull slow
  tests into the fast lane.
- Recommended entry-point naming for readability:
  - `e2e_*.sh` for shell runners.
  - `e2e_*.py` for Python orchestration.
- Each E2E script is self-contained and exits non-zero on failure.

`tests/` excluding its three opt-in subtrees stays reserved for fast pytest tests (see
[PYTEST_STYLE.md](PYTEST_STYLE.md)).

## How to run non-browser E2E tests

- Run a single shell runner: `bash tests/e2e/e2e_<name>.sh`.
- Run a single Python runner: `source source_me.sh && python3 tests/e2e/e2e_<name>.py`.
- Run all non-browser E2E tests with `bash tests/e2e/e2e_run_all.sh`.
- For browser-driven Playwright runs, TypeScript repos include `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder.
- Do not invoke E2E tests from `pytest tests/`. Keep the two suites separate.

## Naming conventions test

File naming conventions are enforced by `tests/test_test_naming_conventions.py`
(present in `REPO_TYPE=typescript` repos) to prevent silent bugs:

- No `test_*.py` files anywhere under `tests/e2e/` (since `collect_ignore` would skip them silently, mismatching the name).
- No `test_*.py` files anywhere under `tests/playwright/` (same trap).
- Direct Python files under `tests/e2e/` must use the `e2e_*.py` prefix.
- All shell files under `tests/e2e/` must use the `e2e_*.sh` prefix.
- Any file with a Playwright import must live under `tests/playwright/`.

## What E2E tests should cover

- Whole-script behavior: run the CLI end to end with realistic arguments and
  check the produced files or exit code.
- I/O round trips: encode a file with one script, decode with another,
  compare to the original.
- Integration with external tools where mocking would defeat the point.
- Anything that needs user input or read/write to files (the `assert` rules
  forbid asserts in plain scripts entirely; cover that behavior here instead;
  see [PYTHON_STYLE.md](PYTHON_STYLE.md#assert)).

## What E2E tests should not cover

- Pure function correctness. That belongs in pytest under `tests/`.
- Anything fast enough to live in pytest. If a check finishes in under a
  second and does not touch the real filesystem in a meaningful way, it is a
  unit test, not an E2E test.

## Asserts and failures

- E2E test scripts may use `assert` (they are test files, not plain scripts).
- Prefer explicit exit codes and clear stderr messages so a failing E2E run
  is easy to diagnose without reading the script.

## Opt-in UI walkthrough

The Python-backed UI walkthrough is an opt-in real-stack check, never part of
the fast baseline:

```bash
bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42
```

The dedicated folder owns its shell/Python entry points, fixed TypeScript
children, and reusable `walklib/` package. The historical
`tests/e2e/e2e_ui_walkthrough.sh` path remains a thin compatibility launcher.
Playwright journey specs remain under `tests/playwright/` rather than becoming
hidden runner internals.

It accepts only local IPv4 loopback origins. AUTO reuses safe `dist/` outputs
when present and builds when absent; `--build` is the only explicit build
option and forces a refresh. The private report is
`test-results/ui_walkthrough/ui_walkthrough_seed_42.json` by default, in a
mode-0700 directory with a mode-0600 file. It reports only redacted status,
seed, stage, public arrangements, and eligible visible outcomes.

The corrected schema-v2 local no-email pilot is accepted: manager and
independent `--build` runs each passed J11/J12/J13/J1/J2/J3/J4/J5/J8 with the
private report modes and no-volume cleanup above. They visibly create a fresh
course, activate the configured local student, create the corpus-backed
Mastery assignment, verify the instructor-facing problem number and backend label, complete the
representative four-question Genetics Chapter 1 assignment, complete two keyboard-driven focused
student runs, and confirm Best `100%`, Latest `100%`, Completed `2`, and two completed history
entries. The Chapter 1 phase is a required browser phase rather than a new public report row.
Native `target="_self"` pagination links land in named `tabindex="-1"`
regions, then Tab reaches visible load, retry, or reload controls. The cursor
session keeps opaque cursors, retries the exact failed cursor, deduplicates
rows, and fails closed on protocol errors. Email/canonical onboarding, J6/J7, all eight response
families, multi-learner, and complete two-chapter release acceptance are not walkthrough rows or
prerequisites. The separate `bash tests/e2e/e2e_chapter_one_browser.sh` oracle completes both exact
Chapter 1 assignments. That release oracle always runs `./build.sh` and intentionally exposes no
skip-build switch; the walkthrough's general `--skip-build` policy does not apply to it.

## Related docs

- [PYTEST_STYLE.md](PYTEST_STYLE.md): fast pytest unit and integration tests under `tests/`.
- Browser-driven test conventions: the website family (`website` and its inheriting `typescript`) includes `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder for tests under `tests/playwright/`.
- Browser test authoring style: the website family (`website` and its inheriting `typescript`) includes `PLAYWRIGHT_TEST_STYLE.md` in their propagated `docs/` folder for how to write Playwright tests under `tests/playwright/`.
- [PYTHON_STYLE.md](PYTHON_STYLE.md): repo-wide Python rules, including
  the `assert`-only-in-tests boundary.
