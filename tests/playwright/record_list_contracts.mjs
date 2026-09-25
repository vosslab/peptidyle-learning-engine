// Durable browser contracts for the production RecordList APIs.

import assert from "node:assert/strict";

import { openRecordListHarness, recordIds } from "./helper_record_list_harness.mjs";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";

async function checkAlignment(page) {
  const hiddenPrioritiesByViewport = {
    laptop: [],
    tablet: ["low"],
    phone: ["low", "medium", "high"],
    square: ["low"],
  };
  const alignmentCase = page.locator('[data-record-list-case="alignment"]');

  function assertSameEdge(edges, edge, profileId, regionId) {
    const reference = edges[0];
    assert.notEqual(reference, undefined, `${profileId}:${regionId} has a ${edge} edge`);
    assert.deepEqual(
      edges,
      edges.map(() => reference),
      `${profileId}:${regionId} ${edge} edges share one RecordList grid track`,
    );
  }

  for (const [id, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    const profile = { id, ...viewport, hiddenPriorities: hiddenPrioritiesByViewport[id] };
    await page.setViewportSize({ width: profile.width, height: profile.height });
    const list = alignmentCase.getByRole("list", { name: "Alignment records", exact: true });
    const rows = await list.getByRole("listitem").evaluateAll((elements) =>
      elements.map((row) => {
        const regions = [...row.querySelectorAll("[data-record-region-id]")].map((region) => {
          const bounds = region.getBoundingClientRect();
          return {
            id: region.getAttribute("data-record-region-id"),
            priority: region.getAttribute("data-record-list-priority"),
            display: getComputedStyle(region).display,
            left: bounds.left,
            right: bounds.right,
          };
        });
        const action = row.querySelector('[data-record-region-id="actions"] button');
        return {
          recordId: row.getAttribute("data-record-id"),
          regions,
          actionVisible:
            action instanceof HTMLElement &&
            getComputedStyle(action).display !== "none" &&
            action.getBoundingClientRect().width > 0,
        };
      }),
    );
    assert.deepEqual(await recordIds(list), ["brief", "detailed"], `${profile.id} record order`);
    assert.equal(rows.length, 2, `${profile.id} retains all alignment records`);
    assert.equal(
      rows.every((row) => row.actionVisible),
      true,
      `${profile.id} retains actions`,
    );

    const visibleRegions = new Map();
    for (const row of rows) {
      for (const requiredId of ["identity", "actions"]) {
        assert.notEqual(
          row.regions.find((region) => region.id === requiredId)?.display,
          "none",
          `${profile.id}:${row.recordId} retains ${requiredId}`,
        );
      }
      for (const region of row.regions) {
        if (region.display === "none") continue;
        const edges = visibleRegions.get(region.id) ?? { left: [], right: [] };
        edges.left.push(region.left);
        edges.right.push(region.right);
        visibleRegions.set(region.id, edges);
      }
      for (const priority of profile.hiddenPriorities) {
        const regions = row.regions.filter((region) => region.priority === priority);
        assert.equal(
          regions.every((region) => region.display === "none"),
          true,
          `${profile.id}:${row.recordId} removes ${priority}-priority regions`,
        );
      }
    }
    for (const [regionId, edges] of visibleRegions) {
      if (["identity", "metadata", "note"].includes(regionId)) {
        assertSameEdge(edges.left, "left", profile.id, regionId);
      }
      if (["status", "actions"].includes(regionId)) {
        assertSameEdge(edges.right, "right", profile.id, regionId);
      }
    }
  }
  await page.setViewportSize({ width: 1024, height: 768 });
}

async function checkPrimitiveStates(page) {
  const ready = page.locator('[data-record-list-case="primitive-ready"]');
  await ready.getByRole("listitem").first().waitFor({ state: "visible" });
  assert.equal(
    await ready.getByRole("listitem").count(),
    3,
    "ready state renders deterministic rows",
  );

  await page
    .locator('[data-record-list-case="primitive-empty"]')
    .getByRole("heading", { name: "No primitive records", exact: true })
    .waitFor({ state: "visible" });
  const loading = page.locator('[data-record-list-case="primitive-loading"]');
  await loading.getByRole("status").waitFor({ state: "visible" });
  assert.match(await loading.innerText(), /Loading primitive records/);
  const error = page.locator('[data-record-list-case="primitive-error"]');
  await error.getByRole("alert").waitFor({ state: "visible" });
  assert.match(await error.innerText(), /Primitive records are unavailable/);
}

async function checkRecordFamily(page) {
  const family = page.locator('[data-record-list-case="record-family"]');
  await family.locator("[data-record-family-tab-start]").focus();
  const expectedTabStops = [
    "Select Enzyme kinetics",
    "Select Genetics review",
    "Select Protein structure",
    "Open DNA",
    "Open Genome-wide association study preparation",
    "Open assessment one",
    "Review Question 1",
    "Review Question 2",
  ];
  for (const expectedText of expectedTabStops) {
    await page.keyboard.press("Tab");
    assert.equal(
      await page.evaluate(() => document.activeElement?.textContent?.trim()),
      expectedText,
      `${expectedText} is reachable in document Tab order`,
    );
  }

  const sequence = family.locator('[data-record-family-case="sequence"]');
  const orderedRecords = sequence.getByRole("list", {
    name: "Ordered course records",
    exact: true,
  });
  assert.equal(await orderedRecords.evaluate((element) => element.tagName), "OL");
  assert.deepEqual(await recordIds(orderedRecords), ["enzyme", "genetics", "proteins"]);
  await family
    .locator('[data-record-family-case="sequence-empty"]')
    .getByRole("heading", { name: "No ordered records", exact: true })
    .waitFor({ state: "visible" });
  const loading = family.locator('[data-record-family-case="sequence-loading"]');
  assert.match(await loading.getByRole("status").innerText(), /Loading ordered course records/);
  const error = family.locator('[data-record-family-case="sequence-error"]');
  assert.match(await error.getByRole("alert").innerText(), /unavailable/);

  const table = family.getByRole("table", { name: "Course roster records", exact: true });
  const tableRows = table.locator("tbody tr");
  const tableRowCount = await tableRows.count();
  assert.ok(tableRowCount > 0, "ready state renders table records");
  assert.deepEqual(
    await table.getByRole("columnheader").allTextContents(),
    ["Student", "Week", "Action"],
    "table names each column",
  );
  const columnCount = await table.getByRole("columnheader").count();
  for (let rowIndex = 0; rowIndex < tableRowCount; rowIndex += 1) {
    const row = tableRows.nth(rowIndex);
    assert.equal(await row.getByRole("rowheader").count(), 1, "each record has a row header");
    assert.equal(
      await row.getByRole("cell").count(),
      columnCount - 1,
      "each record retains one cell per remaining column",
    );
  }
  assert.equal(
    await table
      .getByRole("columnheader", { name: "Action" })
      .evaluate((element) => getComputedStyle(element).textAlign),
    "end",
    "column alignment applies to headers",
  );
  assert.equal(
    await table
      .getByRole("row")
      .nth(1)
      .getByRole("cell")
      .nth(1)
      .evaluate((element) => getComputedStyle(element).textAlign),
    "end",
    "column alignment applies to data cells",
  );
  const outline = family.getByRole("list", { name: "Course modules", exact: true });
  assert.equal(await outline.evaluate((element) => element.tagName), "OL");
  const nestedOutline = family.getByRole("list", { name: "Module one assessments", exact: true });
  assert.equal(await nestedOutline.evaluate((element) => element.parentElement?.tagName), "LI");
  assert.deepEqual(await recordIds(nestedOutline), ["assessment-one"]);

  const details = family.getByRole("list", { name: "Attempt review records", exact: true });
  assert.deepEqual(await recordIds(details), ["attempt-one", "attempt-two"]);
  assert.ok((await details.innerText()).trim().length > 0, "expanded records retain their content");
}

async function checkPresentation(page) {
  const oneVariant = page.locator('[data-record-list-case="one-variant"]');
  assert.equal(
    await oneVariant.getByRole("group", { name: "Presentation choices" }).count(),
    0,
    "one declared presentation has no redundant switcher",
  );

  const twoVariants = page.locator('[data-record-list-case="two-variants"]');
  const choices = twoVariants.getByRole("group", { name: "Presentation choices" });
  await choices.waitFor({ state: "visible" });
  assert.equal(await choices.getByRole("button").count(), 2);
  assert.equal(
    await twoVariants.getByRole("button", { name: "Table" }).getAttribute("aria-pressed"),
    "true",
  );
  const list = twoVariants.getByRole("list", { name: "two-variants records" });
  const initialIds = await recordIds(list);

  await twoVariants.getByRole("button", { name: "Select Enzyme kinetics" }).click();
  await page.waitForFunction(
    () =>
      document
        .querySelector('[data-record-list-case="two-variants"] [data-record-list-selection]')
        ?.getAttribute("data-record-list-selection") === "enzyme",
  );
  await twoVariants.getByRole("button", { name: "Cards" }).click();
  await page.waitForFunction(
    () =>
      document
        .querySelector('[data-record-list-case="two-variants"] [data-record-list-presentation]')
        ?.getAttribute("data-record-list-presentation") === "cards",
  );
  assert.equal(
    await twoVariants
      .locator("[data-record-list-selection]")
      .getAttribute("data-record-list-selection"),
    "enzyme",
    "changing presentation retains the current record selection",
  );
  assert.deepEqual(
    await recordIds(list),
    initialIds,
    "presentation retains record identity and order",
  );
  assert.equal(
    await twoVariants.getByRole("button", { name: "Cards" }).getAttribute("aria-pressed"),
    "true",
  );
  assert.equal(
    await twoVariants.getByRole("button", { name: "Table" }).getAttribute("aria-pressed"),
    "false",
  );
}

async function checkReorder(page) {
  const reorderCase = page.locator('[data-record-list-case="reorder"]');
  const list = reorderCase.getByRole("list", { name: "Reorderable records" });
  await reorderCase.getByRole("button", { name: "Move Bravo earlier" }).press("ArrowUp");
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll('[data-record-list-case="reorder"] [data-record-id]')]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "bravo,alpha,charlie",
  );
  assert.deepEqual(await recordIds(list), ["bravo", "alpha", "charlie"]);
  assert.equal(await reorderCase.getByRole("status").innerText(), "Bravo moved to position 1.");
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Bravo later",
  );

  await reorderCase
    .getByRole("button", { name: "Drag Alpha to a new position" })
    .dragTo(reorderCase.getByRole("button", { name: "Drag Charlie to a new position" }));
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll('[data-record-list-case="reorder"] [data-record-id]')]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "bravo,charlie,alpha",
  );
  assert.deepEqual(await recordIds(list), ["bravo", "charlie", "alpha"]);
  assert.equal(await reorderCase.getByRole("status").innerText(), "Alpha moved to position 3.");
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Alpha earlier",
  );
}

