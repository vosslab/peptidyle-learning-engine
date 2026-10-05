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
let searchRequests = 0;
page.on("request", (request) => {
  if (new URL(request.url()).pathname === "/api/questions/search") searchRequests += 1;
});

async function createVerificationPool() {
  const poolTitle = `Question Library browser verification ${Date.now()}`;
  const result = await page.evaluate(
    async ({ poolTitle: title }) => {
      const query = new URLSearchParams({
        kind: "questions",
        membership: "all",
        authorship: "any",
        sort: "titleAscending",
        page_size: "50",
        text: "Genetic disorders: Which one?",
      });
      const response = await fetch(`/api/questions/search?${query.toString()}`);
      if (!response.ok) throw new Error(`Pilot Question search status ${response.status}`);
      const body = await response.json();
      const matches = body.items.filter(
        (item) =>
          item.kind === "question" &&
          item.question?.summary?.metadata?.questionTitle === "Genetic disorders: Which one?" &&
          typeof item.question.summary.questionId === "string" &&
          typeof item.question.summary.publishedQuestionRevisionTuple?.revisionNumber === "number",
      );
      if (matches.length !== 1) {
        throw new Error(`Expected one exact Pilot Question, received ${matches.length}`);
      }
      const source = matches[0].question.summary;
      const created = await fetch("/api/question-pools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          description: "One exact Pilot Question for this browser verification.",
          members: [
            {
              questionId: source.questionId,
              revisionNumber: source.publishedQuestionRevisionTuple.revisionNumber,
            },
          ],
          interchangeabilityAttested: true,
        }),
      });
      if (created.status !== 201) throw new Error(`Pool creation status ${created.status}`);
    },
    { poolTitle },
  );
  if (result !== undefined) throw new Error("Question Library browser Pool setup returned a value");
  return poolTitle;
}

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
    .getByRole("region", { name: "Question Library results", exact: true })
    .getByRole("list", { name: "Question Library results", exact: true });
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

  const verificationPoolTitle = await createVerificationPool();
  const search = page.getByLabel("Search Question Library");
  await search.fill("x");
  await search.press("Enter");
  await page.getByLabel("Backend").waitFor();
  await search.fill("");
  await search.press("Enter");
  await rows.first().waitFor({ state: "visible" });
  const restoredRowCount = await rows.count();
  if (restoredRowCount === 0 || restoredRowCount > 50) {
    throw new Error(
      `Clearing Question Library search did not restore one bounded result page (${restoredRowCount} rows)`,
    );
  }
  const show = page.getByRole("combobox", { name: "Show", exact: true });
  const membership = page.getByRole("combobox", { name: "Question membership", exact: true });
  if ((await show.inputValue()) !== "both" || (await membership.inputValue()) !== "noPool") {
    throw new Error(
      "Question Library did not start with Questions and Pools, excluding Pool members",
    );
  }
  const verificationMemberTitle = "Genetic disorders: Which one?";
  const verificationPoolRow = rows.filter({ hasText: verificationPoolTitle });
  const beforeTyping = searchRequests;
  await search.fill(verificationMemberTitle);
  if (searchRequests !== beforeTyping) throw new Error("Typing submitted the search");
  await search.press("Enter");
  await page.getByText("No Library objects match these filters", { exact: true }).waitFor();
  if (searchRequests !== beforeTyping + 1) throw new Error("Enter did not submit exactly once");
  await membership.selectOption("all");
  await rows.filter({ hasText: verificationMemberTitle }).first().waitFor({ state: "visible" });
  if ((await rows.count()) !== 1) {
    throw new Error("Question Library exact title search did not render one result");
  }
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

  const beforeDisplay = searchRequests;
  const description = selectedRow.locator(".record-list__description");
  const savedDescription = await description.innerText();
  const displays = page.getByRole("group", { name: "Library result display", exact: true });
  for (const label of ["Compact", "List", "Visual boxes"]) {
    await displays.getByRole("button", { name: label, exact: true }).click();
    if (
      (await displays
        .getByRole("button", { name: label, exact: true })
        .getAttribute("aria-pressed")) !== "true"
    ) {
      throw new Error(`Library result display did not select ${label}`);
    }
    if (!(await description.isVisible()) || (await description.innerText()) !== savedDescription) {
      throw new Error(`Question description was lost in ${label}`);
    }
  }

  if (searchRequests !== beforeDisplay) throw new Error("Changing display refetched the search");
  await page.getByRole("link", { name: "My Questions", exact: true }).click();
  await page.getByRole("heading", { name: "Leave this search?", exact: true }).waitFor();
  await page.getByRole("button", { name: "Stay on page", exact: true }).click();
  if ((await search.inputValue()) !== selectedTitle || (await rows.count()) !== 1) {
    throw new Error("Staying on the search page discarded its current results");
  }

  await search.fill(verificationPoolTitle);
  await search.press("Enter");
  await verificationPoolRow.first().waitFor({ state: "visible" });
  await show.selectOption("pools");
  await verificationPoolRow.first().waitFor({ state: "visible" });
  if (!(await membership.isDisabled())) {
    throw new Error("Question membership remained enabled for Pool-only results");
  }
  if (
    (await rows.count()) === 0 ||
    (await rows.filter({ hasText: "Question Pool" }).count()) === 0
  ) {
    throw new Error("Pool-only Question Library results did not render Pool rows");
  }
  const poolLink = verificationPoolRow.getByRole("link", {
    name: "Open Question Pool",
    exact: true,
  });
  if (
    (await poolLink.getAttribute("target")) !== "_blank" ||
    (await poolLink.getAttribute("rel")) !== "noopener"
  ) {
    throw new Error("Question Pool result did not open in a protected new tab");
  }
  const openedPoolPage = context.waitForEvent("page");
  await poolLink.click();
  const poolPage = await openedPoolPage;
  await poolPage.waitForURL(
    new RegExp(`^${origin}/library/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$`),
  );
  await poolPage.getByRole("heading", { name: verificationPoolTitle, exact: true }).waitFor();
  await poolPage.close();
  await page.getByRole("link", { name: "My Questions", exact: true }).click();
  await page.getByRole("button", { name: "Leave page", exact: true }).click();
  await page.waitForURL(`${origin}/authoring/questions`);
  console.log("Question Library browser: navigation, search, and detail complete");
} finally {
  await context.close();
  await browser.close();
}
