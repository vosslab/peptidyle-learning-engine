// Theme browser evidence: saved selections persist and abandoned previews are released.
// Selector contract: CourseAppearancePage renders the route surface, native radios, and Save theme
// button. The rendered course scope publishes the selected preview theme.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

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
  throw new Error("Course Appearance theme evidence server did not bind TCP.");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(`http://127.0.0.1:${String(address.port)}/`);
  await page.waitForSelector('[data-route-surface="courseAppearance"]');
  await page.waitForFunction(() => window.courseAppearanceM7 !== undefined);

  const shellTheme = page.locator(".course-theme-scope");
  assert.equal(await shellTheme.getAttribute("data-course-theme"), "grass");
  await page.getByRole("radio", { name: "Ocean" }).check();
  await page.waitForFunction(
    () =>
      document.querySelector(".course-theme-scope")?.getAttribute("data-course-theme") === "ocean",
  );
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.saveCalls()), 0);

  await page.getByRole("button", { name: "Save theme" }).click();
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.saveCalls()), 1);
  await page.evaluate(() =>
    window.courseAppearanceM7.resolveSave({ theme: "ocean", banner: null }),
  );
  await page.getByText("Theme saved.", { exact: true }).waitFor({ state: "visible" });
  assert.equal(await shellTheme.getAttribute("data-course-theme"), "ocean");

  await page.getByRole("radio", { name: "Magma" }).check();
  await page.waitForFunction(
    () =>
      document.querySelector(".course-theme-scope")?.getAttribute("data-course-theme") === "magma",
  );
  await page.evaluate(() => window.courseAppearanceM7.hidePage());
  await page.locator("[data-m7-page-removed]").waitFor({ state: "visible" });
  assert.equal(await shellTheme.getAttribute("data-course-theme"), "ocean");
  assert.deepEqual(pageErrors, []);
} finally {
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
