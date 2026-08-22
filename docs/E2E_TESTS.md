# E2E_TESTS.md

End-to-end (E2E) testing conventions for this repo.

## Two E2E homes

This repo has one browser home and one browser-free service home:

- `tests/playwright/e2e/` - **browser-based E2E** owned by `run_playwright_tests.sh`, using the
  production `dist/` bundle through one disposable HTTPS gateway and fixed real-stack owner.
- `tests/e2e/` - **non-browser E2E**: shell/Python orchestration for whole-system service claims,
  such as publication, renderer/cache/outage, database/RLS, and lifecycle cleanup.

Both are excluded from `pytest tests/` via `collect_ignore = ["e2e", "playwright"]` in `tests/conftest.py`.

## Test layout overview

This repo organizes tests in four tiers, all under the `tests/` umbrella:

- `tests/test_*.py` - fast pytest unit and integration tests. Run with `pytest tests/`.
- `tests/test_*.mjs` - pure Node tests, if any (rare; not browser-driven).
- `tests/playwright/e2e/` - browser-driven Playwright scenarios, selected through the canonical
  owner. TypeScript repos include `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder.
- `tests/e2e/` - non-browser whole-system E2E. Shell/Python orchestration (`e2e_*.sh`, `e2e_*.py`). Run directly, not via pytest.

## Why tests/e2e/ is excluded from pytest

Pytest is the fast lane. Tests under `tests/` should run in seconds so the
suite stays useful during development. End-to-end tests are by nature slow:
they invoke real scripts, read and write real files, and may hit the network
or external tools. Mixing them into `pytest tests/` makes the fast lane slow
and discourages running it.

Pytest's `collect_ignore = ["e2e", "playwright"]` in `tests/conftest.py` actively excludes
both the `tests/e2e/` and `tests/playwright/` subtrees from pytest collection, regardless of filenames
inside them. This is the primary safety mechanism. Additionally, `.mjs` and `.sh`
files are invisible to pytest by extension, and Python orchestration scripts use
the `e2e_*` prefix as a secondary, human-readable convention.

## Where non-browser E2E tests live

- Folder: `tests/e2e/` under `tests/` at the repo root.
- Pytest is configured to ignore the subtree via `collect_ignore = ["e2e", "playwright"]` in
  `tests/conftest.py`, so file naming inside `tests/e2e/` cannot accidentally pull slow tests into the fast lane.
- Recommended naming for readability:
  - `e2e_*.sh` for shell runners.
  - `e2e_*.py` for Python orchestration.
- Each E2E script is self-contained and exits non-zero on failure.

`tests/` (excluding `tests/e2e/` and `tests/playwright/`) stays reserved for fast pytest tests (see
[PYTEST_STYLE.md](PYTEST_STYLE.md)).

## How to run non-browser E2E tests

- Run a single shell runner: `bash tests/e2e/e2e_<name>.sh`.
- Run a single Python runner: `source source_me.sh && python3 tests/e2e/e2e_<name>.py`.
- Run all E2E tests: provide a `tests/e2e/run_all.sh` that iterates over the
  `e2e_*` files and reports pass/fail for each.
- Run browser-driven scenarios only through `./run_playwright_tests.sh`; use `--screenshots` via
  `./capture_screenshots.sh`. A focused selection gets a fresh fixed stack; an acceptance run has
  one browser-suite invocation.
- Do not invoke E2E tests from `pytest tests/`. Keep the two suites separate.

## Naming conventions test

File naming conventions are enforced by `tests/test_test_naming_conventions.py`
(present in `REPO_TYPE=typescript` repos) to prevent silent bugs:

- No `test_*.py` files anywhere under `tests/e2e/` (since `collect_ignore` would skip them silently, mismatching the name).
- No `test_*.py` files anywhere under `tests/playwright/` (same trap).
- All Python files under `tests/e2e/` must use the `e2e_*.py` prefix.
- All shell files under `tests/e2e/` must use the `e2e_*.sh` prefix.
- Any file with a Playwright import must live under `tests/playwright/`.

## What E2E tests should cover

- Whole-script behavior: run the CLI end to end with realistic arguments and
  check the produced files or exit code.
- I/O round trips: encode a file with one script, decode with another,
  compare to the original.
- Integration with external tools where a fake integration would defeat the point. Retained service oracles
  stay browser-free and must not launch Chromium, import a browser configuration, or claim a UI
  journey.
- Anything that needs user input or read/write to files (the `assert` rules
  forbid asserts in plain scripts entirely; cover that behavior here instead;
  see [PYTHON_STYLE.md](PYTHON_STYLE.md#assert)).

## What E2E tests should not cover

- Pure function correctness. That belongs in pytest under `tests/`.
- Anything fast enough to live in pytest. If a check finishes in under a
  second and does not touch the real filesystem in a meaningful way, it is a
  unit test, not an E2E test.

The canonical instructor scenario visibly asserts the `WebAssembly` runtime mode. Decoder,
serialization, strict transport, and failure-mapping checks remain narrow browser-free unit tests.

## Asserts and failures

- E2E test scripts may use `assert` (they are test files, not plain scripts).
- Prefer explicit exit codes and clear stderr messages so a failing E2E run
  is easy to diagnose without reading the script.

## Related docs

- [PYTEST_STYLE.md](PYTEST_STYLE.md): fast pytest unit and integration tests under `tests/`.
- Browser-driven test conventions: the website family (`website` and its inheriting `typescript`) includes `PLAYWRIGHT_USAGE.md` in their propagated `docs/` folder for tests under `tests/playwright/`.
- Browser test authoring style: the website family (`website` and its inheriting `typescript`) includes `PLAYWRIGHT_TEST_STYLE.md` in their propagated `docs/` folder for how to write Playwright tests under `tests/playwright/`.
- [PYTHON_STYLE.md](PYTHON_STYLE.md): repo-wide Python rules, including
  the `assert`-only-in-tests boundary.
