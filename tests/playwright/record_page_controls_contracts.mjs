// Durable browser contract for the shared RecordPageControls API.

import assert from "node:assert/strict";

import { chromium } from "playwright";

import { bundleRecordPageControlsHarness } from "../support/record_page_controls_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const bundle = await bundleRecordPageControlsHarness();
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
      throw new Error("RecordPageControls harness root is missing.");
    window.PleRecordPageControlsHarness.mountRecordPageControlsHarness(target);
  });

  const controls = page.getByRole("navigation", { name: "Question Library pages", exact: true });
  const previous = controls.getByRole("button", { name: "Previous", exact: true });
  const next = controls.getByRole("button", { name: "Next", exact: true });
  assert.equal(await previous.isDisabled(), true, "the unavailable previous cursor is disabled");
  assert.equal(await next.isDisabled(), false, "the available next cursor is enabled");

  await next.focus();
  await page.keyboard.press("Enter");
  const state = page.locator("[data-record-page-controls-action]");
  await page.waitForFunction(
    () =>
      document
        .querySelector("[data-record-page-controls-action]")
        ?.getAttribute("data-record-page-controls-action") === "next",
  );
  assert.equal(await state.getAttribute("data-record-page-controls-action"), "next");
  assert.equal(
    await previous.isDisabled(),
    false,
    "caller state enables the previous cursor after next",
  );
  assert.equal(
    await next.isDisabled(),
    true,
    "caller state disables the next cursor at its boundary",
  );

  const size = controls.getByRole("combobox", { name: "Records per page", exact: true });
  assert.deepEqual(await size.locator("option").allTextContents(), ["50", "100", "250"]);
  await size.selectOption("100");
  await page.waitForFunction(
    () =>
      document
        .querySelector("[data-record-page-controls-size]")
        ?.getAttribute("data-record-page-controls-size") === "100",
  );
  assert.equal(await size.inputValue(), "100", "the caller controls the emitted page size");

  await page.getByRole("button", { name: "Toggle page loading", exact: true }).click();
  assert.equal(await controls.getAttribute("aria-busy"), "true");
  assert.equal(await previous.isDisabled(), true, "loading disables available navigation");
  assert.equal(await size.isDisabled(), true, "loading disables the page-size choice");

  const taskControls = page.getByRole("navigation", {
    name: "Assessment entries pages",
    exact: true,
  });
  assert.equal(await taskControls.getByRole("combobox").count(), 0, "local lists omit page size");
  assert.deepEqual(pageErrors, [], "page-controls harness has no page errors");
  assert.deepEqual(consoleErrors, [], "page-controls harness has no console errors");
  process.stdout.write(
    "RecordPageControls native controlled-navigation browser contract passed.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
