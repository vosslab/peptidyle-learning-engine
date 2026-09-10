// Scope recovery evidence: Course Appearance exposes loading, failure, and a fresh retry.

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
    window.courseAppearanceM10 = mountCourseAppearanceM7Harness(document.querySelector("#root"), "pending");
  </script></body></html>`);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string")
  throw new Error("Course Appearance recovery evidence server did not bind TCP.");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(`http://127.0.0.1:${String(address.port)}/`);
  await page.waitForFunction(() => window.courseAppearanceM10 !== undefined);

  const surface = page.locator('[data-route-surface="courseAppearance"]');
  await page
    .getByRole("heading", { name: "Loading Course Appearance" })
    .waitFor({ state: "visible" });
  assert.equal(await surface.getAttribute("aria-busy"), "true");

  await page.evaluate(() => window.courseAppearanceM10.rejectScope());
  await page
    .getByRole("heading", { name: "Course Appearance could not load" })
    .waitFor({ state: "visible" });
  const alert = page.getByRole("alert");
  assert.match(await alert.textContent(), /could not be loaded/u);
  const retry = page.getByRole("button", { name: "Retry loading Course Appearance" });
  await retry.click();
  await page
    .getByRole("heading", { name: "Loading Course Appearance" })
    .waitFor({ state: "visible" });

  await page.evaluate(() => window.courseAppearanceM10.resolveScope());
  await page.getByRole("heading", { name: "Course Appearance" }).waitFor({ state: "visible" });
  assert.equal(await surface.getAttribute("aria-busy"), null);
  await page.getByRole("radio", { name: "Grass" }).waitFor({ state: "visible" });
  assert.deepEqual(pageErrors, []);
} finally {
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
