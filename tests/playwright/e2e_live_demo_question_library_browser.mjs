// Production-browser proof for the restored Instructor Question Library task.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  // The production gateway keeps its own readiness probes active.  Wait for
  // the document and then the actual demo control rather than requiring every
  // background request in the live stack to go idle.
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("link", { name: "Question Library" }).waitFor();

  const rows = page.locator("article.question-library-row");
  await page.waitForFunction(
    () => document.querySelectorAll("article.question-library-row").length === 8,
  );
  const visibleQuestionIds = await rows.evaluateAll((elements) =>
    elements.map((element) => element.textContent ?? ""),
  );
  if (
    visibleQuestionIds.some((text) => !/[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}/.test(text))
  ) {
    throw new Error("Question Library did not display canonical deployment-issued Question IDs");
  }

  await page.getByLabel("Backend").selectOption("ple");
  await page.waitForFunction(
    () => document.querySelectorAll("article.question-library-row").length === 4,
  );
  const selectedRow = rows.first();
  const selectedTitle = await selectedRow.getByRole("heading", { level: 2 }).textContent();
  const selectedLink = selectedRow.getByRole("link", { name: "Open question" });
  const selectedPath = await selectedLink.getAttribute("href");
  if (
    selectedTitle === null ||
    selectedPath === null ||
    !/^\/library\/[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/.test(selectedPath)
  ) {
    throw new Error("Question Library did not expose a canonical exact Question Revision route");
  }

  const search = page.getByLabel("Search published questions");
  await search.fill(selectedTitle);
  await page.waitForFunction(
    () => document.querySelectorAll("article.question-library-row").length === 1,
  );
  await selectedLink.click();
  await page.waitForURL(`${origin}${selectedPath}`);
  await page.getByRole("heading", { name: selectedTitle, exact: true }).waitFor();
  await page.getByRole("region", { name: "Question prompt" }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
