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

async function assertNoStudentResponseOrSubmissionControl(target) {
  for (const name of [/response/i, /submit/i, /start assignment/i, /student attempt/i]) {
    if ((await target.getByRole("button", { name }).count()) !== 0) {
      throw new Error("Assignment Preview exposed a Student response or submission control");
    }
  }
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);

  const coursesTab = page
    .getByRole("navigation", { name: "Ribbon tabs" })
    .getByRole("link", { name: "Courses", exact: true });
  if ((await coursesTab.getAttribute("aria-current")) !== "page") {
    await coursesTab.click();
    await page.waitForURL(`${origin}/`);
  }
  await page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
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

  await coursesTab.click();
  await page.waitForURL(`${origin}/`);
  await page.getByLabel("Blueprint Course Revision").selectOption({
    label: `${blueprintTitle} · Revision 1`,
  });
  await page.getByLabel("Course Instance title").fill(courseTitle);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance" }).click();
  await page.getByRole("heading", { name: courseTitle }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance" }).first().click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);

  await page.getByRole("link", { name: "Create Assignment" }).click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/new$/u);
  await page.getByRole("heading", { name: "Create an Assignment" }).waitFor();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Create Assignment" }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/questions$/u,
  );
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();

  await page.getByRole("button", { name: "Search question library", exact: true }).click();
  const picker = page.getByRole("dialog", { name: "Choose assignment questions", exact: true });
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.locator(".question-picker-result input").first().check();
  await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await page
    .getByText("Questions and order saved. Review assignment policies when you are ready.")
    .waitFor();
  await page.getByRole("link", { name: "Review assignment policies", exact: true }).click();
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  const dueSchedule = page.getByRole("group", { name: "Due date and time", exact: true });
  await dueSchedule.locator('input[type="date"]').fill("2026-12-01");
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByLabel("Time limit in seconds").fill("1800");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await page
    .getByText("Assignment policies saved. The current assignment now uses the new revision.")
    .waitFor();
  const deliveryCheckPage = context.waitForEvent("page");
  await page.getByRole("link", { name: "Check assignment delivery", exact: true }).click();
  const deliveryCheck = await deliveryCheckPage;
  await deliveryCheck
    .getByRole("heading", { name: "Assignment delivery check", exact: true })
    .waitFor();
  await deliveryCheck
    .getByText("Preview only - no Student work or grades are created.", { exact: true })
    .waitFor();
  await assertNoStudentResponseOrSubmissionControl(deliveryCheck);
  await deliveryCheck
    .getByRole("link", { name: "Return to assignment policies", exact: true })
    .waitFor();
  await deliveryCheck.close();
  const readiness = page.getByRole("button", { name: "Check release readiness", exact: true });
  await readiness.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
  await page
    .getByText("Release readiness checked. This saved assignment is ready to release.")
    .waitFor();
  await page.getByRole("button", { name: "Release assignment", exact: true }).click();
  await page.getByText("Assignment released as revision 1.").waitFor();
} finally {
  await context.close();
  await browser.close();
}
