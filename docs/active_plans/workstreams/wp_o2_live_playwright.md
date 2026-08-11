# WP-O2 live Playwright configuration

## Scope

- Package: WP-O2, validated live Playwright mode.
- Owner: Playwright/E2E engineer.
- Status: independently ACCEPTED; real-stack acceptance completed 2026-08-11.
- Files: `playwright.config.ts`, `tests/playwright/live_mode_activation.ts`,
  `tests/playwright/ui_walkthrough_live_config.ts`, and focused Playwright specs.

## Delivered contract

- `PLE_UI_WALKTHROUGH_LIVE_REQUIRED` is disabled only when unset, empty, or
  `0`; it is enabled only when exactly `1`.
- The pure activation helper rejects invalid values and simultaneous WebWork and
  walkthrough live modes before either credential parser reads a file.
- Walkthrough live mode validates an origin-only HTTP(S) URL, permits cleartext
  HTTP only for `127.0.0.1` or `localhost`, checks a regular
  non-symlink mode-0600 credential file on non-Windows hosts, requires exactly
  one valid `student=` line without returning it, and canonicalizes a decimal
  uint32 master seed.
- Either exact live mode disables the mock preview server. The walkthrough
  smoke is skipped offline and otherwise visits public same-origin `/health`,
  requires HTTP 200 and exact `{"status":"ready"}`, and checks the final
  browser origin.

## Evidence

- `npm ci --cache /private/tmp/peptidyle-npm-cache` succeeded without changing
  package files. The default npm cache was root-owned and failed before this
  scoped cache retry.
- Focused Prettier check passed.
- Isolated strict TypeScript check passed for the WP-O2 files.
- Focused Playwright selection reported four passed parser/config tests and one deliberately
  offline-skipped smoke after `bash build.sh` supplied the two dist outputs.
- Repository-wide `npx tsc -p tsconfig.lint.json` remains blocked by missing
  generated API modules outside this package.
- The exact runner command passed with elevated macOS Chromium and Podman access: it opened the
  public IPv4 `/health` origin, required HTTP 200 and exact `{"status":"ready"}`, and proved mock
  preview disabled. The runner report was PASS and its cleanup left no containers. See the
  independent [WP-O2 review](../audits/wp_o2_live_playwright_review.md).
- The same command under the ordinary execution sandbox hit macOS Chromium's browser-sandbox
  Mach-port denial before any browser context opened. That is environment evidence, not a product
  failure; the runner correctly failed closed.

## Completed gate

On 2026-08-11, `bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42` passed with the real
Podman gateway and mock preview disabled. M2 has accepted runner and browser-smoke evidence; this
does not claim a learner journey, authentication, enrollment, or content arrangement.
