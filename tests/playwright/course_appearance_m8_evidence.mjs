// Banner browser evidence: changes stay local until saved, independently of a theme draft.

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
  throw new Error("Course Appearance banner evidence server did not bind TCP.");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.addInitScript(() => {
    window.revokedBannerUrls = [];
    const revoke = URL.revokeObjectURL.bind(URL);
    URL.revokeObjectURL = (url) => {
      window.revokedBannerUrls.push(url);
      revoke(url);
    };
  });
  await page.goto(`http://127.0.0.1:${String(address.port)}/`);
  await page.waitForSelector('[data-route-surface="courseAppearance"]');
  const file = page.locator("[data-course-banner-file]");
  await page.getByRole("radio", { name: "Ocean" }).check();
  await file.setInputFiles({
    name: "banner.png",
    mimeType: "image/png",
    buffer: Buffer.from("not-an-image"),
  });
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerUploadCalls()), 0);
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerSetCalls()), 0);
  const local = page.locator("[data-course-banner-local-preview]");
  await local.waitFor({ state: "visible" });
  await page.getByRole("button", { name: "Save theme" }).click();
  await page.evaluate(() =>
    window.courseAppearanceM7.resolveSave({ theme: "ocean", banner: null }),
  );
  await page.getByText("Theme saved.", { exact: true }).waitFor({ state: "visible" });
  assert.equal(await local.count(), 1);
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerUploadCalls()), 0);
  await page.getByRole("radio", { name: "Magma" }).check();
  await page.getByRole("radio", { name: "Informative" }).check();
  await page.locator("[data-course-banner-alt]").fill("Microscope and protein structure");
  await page.getByRole("button", { name: "Save banner" }).click();
  await page.getByText("Banner saved.", { exact: true }).waitFor({ state: "visible" });
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerUploadCalls()), 1);
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerSetCalls()), 1);
  assert.equal(
    await page.locator(".course-theme-scope").getAttribute("data-course-theme"),
    "magma",
  );
  assert.equal(await page.getByRole("radio", { name: "Magma" }).isChecked(), true);
  await page.locator("[data-course-banner-saved-preview]").waitFor({ state: "visible" });
  await page.getByRole("button", { name: "Save theme" }).click();
  await page.getByText("Theme saved.", { exact: true }).waitFor({ state: "visible" });
  await page.getByRole("button", { name: "Replace banner" }).waitFor({ state: "visible" });
  await page.getByRole("radio", { name: "Ocean" }).check();
  await page.getByRole("button", { name: "Remove banner" }).click();
  await page.getByText("Banner removed.", { exact: true }).waitFor({ state: "visible" });
  assert.equal(await page.evaluate(() => window.courseAppearanceM7.bannerRemoveCalls()), 1);
  assert.equal(await page.locator("[data-course-banner-saved-preview]").count(), 0);
  assert.equal(
    await page.locator(".course-theme-scope").getAttribute("data-course-theme"),
    "ocean",
  );
  assert.equal(await page.getByRole("radio", { name: "Ocean" }).isChecked(), true);
  await page.evaluate(() => window.courseAppearanceM7.hidePage());
  await page.waitForSelector("[data-m7-page-removed]");
  assert.ok((await page.evaluate(() => window.revokedBannerUrls.length)) >= 1);
} finally {
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
