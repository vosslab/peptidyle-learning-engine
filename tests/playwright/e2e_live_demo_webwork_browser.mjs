// Disposable Chromium proof for the answer-free WeBWorK Assignment Attempt surface.
// Visible navigation selects the Student Course and Assignment, then the current live delivery
// lane redirects the active public Attempt into its one-question presentation.

import { chromium } from "playwright";

const [port, course, assignment] = process.argv.slice(2);
if (
  !/^[0-9]+$/u.test(port ?? "") ||
  !/^C-[1-9][0-9]{0,9}$/u.test(course ?? "") ||
  !/^A-[1-9][0-9]{0,9}$/u.test(assignment ?? "")
) {
  throw new Error("expected fixed gateway port and public C-/A- Assignment references");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();
const protectedFields =
  /\b(?:answer|correct|solution|grading|score|replay|source)\b|webworkpgpath|questionattemptid|binding|checksum/iu;

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Okafor" }).click();
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  const courseCard = page.getByRole("article").filter({
    has: page.locator(`a[href="/student/courses/${course}"]`),
  });
  await courseCard.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await page.waitForURL(`${origin}/student/courses/${course}`);
  const assignmentCard = page.getByRole("article").filter({
    has: page.locator(`a[href="/courses/${course}/assignments/${assignment}"]`),
  });
  await assignmentCard.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.waitForURL(`${origin}/courses/${course}/assignments/${assignment}`);
  await page.waitForURL(new RegExp(`${origin}/assignment-attempts/R-[1-9][0-9]*$`, "u"));
  const attempt = page.locator('[data-route-surface="assignmentAttempt"]');
  await attempt.waitFor({ state: "visible" });
  await page.getByText("Question 1 of 1", { exact: true }).waitFor();
  await page.getByRole("radio").nth(1).waitFor({ state: "visible" });
  const visibleText = await attempt.innerText();
  if (protectedFields.test(visibleText)) {
    throw new Error("WeBWorK Assignment Attempt crossed the private renderer boundary");
  }
} finally {
  await context.close();
  await browser.close();
}
