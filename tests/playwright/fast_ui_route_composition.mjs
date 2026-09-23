// Stack-free current-source route composition for the fast UI lane.

import assert from "node:assert/strict";

import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";

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
  await gradebook.locator('[role="listitem"][data-record-id]').waitFor({ state: "visible" });
  for (const [profile, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    const columns = await gradebook.evaluate((root) => {
      const header = root.querySelector(".record-list__header");
      const headerRegions = [...(header?.querySelectorAll("[data-record-region-id]") ?? [])];
      const firstRow = root.querySelector('[role="listitem"][data-record-id]');
      const rowRegions =
        firstRow === null
          ? []
          : [...firstRow.querySelectorAll("[data-record-region-id]")].filter(
              (region) => getComputedStyle(region).display !== "none",
            );
      const visibleHeaderRegions = headerRegions.filter(
        (region) => getComputedStyle(region).display !== "none",
      );
      const alignedRegions = visibleHeaderRegions.every((headerRegion) => {
        const id = headerRegion.getAttribute("data-record-region-id");
        const rowRegion = rowRegions.find(
          (region) => region.getAttribute("data-record-region-id") === id,
        );
        if (rowRegion === undefined) return false;
        const headerRect = headerRegion.getBoundingClientRect();
        const rowRect = rowRegion.getBoundingClientRect();
        const alignment = getComputedStyle(headerRegion).justifySelf;
        if (alignment === "end") return headerRect.right === rowRect.right;
        if (alignment === "center") {
          return headerRect.left + headerRect.width / 2 === rowRect.left + rowRect.width / 2;
        }
        if (alignment === "stretch") {
          return headerRect.left === rowRect.left && headerRect.right === rowRect.right;
        }
        return headerRect.left === rowRect.left;
      });
      return {
        headerColumns: visibleHeaderRegions.map((region) =>
          region.getAttribute("data-record-region-id"),
        ),
        rowColumns: rowRegions.map((region) => region.getAttribute("data-record-region-id")),
        alignedRegions,
      };
    });
    assert.deepEqual(
      columns.headerColumns,
      columns.rowColumns,
      `${profile}: Gradebook headers describe visible RecordList regions in order`,
    );
    assert.equal(
      columns.alignedRegions,
      true,
      `${profile}: each Gradebook header region aligns with its corresponding RecordList region`,
    );
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
