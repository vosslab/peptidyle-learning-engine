// Browser component evidence for src/pages/student_courses_page.tsx and
// src/pages/student_course_landing_page.tsx. The harness controls only the
// server-owned current-Course projection; navigation uses real visible links.

import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";

import AxeBuilder from "@axe-core/playwright";
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
const context = await browser.newContext();
async function criticalOrSeriousViolations(page) {
  const result = await new AxeBuilder({ page }).include("#root").analyze();
  return result.violations
    .filter((violation) => violation.impact === "critical" || violation.impact === "serious")
    .map((violation) => violation.id);
}
try {
  const page = await context.newPage();
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
    () =>
      document.querySelector("[data-m6-location]")?.textContent === "/student/courses/CI7K3M2QAZ",
  );

  await page.goto(`${origin}/?mode=choose`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open assigned work", exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student?choose=1");

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
  const landingDue = page.locator("[data-assessment-decision-due]").first();
  await landingDue.waitFor({ state: "visible" }).catch(async (error) => {
    throw new Error(
      `Student Assessment landing did not render: ${pageErrors.join(" | ")}\n${await page.locator("body").innerText()}`,
      { cause: error },
    );
  });
  assert.deepEqual(await criticalOrSeriousViolations(page), []);
  const landingDueText = await landingDue.textContent();
  assert.notEqual(landingDueText, null);
  const regularCard = page.getByRole("article").filter({
    has: page.getByRole("heading", { name: "Protein structure practice", exact: true }),
  });
  await regularCard
    .getByText("1 of 4 responses saved", { exact: true })
    .waitFor({ state: "visible" });
  assert.equal(await regularCard.getByText(/questions graded|Assessment score/u).count(), 0);
  const bonusCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: "Bonus protein challenge", exact: true }) });
  await bonusCard.getByText("Bonus Assignment", { exact: true }).waitFor({ state: "visible" });
  await bonusCard
    .getByText("2 of 2 questions graded · Assessment score 3 / 0", { exact: true })
    .waitFor({ state: "visible" });
  const withheldCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: "Peptide quiz", exact: true }) });
  assert.equal(await withheldCard.getByText(/Assessment score/u).count(), 0);
  assert.equal(await page.getByText(/Score so far/u).count(), 0);
  assert.deepEqual(await criticalOrSeriousViolations(page), []);
  for (const width of [1280, 390]) {
    await page.setViewportSize({ width, height: 900 });
    for (const name of ["Resume Regular Assignment", "Review Bonus Assignment", "Open Quiz"]) {
      await page.getByRole("link", { name, exact: true }).waitFor({ state: "visible" });
    }
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
    );
  }
  await page.getByRole("link", { name: "Resume Regular Assignment", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page
    .getByText("Active Attempt destination", { exact: true })
    .waitFor({ state: "visible", timeout: 5_000 })
    .catch(async (error) => {
      throw new Error(
        `Resume destination failed: location=${await page.locator("[data-m6-location]").textContent()}; errors=${pageErrors.join(" | ")}; body=${await page.locator("body").innerText()}`,
        { cause: error },
      );
    });
  assert.equal(
    await page.locator("[data-m6-location]").textContent(),
    "/assessment-attempts/00000000-0000-0000-0000-000000000006",
  );

  await page.goto(`${origin}/?mode=landing`);
  await page.getByRole("link", { name: "Review Bonus Assignment", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page
    .getByRole("heading", { name: "Previous attempts", exact: true })
    .waitFor({ state: "visible" });
  await page
    .getByRole("button", { name: "Start Bonus Assignment", exact: true })
    .waitFor({ state: "visible" });
  assert.equal(
    await page.getByRole("link", { name: "Attempt 1", exact: true }).getAttribute("href"),
    "/assessment-attempts/00000000-0000-0000-0000-000000000005/summary",
  );

  await page.goto(`${origin}/?mode=landing`);
  await page.getByRole("link", { name: "Open Quiz", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page.locator('[data-route-surface="assessmentOverview"]').waitFor({ state: "visible" });
  assert.deepEqual(await criticalOrSeriousViolations(page), []);
  const overviewDueText = await page.locator("[data-assessment-decision-due]").textContent();
  assert.equal(overviewDueText, landingDueText);
  await page.getByText("Can start", { exact: true }).waitFor({ state: "visible" });
  const timeLimit = page.getByText("1 hour per attempt", { exact: true });
  const startButton = page.getByRole("button", {
    name: "Start Quiz",
    exact: true,
  });
  assert.equal(
    await timeLimit.evaluate(
      (element, button) =>
        (element.compareDocumentPosition(button) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0,
      await startButton.elementHandle(),
    ),
    true,
  );

  await page.goto(`${origin}/?mode=landing`);
  await page.getByRole("link", { name: "Your courses", exact: true }).waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Your courses", exact: true }).click();
  await page.waitForFunction(
    () => document.querySelector("[data-m6-location]")?.textContent === "/student?choose=1",
  );
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  assert.deepEqual(pageErrors, []);
} finally {
  await context.close();
  await browser.close();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
