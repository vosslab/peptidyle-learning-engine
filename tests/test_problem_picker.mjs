import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_PROBLEM_PICKER_SELECTION_CAP,
  ProblemPickerSession,
  moveProblemPickerSelection,
  problemPickerSelection,
  toggleProblemPickerSelection,
} from "../src/features/problem_picker/problem_picker_model.ts";

function row(displayId, title = "Question") {
  return {
    displayId,
    title,
    summary: "Answer-free summary.",
    byline: ["Published author"],
    taxonomy: ["biology:protein"],
    capabilities: [],
    license: "ccBy",
    evidence: { state: "insufficientEvidence" },
  };
}

function canonicalQuestionIdFor(index) {
  const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  const characters = Array.from({ length: 7 }, () => "0");
  let remaining = index;
  for (let position = characters.length - 1; position >= 0; position -= 1) {
    const digit = remaining % alphabet.length;
    characters[position] = alphabet[digit];
    remaining = Math.floor(remaining / alphabet.length);
  }
  return `${characters.slice(0, 3).join("")}-${characters.slice(3).join("")}`;
}

test("problem picker preserves public Question ID order and safe row metadata", () => {
  const selection = problemPickerSelection("many", 200, [
    row("7K3-M9QP", "First"),
    row("2R5-X7YA", "Second"),
  ]);
  assert.deepEqual(selection.questionIds, ["7K3-M9QP", "2R5-X7YA"]);
  assert.equal(selection.questions[1]?.row.title, "Second");
});

test("single-selection mode replaces the prior result", () => {
  const initial = problemPickerSelection("one", 1, [row("7K3-M9QP")]);
  const next = toggleProblemPickerSelection("one", 1, initial, row("2R5-X7YA"), true);
  assert.deepEqual(next.questionIds, ["2R5-X7YA"]);
});

test("picker removes a selected row and preserves the other selected order", () => {
  const initial = problemPickerSelection("many", 200, [row("7K3-M9QP"), row("2R5-X7YA")]);
  const next = toggleProblemPickerSelection("many", 200, initial, row("7K3-M9QP"), false);
  assert.deepEqual(next.questionIds, ["2R5-X7YA"]);
});

test("picker reorders the selected tray without changing membership", () => {
  const initial = problemPickerSelection("many", 200, [row("7K3-M9QP"), row("2R5-X7YA")]);
  const next = moveProblemPickerSelection("many", 200, initial, 1, -1);
  assert.deepEqual(next.questionIds, ["2R5-X7YA", "7K3-M9QP"]);
});

test("picker enforces the shared bounded selection limit", () => {
  const rows = Array.from({ length: MAX_PROBLEM_PICKER_SELECTION_CAP + 1 }, (_value, index) => {
    return row(canonicalQuestionIdFor(index));
  });
  assert.throws(
    () => problemPickerSelection("many", MAX_PROBLEM_PICKER_SELECTION_CAP, rows),
    /at most/,
  );
});

test("picker session drops a stale source response before publishing it", async () => {
  let resolveCatalog;
  const catalog = new Promise((resolve) => {
    resolveCatalog = resolve;
  });
  const states = [];
  const session = new ProblemPickerSession(
    {
      search: async (request) => {
        if (request.source.kind === "catalog") return await catalog;
        return { items: [row("2R5-X7YA", "Mine")], aggregates: [], nextCursor: null };
      },
    },
    (state) => states.push(state),
  );
  const first = session.reset({ kind: "catalog", label: "Library" }, { ...emptyQuery() });
  const second = session.reset({ kind: "mine", label: "My questions" }, { ...emptyQuery() });
  resolveCatalog({ items: [row("7K3-M9QP", "Stale")], aggregates: [], nextCursor: null });
  await Promise.all([first, second]);
  assert.equal(states.at(-1)?.kind, "ready");
  assert.equal(states.at(-1)?.rows[0]?.title, "Mine");
});

test("picker selection remains ordered while a source and query change", async () => {
  const selection = problemPickerSelection("many", 200, [
    row("7K3-M9QP", "Preserved first"),
    row("2R5-X7YA", "Preserved second"),
  ]);
  const session = new ProblemPickerSession(
    {
      search: async (request) => ({
        items: [row(request.source.kind === "catalog" ? "3S8-B4DZ" : "4T9-C5EW")],
        aggregates: [],
        nextCursor: null,
      }),
    },
    () => undefined,
  );
  await session.reset({ kind: "catalog", label: "Library" }, { ...emptyQuery(), search: "first" });
  await session.reset(
    { kind: "mine", label: "My questions" },
    { ...emptyQuery(), search: "second" },
  );
  assert.deepEqual(selection.questionIds, ["7K3-M9QP", "2R5-X7YA"]);
});

test("pagination failure retains loaded rows while external selection remains usable", async () => {
  const states = [];
  const selection = problemPickerSelection("many", 200, [row("7K3-M9QP")]);
  const session = new ProblemPickerSession(
    {
      search: async (request) => {
        if (request.cursor === null) {
          return { items: [row("2R5-X7YA")], aggregates: [], nextCursor: "next" };
        }
        throw new Error("temporary source failure");
      },
    },
    (state) => states.push(state),
  );
  await session.reset({ kind: "catalog", label: "Library" }, emptyQuery());
  await session.loadNext();
  assert.equal(states.at(-1)?.kind, "error");
  assert.equal(states.at(-1)?.rows[0]?.displayId, "2R5-X7YA");
  assert.deepEqual(selection.questionIds, ["7K3-M9QP"]);
});

function emptyQuery() {
  return {
    search: "",
    byline: null,
    backend: null,
    tag: null,
    responseFamily: null,
    taxonomy: null,
    capability: null,
    license: null,
    publicationScopes: [],
    evidence: null,
    usedInMyCourses: null,
  };
}
