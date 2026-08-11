# WP-O1 walkthrough runner

## Status

Independently ACCEPTED. The 2026-08-11 elevated live command completed with exit 0:

Superseded ownership note: WP-E2 later made
`tests/walkthrough/run_ui_walkthrough.sh` the canonical entrypoint and retained
the `tests/e2e/` command below as a directly runnable compatibility facade. The
original command and result remain historical acceptance evidence.

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

AUTO reused the two safe `dist` outputs, then launcher check, launch, WP-O2 Playwright smoke, and
runner-owned cleanup completed. The browser verified the public IPv4 `/health` origin, HTTP 200,
and exact `{"status":"ready"}` body with mock preview disabled. The final report was exactly
`{"status":"PASS","masterSeed":42,"stage":"complete"}` in a mode-0700 directory as a regular
mode-0600 file; `podman ps --all --quiet` was empty after cleanup. See the independent
[WP-O1 review](../audits/wp_o1_python_runner_review.md) and
[WP-O2 review](../audits/wp_o2_live_playwright_review.md).

## Contract

- `tests/e2e/e2e_ui_walkthrough.sh` is the stable thin entrypoint. It sources `source_me.sh` and
  executes the typed Python argparse CLI, which requires `--master-seed UINT32` and accepts
  `--env-file PATH`, `--report-file BASENAME`, `--keep`, and `--build`. Invalid
  arguments fail before report creation or Podman work.
- The seed is ASCII decimal unsigned 32-bit input, matching WP-V1. Leading-zero input is
  normalized before it becomes the report identity or the value exported to WP-O2. The runner
  rejects an invalid seed, report basename, selected environment file, selected gateway port, or
  an existing unsafe sibling credential file before it creates a report directory or launches the
  stack.
- AUTO is the default build choice: it reuses only readable, non-symlink
  `dist/index.html` and `dist/main.js`; otherwise it invokes a build. `--build`
  always invokes a build. The launcher's reuse flag is an internal runner detail,
  not a public walkthrough option.
- The report is a private mode-0600 JSON file under the private mode-0700
  `test-results/ui_walkthrough/` directory. A caller may choose only a safe `.json` basename;
  traversal and symlink report paths are refused. Finalization revalidates every report component,
  recreates the private directory if Playwright cleared it, and uses descriptor-relative atomic
  replacement so a symlink replacement fails closed. The report records only `status`,
  `masterSeed`, and redacted lifecycle `stage`.
- The runner validates with `launch_local_stack.sh --check`, then starts that same selected
  environment with `--no-open` and the selected build behavior. It uses the launcher's
  `PLE_LAUNCH_TIMEOUT_SECONDS` behavior rather than adding a second timeout.
- Before and after launch, the runner derives and bounds-checks the gateway port with the
  launcher's exact precedence: a nonempty inherited `PLE_GATEWAY_HOST_PORT` overrides the
  selected env-file value, which otherwise falls back to 8080. It requires a non-symlink sibling
  `local-login.txt` with mode 0600 after launch and exports separate `PLE_UI_WALKTHROUGH_*` live
  variables only to WP-O2's child process.
- Cleanup runs only after this runner invoked the launcher. It uses the same selected environment
  and Compose provider for `down --remove-orphans`, never removes volumes, and `--keep` preserves
  the stack for diagnosis. Before report creation and again before launch, direct
  provider-independent Podman storage queries for both exact `containers` labels include stopped
  containers; either query failure or any matching ID fails closed. An inherited-over-env-file
  `COMPOSE_PROJECT_NAME` must be unset, empty, or exactly `containers`, so cleanup cannot claim a
  differently named stack.

## Scope boundary

WP-O1 does not read `containers/local-demo.json`, credentials, enrollment data, corpus data, or
browser output. It invokes only the WP-O2 smoke command:

```bash
bash run_playwright_tests.sh tests/playwright/ui_walkthrough_smoke.spec.ts
```

## Validation

The following checks passed without starting a stack:

```bash
bash -n tests/e2e/e2e_ui_walkthrough.sh
source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py
source source_me.sh && python3 tests/check_ascii_compliance.py -i tests/e2e/e2e_ui_walkthrough.sh
source source_me.sh && python3 tests/check_ascii_compliance.py -i tests/e2e/e2e_ui_walkthrough.py
```

The focused 18-test offline suite covers parser conflict rejection, AUTO/forced-build/reuse
behavior, preflight, cleanup without volumes, private redacted failure reports, and report-parent
deletion or symlink replacement. It also proves malformed seed, basename, port, environment bytes,
or credential permissions fail before report creation. Shell syntax, Python compilation, ASCII,
line-length, and diff-whitespace checks passed. The ordinary macOS execution sandbox denied
Chromium's Mach-port startup before a browser context opened; the runner failed closed with a
redacted `playwright_smoke` report. The elevated result above is the acceptance evidence, not a
product journey, authentication, enrollment, or content-arrangement claim.
