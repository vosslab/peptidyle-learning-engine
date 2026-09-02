import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_QUESTION_PICKER_SELECTION_CAP,
  QuestionPickerSession,
  moveQuestionPickerSelection,
  questionPickerSelection,
  toggleQuestionPickerSelection,
} from "../src/features/question_picker/question_picker_model.ts";

function row(displayId, title = "Question") {
  return {
    displayId,
    title,
    summary: "Answer-free summary.",
    authorNames: ["Published author"],
    classifications: ["biology:protein"],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "insufficientEvidence" },
  };
}

function questionIdFor(index) {
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

test("Question Picker preserves public Question ID order and safe row metadata", () => {
  const selection = questionPickerSelection("many", 200, [
    row("7K3-M9QP", "First"),
    row("2R5-X7YA", "Second"),
  ]);
  assert.deepEqual(selection.questionIds, ["7K3-M9QP", "2R5-X7YA"]);
  assert.equal(selection.questions[1]?.row.title, "Second");
});

test("single-selection mode replaces the prior result", () => {
  const initial = questionPickerSelection("one", 1, [row("7K3-M9QP")]);
  const next = toggleQuestionPickerSelection("one", 1, initial, row("2R5-X7YA"), true);
  assert.deepEqual(next.questionIds, ["2R5-X7YA"]);
});

test("picker removes a selected row and preserves the other selected order", () => {
  const initial = questionPickerSelection("many", 200, [row("7K3-M9QP"), row("2R5-X7YA")]);
  const next = toggleQuestionPickerSelection("many", 200, initial, row("7K3-M9QP"), false);
  assert.deepEqual(next.questionIds, ["2R5-X7YA"]);
});

test("picker reorders the selected tray without changing membership", () => {
  const initial = questionPickerSelection("many", 200, [row("7K3-M9QP"), row("2R5-X7YA")]);
  const next = moveQuestionPickerSelection("many", 200, initial, 1, -1);
  assert.deepEqual(next.questionIds, ["2R5-X7YA", "7K3-M9QP"]);
});

test("picker enforces the shared bounded selection limit", () => {
  const rows = Array.from({ length: MAX_QUESTION_PICKER_SELECTION_CAP + 1 }, (_value, index) => {
    return row(questionIdFor(index));
  });
  assert.throws(
    () => questionPickerSelection("many", MAX_QUESTION_PICKER_SELECTION_CAP, rows),
    /at most/,
  );
});

test("picker session drops a stale source response before publishing it", async () => {
  let resolveQuestionLibrary;
  const questionLibrary = new Promise((resolve) => {
    resolveQuestionLibrary = resolve;
  });
  const states = [];
  const session = new QuestionPickerSession(
    {
      search: async (request) => {
        if (request.source.kind === "library") return await questionLibrary;
        return { items: [row("2R5-X7YA", "Mine")], aggregates: [], nextCursor: null };
      },
    },
    (state) => states.push(state),
  );
  const first = session.reset({ kind: "library", label: "Library" }, { ...emptyQuery() });
  const second = session.reset({ kind: "mine", label: "My questions" }, { ...emptyQuery() });
  resolveQuestionLibrary({ items: [row("7K3-M9QP", "Stale")], aggregates: [], nextCursor: null });
  await Promise.all([first, second]);
  assert.equal(states.at(-1)?.kind, "ready");
  assert.equal(states.at(-1)?.rows[0]?.title, "Mine");
});

test("picker selection remains ordered while a source and query change", async () => {
  const selection = questionPickerSelection("many", 200, [
    row("7K3-M9QP", "Preserved first"),
    row("2R5-X7YA", "Preserved second"),
  ]);
  const session = new QuestionPickerSession(
    {
      search: async (request) => ({
        items: [row(request.source.kind === "library" ? "3S8-B4DZ" : "4T9-C5EW")],
        aggregates: [],
        nextCursor: null,
      }),
    },
    () => undefined,
  );
  await session.reset({ kind: "library", label: "Library" }, { ...emptyQuery(), search: "first" });
  await session.reset(
    { kind: "mine", label: "My questions" },
    { ...emptyQuery(), search: "second" },
  );
  assert.deepEqual(selection.questionIds, ["7K3-M9QP", "2R5-X7YA"]);
});

test("pagination failure retains loaded rows while external selection remains usable", async () => {
  const states = [];
  const selection = questionPickerSelection("many", 200, [row("7K3-M9QP")]);
  const session = new QuestionPickerSession(
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
  await session.reset({ kind: "library", label: "Library" }, emptyQuery());
  await session.loadNext();
  assert.equal(states.at(-1)?.kind, "error");
  assert.equal(states.at(-1)?.rows[0]?.displayId, "2R5-X7YA");
  assert.deepEqual(selection.questionIds, ["7K3-M9QP"]);
});

function emptyQuery() {
  return {
    search: "",
    authorName: null,
    backend: null,
    tag: null,
    questionType: null,
    classification: null,
    capability: null,
    questionLicense: null,
    evidence: null,
    usedInMyCourses: null,
  };
}
