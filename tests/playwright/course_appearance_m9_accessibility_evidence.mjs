// Accessibility evidence for the Course Appearance page in its compiled component harness.
// Selector contract: src/pages/course_appearance_page.tsx supplies the named native radios,
// file input, buttons, and live status; the page surface carries data-route-surface.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

import AxeBuilder from "@axe-core/playwright";
import { expect } from "@playwright/test";
import { chromium } from "playwright";

import { bundleCourseAppearanceM7Harness } from "../support/course_appearance_m7_loader.ts";

const bundle = await bundleCourseAppearanceM7Harness();
const css = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/styles/accessibility.css", import.meta.url), "utf8"),
].join("\n");
const server = createServer((request, response) => {
  if (request.url === "/course_appearance_m7_harness.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(bundle.javascript);
    return;
  }
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(`<!doctype html><html><head><style>${css}</style></head><body><div id="root"></div><script type="module">
    import { mountCourseAppearanceM7Harness } from "/course_appearance_m7_harness.js";
    window.courseAppearanceM7 = mountCourseAppearanceM7Harness(document.querySelector("#root"));
  </script></body></html>`);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string")
  throw new Error("Course Appearance accessibility evidence server did not bind TCP.");

const url = `http://127.0.0.1:${String(address.port)}/`;
const browser = await chromium.launch({ headless: true });
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(url);
  await page.locator('[data-route-surface="courseAppearance"]').waitFor({ state: "visible" });

  // Native keyboard selection changes the preview and exposes a textual selection name.
  const startingTheme = page.getByRole("radio", { name: "Ocean" });
  await startingTheme.check();
  await startingTheme.focus();
  await page.keyboard.press("ArrowRight");
  const selectedTheme = page.locator('input[name="course-theme"]:checked');
  assert.notEqual(await selectedTheme.inputValue(), await startingTheme.inputValue());
  assert.equal(
    await page.locator(".course-theme-scope").getAttribute("data-course-theme"),
    await selectedTheme.inputValue(),
  );
  await expect(selectedTheme).toHaveAccessibleName(/\S/);
  const selectedLabel = selectedTheme.locator("..").locator(".course-appearance-theme-label");
  await expect(selectedLabel).toBeVisible();
  await expect(selectedLabel).toHaveText(/\S/);

  // Keyboard activation opens the native chooser; Playwright supplies the chosen local file.
  const file = page.getByLabel("Banner image");
  await file.focus();
  assert.equal(await file.evaluate((element) => document.activeElement === element), true);
  const chooserPromise = page.waitForEvent("filechooser");
  await page.keyboard.press("Enter");
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: "course-banner.png",
    mimeType: "image/png",
    buffer: Buffer.from("browser-owned local preview fixture"),
  });
  await page.locator("[data-course-banner-local-preview]").waitFor({ state: "visible" });
  assert.equal(await page.getByRole("button", { name: "Save banner" }).isEnabled(), true);

  // The save result is announced, and a failed theme save announces an assertive recovery state.
  await page.getByRole("radio", { name: "Ocean" }).check();
  await page.getByRole("button", { name: "Save theme" }).click();
  await page.evaluate(() => window.courseAppearanceM7.rejectSave());
  const failedTheme = page.getByRole("alert");
  await failedTheme.waitFor({ state: "visible" });
  assert.equal(
    await failedTheme.textContent(),
    "Theme could not save. The saved theme is still displayed.",
  );
  assert.equal(
    await page.locator(".course-theme-scope").getAttribute("data-course-theme"),
    "grass",
  );

  await page.getByRole("radio", { name: "Ocean" }).check();
  await page.getByRole("button", { name: "Save theme" }).click();
  const savedTheme = page.getByRole("status");
  await savedTheme.waitFor({ state: "visible" });
  assert.equal(await savedTheme.textContent(), "Theme saved.");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('[data-route-surface="courseAppearance"]').waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    true,
  );
  await page.getByRole("button", { name: "Save banner" }).waitFor({ state: "visible" });

  const results = await new AxeBuilder({ page })
    .include('[data-route-surface="courseAppearance"]')
    .analyze();
  const serious = results.violations.filter(
    (violation) => violation.impact === "serious" || violation.impact === "critical",
  );
  assert.deepEqual(
    serious.map(
      (violation) =>
        `${violation.id}: ${violation.help} (${violation.nodes.map((node) => node.target.join(" ")).join(", ")})`,
    ),
    [],
  );
  assert.deepEqual(pageErrors, []);
  await context.close();
} finally {
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
