// Production-browser proof for visible Blueprint Course creation and first publication.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const courseTitle = `Browser Blueprint Course ${Date.now()}`;
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
  await page.getByLabel("Blueprint Course title").fill(courseTitle);
  await page.getByRole("button", { name: "Choose published Questions" }).click();
  await page.getByRole("heading", { name: "Choose the first reusable Questions" }).waitFor();
  await page.getByRole("button", { name: "Search questions" }).click();
  const firstQuestion = page.locator(".question-picker-result input").first();
  await firstQuestion.check();
  await page.getByRole("button", { name: "Use selected Questions" }).click();
  await page.getByText("1 fixed Question selected in order.").waitFor();
  await page.getByRole("dialog").getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.waitForURL(/\/blueprint-courses\/BP-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: courseTitle }).waitFor();
  await page
    .getByText("Blueprint Course loaded. Update its Blueprint Assignments deliberately.")
    .waitFor();
  await page.getByRole("link", { name: "Return to Blueprint Courses" }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("link", { name: courseTitle }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
