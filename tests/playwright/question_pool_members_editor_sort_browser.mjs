// Browser contract for editable Pool draft sorting that leaves Save membership unchanged.

import assert from "node:assert/strict";

import { chromium } from "playwright";

import { bundleQuestionPoolMembersEditorSortHarness } from "../support/question_pool_members_editor_sort_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const bundle = await bundleQuestionPoolMembersEditorSortHarness();
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
    if (!(target instanceof HTMLElement)) throw new Error("Pool editor harness root is missing.");
    window.PleQuestionPoolMembersEditorSortHarness.mountQuestionPoolMembersEditorSortHarness(
      target,
    );
  });

  const table = page.getByRole("table", { name: "Questions being edited" });
  await table.waitFor();
  const rowIds = async () =>
    table
      .locator("tbody tr")
      .evaluateAll((rows) => rows.map((row) => row.getAttribute("data-record-id")));
  await page.getByRole("button", { name: "Remove Mike from this Pool" }).click();

  const beforeRows = await rowIds();
  assert.deepEqual(beforeRows, ["7K3M-79QP:2", "8B7D-3C9F:1"]);
  const beforeTupleSet = [...beforeRows].sort();
  const saveButton = page.getByRole("button", { name: "Save Pool Questions" });
  const beforeEditNumber = await page.getByText("Based on Pool Edit Number 7").textContent();
  assert.equal(await saveButton.isEnabled(), true);

  await page.getByRole("combobox", { name: "Sort Pool Questions" }).selectOption("title-ascending");
  await page.waitForFunction(() => {
    const rows = document.querySelectorAll('table[aria-label="Questions being edited"] tbody tr');
    return rows[0]?.getAttribute("data-record-id") === "8B7D-3C9F:1";
  });

  const afterRows = await rowIds();
  assert.deepEqual(afterRows, ["8B7D-3C9F:1", "7K3M-79QP:2"]);
  assert.deepEqual([...afterRows].sort(), beforeTupleSet, "sort preserves the draft tuple set");
  assert.equal(await saveButton.isEnabled(), true, "sort leaves the existing dirty state intact");
  assert.equal(
    (await page.getByText("Based on Pool Edit Number 7").textContent())?.trim(),
    beforeEditNumber?.trim(),
    "sort leaves the acknowledged Edit Number unchanged",
  );

  await saveButton.click();
  await page.getByRole("status").getByText("Pool Questions saved.").waitFor();
  const saved = await page.evaluate(() => window.__pleQuestionPoolSaveRequests?.[0] ?? null);
  assert.equal(saved.questionPoolId, "3S8B-24DZ");
  assert.equal(saved.expectedQuestionPoolEditNumber, 7);
  assert.deepEqual(
    saved.members.map((tuple) => `${tuple.publishedQuestionId}:${tuple.revisionNumber}`).sort(),
    beforeTupleSet,
    "Save keeps the exact unordered Revision tuple set",
  );
  await page.getByText("Based on Pool Edit Number 8").waitFor();
  assert.deepEqual(pageErrors, [], "Pool editor sort has no page errors");
  assert.deepEqual(consoleErrors, [], "Pool editor sort has no console errors");
  process.stdout.write("Editable Pool display-sort/Save contract passed.\n");
} finally {
  await browser.close();
  await harnessServer.close();
}
