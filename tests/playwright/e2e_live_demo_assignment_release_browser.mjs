// Visible Instructor proof: Draft -> published Blueprint Revision -> Course -> current Assignment.
// Selector contract: accessible labels/headings/actions in the Blueprint, Course, and Assignment
// workspaces; the Question picker discovers its seeded published Question through its visible result.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const blueprintTitle = `Browser current Blueprint ${runId}`;
const courseShortName = `Current-${runId}`;
const courseLongName = `Browser current Course ${runId}`;
const assignmentTitle = `Browser current Assignment ${runId}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

async function firstNonEmptyOptionValue(select) {
  const value = await select
    .locator("option")
    .evaluateAll((options) =>
      options.map((option) => option.value).find((optionValue) => optionValue.length > 0),
    );
  if (typeof value !== "string") {
    throw new Error("the current Course did not offer a Blueprint Assignment source");
  }
  return value;
}

async function assertAnswerFreePreview(target) {
  for (const name of [/response/i, /submit/i, /start assignment/i, /student attempt/i]) {
    if ((await target.getByRole("button", { name }).count()) !== 0) {
      throw new Error("Assignment delivery check exposed a Student work control");
    }
  }
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);

  await page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course title").fill(blueprintTitle);
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  const questionPicker = page.getByRole("dialog", {
    name: "Choose the first reusable Questions",
    exact: true,
  });
  await questionPicker.getByRole("button", { name: "Search questions", exact: true }).click();
  await questionPicker.getByRole("checkbox").first().check();
  await questionPicker.getByRole("button", { name: "Use selected Questions", exact: true }).click();
  await page.getByText("1 fixed Question selected in order.", { exact: true }).waitFor();
  await page
    .getByRole("dialog", { name: "Create a Blueprint Course", exact: true })
    .getByRole("button", { name: "Create Blueprint Course", exact: true })
    .click();
  await page.waitForURL(/\/blueprint-courses\/BP-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: blueprintTitle, exact: true }).waitFor();

  await page.getByRole("button", { name: "Publish Blueprint Revision", exact: true }).click();
  await page.getByText("Published Blueprint Revision 1.", { exact: true }).waitFor();

  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page.waitForURL(`${origin}/`);
  await page.getByRole("heading", { name: "Course Instances you teach", exact: true }).waitFor();
  await page.getByLabel("Blueprint Course Revision").selectOption({
    label: `${blueprintTitle} · Revision 1`,
  });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  const createdCourse = page.locator("article").filter({
    has: page.getByRole("heading", { name: courseLongName, exact: true }),
  });
  await createdCourse.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: courseLongName, exact: true }).waitFor();

  await page.getByRole("link", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/new$/u);
  await page.getByRole("heading", { name: "Create an Assignment", exact: true }).waitFor();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  const assignmentSource = page.getByLabel("Blueprint Assignment source");
  await assignmentSource.selectOption(await firstNonEmptyOptionValue(assignmentSource));
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/questions$/u,
  );
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
  const availableQuestions = page
    .getByRole("heading", { name: "Available published Questions", exact: true })
    .locator("..");
  await availableQuestions
    .getByRole("button", { name: "Add Question", exact: true })
    .first()
    .click();
  await page.getByText(/exact revision pin/u).waitFor();
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await page
    .getByText("Questions and order saved. Review assignment policies when you are ready.", {
      exact: true,
    })
    .waitFor();
  await page.getByRole("link", { name: "Review assignment policies", exact: true }).click();
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  await page
    .getByRole("group", { name: "Due date and time", exact: true })
    .getByLabel(/Due date/u)
    .fill("2026-12-01");
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByLabel("Time limit in seconds", { exact: true }).fill("1800");
  await page.getByLabel("Late-work rule", { exact: true }).selectOption("mark_late");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await page
    .getByText("Assignment policies saved. Future Attempts use the current policy values.", {
      exact: true,
    })
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
  await assertAnswerFreePreview(deliveryCheck);
  await deliveryCheck.close();

  await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
  await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
  await page
    .getByText("Release readiness checked. This saved assignment is ready to release.", {
      exact: true,
    })
    .waitFor();
  await page.getByRole("button", { name: "Release assignment", exact: true }).click();
  await page.getByText(/^Assignment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
  await page
    .getByRole("button", { name: "Release assignment", exact: true })
    .waitFor({ state: "hidden" });
  await page
    .getByRole("heading", { name: "Danger Zone: Unrelease assignment", exact: true })
    .waitFor();
} finally {
  await context.close();
  await browser.close();
}
