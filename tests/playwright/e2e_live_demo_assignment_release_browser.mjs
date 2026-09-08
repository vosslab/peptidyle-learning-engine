// Production-browser proof for visible Instructor Assignment creation and release.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const blueprintTitle = `Browser M10 Blueprint ${Date.now()}`;
const courseTitle = `Browser M10 Course ${Date.now()}`;
const assignmentTitle = `Browser M10 Assignment ${Date.now()}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

async function assertNoStudentResponseOrSubmissionControl() {
  for (const name of [/response/i, /submit/i, /start assignment/i, /student attempt/i]) {
    if ((await page.getByRole("button", { name }).count()) !== 0) {
      throw new Error("Assignment Preview exposed a Student response or submission control");
    }
  }
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);

  await page.getByRole("link", { name: "Blueprint Courses" }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course title").fill(blueprintTitle);
  await page.getByRole("button", { name: "Choose published Questions" }).click();
  await page.getByRole("heading", { name: "Choose the first reusable Questions" }).waitFor();
  await page.getByRole("button", { name: "Search questions" }).click();
  await page.locator(".question-picker-result input").first().check();
  await page.getByRole("button", { name: "Use selected Questions" }).click();
  await page.getByText("1 fixed Question selected in order.").waitFor();
  await page.getByRole("dialog").getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.waitForURL(/\/blueprint-courses\/BP-[1-9][0-9]*$/u);

  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page.waitForURL(`${origin}/`);
  await page.getByLabel("Blueprint Course Revision").selectOption({
    label: `${blueprintTitle} · Revision 1`,
  });
  await page.getByLabel("Course Instance title").fill(courseTitle);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create Course Instance" }).click();
  await page.getByRole("heading", { name: courseTitle }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance" }).first().click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);

  await page.getByRole("link", { name: "Open Assignments" }).click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/new$/u);
  await page.getByRole("heading", { name: "Create Assignment" }).waitFor();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByLabel("Instructions").fill("Complete the selected published Question.");
  await page.getByRole("button", { name: "Create Assignment" }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/release$/u,
  );
  await page.getByRole("heading", { name: "Assignment Workspace" }).waitFor();

  const availableQuestions = page.getByRole("group", { name: "Available Published Questions" });
  await availableQuestions.getByRole("checkbox").first().check();
  await page.getByLabel("Due date").fill("2026-12-01T12:00");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save Assignment" }).click();
  await page.getByText("Assignment saved. Validate it before release.").waitFor();
  await page.getByRole("button", { name: "Validate Assignment" }).click();
  await page
    .getByText(
      "Assignment validation passed. You can review the answer-free Assignment Preview or release it.",
    )
    .waitFor();
  await page.getByRole("button", { name: "Open Assignment Preview" }).click();
  await page.getByRole("heading", { name: "Assignment Preview" }).waitFor();
  await page
    .getByText("This answer-free preview does not create a Student attempt or access.")
    .waitFor();
  await assertNoStudentResponseOrSubmissionControl();
  await page.getByRole("button", { name: "Return to Assignment Workspace" }).click();
  await page.getByRole("heading", { name: "Assignment Preview" }).waitFor({ state: "hidden" });
  await page.getByRole("button", { name: "Release Assignment" }).click();
  await page.getByText("Released Assignment Revision 1.").waitFor();
  await page.getByText(/Released state · current edit [1-9][0-9]*/u).waitFor();
} finally {
  await context.close();
  await browser.close();
}
