// Production-browser proof for the restored Instructor Question Library task.

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();

try {
  // The production gateway keeps its own readiness probes active.  Wait for
  // the document and then the actual demo control rather than requiring every
  // background request in the live stack to go idle.
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("link", { name: "Question Library" }).waitFor();

  const rows = page
    .getByRole("region", { name: "Published questions", exact: true })
    .getByRole("list", { name: "Published questions", exact: true })
    .getByRole("listitem");
  const searchTips = page.locator("details.question-library-search-tips");
  if ((await rows.count()) !== 0 || (await page.getByLabel("Backend").count()) !== 0) {
    throw new Error("Fresh Question Library did not begin with the simple Search entry");
  }
  if (await searchTips.evaluate((element) => element.hasAttribute("open"))) {
    throw new Error("Question Library Search tips were not initially collapsed");
  }

  const search = page.getByLabel("Search published questions");
  await search.fill("x");
  await page.getByLabel("Backend").waitFor();
  await search.fill("");
  await page.waitForFunction((expectedCount) => {
    const results = document.querySelector(
      '[role="region"][aria-label="Published questions"] [role="list"][aria-label="Published questions"]',
    );
    return results?.querySelectorAll('[role="listitem"]').length === expectedCount;
  }, 8);
  const visibleQuestionIds = await rows.evaluateAll((elements) =>
    elements.map((element) => element.textContent ?? ""),
  );
  if (
    visibleQuestionIds.some((text) => !/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}/.test(text))
  ) {
    throw new Error("Question Library did not display canonical deployment-issued Question IDs");
  }

  await page.getByLabel("Backend").selectOption("ple");
  await page.waitForFunction(() => {
    const results = document.querySelector(
      '[role="region"][aria-label="Published questions"] [role="list"][aria-label="Published questions"]',
    );
    return results?.querySelectorAll('[role="listitem"]').length === 4;
  });
  const selectedRow = rows.first();
  const selectedTitle = await selectedRow.getByRole("heading", { level: 2 }).textContent();
  const selectedLink = selectedRow.getByRole("link", { name: "Open question" });
  const selectedPath = await selectedLink.getAttribute("href");
  if (
    selectedTitle === null ||
    selectedPath === null ||
    !/^\/library\/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}\?libraryReturn=[0-9a-f-]{36}$/.test(
      selectedPath,
    )
  ) {
    throw new Error("Question Library did not expose a canonical exact Question Revision route");
  }

  await search.fill(selectedTitle);
  await page.waitForFunction(() => {
    const results = document.querySelector(
      '[role="region"][aria-label="Published questions"] [role="list"][aria-label="Published questions"]',
    );
    return results?.querySelectorAll('[role="listitem"]').length === 1;
  });
  await selectedLink.click();
  await page.waitForURL(`${origin}${selectedPath}`);
  await page.getByRole("heading", { name: selectedTitle, exact: true }).waitFor();
  await page.getByRole("region", { name: "Question prompt" }).waitFor();

  await page.goBack();
  await page.waitForURL(new RegExp(`${origin}/library\\?libraryReturn=`));
  if ((await search.inputValue()) !== selectedTitle) {
    throw new Error("Browser Back did not restore the Question Library search");
  }
  if ((await page.getByLabel("Backend").inputValue()) !== "ple" || (await rows.count()) !== 1) {
    throw new Error("Browser Back did not restore the Question Library filters and results");
  }

  await selectedLink.click();
  await page.getByRole("link", { name: "Return to question library" }).click();
  if ((await search.inputValue()) !== selectedTitle || (await rows.count()) !== 1) {
    throw new Error("The visible Question return link did not restore the Library view");
  }
} finally {
  await context.close();
  await browser.close();
}
