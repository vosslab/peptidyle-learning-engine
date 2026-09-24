// Browser component evidence for src/pages/student_courses_page.tsx and
// src/pages/student_course_landing_page.tsx. The harness controls only the
// server-owned current-Course projection; navigation uses real visible links.

import assert from "node:assert/strict";

import AxeBuilder from "@axe-core/playwright";
import { chromium } from "playwright";

import { bundleStudentCourseEntryM6Harness } from "../support/student_course_entry_m6_loader.ts";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const bundle = await bundleStudentCourseEntryM6Harness();
const harnessServer = await startHarnessServer(
  `<!doctype html><head><style>${bundle.stylesheet}</style></head><body><div id="root"></div><script type="module">
    import { mountStudentCourseEntryM6Harness } from "/student_course_entry_m6_harness.js";
    const mode = new URLSearchParams(window.location.search).get("mode");
    window.studentCourseEntryM6 = mountStudentCourseEntryM6Harness(document.querySelector("#root"), mode);
  </script>`,
  bundle.stylesheet,
  new Map([
    [
      "/student_course_entry_m6_harness.js",
      { body: bundle.javascript, contentType: "text/javascript; charset=utf-8" },
    ],
  ]),
);
const origin = harnessServer.evidenceUrl;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
async function criticalOrSeriousViolations(page) {
  const result = await new AxeBuilder({ page }).include("#root").analyze();
  return result.violations
    .filter((violation) => violation.impact === "critical" || violation.impact === "serious")
    .map((violation) => violation.id);
}

async function assertCourseworkRows(page) {
  const coursework = page.getByRole("list", { name: "Available Coursework", exact: true });
  const rows = coursework.getByRole("listitem");
  const expectedRows = [
    ["Protein structure practice", "Resume Regular Assignment"],
    ["Bonus protein challenge", "Review Bonus Assignment"],
    ["Peptide quiz", "Open Quiz"],
  ];
  assert.equal(await rows.count(), expectedRows.length);
  for (const [index, [title, action]] of expectedRows.entries()) {
    const row = rows.nth(index);
    await row.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible" });
    await row.getByRole("link", { name: action, exact: true }).waitFor({ state: "visible" });
  }
}
try {
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`${origin}?mode=zero`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByText("You do not have any current courses.", { exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(await page.getByRole("article").count(), 0);

  await page.goto(`${origin}?mode=one`);
  await page.locator("[data-m6-location]").waitFor({ state: "attached" });
  await page.waitForFunction(
    () =>
      document.querySelector("[data-m6-location]")?.textContent ===
      "/student/courses/CI7K3M2QAZ/progress",
  );

  await page.goto(`${origin}?mode=choose`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open Progress", exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student?choose=1");

  await page.goto(`${origin}?mode=many`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open Progress", exact: true }).first().waitFor({
    state: "visible",
  });
  assert.equal(await page.getByRole("article").count(), 2);
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/");

  await page.goto(`${origin}?mode=landing`);
  const landingDue = page.locator("[data-assessment-decision-due]").first();
  await landingDue.waitFor({ state: "visible" }).catch(async (error) => {
    throw new Error(
      `Student Assessment landing did not render: ${pageErrors.join(" | ")}\n${await page.locator("body").innerText()}`,
      { cause: error },
    );
  });
  assert.deepEqual(await criticalOrSeriousViolations(page), []);
  assert.equal(
    await page.locator("[data-m6-location]").isVisible(),
    false,
    "Landing fixture route probe is not visible",
  );
  const landingDueText = await landingDue.textContent();
  assert.notEqual(landingDueText, null);
  for (const [viewportId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize(viewport);
    await assertCourseworkRows(page);
    assert.equal(
      await page.locator('[data-ribbon-row="top"]').getByText("BCHM 301", { exact: true }).count(),
      0,
      `${viewportId}: Student Ribbon top row does not repeat the course short name`,
    );
    await page
      .locator(".page-frame__header")
      .getByRole("heading", { name: "Biochemistry 301: Proteins and Peptides", exact: true })
      .waitFor({ state: "visible" });
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
      `${viewportId}: Coursework scan rows do not create horizontal overflow`,
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

  await page.goto(`${origin}?mode=landing`);
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

  await page.goto(`${origin}?mode=landing`);
  await page.getByRole("link", { name: "Open Quiz", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page.locator('[data-route-surface="assessmentOverview"]').waitFor({ state: "visible" });
  assert.deepEqual(await criticalOrSeriousViolations(page), []);
  const overviewDueText = await page.locator("[data-assessment-decision-due]").textContent();
  assert.equal(overviewDueText, landingDueText);
  await page.getByRole("status").filter({ hasText: "Cannot start" }).waitFor({ state: "visible" });
  assert.equal(await page.getByRole("button", { name: /^Start /u }).count(), 0);

  await page.goto(`${origin}?mode=landing`);
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
  await harnessServer.close();
}
