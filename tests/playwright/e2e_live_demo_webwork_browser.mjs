// Disposable Chromium proof for the M14 answer-free WeBWorK render surface.

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
const protectedFields = /answer|correct|solution|grading|score|replay|source|webworkpgpath|questionattemptid|binding|checksum/iu;

try {
  const startResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/course-instances/${course}/assignments/${assignment}/start`) &&
      response.request().method() === "POST",
  );
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Student" }).click();
  await page.goto(`${origin}/courses/${course}/assignments/${assignment}`, {
    waitUntil: "domcontentloaded",
  });
  await page.getByRole("button", { name: "Start Assignment" }).click();
  const response = await startResponse;
  if (response.status() !== 201 || protectedFields.test(await response.text())) {
    throw new Error("WeBWorK start response crossed the private renderer boundary");
  }
  await page.getByRole("heading", { name: "Questions" }).waitFor();
  await page.getByRole("heading", { name: /^Question 1:/u }).waitFor();
  if ((await page.getByRole("radio").count()) < 2) {
    throw new Error("WeBWorK Question Presentation did not render its browser-safe response format");
  }
  if ((await page.getByText(/answer|correct|solution|grading|score/i).count()) !== 0) {
    throw new Error("WeBWorK render exposed an outcome or grader control");
  }
} finally {
  await context.close();
  await browser.close();
}
