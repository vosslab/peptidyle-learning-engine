// Browser evidence for the Student Course list, cross-Course home, and Course landing.

import assert from "node:assert/strict";

import AxeBuilder from "@axe-core/playwright";
import { chromium } from "playwright";

import { bundleStudentCourseEntryM6Harness } from "../support/student_course_entry_m6_loader.ts";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const STUDENT_WORKFLOW_SENTENCE =
  "Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.";
const STUDENT_KEYBOARD_SENTENCE =
  "Every Student browser action should be usable with the keyboard alone.";
const HEADING_ACTION_SENTENCE =
  "Use headings and action labels that reflect the current state and next useful step.";

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
  { historyFallback: true },
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

async function assertWorkflowFits(page, locator, viewportId) {
  await locator.waitFor({ state: "visible" });
  const box = await locator.boundingBox();
  const viewport = page.viewportSize();
  assert.ok(box !== null, `${viewportId}: ${STUDENT_WORKFLOW_SENTENCE}`);
  assert.ok(box.x >= -1, `${viewportId}: ${STUDENT_WORKFLOW_SENTENCE}`);
  assert.ok(box.x + box.width <= viewport.width + 1, `${viewportId}: ${STUDENT_WORKFLOW_SENTENCE}`);
  const hittable = await locator.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    if (x < 0 || y < 0 || x > window.innerWidth || y > window.innerHeight) return false;
    const hit = document.elementFromPoint(x, y);
    return hit === element || (hit !== null && element.contains(hit));
  });
  assert.equal(hittable, true, `${viewportId}: ${STUDENT_WORKFLOW_SENTENCE}`);
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1),
    true,
    `${viewportId}: ${STUDENT_WORKFLOW_SENTENCE}`,
  );
}

async function activateWithKeyboard(page, name) {
  await page.evaluate(() => {
    const active = document.activeElement;
    if (active instanceof HTMLElement) active.blur();
  });
  const seen = [];
  for (let step = 0; step < 40; step += 1) {
    await page.keyboard.press("Tab");
    const current = await page.evaluate(() => {
      const element = document.activeElement;
      if (!(element instanceof HTMLElement) || element === document.body) return null;
      return {
        tag: element.tagName,
        label: (element.getAttribute("aria-label") ?? element.innerText ?? "")
          .replace(/\s+/gu, " ")
          .trim(),
      };
    });
    if (current === null) continue;
    seen.push(`${current.tag}:${current.label}`);
    if (!["A", "BUTTON", "INPUT", "SELECT", "TEXTAREA", "SUMMARY"].includes(current.tag)) {
      throw new Error(
        `${STUDENT_KEYBOARD_SENTENCE} Focus landed on ${current.tag} ${current.label}`,
      );
    }
    if (current.label === name) {
      await page.keyboard.press("Enter");
      return;
    }
  }
  throw new Error(
    `${STUDENT_KEYBOARD_SENTENCE} Tab did not reach ${name}. Saw ${seen.join(" | ")}`,
  );
}

async function activatePopupWithKeyboard(context, page, name, pageErrors) {
  const popupPromise = context.waitForEvent("page");
  await activateWithKeyboard(page, name);
  const popup = await popupPromise;
  popup.on("pageerror", (error) => pageErrors.push(error.message));
  await popup.waitForLoadState();
  return popup;
}

