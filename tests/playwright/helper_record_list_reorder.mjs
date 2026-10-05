// Reordering checks for the shared RecordList browser contract.

import assert from "node:assert/strict";
import { recordIds } from "./helper_record_list_harness.mjs";

export async function checkSequenceReorder(page) {
  const reorderCase = page.locator('[data-record-list-case="sequence-reorder"]');
  const local = reorderCase.getByRole("list", { name: "Locally reordered records", exact: true });
  const asynchronous = reorderCase.getByRole("list", {
    name: "Asynchronously reordered records",
    exact: true,
  });
  const failed = reorderCase.getByRole("list", { name: "Failed reordered records", exact: true });
  const unchanged = reorderCase.getByRole("list", {
    name: "Unchanged reordered records",
    exact: true,
  });
  assert.equal(await local.evaluate((element) => element.tagName), "OL");
  assert.equal(
    await local.getByRole("button", { name: "Move Alpha earlier", exact: true }).isDisabled(),
    true,
    "first ordered record cannot move earlier",
  );
  assert.equal(
    await local.getByRole("button", { name: "Move Charlie later", exact: true }).isDisabled(),
    true,
    "last ordered record cannot move later",
  );
  assert.equal(
    await local.getByRole("button", { name: "Move Bravo earlier", exact: true }).isDisabled(),
    true,
    "caller disabled policy disables the record movement controls",
  );

  await local.getByRole("button", { name: "Move Alpha later", exact: true }).click();
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll('[aria-label="Locally reordered records"] [data-record-id]')]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "bravo,alpha,charlie",
  );
  assert.deepEqual(await recordIds(local), ["bravo", "alpha", "charlie"]);
  assert.equal(
    await reorderCase.getByRole("status").first().innerText(),
    "Alpha moved to position 2.",
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Alpha later",
    "successful movement restores focus by stable record ID",
  );
  await local
    .getByRole("button", { name: "Drag Alpha to a new position", exact: true })
    .dragTo(local.getByRole("button", { name: "Drag Charlie to a new position", exact: true }));
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll('[aria-label="Locally reordered records"] [data-record-id]')]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "bravo,charlie,alpha",
  );
  assert.equal(
    await reorderCase.getByRole("status").first().innerText(),
    "Alpha moved to position 3.",
    "native drag uses the same controlled movement contract",
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Alpha earlier",
    "native drag restores focus to the moved record's valid control by stable ID",
  );

  await asynchronous.getByRole("button", { name: "Move Bravo later", exact: true }).click();
  await page.waitForFunction(
    () =>
      [
        ...document.querySelectorAll(
          '[aria-label="Asynchronously reordered records"] [data-record-id]',
        ),
      ]
        .map((row) => row.getAttribute("data-record-id"))
        .join(",") === "alpha,charlie,bravo",
  );
  assert.deepEqual(await recordIds(asynchronous), ["alpha", "charlie", "bravo"]);
  assert.equal(
    await reorderCase.getByRole("status").nth(1).innerText(),
    "Bravo moved to position 3.",
    "asynchronous movement announces only after its supplied order changes",
  );

  await failed.getByRole("button", { name: "Move Bravo later", exact: true }).click();
  await reorderCase
    .getByRole("alert", { name: "" })
    .getByText("The saved order was not updated.", { exact: true })
    .waitFor({ state: "visible" });
  assert.deepEqual(
    await recordIds(failed),
    ["alpha", "bravo", "charlie"],
    "a failed move retains the controlled order",
  );
  assert.equal(
    await reorderCase.getByRole("status").nth(2).innerText(),
    "",
    "a failed move has no success announcement",
  );

  const unchangedMove = unchanged.getByRole("button", { name: "Move Bravo later", exact: true });
  await unchangedMove.focus();
  await unchangedMove.click();
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  assert.deepEqual(
    await recordIds(unchanged),
    ["alpha", "bravo", "charlie"],
    "a fulfilled no-op move retains the controlled order",
  );
  assert.equal(
    await reorderCase.getByRole("status").nth(3).innerText(),
    "",
    "a fulfilled no-op move has no success announcement",
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Bravo later",
    "a fulfilled no-op move leaves focus at the caller's active control",
  );
}