async function recordRegions(list) {
  return list.locator("[data-record-id]").evaluateAll((rows) =>
    rows.map((row) => ({
      id: row.getAttribute("data-record-id"),
      regions: [...row.querySelectorAll("[data-record-region-id]")].map((region) =>
        region.getAttribute("data-record-region-id"),
      ),
    })),
  );
}

async function checkWindowing(page) {
  const windowCase = page.locator('[data-record-list-case="window"]');
  const list = windowCase.getByRole("list", { name: "Windowed records" });
  const fullList = windowCase.getByRole("list", { name: "All matched records" });
  assert.deepEqual(await recordIds(fullList), [
    "alpha",
    "bravo",
    "charlie",
    "delta",
    "echo",
    "foxtrot",
  ]);
  const fullRegionsById = new Map(
    (await recordRegions(fullList)).map((row) => [row.id, row.regions]),
  );
  assert.deepEqual(
    await recordIds(list),
    ["bravo", "charlie"],
    "window renders only its visible subset",
  );
  for (const row of await recordRegions(list)) {
    assert.deepEqual(
      row.regions,
      fullRegionsById.get(row.id),
      `windowed record ${row.id} keeps its regions`,
    );
  }

  await list.getByRole("button", { name: "Select Charlie" }).click();
  assert.deepEqual(
    await recordIds(list),
    ["bravo", "charlie"],
    "selection preserves the mounted subset",
  );
  await page.evaluate(() => window.recordListHarness.setWindowFocusedRecord("alpha"));
  await page.waitForFunction(() =>
    document.querySelector('[role="list"][aria-label="Windowed records"] [data-record-id="alpha"]'),
  );
  await list.getByRole("button", { name: "Select Alpha" }).focus();
  await page.evaluate(() => window.recordListHarness.setWindowScrollTop(250));
  await page.waitForFunction(() =>
    document.querySelector('[role="list"][aria-label="Windowed records"] [data-record-id="alpha"]'),
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.textContent?.trim()),
    "Select Alpha",
  );

  await page.evaluate(() => window.recordListHarness.setWindowFocusedRecord(undefined));
  await page.evaluate(() => window.recordListHarness.setWindowScrollTop(144));
  await page.waitForFunction(
    () =>
      [
        ...document.querySelectorAll(
          '[role="list"][aria-label="Windowed records"] [data-record-id]',
        ),
      ]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "delta",
  );
  assert.deepEqual(
    await recordIds(list),
    ["delta"],
    "measured row heights select the visible record",
  );
  assert.deepEqual(
    await recordRegions(list),
    [{ id: "delta", regions: fullRegionsById.get("delta") }],
    "the windowed row retains its full-list regions",
  );

  await windowCase.getByRole("button", { name: "Scroll to Foxtrot" }).click();
  await list.locator('[data-record-id="foxtrot"]').waitFor({ state: "visible" });
  assert.deepEqual(await recordIds(list), ["foxtrot"], "scroll-to-record mounts its target");
}

const { browser, consoleErrors, harnessServer, page, pageErrors } = await openRecordListHarness();
try {
  await checkAlignment(page);
  await checkPrimitiveStates(page);
  await checkRecordFamily(page);
  await checkPresentation(page);
  await checkReorder(page);
  await checkWindowing(page);
  assert.deepEqual(pageErrors, [], "RecordList contracts raise no browser page errors");
  assert.deepEqual(consoleErrors, [], "RecordList contracts raise no console errors");
  process.stdout.write(
    "RecordList family alignment, states, semantics, presentation, reorder, and windowing contracts passed.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
