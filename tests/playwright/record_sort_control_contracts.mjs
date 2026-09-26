// Durable browser contract for the shared RecordSortControl API.

import assert from "node:assert/strict";

import { chromium } from "playwright";

import { bundleRecordSortControlHarness } from "../support/record_sort_control_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const bundle = await bundleRecordSortControlHarness();
const harnessServer = await startHarnessServer(
  `<!doctype html><html><head><style>${bundle.stylesheet}</style></head><body><div id="root"></div></body></html>`,
  bundle.stylesheet,
);
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
const pageErrors = [];
const consoleErrors = [];
page.on("pageerror", (error) => pageErrors.push(error.message));
page.on("console", (message) => {
  if (message.type() === "error") consoleErrors.push(message.text());
});

try {
  await page.goto(harnessServer.evidenceUrl);
  await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
  await page.evaluate(() => {
    const target = document.querySelector("#root");
    if (!(target instanceof HTMLElement))
      throw new Error("RecordSortControl harness root is missing.");
    window.PleRecordSortControlHarness.mountRecordSortControlHarness(target);
  });

  const sortControl = page.getByRole("combobox", { name: "Sort Question Library", exact: true });
  assert.equal(await sortControl.evaluate((element) => element.tagName), "SELECT");
  assert.deepEqual(await sortControl.locator("option").allTextContents(), [
    "Title",
    "Most recently published",
  ]);
  assert.equal(await sortControl.inputValue(), "title", "the caller controls the initial value");

  await sortControl.selectOption("recent-publication");
  const currentSort = page.locator("[data-record-sort-control-current]");
  await page.waitForFunction(() =>
    document
      .querySelector("[data-record-sort-control-current]")
      ?.textContent?.includes("recent-publication"),
  );
  assert.equal(await sortControl.inputValue(), "recent-publication");
  assert.equal(await currentSort.getAttribute("data-record-sort-control-change-count"), "1");

  const disabledControl = page.getByRole("combobox", {
    name: "Sort unavailable results",
    exact: true,
  });
  assert.equal(
    await disabledControl.isDisabled(),
    true,
    "caller disabled state reaches the native select",
  );
  assert.deepEqual(pageErrors, [], "sort-control harness has no page errors");
  assert.deepEqual(consoleErrors, [], "sort-control harness has no console errors");
  process.stdout.write("RecordSortControl native controlled-select browser contract passed.\n");
} finally {
  await browser.close();
  await harnessServer.close();
}
