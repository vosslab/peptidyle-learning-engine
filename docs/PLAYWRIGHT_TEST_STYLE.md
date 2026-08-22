# Playwright test style

House rules for writing Playwright browser tests. This doc sets the authoring
standard for new and revised browser tests in any repo that serves HTML: a
TypeScript game, a MkDocs-Material site, or any page-driven app. The tests are
always Node + Playwright even when the app itself is Python or Markdown.

Read this before writing a browser test. For install and run mechanics
(installing Playwright, running scripts, screenshots, PDF export), see the
`PLAYWRIGHT_USAGE.md` doc where it ships. For the fast unit lane and the e2e
folder layout, see the `PYTEST_STYLE.md` and `E2E_TESTS.md` docs, which land
beside this one in a consumer repo's docs/ folder.

Existing tests are evidence of what works, not a compliance checklist. Apply
this guide to new and revised tests; leave working tests in place.

## One execution model

PLE uses the Playwright test runner through one owner. `run_playwright_tests.sh` owns the build,
disposable HTTPS gateway, real services, environment handoff, scenario selection, and cleanup;
`playwright.config.ts` owns collection under `tests/playwright/e2e/`. Do not add a second browser
launcher, bare Chromium runner, test-only application, alternate browser client, or
alternate configuration. Browser-free decoder and serialization behavior belongs in narrow Node, Rust, or
pytest tests.

## File layout and naming

- Put browser tests under `tests/playwright/` at the repo root.
- Name scenario children after their product behavior and use `.spec.ts`.
- Prefix non-test helper files with `helper_` (`helper_server.mjs`) so they
  read as support, not tests. Reserve a bare leading underscore for deletable
  scratch: `_name` files match the hook's rm-allowed patterns and are treated
  as temporary.
- Import the propagated `tests/playwright/repo_root.mjs` anchor to resolve paths
  from the git root.
- Group all catalog-owned production journeys in `tests/playwright/e2e/`.

Keep every file that imports Playwright under `tests/playwright/`; that keeps
the browser tests out of the fast pytest lane.

## Load model

Test the shipped or rendered output over HTTP, so a passing test reflects what a
user actually receives.

- Build first, then serve the production `dist/` bundle through the disposable HTTPS gateway.
- The fixed owner supplies the exact origin and sanitized environment to `playwright.config.ts`.
- Run one worker serially; scenario isolation comes from a fresh fixed stack and unique namespace,
  not parallel browser processes or random compatibility projects.
- The owner starts and cleans the stack. A spec must not start a server, choose a port, or select
  authentication and transport settings.

The wrapper accepts the closed scenario selections and `--screenshots`; screenshots must use the
same wrapper. Do not invoke `npx playwright test` directly for application behavior.

## Selectors

Choose selectors that describe what the user sees, then fall back to app-state
attributes only where roles cannot reach.

- Reach first for accessible selectors that capture user intent: `getByRole`
  and `getByLabel`.
- Use domain `data-*` attributes (`data-item-id`, `data-phase`,
  `data-school-index`) for app or canvas state that accessibility APIs cannot
  express.
- Document a spec's selector contract in a header comment, citing the source
  `file:line` each selector depends on, so a UI change surfaces the coupling.

## Waiting

Wait for the state that proves the app is ready, so a test passes for the right
reason.

- Assert with the runner's web-first `expect(...)`, which auto-retries until the
  condition holds.
- Poll app or DOM state with `expect.poll`, `page.waitForFunction`, or
  `locator(...).waitFor({ state })`.
- Advance through real, visible clicks. A click that a user could perform is the
  point of a browser test: if the control is hidden or unreachable, the test
  should fail there.

## Assertions and pass/fail signaling

Assert visible behavior and app state, and signal pass or fail one way per file.

- In the runner, use web-first `expect(...)` for both assertions and readiness.
- In a library script, assert with `node:assert/strict` and throw on failure;
  use a single top-level `process.exit(1)` path when a script needs an explicit
  non-zero exit.
- Keep one signaling style within a file or workflow so a failure reads clearly.

## Setup idioms

- Seed only non-product browser state with `page.addInitScript(...)` when the scenario contract
  permits it. Product state (courses, assignments, invitations, runs, and submissions) is created
  through visible PLE controls.
- Capture diagnostics by subscribing to `page.on("console", ...)` and
  `page.on("pageerror", ...)` so console errors surface in the test output.
- Share setup through plain exported helper functions. These tests are small and
  repo-local, so simple helpers fit better than heavier abstractions.

## Screenshots and headless policy

- Run headless Chromium (`chromium.launch()` with no arguments defaults to
  headless).
- Write screenshots to `test-results/`, which is gitignored.
- Capture manifest-owned scenario steps through `capture_screenshots.sh`; do not create a second
  visual corpus or test-only screenshot lane.
- Sweep a matrix with `browser.newContext({ viewport, colorScheme })` when you
  need desktop and mobile across light and dark.

## Common pitfalls

Each row pairs a house default with the pitfall it replaces.

| Use this | Instead of | Why |
| --- | --- | --- |
| Web-first waits (`expect`, `expect.poll`, `waitForFunction`) | Fixed `waitForTimeout` sleeps | Sleeps flake as timing shifts; readiness waits are stable |
| Real visible clicks | Synthetic event dispatch on hidden nodes | A real click proves the control is reachable |
| Production `dist/` over the HTTPS gateway | Loading a raw file or test-only page | The gateway matches shipped auth, API, and Wasm behavior |
| `getByRole` / `getByLabel`, then `data-*` | `data-testid` hooks | Accessible selectors test user intent |
| One signaling style per file | Mixing `expect`, `assert`, and bare exits | Consistent signaling makes failures unambiguous |
| Behavior and visibility assertions | Pixel, elapsed-ms, or motion-magnitude checks | Behavioral checks stay deterministic |
| One repo-local server helper | A fresh `node:http` server in every file | One helper keeps MIME and path handling correct |

## Minimal good test examples

A runner test (`tests/playwright/smoke.spec.ts`), served by the config
`webServer` block:

```typescript
import { test, expect } from "@playwright/test";

test("smoke: the app boots and adds a row", async ({ page }) => {
	await page.goto("/");
	await expect(page.getByRole("heading", { name: "My App" })).toBeVisible();
	await page.getByRole("button", { name: "Add row" }).click();
	await expect(page.getByRole("row")).toHaveCount(1);
});
```

The canonical instructor scenario also asserts the visible `WebAssembly` runtime status. This is the
browser-Wasm proof; the Node/Rust decoder and serialization checks remain narrow browser-free unit
evidence and do not replace it.

Each production scenario loads the HTTPS gateway, selects accessible controls, waits for visible
behavior, and signals pass or fail through the shared owner.
