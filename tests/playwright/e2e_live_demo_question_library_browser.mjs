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
  await page.getByRole("link", { name: "Question Library", exact: true }).waitFor();

  const resultList = page
    .getByRole("region", { name: "Published questions", exact: true })
    .getByRole("list", { name: "Published questions", exact: true });
  const rows = resultList.getByRole("listitem");
  const searchTips = page.locator("details.question-library-search-tips");
  if ((await rows.count()) !== 0 || (await page.getByLabel("Backend").count()) !== 0) {
    throw new Error("Fresh Question Library did not begin with the simple Search entry");
  }
  if (await searchTips.evaluate((element) => element.hasAttribute("open"))) {
    throw new Error("Question Library Search tips were not initially collapsed");
  }
  await page.getByRole("link", { name: "My Questions", exact: true }).click();
  await page.waitForURL(`${origin}/authoring/questions`);
  if (await page.getByRole("heading", { name: "Leave this search?", exact: true }).count()) {
    throw new Error("An untouched Question Library page prompted before leaving");
  }
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "Search Question Library", exact: true })
    .click();
  await page.waitForURL(`${origin}/library`);

  const search = page.getByLabel("Search published questions");
  await search.fill("x");
  await page.getByLabel("Backend").waitFor();
  await search.fill("");
  await page.waitForFunction((expectedCount) => {
    const results = document.querySelector(
      '[role="region"][aria-label="Published questions"] [role="list"][aria-label="Published questions"]',
    );
    return results?.querySelectorAll('[role="listitem"]').length === expectedCount;
  }, 50);
  await search.fill("Genetic disorders: Which one?");
  await page.waitForFunction(() => {
    const results = document.querySelector(
      '[role="region"][aria-label="Published questions"] [role="list"][aria-label="Published questions"]',
    );
    return results?.querySelectorAll('[role="listitem"]').length === 1;
  });
  const visibleQuestionIds = await rows.evaluateAll((elements) =>
    elements.map((element) => element.textContent ?? ""),
  );
  if (
    visibleQuestionIds.some((text) => !/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}/.test(text))
  ) {
    throw new Error("Question Library did not display canonical deployment-issued Question IDs");
  }
  const selectedRow = rows.first();
  const selectedLink = selectedRow.getByRole("link", { name: "Open", exact: true });
  const selectedLinkTitle = await selectedLink.getAttribute("title");
  const selectedTitle = selectedLinkTitle?.replace(/^Open /u, "") ?? null;
  const selectedPath = await selectedLink.getAttribute("href");
  if (
    selectedTitle === null ||
    selectedPath === null ||
    !/^\/library\/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/.test(selectedPath)
  ) {
    throw new Error("Question Library did not expose a canonical exact Question Revision route");
  }
  if (
    (await selectedLink.getAttribute("target")) !== "_blank" ||
    (await selectedLink.getAttribute("rel")) !== "noopener"
  ) {
    throw new Error("Question Library result did not open in a protected new tab");
  }

  const openedPage = context.waitForEvent("page");
  await selectedLink.click();
  const resultPage = await openedPage;
  await resultPage.waitForURL(`${origin}${selectedPath}`);
  await resultPage.getByRole("heading", { name: selectedTitle, exact: true }).waitFor();
  await resultPage.getByRole("region", { name: "Question prompt" }).waitFor();
  if (
    (await resultPage.getByRole("button", { name: /discussion/i }).count()) !== 0 ||
    (await resultPage.getByRole("link", { name: /discussion/i }).count()) !== 0
  ) {
    throw new Error("Question detail retained a discussion control");
  }
  const questionId = selectedPath.split("/").at(-1);
  const removedDiscussion = await context.request.get(
    `${origin}/api/library-objects/question/${questionId}/discussions`,
  );
  if (removedDiscussion.status() !== 404) {
    throw new Error("Removed general Question discussion route remained available");
  }
  if ((await search.inputValue()) !== selectedTitle) {
    throw new Error("Opening a Question changed the original Question Library search");
  }
  if ((await page.getByLabel("Backend").inputValue()) !== "" || (await rows.count()) !== 1) {
    throw new Error("Opening a Question changed the original Question Library filters or results");
  }
  await resultPage.close();

  const displays = page.getByRole("group", { name: "Question result display", exact: true });
  for (const [label, expectedClass] of [
    ["Compact", "record-list--compact"],
    ["List", "record-list--semantic"],
    ["Visual boxes", "record-list--poster"],
  ]) {
    await displays.getByRole("button", { name: label, exact: true }).click();
    if (
      (await displays
        .getByRole("button", { name: label, exact: true })
        .getAttribute("aria-pressed")) !== "true"
    ) {
      throw new Error(`Question result display did not select ${label}`);
    }
    if (
      !(await resultList.evaluate(
        (element, className) => element.classList.contains(className),
        expectedClass,
      ))
    ) {
      throw new Error(`Question result display did not render ${label}`);
    }
  }

  await page.getByRole("link", { name: "My Questions", exact: true }).click();
  await page.getByRole("heading", { name: "Leave this search?", exact: true }).waitFor();
  await page.getByRole("button", { name: "Stay on page", exact: true }).click();
  if ((await search.inputValue()) !== selectedTitle || (await rows.count()) !== 1) {
    throw new Error("Staying on the search page discarded its current results");
  }
  await page.getByRole("link", { name: "My Questions", exact: true }).click();
  await page.getByRole("button", { name: "Leave page", exact: true }).click();
  await page.waitForURL(`${origin}/authoring/questions`);
  console.log("Question Library browser: navigation, search, and detail complete");
} finally {
  await context.close();
  await browser.close();
}
