// Production-browser proof for the Student Assignment Attempt journey.
// Selector contract: student_courses_page.tsx:13-15 supplies the visible chooser entry whose
// public href binds the prepared Course reference; student_course_landing_page.tsx:42-47
// supplies its Assignment entry whose public href binds the prepared Assignment reference; and
// assignment_overview_page.tsx:91-117 supplies Start facts and the named Start control.
// assignment_attempt_page.tsx supplies the attempt navigation, native response, save, timer,
// final-submit, and terminal-status controls. The keyboard helper proves their real tab order
// rather than assigning focus to a control.

import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import path from "node:path";

import AxeBuilder from "@axe-core/playwright";
import { chromium } from "playwright";

import { REPO_ROOT } from "./repo_root.mjs";

const [port, course, assignment] = process.argv.slice(2);
if (
  !/^[0-9]+$/u.test(port ?? "") ||
  !/^C-[1-9][0-9]{0,9}$/u.test(course ?? "") ||
  !/^A-[1-9][0-9]{0,9}$/u.test(assignment ?? "")
) {
  throw new Error("expected fixed gateway port and public C-/A- Assignment references");
}

const origin = `https://localhost:${port}`;
const screenshotDirectory = path.join(REPO_ROOT, "test-results", "m5-live-assignment-attempt");
mkdirSync(screenshotDirectory, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  ignoreHTTPSErrors: true,
  viewport: { width: 1280, height: 800 },
});
const page = await context.newPage();
const pageErrors = [];
page.on("pageerror", (error) => pageErrors.push(error.message));

function attemptSurface() {
  return page.locator('[data-route-surface="assignmentAttempt"]');
}

async function assertNoSeriousOrCriticalAxeFindings(surface, description) {
  const axeResults = await new AxeBuilder({ page }).include(surface).analyze();
  const findings = axeResults.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      help: violation.help,
      targets: violation.nodes.map((node) => node.target.join(" ")),
    }));
  assert.deepEqual(findings, [], `${description} has serious or critical axe findings`);
}

function questionNavigation() {
  return page.getByRole("navigation", { name: "Assignment questions" });
}

function questionButton(position, state, current) {
  const currentSuffix = current ? ", current" : "";
  return questionNavigation().getByRole("button", {
    name: `Question ${position}: ${state}${currentSuffix}`,
    exact: true,
  });
}

async function hasKeyboardFocus(locator) {
  return locator.evaluate((element) => document.activeElement === element);
}

