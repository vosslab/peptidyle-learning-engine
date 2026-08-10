# Playwright usage

Playwright drives Chromium, Firefox, and WebKit for browser tests, screenshots,
PDF output, and bounded browser automation. This guide covers the repository's
normal test and capture workflows. For durable test authoring rules, read
[PLAYWRIGHT_TEST_STYLE.md](PLAYWRIGHT_TEST_STYLE.md).

## Installation

This repository keeps Playwright in its Node development dependencies. From the
repository root, install dependencies and the required browser when setting up a
new checkout:

```bash
npm install
npx playwright install
```

Other repositories may provide a setup helper. Use the repository-owned helper
when one exists, because it may install the expected browsers and dependencies.

## Run browser tests

Run the maintained browser suite through the repository runner:

```bash
./run_playwright_tests.sh
```

The runner rebuilds `dist/` when its required artifacts are absent. Pass
`--build` to force a fresh build, and pass remaining arguments through to the
Playwright test runner:

```bash
./run_playwright_tests.sh --build
./run_playwright_tests.sh tests/playwright/course_appearance.spec.ts
```

The runner delegates server ownership to `playwright.config.ts`. The config
serves the built `dist/` application at `http://127.0.0.1:<port>/`; managed
tests use that HTTP URL through their configured `baseURL`. Do not start
`run_web_server.sh` for this suite.

## Maintained test rules

Tests under `tests/playwright/` exercise the shipped browser output, not raw
source files.

- Build first and serve `dist/` over HTTP through the config `webServer` block.
- Navigate with a configured relative URL such as `await page.goto("/")`.
- Let Playwright wait for server readiness through `webServer.url` and wait for
  application readiness with web-first assertions such as `expect(locator).toBeVisible()`.
- Use `expect.poll`, `page.waitForFunction`, or locator state waits when an
  explicit app-state condition is needed.
- Drive visible controls with accessible selectors, usually `getByRole` or
  `getByLabel`, and assert visible behavior or meaningful application state.

Do not use `file://` loading or fixed `page.waitForTimeout(...)` delays in
permanent tests. They bypass shipped HTTP behavior or make the test depend on
machine timing rather than a condition that proves readiness.

## Ad-hoc captures

For documentation or local debugging captures, use the repository's existing
capture script when it fits the target:

```bash
node tools/capture_readme_screenshot.mjs /tmp/peptidyle_assignment_overview.png
```

It starts a local HTTP preview of built `dist/`, waits for the intended visible
controls, and writes a disposable image. Keep such capture scripts outside the
permanent suite unless they assert durable user-visible behavior.

A `file://` URL is acceptable only for a one-off capture of a self-contained
static HTML file that has no fetches, routes, modules, or browser-security
requirements. It is not a test of the shipped app:

```javascript
import { chromium } from "playwright";
import path from "node:path";

const htmlPath = path.resolve("scratch.html");
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(`file://${htmlPath}`);
await page.screenshot({ path: "/tmp/scratch.png", fullPage: true });
await browser.close();
```

For any capture that depends on normal application loading, use an HTTP server
and wait for the visible page state before writing the screenshot.

## Common automation

### Take a screenshot

```javascript
await page.screenshot({
  path: "capture.png",
  fullPage: true,
});
```

### Generate a PDF

PDF generation requires Chromium:

```javascript
await page.pdf({
  path: "report.pdf",
  format: "Letter",
  printBackground: true,
});
```

### Evaluate page state

```javascript
const value = await page.evaluate(() => document.title);
```

### Wait for a condition

Wait for the state that makes the next operation valid:

```javascript
await page.getByRole("button", { name: "Start or resume practice" }).waitFor();
```

## Output locations

Choose output locations according to the purpose of the generated files.

| Purpose                 | Suggested location                   |
| ----------------------- | ------------------------------------ |
| Temporary test evidence | `test-results/`                      |
| Temporary debugging     | `/tmp/` or another ignored directory |
| Documentation assets    | `docs/screenshots/` by default       |
| Product assets          | Repository output location           |
| Reference images        | Repository asset folder              |

Follow established repository locations when they differ. Keep temporary
screenshots, recordings, traces, and local PDF output out of version control.

## Test locations

Browser tests live under `tests/playwright/`; longer walkthroughs may use its
`e2e/` subfolder. They remain separate from the fast pytest lane. For the full
test-tier layout, see [E2E_TESTS.md](E2E_TESTS.md).

## Troubleshooting

| Problem                       | Resolution                                                                      |
| ----------------------------- | ------------------------------------------------------------------------------- |
| Browser executable is missing | Run `npx playwright install`.                                                   |
| `node_modules/` is absent     | Run `npm install` from the repository root.                                     |
| A test server does not start  | Build with `./run_playwright_tests.sh --build` and inspect the server error.    |
| An element times out          | Verify its selector and wait for the state that makes it visible or actionable. |
| PDF generation fails          | Use Chromium and confirm the browser installation.                              |

## Related documentation

- [PLAYWRIGHT_TEST_STYLE.md](PLAYWRIGHT_TEST_STYLE.md)
- [E2E_TESTS.md](E2E_TESTS.md)
- [MARKDOWN_STYLE.md](MARKDOWN_STYLE.md)
- [REPO_STYLE.md](REPO_STYLE.md)
