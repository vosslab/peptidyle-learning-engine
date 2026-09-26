// Durable browser contracts for the production RecordList APIs.

import assert from "node:assert/strict";

import { openRecordListHarness, recordIds } from "./helper_record_list_harness.mjs";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";

async function checkAlignment(page) {
  const alignmentCase = page.locator('[data-record-list-case="alignment"]');
  const expected = [
    { recordId: "brief", title: "DNA", status: "Ready", action: "Open DNA" },
    {
      recordId: "detailed",
      title: "Genome-wide association study preparation",
      status: "Needs review",
      action: "Open Genome-wide association study preparation",
    },
  ];

  for (const [id, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    const list = alignmentCase.getByRole("list", { name: "Alignment records", exact: true });
    const rows = await list.getByRole("listitem").evaluateAll((elements) =>
      elements.map((row) => {
        const visible = (element) =>
          element instanceof HTMLElement &&
          getComputedStyle(element).display !== "none" &&
          element.getBoundingClientRect().width > 0;
        const title = row.querySelector(".record-list__title");
        const status = [...row.querySelectorAll(".record-list__fact")].find((fact) =>
          fact.textContent?.includes("Status"),
        );
        const action = row.querySelector("button");
        return {
          recordId: row.getAttribute("data-record-id"),
          title: title?.textContent?.trim() ?? "",
          titleVisible: visible(title),
          status: status?.textContent?.trim() ?? "",
          statusVisible: visible(status),
          action: action?.textContent?.trim() ?? "",
          actionVisible: visible(action),
        };
      }),
    );
    assert.deepEqual(await recordIds(list), ["brief", "detailed"], `${id} record order`);
    assert.equal(rows.length, 2, `${id} retains all alignment records`);
    for (const [index, row] of rows.entries()) {
      assert.equal(row.recordId, expected[index].recordId, `${id} keeps record identity`);
      assert.equal(row.title, expected[index].title, `${id} retains the record title`);
      assert.equal(row.titleVisible, true, `${id} keeps the title visible`);
      assert.match(row.status, new RegExp(expected[index].status), `${id} retains status`);
      assert.equal(row.statusVisible, true, `${id} keeps status visible`);
      assert.equal(row.action, expected[index].action, `${id} retains the record action`);
      assert.equal(row.actionVisible, true, `${id} retains actions`);
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

async function checkSemanticContent(page) {
  const semanticCase = page.locator('[data-record-list-case="semantic-content"]');
  const list = semanticCase.getByRole("list", { name: "Semantic content records", exact: true });
  await list.getByRole("listitem").first().waitFor({ state: "visible" });
  assert.deepEqual(await recordIds(list), ["question-dna", "avatar-amber", "student-score"]);
  assert.match(await list.innerText(), /Score not released/);
  assert.equal(
    await list
      .getByRole("link", { name: "Course: Molecular Biology", exact: true })
      .getAttribute("href"),
    "/courses/molecular-biology",
    "linked facts preserve their native destination",
  );
  assert.equal(
    await list
      .getByRole("button", { name: "Use template", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  assert.equal(
    await list
      .getByRole("button", { name: "Known Forks", exact: true })
      .getAttribute("aria-expanded"),
    "true",
  );
  assert.equal(
    await list
      .getByRole("button", { name: "Known Forks", exact: true })
      .getAttribute("aria-controls"),
    "semantic-known-forks",
  );
  const timeFact = list.locator('time[datetime="2026-09-25T14:30:00.000Z"]');
  assert.equal(await timeFact.innerText(), "September 25, 2026, 9:30 AM");
  await page.evaluate(() => {
    window.recordListClipboardWrites = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (value) => {
          window.recordListClipboardWrites.push(value);
        },
      },
    });
  });
  await list.getByRole("button", { name: "Copy Question ID 7K3M-79QP", exact: true }).click();
  await page.waitForFunction(() => window.recordListClipboardWrites[0] === "7K3M-79QP");
  assert.deepEqual(
    await page.evaluate(() => window.recordListClipboardWrites),
    ["7K3M-79QP"],
    "copyable Question IDs write their exact canonical value",
  );
  await list
    .locator(".copyable-question-id-status")
    .getByText("Copied 7K3M-79QP.", { exact: true })
    .waitFor({ state: "visible" });
  await list.getByRole("img", { name: /Amber arch/u }).waitFor({ state: "visible" });
  const classificationFact = list.locator(".record-list__fact--course-classification");
  await classificationFact.getByText("Molecular Biology (retired)", { exact: false }).waitFor({
    state: "visible",
  });
  assert.match(
    await classificationFact.innerText(),
    /Subject: Nucleic acids; Tags: replication, evidence/u,
    "classification facts preserve current vocabulary labels and tags",
  );
  assert.equal(
    await classificationFact.evaluate(
      (element) => element.closest('[data-record-id="question-dna"]') !== null,
    ),
    true,
    "classification renders in the shared Question record facts",
  );

  const draft = list.getByRole("textbox", { name: "Unsaved Question note", exact: true });
  const templateAction = await list
    .getByRole("button", { name: "Use template", exact: true })
    .elementHandle();
  assert.notEqual(templateAction, null, "template action has a native button owner");
  await draft.fill("A live unsaved draft");
  await draft.focus();
  await page.evaluate(() => window.recordListHarness.refreshSemanticRecords());
  await page.waitForFunction(() => document.body.textContent?.includes("Refreshed metadata"));
  assert.equal(await draft.inputValue(), "A live unsaved draft");
  assert.equal(
    await page.evaluate(
      () =>
        document.activeElement instanceof HTMLInputElement &&
        document.activeElement.value === "A live unsaved draft",
    ),
    true,
    "same-ID refresh retains the focused body input",
  );
  assert.equal(
    await list.getByRole("button", { name: "Use template", exact: true }).isDisabled(),
    true,
  );
  assert.equal(
    await templateAction.evaluate(
      (element) =>
        element.isConnected &&
        element ===
          document.querySelector('[data-record-id="question-dna"] button[aria-pressed="true"]'),
    ),
    true,
    "same-ID action state update retains its native button owner",
  );

  await semanticCase.getByRole("button", { name: "Show retained loading", exact: true }).click();
  await semanticCase
    .getByText("Refreshing records...", { exact: true })
    .waitFor({ state: "visible" });
  assert.equal(
    await list.getByRole("listitem").count(),
    3,
    "loading notice retains loaded records",
  );
  await semanticCase.getByRole("button", { name: "Show retained error", exact: true }).click();
  await semanticCase.getByRole("alert").waitFor({ state: "visible" });
  assert.equal(await list.getByRole("listitem").count(), 3, "error notice retains loaded records");
  await semanticCase.getByRole("button", { name: "Retry", exact: true }).click();

  await page.setViewportSize({ width: 393, height: 844 });
  assert.equal(
    await classificationFact.evaluate((element) => {
      const bounds = element.getBoundingClientRect();
      return getComputedStyle(element).display !== "none" && bounds.width > 0 && bounds.height > 0;
    }),
    true,
    "classification fact remains visible in the shared narrow row",
  );
  const visibleSemanticText = await list
    .locator(".record-list__title, .record-list__fact, .record-list__action")
    .evaluateAll((elements) =>
      elements.every((element) => {
        const bounds = element.getBoundingClientRect();
        return (
          getComputedStyle(element).display !== "none" && bounds.width > 0 && bounds.height > 0
        );
      }),
    );
  assert.equal(visibleSemanticText, true, "shared semantic content reflows without omitted fields");
  await page.setViewportSize({ width: 1024, height: 768 });
}

async function checkSelection(page) {
  const selectionCase = page.locator('[data-record-list-case="selection"]');
  const radioList = selectionCase.getByRole("list", {
    name: "Single record selection",
    exact: true,
  });
  const checkboxList = selectionCase.getByRole("list", {
    name: "Multiple record selection",
    exact: true,
  });
  const radioDna = radioList.getByLabel("Select DNA replication evidence", { exact: true });
  const radioScore = radioList.getByLabel("Select Week 2 Coursework", { exact: true });
  const radioAvatar = radioList.getByLabel("Select Amber arch", { exact: true });

  assert.equal(await radioDna.getAttribute("type"), "radio");
  assert.equal(await radioDna.isChecked(), true, "radio selection reflects page state");
  assert.equal(
    await radioAvatar.isDisabled(),
    true,
    "radio disabled decision remains caller-owned",
  );
  assert.equal(
    await radioDna.getAttribute("name"),
    await radioScore.getAttribute("name"),
    "shared radio controls form one native group",
  );
  assert.notEqual(
    await radioDna.getAttribute("name"),
    null,
    "shared radio group has a native name",
  );
  await radioScore.check();
  assert.equal(await radioScore.isChecked(), true, "radio callback updates controlled selection");
  assert.equal(
    await radioDna.isChecked(),
    false,
    "radio group clears the prior controlled selection",
  );
  await radioScore.focus();
  await page.keyboard.press("ArrowUp");
  assert.equal(
    await radioDna.isChecked(),
    true,
    "native radio keyboard movement updates page state",
  );

  const checkboxDna = checkboxList.getByLabel("Select DNA replication evidence", { exact: true });
  const checkboxScore = checkboxList.getByLabel("Select Week 2 Coursework", { exact: true });
  const checkboxAvatar = checkboxList.getByLabel("Select Amber arch", { exact: true });
  assert.equal(await checkboxDna.getAttribute("type"), "checkbox");
  assert.equal(await checkboxScore.isChecked(), true, "checkbox selection reflects page state");
  assert.equal(
    await checkboxAvatar.isDisabled(),
    true,
    "checkbox disabled decision remains caller-owned",
  );
  await checkboxDna.check();
  assert.equal(
    await checkboxDna.isChecked(),
    true,
    "checkbox callback adds a controlled selection",
  );
  assert.equal(await checkboxScore.isChecked(), true, "checkbox retains independent selections");
  await checkboxDna.focus();
  await page.keyboard.press("Space");
  assert.equal(
    await checkboxDna.isChecked(),
    false,
    "native checkbox keyboard movement updates page state",
  );
  await checkboxScore.uncheck();
  assert.equal(
    await checkboxScore.isChecked(),
    false,
    "checkbox callback removes a controlled selection",
  );

  const selectedRow = radioScore.locator("xpath=ancestor::article");
  await radioScore.focus();
  assert.equal(
    await selectedRow
      .locator(".record-list__selection")
      .evaluate((element) => getComputedStyle(element).outlineStyle),
    "solid",
    "selection focus receives the shared visible treatment",
  );
  assert.equal(
    await selectedRow.getByRole("link", { name: "Open", exact: true }).count(),
    1,
    "record commands remain outside the selection label",
  );
  assert.equal(
    await selectedRow.locator("label").locator("a, button").count(),
    0,
    "selection labels do not contain record links or commands",
  );
}

async function checkImageBrowser(page) {
  const browserCase = page.locator('[data-record-list-case="image-browser"]');
  const list = browserCase.getByRole("list", { name: "Image browser records", exact: true });
  const choices = browserCase.getByRole("group", { name: "Record presentation", exact: true });
  const expectedIds = ["amber-arch", "fallback-arch"];

  await list.getByRole("listitem").first().waitFor({ state: "visible" });
  assert.equal(
    await browserCase
      .locator(".record-list-image-browser")
      .getAttribute("data-record-list-presentation"),
    "gallery",
    "image browsing starts in the shared visual Gallery",
  );
  assert.deepEqual(await recordIds(list), expectedIds, "Gallery retains every record identity");
  await list.getByText("Amber arch", { exact: true }).waitFor({ state: "visible" });
  await list.getByText("Fallback arch", { exact: true }).waitFor({ state: "visible" });

  const amber = list.getByLabel("Select Amber arch", { exact: true });
  const fallback = list.getByLabel("Select Fallback arch", { exact: true });
  await amber.focus();
  await page.keyboard.press("ArrowDown");
  assert.equal(
    await fallback.isChecked(),
    true,
    "Gallery keeps native controlled radio navigation",
  );

  await choices.getByRole("button", { name: "List", exact: true }).click();
  assert.equal(
    await browserCase
      .locator(".record-list-image-browser")
      .getAttribute("data-record-list-presentation"),
    "list",
    "shared switcher changes the image presentation",
  );
  assert.deepEqual(
    await recordIds(list),
    expectedIds,
    "List keeps the same record identity and order",
  );
  assert.equal(await fallback.isChecked(), true, "view changes retain caller-controlled selection");
  await list.getByText("Fallback arch", { exact: true }).waitFor({ state: "visible" });

  const failedMedia = list.locator('[data-record-id="fallback-arch"] .record-list__media');
  await failedMedia.locator("img").evaluate((image) => image.dispatchEvent(new Event("error")));
  await page.waitForFunction(() =>
    document
      .querySelector('[data-record-id="fallback-arch"] .record-list__media')
      ?.hasAttribute("hidden"),
  );
  assert.equal(await failedMedia.isHidden(), true, "failed images use the shared fallback");
  assert.equal(await fallback.isVisible(), true, "image fallback leaves native selection usable");
  await list.getByText("Fallback arch", { exact: true }).waitFor({ state: "visible" });
  assert.equal(await amber.isVisible(), true, "image fallback leaves sibling records usable");

  await failedMedia.locator("img").evaluate((image) => image.dispatchEvent(new Event("load")));
  await page.waitForFunction(
    () =>
      !document
        .querySelector('[data-record-id="fallback-arch"] .record-list__media')
        ?.hasAttribute("hidden"),
  );
  assert.equal(
    await failedMedia.isVisible(),
    true,
    "a later successful image load restores shared media",
  );
}

async function checkSemanticSequence(page) {
  const sequenceCase = page.locator('[data-record-list-case="semantic-sequence"]');
  const sequence = sequenceCase.getByRole("list", {
    name: "Semantic ordered records",
    exact: true,
  });
  await sequence.getByRole("listitem").first().waitFor({ state: "visible" });
  assert.equal(await sequence.evaluate((element) => element.tagName), "OL");
  assert.deepEqual(await recordIds(sequence), ["question-dna", "avatar-amber"]);

  const questionRecord = sequence.locator('[data-record-id="question-dna"]');
  const draft = sequence.getByRole("textbox", { name: "Unsaved ordered note", exact: true });
  const inspectAction = await questionRecord
    .getByRole("button", { name: "Inspect", exact: true })
    .elementHandle();
  assert.notEqual(inspectAction, null, "ordered action has a native button owner");
  await draft.fill("A live ordered draft");
  await draft.focus();
  await page.evaluate(() => window.recordListHarness.refreshSemanticSequence());
  await page.waitForFunction(() =>
    document.body.textContent?.includes("Refreshed ordered metadata"),
  );
  assert.equal(await draft.inputValue(), "A live ordered draft");
  assert.equal(
    await page.evaluate(
      () =>
        document.activeElement instanceof HTMLInputElement &&
        document.activeElement.value === "A live ordered draft",
    ),
    true,
    "same-ID Sequence refresh retains the focused body input",
  );
  assert.equal(
    await questionRecord.getByRole("button", { name: "Inspect", exact: true }).isDisabled(),
    true,
  );
  assert.equal(
    await inspectAction.evaluate(
      (element) => element.isConnected && element instanceof HTMLButtonElement && element.disabled,
    ),
    true,
    "same-ID Sequence refresh retains its native action owner",
  );

  await sequenceCase
    .getByRole("button", { name: "Show retained sequence loading", exact: true })
    .click();
  await sequenceCase
    .getByText("Refreshing order...", { exact: true })
    .waitFor({ state: "visible" });
  assert.equal(await sequence.getByRole("listitem").count(), 2, "loading retains ordered records");
  await sequenceCase
    .getByRole("button", { name: "Show retained sequence error", exact: true })
    .click();
  await sequenceCase.getByRole("alert").waitFor({ state: "visible" });
  assert.equal(await sequence.getByRole("listitem").count(), 2, "error retains ordered records");
  await sequenceCase.getByRole("button", { name: "Retry", exact: true }).click();
  await sequenceCase
    .getByRole("heading", { name: "No semantic ordered records", exact: true })
    .waitFor({ state: "visible" });

  await page.setViewportSize({ width: 393, height: 844 });
  const visibleSequenceContent = await sequence
    .locator(".record-list__title, .record-list__fact, .record-list__action")
    .evaluateAll((elements) =>
      elements.every((element) => {
        const bounds = element.getBoundingClientRect();
        return (
          getComputedStyle(element).display !== "none" && bounds.width > 0 && bounds.height > 0
        );
      }),
    );
  assert.equal(visibleSequenceContent, true, "shared semantic sequence content reflows naturally");
  await page.setViewportSize({ width: 1024, height: 768 });
}

async function checkSequenceReorder(page) {
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

async function checkOutlineReorder(page) {
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

async function checkDuplicateActionIds(page) {
  const duplicateCase = page.locator('[data-record-list-case="duplicate-action-id"]');
  await duplicateCase
    .getByRole("button", { name: "Render duplicate action IDs", exact: true })
    .click();
  const alert = duplicateCase.getByRole("alert");
  await alert.waitFor({ state: "visible" });
  assert.match(await alert.innerText(), /duplicate action ID open/u);
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

const { browser, consoleErrors, harnessServer, page, pageErrors } = await openRecordListHarness();
try {
  await checkAlignment(page);
  await checkPrimitiveStates(page);
  await checkSemanticContent(page);
  await checkSelection(page);
  await checkImageBrowser(page);
  await checkSemanticSequence(page);
  await checkSequenceReorder(page);
  await checkOutlineReorder(page);
  await checkDuplicateActionIds(page);
  await checkRecordFamily(page);
  await checkPresentation(page);
  await checkReorder(page);
  assert.deepEqual(pageErrors, [], "RecordList contracts raise no browser page errors");
  assert.deepEqual(consoleErrors, [], "RecordList contracts raise no console errors");
  process.stdout.write(
    "RecordList shared semantic-content, selection, image presentation, state, family, and reorder contracts passed.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
