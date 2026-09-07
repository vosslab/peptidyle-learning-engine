// Production-browser proof for M11 Student Assignment Access and initial issue.

import { chromium } from "playwright";

const [port, course, assignment] = process.argv.slice(2);
if (
  !/^[0-9]+$/.test(port ?? "") ||
  !/^C-[1-9][0-9]{0,9}$/u.test(course ?? "") ||
  !/^A-[1-9][0-9]{0,9}$/u.test(assignment ?? "")
) {
  throw new Error("expected fixed gateway port and public C-/A- Assignment references");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Student" }).click();
  await page.goto(`${origin}/courses/${course}/assignments/${assignment}`, {
    waitUntil: "domcontentloaded",
  });
  await page.getByRole("button", { name: "Start Assignment" }).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("heading", { name: "Questions" }).waitFor();
  await page.getByRole("heading", { name: /^Question 1:/u }).waitFor();
  const presentation = page.locator(
    "section[data-route-surface='assignmentOverview'] article.question-presentation",
  );
  if (
    (await presentation.getByRole("button", { name: /Submit answer|submit assignment|grade|feedback/i }).count()) !==
    0
  ) {
    throw new Error("M11 initial presentation exposed an M13 submission, grading, or feedback control");
  }
} finally {
  await context.close();
  await browser.close();
}