async function assertCourseworkRows(page) {
  const coursework = page.getByRole("list", { name: "Available Coursework", exact: true });
  const rows = coursework.getByRole("listitem");
  const expectedRows = [
    ["Protein structure practice", "Resume Weekly Assignment", "In progress"],
    ["Bonus protein challenge", "Review Bonus Assignment", "Completed"],
    ["Peptide quiz", "Open Quiz", "Upcoming"],
  ];
  assert.equal(await rows.count(), expectedRows.length);
  for (const [index, [title, action, stateLabel]] of expectedRows.entries()) {
    const row = rows.nth(index);
    await row.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible" });
    await row.getByRole("link", { name: action, exact: true }).waitFor({ state: "visible" });
    const status = row.locator(".record-list__fact").filter({ hasText: "Status:" });
    await status.getByText(stateLabel, { exact: true }).waitFor({ state: "visible" });
    assert.equal(
      await row.getByRole("heading", { name: title, exact: true }).isVisible(),
      true,
      HEADING_ACTION_SENTENCE,
    );
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
  assert.equal(await page.locator("[data-record-id]").count(), 0);
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student/courses");

  await page.goto(`${origin}?mode=one`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  const course = page.getByRole("listitem").filter({
    has: page.getByRole("heading", {
      name: "Biochemistry 301: Proteins and Peptides",
      exact: true,
    }),
  });
  await course
    .getByRole("heading", { name: "Biochemistry 301: Proteins and Peptides", exact: true })
    .waitFor({
      state: "visible",
    });
  const openCourse = course.getByRole("link", { name: "Open Course", exact: true });
  await openCourse.waitFor({
    state: "visible",
  });
  assert.equal(await openCourse.getAttribute("href"), "/student/courses/CI7K3M2QAZ");
  assert.equal(await page.locator("[data-record-id]").count(), 1);
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student/courses");

  await page.goto(`${origin}?mode=many`);
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("link", { name: "Open Course", exact: true }).first().waitFor({
    state: "visible",
  });
  assert.equal(await page.locator("[data-record-id]").count(), 2);
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student/courses");

  await page.goto(`${origin}?mode=home`);
  await page.getByRole("heading", { name: "All Coursework", exact: true }).waitFor({
    state: "visible",
  });
  await page
    .getByRole("heading", {
      name: "BCHM 301: Biochemistry 301: Proteins and Peptides",
      exact: true,
    })
    .waitFor({ state: "visible" });
  await page
    .getByRole("heading", { name: "BIOL 302: Molecular Genetics: Gene Regulation", exact: true })
    .waitFor({ state: "visible" });
  assert.equal(await page.locator("[data-m6-location]").textContent(), "/student");

  await page.goto(`${origin}?mode=landing`);
  const landingCoursework = page.getByRole("list", { name: "Available Coursework", exact: true });
  const landingAssessment = landingCoursework.getByRole("listitem").filter({
    has: page.getByRole("heading", { name: "Protein structure practice", exact: true }),
  });
  const landingDue = landingAssessment.locator("time");
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
  assert.equal(await landingDue.getAttribute("datetime"), "2026-09-16T22:00:00.000Z");
  const laptop = CANONICAL_VIEWPORTS.laptop;
  const tablet = CANONICAL_VIEWPORTS.tablet;
  const phone = CANONICAL_VIEWPORTS.phone;
  const square = CANONICAL_VIEWPORTS.square;
  assert.ok(laptop.width >= 1280 && laptop.width > laptop.height, STUDENT_WORKFLOW_SENTENCE);
  assert.ok(tablet.height > tablet.width, STUDENT_WORKFLOW_SENTENCE);
  assert.ok(phone.width <= 400 && phone.height > phone.width, STUDENT_WORKFLOW_SENTENCE);
  assert.equal(square.width, square.height, STUDENT_WORKFLOW_SENTENCE);
  for (const [viewportId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize(viewport);
    await page.goto(`${origin}?mode=one`);
    await assertWorkflowFits(
      page,
      page.getByRole("link", { name: "Open Course", exact: true }),
      viewportId,
    );
    await page.goto(`${origin}?mode=home`);
    await page
      .getByRole("heading", { name: "All Coursework", exact: true })
      .waitFor({ state: "visible" });
    await assertWorkflowFits(
      page,
      page.getByRole("link", { name: "Resume Weekly Assignment", exact: true }),
      viewportId,
    );
    await page.goto(`${origin}?mode=landing`);
    await assertCourseworkRows(page);
    await assertWorkflowFits(
      page,
      page.getByRole("link", { name: "Resume Weekly Assignment", exact: true }),
      viewportId,
    );
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
  const resumedAttemptPage = await activatePopupWithKeyboard(
    context,
    page,
    "Resume Weekly Assignment",
    pageErrors,
  );
  await resumedAttemptPage
    .getByText("Active Attempt destination", { exact: true })
    .waitFor({ state: "visible", timeout: 5_000 })
    .catch(async (error) => {
      throw new Error(
        `Resume destination failed: location=${await resumedAttemptPage.locator("[data-m6-location]").textContent()}; errors=${pageErrors.join(" | ")}; body=${await resumedAttemptPage.locator("body").innerText()}`,
        { cause: error },
      );
    });
  assert.equal(
    await resumedAttemptPage.locator("[data-m6-location]").textContent(),
    "/courses/CI7K3M2QAZ/attempt",
  );
  assert.equal(
    await resumedAttemptPage.locator("[data-m6-history-state]").textContent(),
    '{"assessmentAttemptId":"00000000-0000-0000-0000-000000000006"}',
  );
  await resumedAttemptPage.close();

  const bonusAssessmentPage = await activatePopupWithKeyboard(
    context,
    page,
    "Review Bonus Assignment",
    pageErrors,
  );
  await bonusAssessmentPage
    .getByRole("heading", { name: "Previous attempts", exact: true })
    .waitFor({ state: "visible" });
  await bonusAssessmentPage
    .getByRole("button", { name: "Start Bonus Assignment", exact: true })
    .waitFor({ state: "visible" });
  const previousAttempts = bonusAssessmentPage
    .getByRole("list", { name: "Previous attempts", exact: true })
    .getByRole("listitem");
  assert.equal(await previousAttempts.count(), 2);
  const newestAttempt = previousAttempts.nth(0);
  await newestAttempt.getByRole("heading", { name: "Attempt 2", exact: true }).waitFor({
    state: "visible",
  });
  await newestAttempt.getByText("Closed", { exact: true }).waitFor({ state: "visible" });
  await newestAttempt.getByText("Score pending", { exact: true }).waitFor({
    state: "visible",
  });
  await newestAttempt.getByRole("button", { name: "Review Attempt", exact: true }).waitFor();
  await newestAttempt.getByRole("button", { name: "Review Attempt", exact: true }).click();
  await bonusAssessmentPage.getByText("Attempt summary destination", { exact: true }).waitFor();
  assert.equal(
    await bonusAssessmentPage.locator("[data-m6-location]").textContent(),
    "/courses/CI7K3M2QAZ/review",
  );
  assert.equal(
    await bonusAssessmentPage.locator("[data-m6-history-state]").textContent(),
    '{"assessmentAttemptId":"00000000-0000-0000-0000-000000000007"}',
  );
  await bonusAssessmentPage.close();
  const releasedAssessmentPage = await activatePopupWithKeyboard(
    context,
    page,
    "Review Bonus Assignment",
    pageErrors,
  );
  await releasedAssessmentPage
    .getByRole("heading", { name: "Previous attempts", exact: true })
    .waitFor();
  const releasedAttempt = releasedAssessmentPage
    .getByRole("list", { name: "Previous attempts", exact: true })
    .getByRole("listitem")
    .nth(1);
  await releasedAttempt.getByRole("heading", { name: "Attempt 1", exact: true }).waitFor({
    state: "visible",
  });
  await releasedAttempt.getByText("Submitted", { exact: true }).waitFor({ state: "visible" });
  await releasedAttempt.getByText("3 of 8 points", { exact: true }).waitFor({ state: "visible" });
  await releasedAttempt.getByRole("button", { name: "Review Attempt", exact: true }).waitFor();
  await activateWithKeyboard(releasedAssessmentPage, "Start Bonus Assignment");
  await releasedAssessmentPage
    .getByText("Active Attempt destination", { exact: true })
    .waitFor({ state: "visible" });
  await releasedAssessmentPage.close();

  const quizPage = await activatePopupWithKeyboard(context, page, "Open Quiz", pageErrors);
  await quizPage.locator('[data-route-surface="assessmentOverview"]').waitFor({ state: "visible" });
  assert.deepEqual(await criticalOrSeriousViolations(quizPage), []);
  const overviewDueText = await quizPage.locator("[data-assessment-decision-due]").textContent();
  assert.equal(overviewDueText, landingDueText);
  await quizPage
    .getByRole("status")
    .filter({ hasText: "Cannot start" })
    .waitFor({ state: "visible" });
  assert.equal(await quizPage.getByRole("button", { name: /^Start /u }).count(), 0);
  await quizPage.close();

  await page.goto(`${origin}?mode=landing`);
  await page.getByRole("link", { name: "Your Courses", exact: true }).waitFor({ state: "visible" });
  await activateWithKeyboard(page, "Your Courses");
  await page.waitForFunction(
    () => document.querySelector("[data-m6-location]")?.textContent === "/student/courses",
  );
  await page
    .getByRole("heading", { name: "Your courses", exact: true })
    .waitFor({ state: "visible" });
  const coursePage = await activatePopupWithKeyboard(context, page, "Open Course", pageErrors);
  await coursePage
    .getByRole("heading", { name: "Biochemistry 301: Proteins and Peptides", exact: true })
    .waitFor({ state: "visible" });
  await coursePage.close();
  assert.deepEqual(pageErrors, []);
} finally {
  await context.close();
  await browser.close();
  await harnessServer.close();
}
