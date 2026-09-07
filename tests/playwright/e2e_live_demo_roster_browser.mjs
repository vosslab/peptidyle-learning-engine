// Production-browser proof for visible Course Roster Import and current roster projection.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const blueprintTitle = `Browser M9 Blueprint ${Date.now()}`;
const courseTitle = `Browser M9 Course ${Date.now()}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Instructor" }).click();
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
  const source = page.getByLabel("Blueprint Course Revision");
  await source.selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course Instance title").fill(courseTitle);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create Course Instance" }).click();
  await page.getByRole("heading", { name: courseTitle }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance" }).first().click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);
  await page.getByRole("link", { name: "Open Students" }).click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/students$/u);
  await page.getByRole("heading", { name: "Students" }).waitFor();
  await page
    .getByLabel("Email, roster ID")
    .fill("mary.student@live-demo.invalid,m9-browser-seeded");
  await page.getByRole("button", { name: "Import roster" }).click();
  await page.getByText("Roster import recorded.").waitFor();
  await page.getByText("Invitation pending").waitFor();
  await page.getByText("m9-browser-seeded").waitFor();
} finally {
  await context.close();
  await browser.close();
}
