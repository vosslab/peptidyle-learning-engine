// Stack-free current-source route composition for the fast UI lane.

import assert from "node:assert/strict";

import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";

const tableReviewViewports = [
  ["laptop", CANONICAL_VIEWPORTS.laptop],
  ["phone", CANONICAL_VIEWPORTS.phone],
];

async function renderedRecordIds(container) {
  return container
    .locator("[data-record-id]")
    .evaluateAll((rows) => rows.map((row) => row.getAttribute("data-record-id")));
}

async function recordRegions(container) {
  return container.locator("[data-record-id]").evaluateAll((rows) =>
    rows.map((row) => ({
      id: row.getAttribute("data-record-id"),
      regions: [...row.querySelectorAll("[data-record-region-id]")].map((region) =>
        region.getAttribute("data-record-region-id"),
      ),
    })),
  );
}

const { browser, consoleErrors, harnessServer, page, pageErrors } =
  await openRibbonShellEvidencePage();
try {
  const failedResources = [];
  page.on("response", (response) => {
    if (response.status() >= 400) failedResources.push(response.url());
  });
  await page.evaluate(() => window.ribbonShell.releaseSession());

  await page.evaluate(() =>
    window.ribbonShell.currentNavigate("/instructor/courses/CI7K3M2QAZ/gradebook"),
  );
  const gradebook = page.locator(
    '[data-m10-case="current-production"] [data-route-surface="gradebook"]',
  );
  await gradebook.getByRole("heading", { name: "Gradebook", exact: true }).waitFor({
    state: "visible",
  });
  assert.equal(
    await gradebook.getAttribute("data-content-layout"),
    "fullWidth",
    "GradebookPage retains its declared full-width PageFrame",
  );
  const gradebookTable = gradebook.getByRole("table", {
    name: "Student progress and scores",
    exact: true,
  });
  await gradebookTable.locator("tbody tr[data-record-id]").first().waitFor({ state: "visible" });
  assert.deepEqual(await gradebookTable.getByRole("columnheader").allTextContents(), [
    "Student",
    "Coursework",
    "Progress status",
    "Current score",
  ]);
  assert.ok(
    (await gradebookTable.getByRole("rowheader").first().innerText()).trim().length > 0,
    "Gradebook identifies each student row",
  );
  assert.ok((await gradebookTable.getByRole("cell").count()) > 0, "Gradebook renders score cells");

  for (const [profile, viewport] of tableReviewViewports) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    const tableViewport = await gradebook.evaluate((root) => {
      const scroll = root.querySelector(".record-table__scroll");
      if (!(scroll instanceof HTMLElement)) return undefined;
      return {
        overflowX: getComputedStyle(scroll).overflowX,
        clientWidth: scroll.clientWidth,
        scrollWidth: scroll.scrollWidth,
      };
    });
    assert.equal(
      tableViewport?.overflowX,
      "auto",
      `${profile}: Gradebook retains horizontal scroll`,
    );
    assert.ok(tableViewport?.clientWidth > 0, `${profile}: Gradebook scroll area has width`);
    assert.deepEqual(
      await gradebookTable.getByRole("columnheader").allTextContents(),
      ["Student", "Coursework", "Progress status", "Current score"],
      `${profile}: Gradebook retains every labeled column`,
    );
    if (profile === "phone") {
      assert.ok(
        tableViewport.scrollWidth > tableViewport.clientWidth,
        "phone Gradebook scrolls horizontally to retain every table column",
      );
    }
  }
  await page.setViewportSize({
    width: CANONICAL_VIEWPORTS.laptop.width,
    height: CANONICAL_VIEWPORTS.laptop.height,
  });

  await page.evaluate(() =>
    window.ribbonShell.currentNavigate("/instructor/courses/CI7K3M2QAZ/students"),
  );
  const roster = page.locator(
    '[data-m10-case="current-production"] [data-route-surface="courseRoster"]',
  );
  await roster.getByRole("heading", { name: "Students", exact: true }).waitFor({
    state: "visible",
  });
  const rosterTable = roster.getByRole("table", {
    name: "Current Course roster",
    exact: true,
  });
  await rosterTable.getByRole("rowheader").first().waitFor({
    state: "visible",
  });
  assert.deepEqual(await rosterTable.getByRole("columnheader").allTextContents(), [
    "Student",
    "State",
    "Action",
  ]);
  const rosterRows = rosterTable.locator("tbody tr[data-record-id]");
  const rosterRowCount = await rosterRows.count();
  assert.ok(rosterRowCount > 0, "roster renders student records");
  for (let rowIndex = 0; rowIndex < rosterRowCount; rowIndex += 1) {
    assert.ok(
      (await rosterRows
        .nth(rowIndex)
        .getByRole("button", { name: "Remove course access" })
        .count()) > 0,
      "each roster record retains its action",
    );
  }
  for (const [profile, viewport] of tableReviewViewports) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    const rosterViewport = await roster.evaluate((root) => {
      const scroll = root.querySelector(".record-table__scroll");
      if (!(scroll instanceof HTMLElement)) return undefined;
      return {
        overflowX: getComputedStyle(scroll).overflowX,
        clientWidth: scroll.clientWidth,
        scrollWidth: scroll.scrollWidth,
        pageWidth: document.documentElement.scrollWidth,
        viewportWidth: window.innerWidth,
      };
    });
    assert.equal(rosterViewport?.overflowX, "auto", `${profile}: roster retains table scrolling`);
    assert.ok(rosterViewport?.clientWidth > 0, `${profile}: roster scroll area has width`);
    assert.deepEqual(await rosterTable.getByRole("columnheader").allTextContents(), [
      "Student",
      "State",
      "Action",
    ]);
    assert.ok(
      rosterViewport.pageWidth <= rosterViewport.viewportWidth,
      `${profile}: roster table does not create page-level horizontal overflow`,
    );
    if (profile === "phone") {
      assert.ok(
        rosterViewport.scrollWidth > rosterViewport.clientWidth,
        "phone roster scrolls horizontally to retain every table column",
      );
    }
  }
  await page.setViewportSize({
    width: CANONICAL_VIEWPORTS.laptop.width,
    height: CANONICAL_VIEWPORTS.laptop.height,
  });

  await page.evaluate(() => window.ribbonShell.currentNavigate("/library/browse?tag=protein"));
  const library = page.locator(
    '[data-m10-case="current-production"] [data-route-surface="library-browse"]',
  );
  await library.getByRole("heading", { name: "Browse Question Library", exact: true }).waitFor({
    state: "visible",
  });
  const libraryWindow = library.getByRole("region", { name: "Published questions", exact: true });
  await libraryWindow.waitFor({ state: "visible" });
  const expectedRows = await page.evaluate(() => window.ribbonShell.questionLibrarySeedRows());
  const expectedSeedIds = expectedRows.map((row) => row.id);
  assert.equal(
    await page.evaluate(() => window.ribbonShell.questionLibrarySeed()),
    "fast-ui-question-library-v1",
    "Question Library uses the named fast UI seed at the Application API boundary",
  );

  await page.waitForFunction((expectedCount) => {
    const rows = document.querySelectorAll(
      '[data-route-surface="library-browse"] [data-record-id]',
    );
    const bottomSpacer = document.querySelector(
      '[data-route-surface="library-browse"] .library-browse-record-list__spacer:last-child',
    );
    return (
      rows.length > 0 &&
      rows.length < expectedCount &&
      bottomSpacer !== null &&
      bottomSpacer.getBoundingClientRect().height > 0
    );
  }, expectedRows.length);
  const visibleIds = await renderedRecordIds(libraryWindow);
  assert.ok(visibleIds.length < expectedSeedIds.length);
  assert.deepEqual(visibleIds, expectedSeedIds.slice(0, visibleIds.length));
  const visibleRows = await libraryWindow.locator("[data-record-id]").evaluateAll((rows) =>
    rows.map((row) => ({
      id: row.getAttribute("data-record-id"),
      title: row.querySelector("h2")?.textContent?.trim(),
    })),
  );
  assert.deepEqual(visibleRows, expectedRows.slice(0, visibleRows.length));
  const visibleRegions = await recordRegions(libraryWindow);
  assert.ok(visibleRegions.length > 0);
  const bottomSpacerHeight = await libraryWindow
    .locator(".library-browse-record-list__spacer")
    .last()
    .evaluate((element) => element.getBoundingClientRect().height);
  assert.ok(bottomSpacerHeight > 0, "the production Library window reserves its remaining rows");

  const libraryContent = library.locator(".page-frame__content.library-page");
  const createPoolButton = library.getByRole("button", { name: "Create Question Pool" });
  await createPoolButton.click();
  const picker = page.getByRole("dialog");
  await picker.waitFor({ state: "visible" });
  await picker.getByRole("checkbox").first().check();
  await picker.getByRole("button", { name: "Review selected Questions" }).click();
  await library.getByRole("heading", { name: "Create Question Pool", exact: true }).waitFor({
    state: "visible",
  });
  await page.waitForFunction(
    (element) => element.classList.contains("question-pool-task-active"),
    await libraryContent.elementHandle(),
  );
  assert.equal(
    await createPoolButton.isVisible(),
    false,
    "entering Question Pool review hides the Library's other content",
  );

  await library.getByRole("button", { name: "Choose different Questions" }).click();
  await picker.waitFor({ state: "visible" });
  await page.waitForFunction(
    (element) => !element.classList.contains("question-pool-task-active"),
    await libraryContent.elementHandle(),
  );
  assert.equal(
    await createPoolButton.isVisible(),
    true,
    "leaving Question Pool review restores the Library's other content",
  );

  assert.deepEqual(pageErrors, [], "production Gradebook and Library routes raise no page errors");
  assert.deepEqual(
    failedResources,
    [],
    "production route assets load from the fixed harness allowlist",
  );
  assert.deepEqual(
    consoleErrors,
    [],
    "production Gradebook and Library routes raise no console errors",
  );
  process.stdout.write("Fast Gradebook and Question Library route composition passed.\n");
} finally {
  await browser.close();
  await harnessServer.close();
}
