// Disposable Chromium proof for the M14 answer-free WeBWorK render surface.
// Visible navigation: student_courses_page.tsx selects C- by its Course Instance
// label; student_course_landing_page.tsx selects its rendered Assignment action.

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
  /answer|correct|solution|grading|score|replay|source|webworkpgpath|questionattemptid|binding|checksum/iu;

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Student" }).click();
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByText(`Course Instance ${course}`, { exact: true }) });
  await courseCard.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await page.waitForURL(`${origin}/student/courses/${course}`);
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("link", { name: "Open Assignment", exact: true }) });
  await assignmentCard.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.waitForURL(`${origin}/courses/${course}/assignments/${assignment}`);
  const startResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/course-instances/${course}/assignments/${assignment}/start`) &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Start Assignment" }).click();
  const response = await startResponse;
  if (response.status() !== 201 || protectedFields.test(await response.text())) {
    throw new Error("WeBWorK start response crossed the private renderer boundary");
  }
  await page.getByRole("heading", { name: "Questions" }).waitFor();
  await page.getByRole("heading", { name: /^Question 1:/u }).waitFor();
  if ((await page.getByRole("radio").count()) < 2) {
    throw new Error(
      "WeBWorK Question Presentation did not render its browser-safe response format",
    );
  }
  const outcomeControls = [
    page.getByRole("heading", { name: "Response received", exact: true }),
    page.getByRole("heading", { name: "Graded", exact: true }),
    page.getByRole("button", { name: "Check grading status", exact: true }),
  ];
  if (
    (await Promise.all(outcomeControls.map((control) => control.count()))).some(
      (count) => count !== 0,
    )
  ) {
    throw new Error("WeBWorK render exposed an outcome or grader control");
  }
} finally {
  await context.close();
  await browser.close();
}
