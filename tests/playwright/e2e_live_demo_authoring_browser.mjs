// Production-browser proof for private Draft Question authoring and publication.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const questionTitle = `Browser publication path ${Date.now()}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Instructor" }).click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("link", { name: "My Question Drafts" }).click();
  await page.waitForURL(`${origin}/authoring/drafts`);
  await page.getByRole("button", { name: "New Draft Question" }).click();
  await page.waitForURL(/\/authoring\/drafts\/D-[1-9][0-9]*$/u);
  await page.getByLabel("Question Title").fill(questionTitle);
  await page.getByLabel("Question License").selectOption("CC-BY-4.0");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByText("Private draft saved. It is not published.").waitFor();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Question Authors").fill("Live Demo Instructor");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await page.getByRole("heading", { name: "Published" }).waitFor();
  await page.getByRole("link", { name: "Open question library" }).click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("heading", { name: questionTitle }).waitFor();
  await page
    .locator("article.question-library-row", {
      has: page.getByRole("heading", { name: questionTitle }),
    })
    .getByRole("link", { name: "Open question" })
    .click();
  await page.waitForURL(/\/library\/[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u);
  await page.getByRole("heading", { name: questionTitle }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