async function focusWithKeyboard(locator, description) {
  await locator.waitFor({ state: "visible" });
  for (let steps = 0; steps < 80; steps += 1) {
    if (await hasKeyboardFocus(locator)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Keyboard focus did not reach ${description} in reading order`);
}

async function expectKeyboardFocus(locator, description) {
  assert.equal(
    await hasKeyboardFocus(locator),
    true,
    `Keyboard focus did not remain on ${description}`,
  );
}

async function chooseAndSaveCurrentResponse({ manualSave }) {
  const responseControl = attemptSurface().locator("section.question-response-control");
  const firstChoice = responseControl.locator("input[type='radio']").first();
  await focusWithKeyboard(firstChoice, "the first response choice");
  await page.keyboard.press("Space");
  if (manualSave) {
    await responseControl
      .getByRole("status", { name: "Response format", exact: true })
      .getByText("Response format is ready to submit.", { exact: true })
      .waitFor();
    const save = responseControl.getByRole("button", { name: "Save response", exact: true });
    await focusWithKeyboard(save, "Save response");
    await page.keyboard.press("Enter");
  }
  await page.getByText("Response saved.", { exact: true }).waitFor();
}

async function assertNoVisibleUuid() {
  const visibleText = await attemptSurface().innerText();
  assert.equal(
    /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/iu.test(visibleText),
    false,
    "Student delivery surface exposed an internal UUID",
  );
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Okafor" }).click();

  await page.goto(`${origin}/?choose=1`, { waitUntil: "domcontentloaded" });
  const courseList = page.locator('[data-route-surface="studentCourses"]');
  await courseList.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  const openAssignedWork = courseList.locator(`a[href="/student/courses/${course}"]`);
  await openAssignedWork.waitFor({ state: "visible" });
  await assertNoSeriousOrCriticalAxeFindings(
    '[data-route-surface="studentCourses"]',
    "Student course chooser",
  );

  await openAssignedWork.click();
  await page.waitForURL(new RegExp(`${origin}/student/courses/${course}$`, "u"));
  const courseLanding = page.locator('[data-route-surface="studentCourseLanding"]');
  const openAssignment = courseLanding.locator(
    `a[href="/courses/${course}/assignments/${assignment}"]`,
  );
  await openAssignment.waitFor({ state: "visible" });
  await assertNoSeriousOrCriticalAxeFindings(
    '[data-route-surface="studentCourseLanding"]',
    "Student course landing",
  );

  await openAssignment.click();
  await page.waitForURL(new RegExp(`${origin}/courses/${course}/assignments/${assignment}$`, "u"));
  const assignmentOverview = page.locator('[data-route-surface="assignmentOverview"]');
  await assignmentOverview
    .getByRole("heading", { name: "Before you start", exact: true })
    .waitFor();
  await assertNoSeriousOrCriticalAxeFindings(
    '[data-route-surface="assignmentOverview"]',
    "Student Assignment Start",
  );

  const start = page.getByRole("button", { name: "Start Assignment", exact: true });
  await focusWithKeyboard(start, "Start Assignment");
  await page.keyboard.press("Enter");
  await page.waitForURL(new RegExp(`${origin}/assignment-attempts/R-[1-9][0-9]*$`, "u"));
  await attemptSurface().waitFor({ state: "visible" });
  await page.getByText("Question 1 of 2", { exact: true }).waitFor();
  await page
    .getByRole("timer")
    .filter({ hasText: /[0-9]+:[0-9]{2} remaining/u })
    .waitFor();
  await questionButton(1, "Not answered", true).waitFor();
  await questionButton(2, "Not answered", false).waitFor();
  await assertNoVisibleUuid();

  await focusWithKeyboard(questionButton(1, "Not answered", true), "Question 1 navigation");
  await page.keyboard.press("Tab");
  await expectKeyboardFocus(questionButton(2, "Not answered", false), "Question 2 navigation");
  await page.keyboard.press("Shift+Tab");
  await expectKeyboardFocus(questionButton(1, "Not answered", true), "Question 1 navigation");

  await assertNoSeriousOrCriticalAxeFindings(
    '[data-route-surface="assignmentAttempt"]',
    "Student Assignment Attempt",
  );

  await chooseAndSaveCurrentResponse({ manualSave: false });
  await questionButton(1, "Saved", true).waitFor();
  await page.screenshot({
    path: path.join(screenshotDirectory, "01-desktop-saved.png"),
    fullPage: true,
  });

  await page.reload({ waitUntil: "domcontentloaded" });
  await attemptSurface().waitFor({ state: "visible" });
  await questionButton(1, "Saved", false).waitFor();
  await questionButton(2, "Not answered", true).waitFor();

  await focusWithKeyboard(questionButton(1, "Saved", false), "Question 1 navigation");
  await page.keyboard.press("Enter");
  await page.getByText("Question 1 of 2", { exact: true }).waitFor();
  await attemptSurface().locator("input[type='radio']:checked").waitFor();
  await page.getByText("Response saved.", { exact: true }).waitFor();

  await focusWithKeyboard(questionButton(2, "Not answered", false), "Question 2 navigation");
  await page.keyboard.press("Enter");
  await page.getByText("Question 2 of 2", { exact: true }).waitFor();
  await chooseAndSaveCurrentResponse({ manualSave: true });
  await questionButton(2, "Saved", true).waitFor();

  await page.setViewportSize({ width: 768, height: 1024 });
  await attemptSurface().waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    true,
    "Student Assignment Attempt overflows at tablet width",
  );
  await page.screenshot({
    path: path.join(screenshotDirectory, "02-tablet-saved.png"),
    fullPage: true,
  });

  await focusWithKeyboard(questionButton(1, "Saved", false), "Question 1 navigation");
  await page.keyboard.press("Enter");
  await focusWithKeyboard(questionButton(2, "Saved", false), "Question 2 navigation");
  await page.keyboard.press("Enter");
  await page.getByText("Question 2 of 2", { exact: true }).waitFor();

  await page.setViewportSize({ width: 390, height: 844 });
  await questionNavigation().waitFor({ state: "visible" });
  await page.getByRole("timer").waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    true,
    "Student Assignment Attempt overflows at narrow width",
  );
  await page.screenshot({
    path: path.join(screenshotDirectory, "03-narrow-active.png"),
    fullPage: true,
  });

  const submit = page.getByRole("button", { name: "Submit Assignment", exact: true });
  await focusWithKeyboard(submit, "Submit Assignment");
  await page.keyboard.press("Enter");
  await page.getByRole("heading", { name: "Assignment submitted", exact: true }).waitFor();

  await attemptSurface().waitFor({ state: "visible" });
  await page.screenshot({
    path: path.join(screenshotDirectory, "03-narrow-submitted.png"),
    fullPage: true,
  });

  await page.reload({ waitUntil: "domcontentloaded" });
  await attemptSurface().waitFor({ state: "visible" });
  await page.getByRole("heading", { name: "Assignment submitted", exact: true }).waitFor();
  assert.equal(
    await page.getByRole("button", { name: "Start Assignment", exact: true }).count(),
    0,
  );
  await assertNoVisibleUuid();
  assert.deepEqual(pageErrors, []);
} finally {
  await context.close();
  await browser.close();
}
