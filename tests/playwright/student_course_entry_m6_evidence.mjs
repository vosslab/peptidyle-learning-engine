// Browser component evidence for src/pages/student_courses_page.tsx and
// src/pages/student_course_landing_page.tsx. The harness controls only the
// server-owned current-Course projection; navigation uses real visible links.

import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";

import { chromium } from "playwright";

import { bundleStudentCourseEntryM6Harness } from "../support/student_course_entry_m6_loader.ts";

const bundle = await bundleStudentCourseEntryM6Harness();
const server = createServer((request, response) => {
  if (request.url?.startsWith("/student_course_entry_m6_harness.js")) {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(bundle.javascript);
    return;
  }
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(`<!doctype html><body><div id="root"></div><script type="module">
    import { mountStudentCourseEntryM6Harness } from "/student_course_entry_m6_harness.js";
    const mode = new URLSearchParams(window.location.search).get("mode");
    window.studentCourseEntryM6 = mountStudentCourseEntryM6Harness(document.querySelector("#root"), mode);
  </script>`);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string")
  throw new Error("Student Course entry evidence server did not bind TCP.");

const origin = `http://127.0.0.1:${String(address.port)}`;
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`${origin}/?mode=zero`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByText("You do not have any current courses.", { exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(await page.getByRole("article").count(), 0);

  await page.goto(`${origin}/?mode=one`);
  await page.locator("[data-m6-location]").waitFor({ state: "visible" });
  await page.waitForFunction(
    () => document.querySelector("[data-m6-location]")?.textContent === "/student/courses/C-1",
  );

  await page.goto(`${origin}/?mode=choose`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open assigned work", exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/?choose=1");

  await page.goto(`${origin}/?mode=many`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open assigned work", exact: true }).first().waitFor({
    state: "visible",
  });
  assert.equal(await page.getByRole("article").count(), 2);
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/");

  await page.goto(`${origin}/?mode=landing`);
  await page.getByRole("link", { name: "Your courses", exact: true }).waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Your courses", exact: true }).click();
  await page.waitForFunction(
    () => document.querySelector("[data-m6-location]")?.textContent === "/?choose=1",
  );
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  assert.deepEqual(pageErrors, []);
} finally {
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
