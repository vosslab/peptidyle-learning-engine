# WP-O2 live Playwright review

## Verdict

ACCEPTED.

WP-O2 accepts a narrowly validated live configuration without changing the normal
offline preview behavior. The required real-stack gateway gate passed on
2026-08-11 with the approved macOS browser and Podman access.

## Findings

No code findings remain in the WP-O2 scope.

`liveModeActivationFromEnvironment` validates both exact live switches before
either credential parser can read a file. Unset, empty, and `0` disable a mode;
only `1` enables it; other values fail. Simultaneous WebWork and walkthrough
modes fail during Playwright configuration loading. Either enabled mode disables
the mock preview server, while the offline default keeps it enabled.

The walkthrough parser accepts only an origin-root HTTP(S) URL. Cleartext HTTP
is limited to `127.0.0.1` and `localhost`, preserving an IPv4 loopback option
without requiring IPv6. It rejects credentials, paths, queries, fragments, and
non-loopback HTTP. The credential reader checks a regular, non-symlink,
mode-0600 file on non-Windows hosts and verifies exactly one valid `student=`
line without returning or logging its secret. The existing launcher credential
file has the expected redacted `instructor` and single `student` shape, regular
file type, and mode 0600. The decimal uint32 seed is validated and canonicalized.

The smoke is browser-driven and public: it opens `/health`, requires HTTP 200,
requires exact `{"status":"ready"}` body text, and verifies the final browser
origin. It does not read local-demo data, authenticate, inspect enrollment, or
reach product internals.

## Live acceptance

The requested command first ran in the ordinary execution sandbox on 2026-08-11:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

The launcher check and launcher start completed. The runner then invoked the
live-only smoke, which failed while Chromium launched with macOS error
`bootstrap_check_in ... MachPortRendezvousServer ... Permission denied (1100)`.
No browser context opened, so there was no `/health` response, exact JSON body,
or final-origin observation. The runner correctly recorded:

```json
{ "status": "FAIL", "masterSeed": 42, "stage": "playwright_smoke" }
```

This is an execution-sandbox permission boundary, not an application finding.

The same exact command then ran with the approved elevated macOS Chromium and
Podman access. It exited 0 after launcher check, launcher start, and the
live-only Playwright smoke completed. The browser smoke reached the public
IPv4 gateway origin, received HTTP 200, and verified the exact
`{"status":"ready"}` body. This also proves the live configuration disabled the
mock preview server. Playwright recorded a passed `.last-run.json` with no
failed tests, and the runner wrote exactly:

```json
{ "status": "PASS", "masterSeed": 42, "stage": "complete" }
```

The report directory was mode 0700, its report file was a regular mode-0600
file, and `podman ps --all --quiet` returned no containers after runner cleanup.
The runner preserved its ownership and redaction guarantees on both outcomes.

## Validation

Passed without Podman or a real browser stack:

```bash
PW_PORT=41999 npx playwright test \
  tests/playwright/ui_walkthrough_live_config.spec.ts \
  tests/playwright/ui_walkthrough_smoke.spec.ts --reporter=line
npx prettier --check playwright.config.ts \
  tests/playwright/live_mode_activation.ts \
  tests/playwright/ui_walkthrough_live_config.ts \
  tests/playwright/ui_walkthrough_live_config.spec.ts \
  tests/playwright/ui_walkthrough_smoke.spec.ts
npx tsc --ignoreConfig --noEmit --strict --noImplicitAny \
  --noUncheckedIndexedAccess --noImplicitOverride --verbatimModuleSyntax \
  --useUnknownInCatchVariables --noFallthroughCasesInSwitch --noImplicitReturns \
  --noUnusedLocals --noUnusedParameters --forceConsistentCasingInFileNames \
  --isolatedModules --esModuleInterop --skipLibCheck --target es2020 \
  --module esnext --moduleResolution bundler --lib es2020,dom,dom.iterable \
  --types node playwright.config.ts tests/playwright/live_mode_activation.ts \
  tests/playwright/ui_walkthrough_live_config.ts \
  tests/playwright/ui_walkthrough_live_config.spec.ts \
  tests/playwright/ui_walkthrough_smoke.spec.ts
PLE_UI_WALKTHROUGH_LIVE_REQUIRED=yes node --import tsx \
  node_modules/@playwright/test/cli.js test --list
PLE_UI_WALKTHROUGH_LIVE_REQUIRED=1 PLE_WEBWORK_LIVE_REQUIRED=1 \
  node --import tsx node_modules/@playwright/test/cli.js test --list
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i docs/active_plans/workstreams/wp_o2_live_playwright.md
git diff --check -- playwright.config.ts tests/playwright/live_mode_activation.ts \
  tests/playwright/ui_walkthrough_live_config.ts \
  tests/playwright/ui_walkthrough_live_config.spec.ts \
  tests/playwright/ui_walkthrough_smoke.spec.ts \
  docs/active_plans/workstreams/wp_o2_live_playwright.md
```

The focused Playwright command reported four passed and one deliberately
offline-skipped smoke. The two direct configuration commands both failed before
test listing: invalid walkthrough activation reported the exact-switch error,
and dual live activation reported the mutual-exclusion error. Formatting,
isolated strict TypeScript, ASCII, and whitespace checks passed.

## Acceptance result

WP-O2's real-stack gate is satisfied. M2 now has accepted runner and browser
smoke evidence; the later walkthrough packages retain their own acceptance
criteria.
