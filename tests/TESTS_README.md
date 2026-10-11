# Tests

The permanent suite keeps focused checks whose assertions protect important, stable behavior.
Vendored hygiene tests are maintained by the shared propagation script and are outside the
repository-owned test-pruning scope. See [../docs/TEST_PRUNING_REPORT.md](../docs/TEST_PRUNING_REPORT.md)
for the inventory and retention decisions.

## Run the checks

| Command | Evidence |
| --- | --- |
| `source source_me.sh && ./launchers/run_fast_checks.sh` | Schema checks, Rust checks, TypeScript checks, Node tests, and pytest. |
| `source source_me.sh && ./launchers/all_test.sh` | Offline checks followed by the disposable PostgreSQL baseline. |
| `source source_me.sh && python3 -m pytest tests/` | Invitation safety and vendored hygiene. |
| `node --import tsx --test 'tests/test_*.mjs'` | Autosave, validation, privacy, calculation, and response-integrity checks. |
| `source source_me.sh && ./devel/run_playwright_tests.sh` | Real-stack authentication/authorization and confirmed Student-data disclosure. |
| `source source_me.sh && bash tests/e2e/e2e_database_baseline.sh` | Database initialization/replay, role isolation, and Unrelease concurrency. |
| `node tests/e2e/e2e_wasm_bridge.mjs` | Existing built WebAssembly bridge checks. |

Browser tests need the controller-managed HTTPS Live Demo and its private runtime inputs.
The database baseline owns a disposable PostgreSQL instance. Passing offline checks does not
establish either connected result.

## Test locations

- `test_*.py`: repo-owned behavior tests and unchanged vendored hygiene tests.
- `test_*.mjs`: Node tests that exercise focused TypeScript behavior without shared mock infrastructure.
- `playwright/e2e/`: two production-browser scenarios and their shared helper.
- `e2e/`: connected SQL oracles, their runners, browser orchestration, and runtime Compose profiles.
- `../crates/`: Rust unit tests beside their owning implementation.
- `_temp/`: ignored one-time verification, removed after the work it supports.

The screenshot publisher under `playwright/screenshot_corpus/` remains operational tooling.
It produces review artifacts and is separate from permanent behavioral tests. Run it through
`source source_me.sh && ./devel/capture_screenshots.sh`.

The old isolated UI fixture lane, broad client/decoder Node suite, and legacy E2E umbrella
runner were deleted. Their old reports are historical evidence, not current test coverage.