export async function checkOutlineReorder(page) {
  const outlineCase = page.locator('[data-record-list-case="outline-reorder"]');
  const outline = outlineCase.getByRole("list", { name: "Reordered fork Modules", exact: true });
  const asynchronous = outlineCase.getByRole("list", {
    name: "Asynchronously reordered fork Modules",
    exact: true,
  });
  const rejected = outlineCase.getByRole("list", {
    name: "Rejected reordered fork Modules",
    exact: true,
  });
  const unchanged = outlineCase.getByRole("list", {
    name: "Unchanged reordered fork Modules",
    exact: true,
  });
  const directOutlineIds = () =>
    outline
      .locator(":scope > [data-record-id]")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-record-id")));
  assert.equal(await outline.evaluate((element) => element.tagName), "OL");
  assert.deepEqual(await directOutlineIds(), ["alpha", "bravo", "charlie"]);
  assert.equal(
    await outline.getByRole("button", { name: "Move Alpha earlier", exact: true }).isDisabled(),
    true,
    "first Module cannot move earlier",
  );
  assert.equal(
    await outline.getByRole("button", { name: "Move Charlie later", exact: true }).isDisabled(),
    true,
    "last Module cannot move later",
  );
  assert.equal(
    await outline.getByRole("button", { name: "Move Bravo later", exact: true }).isDisabled(),
    true,
    "caller disabled policy applies to the Module controls",
  );
  assert.equal(
    await outline
      .getByRole("button", { name: "Drag Alpha to a new position", exact: true })
      .count(),
    1,
    "shared drag control renders for movable Modules",
  );
  assert.equal(
    await outline.getByRole("button", { name: "Move Alpha Assessment later", exact: true }).count(),
    0,
    "nested Assessment controls remain outside the Module reorder boundary",
  );

  await outline.getByRole("button", { name: "Move Alpha later", exact: true }).click();
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll('[aria-label="Reordered fork Modules"] > [data-record-id]')]
        .map((item) => item.getAttribute("data-record-id"))
        .join(",") === "bravo,alpha,charlie",
  );
  assert.deepEqual(await directOutlineIds(), ["bravo", "alpha", "charlie"]);
  assert.equal(
    await outlineCase.getByRole("status").first().innerText(),
    "Alpha moved to position 2.",
  );
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Alpha later",
    "Module movement restores focus by stable record ID",
  );

  await asynchronous.getByRole("button", { name: "Move Alpha later", exact: true }).click();
  await page.waitForFunction(
    () =>
      [
        ...document.querySelectorAll(
          '[aria-label="Asynchronously reordered fork Modules"] > [data-record-id]',
        ),
      ]
        .map((item) => item.getAttribute("data-record-id"))
        .join(",") === "bravo,alpha,charlie",
  );
  assert.deepEqual(
    await asynchronous
      .locator(":scope > [data-record-id]")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-record-id"))),
    ["bravo", "alpha", "charlie"],
  );
  assert.equal(
    await outlineCase.getByRole("status").nth(1).innerText(),
    "Alpha moved to position 2.",
    "async Module movement announces only after the controlled order changes",
  );
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  assert.equal(
    await page.evaluate(
      () =>
        document.activeElement?.getAttribute("aria-label") === "Move Alpha later" &&
        document.activeElement?.closest("ol")?.getAttribute("aria-label") ===
          "Asynchronously reordered fork Modules",
    ),
    true,
    "async Module movement focuses the accepted record control",
  );

  const rejectedMove = rejected.getByRole("button", { name: "Move Alpha later", exact: true });
  await rejectedMove.focus();
  await rejectedMove.click();
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  assert.deepEqual(
    await rejected
      .locator(":scope > [data-record-id]")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-record-id"))),
    ["alpha", "bravo", "charlie"],
    "a rejected Module move retains the controlled order",
  );
  assert.equal(
    await outlineCase.getByRole("status").nth(2).innerText(),
    "",
    "a rejected Module move has no success announcement",
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.getAttribute("aria-label")),
    "Move Alpha later",
    "a rejected Module move leaves focus at the caller's active control",
  );

  const unchangedMove = unchanged.getByRole("button", { name: "Move Alpha later", exact: true });
  await unchangedMove.focus();
  await unchangedMove.click();
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  assert.deepEqual(
    await unchanged
      .locator(":scope > [data-record-id]")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-record-id"))),
    ["alpha", "bravo", "charlie"],
    "a fulfilled Module no-op retains the controlled order",
  );
  assert.equal(
    await outlineCase.getByRole("status").nth(3).innerText(),
    "",
    "a fulfilled Module no-op has no success announcement",
  );
  assert.equal(
    await unchangedMove.evaluate((element) => document.activeElement === element),
    true,
    "a fulfilled Module no-op leaves focus on the exact active Module control",
  );
}

export async function checkReorder(page) {
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
