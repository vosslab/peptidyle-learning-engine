// Production-browser proof for visible Course Instance creation and initial Teaching Team entry.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const blueprintTitle = `Browser M8 Blueprint ${runId}`;
const courseShortName = `M8-${runId}`;
const courseLongName = `Browser M8 Course ${runId}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

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
  await page.getByRole("heading", { name: "Course Instances you teach" }).waitFor();
  const source = page.getByLabel("Blueprint Course Revision");
  await source.selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance" }).click();
  await page.getByRole("heading", { name: courseLongName }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance" }).first().click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: courseLongName }).waitFor();
  await page.getByRole("heading", { name: "Initial Teaching Team" }).waitFor();
  await page.getByText("You are the Assigned Instructor for this Course Instance.").waitFor();
} finally {
  await context.close();
  await browser.close();
}
