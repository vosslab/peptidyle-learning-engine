import assert from "node:assert/strict";
import test from "node:test";

import {
  addMultiFillBlank,
  removeMultiFillBlank,
  reorderMultiFillBlanks,
  setMultiFillBlankAnswer,
} from "../src/features/flat_question_authoring/flat_multi_fill_in_editor_model.ts";
import {
  addOrderingItem,
  moveOrderingItem,
  removeOrderingItem,
  setOrderingItemText,
} from "../src/features/flat_question_authoring/flat_ordering_editor_model.ts";

function source(response) {
  return {
    format: "pleFlatQuestion",
    version: 2,
    title: "Question",
    prompt: "Prompt",
    response,
    feedback: { correct: null, incorrect: null },
    points: 1,
    attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
    timingPolicy: { kind: "untimed" },
    tags: [],
    taxonomy: [],
    license: { kind: "allRightsReserved" },
    language: "en",
  };
}

function multiFillSource() {
  return source({
    kind: "multiFillIn",
    blanks: [
      {
        id: "blank_a",
        label: "Gene",
        answers: ["BRCA1", "breast cancer 1"],
        matchMode: "caseInsensitive",
        maxLength: 256,
      },
      {
        id: "blank_b",
        label: "Chromosome",
        answers: ["17"],
        matchMode: "exact",
        maxLength: 8,
      },
    ],
  });
}

function orderingSource() {
  return source({
    kind: "ordering",
    items: [
      { id: "translation", text: "Translation" },
      { id: "transcription", text: "Transcription" },
      { id: "replication", text: "Replication" },
      { id: "repair", text: "Repair" },
    ],
    correctOrder: ["translation", "transcription", "replication", "repair"],
  });
}

test("MULTI-FIB reorders stable blank records without separating their accepted answers", () => {
  const reordered = reorderMultiFillBlanks(multiFillSource(), ["blank_b", "blank_a"]);

  assert.equal(reordered.changed, true);
  assert.deepEqual(
    reordered.source.response.kind === "multiFillIn"
      ? reordered.source.response.blanks.map((blank) => [blank.id, blank.answers])
      : [],
    [
      ["blank_b", ["17"]],
      ["blank_a", ["BRCA1", "breast cancer 1"]],
    ],
  );
});

test("MULTI-FIB deletion removes only the selected blank and its direct answer relation", () => {
  const deleted = removeMultiFillBlank(multiFillSource(), "blank_a");

  assert.equal(deleted.changed, true);
  assert.equal(deleted.focusId, "blank_b");
  assert.deepEqual(
    deleted.source.response.kind === "multiFillIn" ? deleted.source.response.blanks : [],
    [
      {
        id: "blank_b",
        label: "Chromosome",
        answers: ["17"],
        matchMode: "exact",
        maxLength: 8,
      },
    ],
  );
});

test("MULTI-FIB accepted answer edits enforce their blank-specific maximum", () => {
  const edited = setMultiFillBlankAnswer(multiFillSource(), "blank_b", 0, "17p13.1");
  assert.equal(edited.changed, true);
  const refused = setMultiFillBlankAnswer(edited.source, "blank_b", 0, "chromosome 17");
  assert.equal(refused.changed, false);
  assert.match(refused.error ?? "", /Maximum length|1 to 8/);
});

test("MULTI-FIB generated IDs do not reuse a live stable blank ID", () => {
  const added = addMultiFillBlank(multiFillSource());
  assert.equal(added.changed, true);
  assert.equal(added.focusId, "blank_1");
  assert.equal(
    added.source.response.kind === "multiFillIn"
      ? added.source.response.blanks.at(-1)?.id
      : undefined,
    "blank_1",
  );
});

test("ORDER maintains one canonical private order and derives correctOrder after movement", () => {
  const moved = moveOrderingItem(orderingSource(), "replication", "earlier");
  assert.equal(moved.changed, true);
  assert.equal(moved.focusId, "replication");
  if (moved.source.response.kind !== "ordering") throw new Error("Expected ordering response.");
  assert.deepEqual(
    moved.source.response.items.map((item) => item.id),
    ["translation", "replication", "transcription", "repair"],
  );
  assert.deepEqual(moved.source.response.correctOrder, [
    "translation",
    "replication",
    "transcription",
    "repair",
  ]);
});

test("ORDER deletion changes only the deleted item relationship and preserves the remaining sequence", () => {
  const deleted = removeOrderingItem(orderingSource(), "transcription");
  assert.equal(deleted.changed, true);
  assert.equal(deleted.focusId, "replication");
  if (deleted.source.response.kind !== "ordering") throw new Error("Expected ordering response.");
  assert.deepEqual(
    deleted.source.response.items.map((item) => item.id),
    ["translation", "replication", "repair"],
  );
  assert.deepEqual(deleted.source.response.correctOrder, ["translation", "replication", "repair"]);
});

test("ORDER item text edit retains identity and add creates a distinct stable item", () => {
  const renamed = setOrderingItemText(orderingSource(), "translation", "Translate protein");
  const added = addOrderingItem(renamed.source);
  if (added.source.response.kind !== "ordering") throw new Error("Expected ordering response.");
  assert.equal(added.source.response.items[0]?.id, "translation");
  assert.equal(added.source.response.items[0]?.text, "Translate protein");
  assert.equal(added.focusId, "item_1");
  assert.equal(added.source.response.correctOrder.at(-1), "item_1");
});
