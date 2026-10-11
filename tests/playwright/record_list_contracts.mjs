// Stable record states, accessible selection, metadata retention, image fallback, and movement.
// Fixture sections identify independent journeys; production record IDs identify ordered rows.
import assert from "node:assert/strict";

import { openRecordListHarness, recordIds } from "./helper_record_list_harness.mjs";

async function checkPrimitiveStates(page) {
  const ready = page.locator('[data-record-list-case="primitive-ready"]');
  await ready.getByRole("listitem").first().waitFor({ state: "visible" });
  assert.equal(await ready.getByRole("listitem").count(), 3, "ready state renders records");
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

async function checkMetadataUpdate(page) {
  const fixture = page.locator('[data-record-list-case="metadata-update"]');
  const list = fixture.getByRole("list", { name: "Metadata update records", exact: true });
  const draft = list.getByRole("textbox", { name: "Unsaved Question note", exact: true });
  const selected = list.getByLabel("Select DNA replication evidence", { exact: true });
  await selected.check();
  await draft.fill("A live unsaved draft");
  await fixture.getByRole("button", { name: "Refresh same records", exact: true }).click();
  await list.getByText("Refreshed metadata", { exact: true }).waitFor({ state: "visible" });
  assert.deepEqual(await recordIds(list), ["question-dna"], "metadata refresh keeps the record ID");
  assert.equal(await draft.inputValue(), "A live unsaved draft", "refresh preserves unsaved work");
  assert.equal(await selected.isChecked(), true, "refresh preserves the caller's selection");
}

async function checkSelection(page) {
  const fixture = page.locator('[data-record-list-case="selection"]');
  const radioList = fixture.getByRole("list", { name: "Single record selection", exact: true });
  const radioDna = radioList.getByRole("radio", {
    name: "Select DNA replication evidence",
    exact: true,
  });
  const radioScore = radioList.getByRole("radio", {
    name: "Select Week 2 Coursework",
    exact: true,
  });
  assert.equal(await radioDna.isChecked(), true, "radio reflects the caller's initial selection");
  assert.equal(
    await radioList.getByRole("radio", { name: "Select Amber arch", exact: true }).isDisabled(),
    true,
  );
  await radioScore.check();
  assert.equal(await radioScore.isChecked(), true);
  assert.equal(await radioDna.isChecked(), false, "radio choice clears the earlier selection");
  await radioScore.focus();
  await page.keyboard.press("ArrowUp");
  assert.equal(await radioDna.isChecked(), true, "radio supports native keyboard selection");

  const checkboxList = fixture.getByRole("list", {
    name: "Multiple record selection",
    exact: true,
  });
  const checkboxDna = checkboxList.getByRole("checkbox", {
    name: "Select DNA replication evidence",
    exact: true,
  });
  const checkboxScore = checkboxList.getByRole("checkbox", {
    name: "Select Week 2 Coursework",
    exact: true,
  });
  assert.equal(await checkboxScore.isChecked(), true);
  assert.equal(
    await checkboxList
      .getByRole("checkbox", { name: "Select Amber arch", exact: true })
      .isDisabled(),
    true,
  );
  await checkboxDna.check();
  assert.equal(await checkboxDna.isChecked(), true);
  assert.equal(await checkboxScore.isChecked(), true, "checkbox choices remain independent");
  await checkboxDna.focus();
  await page.keyboard.press("Space");
  assert.equal(await checkboxDna.isChecked(), false, "checkbox supports native keyboard selection");
  await checkboxScore.uncheck();
  assert.equal(await checkboxScore.isChecked(), false, "caller can remove a selection");
}

async function checkImageBrowser(page) {
  const fixture = page.locator('[data-record-list-case="image-browser"]');
  const list = fixture.getByRole("list", { name: "Image browser records", exact: true });
  const choices = fixture.getByRole("group", { name: "Record presentation", exact: true });
  const amber = list.getByRole("radio", { name: "Select Amber arch", exact: true });
  const fallback = list.getByRole("radio", { name: "Select Fallback arch", exact: true });
  await list.getByText("Amber arch", { exact: true }).waitFor({ state: "visible" });
  assert.equal(
    await choices
      .getByRole("button", { name: "Gallery", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await amber.focus();
  await page.keyboard.press("ArrowDown");
  assert.equal(await fallback.isChecked(), true, "Gallery supports native radio selection");
  await choices.getByRole("button", { name: "List", exact: true }).click();
  assert.equal(
    await choices.getByRole("button", { name: "List", exact: true }).getAttribute("aria-pressed"),
    "true",
  );
  assert.deepEqual(
    await recordIds(list),
    ["amber-arch", "fallback-arch"],
    "view change preserves records and order",
  );
  assert.equal(await fallback.isChecked(), true, "view change preserves selection");

  const failedRow = list.locator('[data-record-id="fallback-arch"]');
  await failedRow.locator("figure").waitFor({ state: "hidden" });
  await failedRow.getByText("Fallback arch", { exact: true }).waitFor({ state: "visible" });
  await failedRow
    .getByText("The name and description remain available when its image cannot load.", {
      exact: true,
    })
    .waitFor({ state: "visible" });
  assert.equal(await fallback.isVisible(), true, "failed image leaves selection available");
  await amber.check();
  assert.equal(await amber.isChecked(), true, "failed image leaves sibling selection usable");
}

async function waitForOrder(page, label, ids) {
  await page.waitForFunction(
    ({ label, ids }) => {
      const list = document.querySelector(`[aria-label="${label}"]`);
      const current = [...(list?.querySelectorAll("[data-record-id]") ?? [])].map((row) =>
        row.getAttribute("data-record-id"),
      );
      return current.join(",") === ids.join(",");
    },
    { label, ids },
  );
}

async function checkReorder(page) {
  const fixture = page.locator('[data-record-list-case="reorder"]');
  const list = fixture.getByRole("list", { name: "Reorderable records", exact: true });
  assert.equal(
    await list.getByRole("button", { name: "Move Alpha earlier", exact: true }).isDisabled(),
    true,
    "first record cannot move earlier",
  );
  assert.equal(
    await list.getByRole("button", { name: "Move Charlie later", exact: true }).isDisabled(),
    true,
    "last record cannot move later",
  );
  for (const name of ["Move Bravo earlier", "Move Bravo later", "Drag Bravo to a new position"]) {
    assert.equal(
      await list.getByRole("button", { name, exact: true }).isDisabled(),
      true,
      "caller policy disables movement",
    );
  }
  await list.getByRole("button", { name: "Move Alpha later", exact: true }).press("ArrowDown");
  await waitForOrder(page, "Reorderable records", ["bravo", "alpha", "charlie"]);
  assert.equal(await fixture.getByRole("status").innerText(), "Alpha moved to position 2.");
  await page.waitForFunction(
    () => document.activeElement?.getAttribute("aria-label") === "Move Alpha later",
  );
  await list
    .getByRole("button", { name: "Drag Alpha to a new position", exact: true })
    .dragTo(list.getByRole("button", { name: "Drag Charlie to a new position", exact: true }));
  await waitForOrder(page, "Reorderable records", ["bravo", "charlie", "alpha"]);
  assert.equal(await fixture.getByRole("status").innerText(), "Alpha moved to position 3.");

  const rejected = page.locator('[data-record-list-case="rejected-reorder"]');
  const rejectedList = rejected.getByRole("list", {
    name: "Rejected reordered records",
    exact: true,
  });
  await rejectedList.getByRole("button", { name: "Move Alpha later", exact: true }).click();
  await rejected
    .getByRole("alert")
    .getByText("The saved order was not updated.", { exact: true })
    .waitFor({ state: "visible" });
  assert.deepEqual(
    await recordIds(rejectedList),
    ["alpha", "bravo", "charlie"],
    "rejected save preserves the controlled order",
  );
  assert.equal(
    await rejected.getByRole("status").innerText(),
    "",
    "rejected save announces no successful move",
  );
}

const { browser, consoleErrors, harnessServer, page, pageErrors } = await openRecordListHarness();
try {
  await checkPrimitiveStates(page);
  await checkMetadataUpdate(page);
  await checkSelection(page);
  await checkImageBrowser(page);
  await checkReorder(page);
  assert.deepEqual(pageErrors, [], "RecordList contracts raise no browser page errors");
  assert.deepEqual(consoleErrors, [], "RecordList contracts raise no console errors");
  process.stdout.write(
    "RecordList state, metadata retention, selection, image fallback, and reorder contracts passed.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
