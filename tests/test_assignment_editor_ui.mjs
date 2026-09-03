import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { createMasteryAssignmentEditorState } from "./support/assignment_editor_test_support.ts";
import {
  assignmentContentInput,
  assignmentQuestionLabel,
  moveAssignmentEntry,
  parseExactQuestionIds,
  validateAssignmentEditorState,
  validateQuestionPoolAssignmentEntry,
} from "../src/pages/assignment_editor_model.ts";
import { assignmentPickerMaximum } from "../src/pages/assignment_editor_picker_model.ts";

test("assignment Questions payload uses Question IDs as its only question identity", () => {
  const draft = createMasteryAssignmentEditorState("course-1");
  const item = publishedQuestionFixture.assignment.entries[0];
  assert.ok(item);
  const configured = { ...draft, title: "Practice", entries: [{ ...item, kind: "fixedQuestion" }] };
  assert.deepEqual(assignmentContentInput(configured).entries[0]?.questionId, item.questionId);
  assert.equal(JSON.stringify(assignmentContentInput(configured)).includes("problem"), false);
  assert.equal(JSON.stringify(assignmentContentInput(configured)).includes("version"), false);
});

test("Question ID paste supports instructor punctuation and rejects duplicate choices", () => {
  assert.deepEqual(parseExactQuestionIds("7k3-m9qp"), ["7K3-M9QP"]);
  assert.throws(() => parseExactQuestionIds("7K3-M9QP, 7K3-M9QP"), /once/u);
});

test("assignment Question labels use the exact public Question ID", () => {
  const entry = publishedQuestionFixture.assignment.entries[0];
  assert.ok(entry);
  assert.equal(assignmentQuestionLabel(entry), entry.questionId);
});

test("ordinary editing preserves a fixed question while changing shared entry order", () => {
  const first = publishedQuestionFixture.assignment.entries[0];
  assert.ok(first);
  const second = { ...first, id: "item-2", questionId: "7K4-M9QP" };
  const moved = moveAssignmentEntry(
    {
      ...createMasteryAssignmentEditorState("course-1"),
      entries: [
        { ...first, kind: "fixedQuestion" },
        { ...second, kind: "fixedQuestion" },
      ],
    },
    1,
    -1,
  );
  assert.deepEqual(
    moved.entries.map((entry) => entry.kind),
    ["fixedQuestion", "fixedQuestion"],
  );
  assert.deepEqual(
    assignmentContentInput(moved).entries.map((entry) => entry.questionId),
    [second.questionId, first.questionId],
  );
});

test("Question Pool editor encodes public Item Question IDs in Item order", () => {
  const draft = {
    ...createMasteryAssignmentEditorState("course-1"),
    entries: [
      { ...publishedQuestionFixture.assignment.entries[0], kind: "fixedQuestion" },
      {
        kind: "questionPool",
        items: [
          { questionId: "7K4-M9QP", title: "Item one", backend: "ple" },
          { questionId: "7K5-M9QP", title: "Item two", backend: "ple" },
        ],
        availability: "available",
        scoringRule: "normal",
        selectionCount: 1,
        pointsPerItem: "2",
        selectionRule: { selectedQuestionOrder: "randomOrder" },
        questionAttemptLimit: { maxAttempts: null },
        questionAttemptTimeLimit: { kind: "unlimited" },
      },
    ],
  };
  const body = assignmentContentInput(draft);
  assert.deepEqual(
    body.entries.map((entry) => entry.kind),
    ["fixedQuestion", "questionPool"],
  );
  assert.deepEqual(body.entries[1], {
    kind: "questionPool",
    questionIds: ["7K4-M9QP", "7K5-M9QP"],
    availability: "available",
    scoringRule: "normal",
    selectionCount: 1,
    pointsPerItem: "2",
    selectionRule: { selectedQuestionOrder: "randomOrder" },
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
  });
  assert.equal(JSON.stringify(body).includes('"selectedQuestionOrder":"randomOrder"'), true);
  assert.equal(JSON.stringify(body).includes("algorithm"), false);
});

test("pool validation keeps an actionable correction path", () => {
  const invalid = {
    kind: "questionPool",
    items: [{ questionId: "7K4-M9QP", title: "Item", backend: "ple" }],
    availability: "available",
    selectionCount: 2,
    pointsPerItem: "1",
    selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
  };
  assert.equal(
    validateQuestionPoolAssignmentEntry(invalid),
    "Selection count cannot exceed the number of Question IDs in this Question Pool.",
  );
});

test("pool authoring reports shared cardinality recovery paths before save", () => {
  const item = { questionId: "7K4-M9QP", title: "Item", backend: "ple" };
  const overfullPool = {
    kind: "questionPool",
    items: Array.from({ length: 1025 }, (_value, index) => ({
      ...item,
      questionId: `${index.toString(16).padStart(3, "0").toUpperCase()}-0000`,
    })),
    availability: "available",
    selectionCount: 1,
    pointsPerItem: "1",
    selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
  };
  assert.equal(
    validateQuestionPoolAssignmentEntry(overfullPool),
    "Keep this Question Pool to 1024 Question IDs or fewer.",
  );

  const overfullAssignmentContent = {
    ...createMasteryAssignmentEditorState("course-1"),
    entries: Array.from({ length: 1025 }, (_value, entryIndex) => ({
      ...publishedQuestionFixture.assignment.entries[0],
      kind: "fixedQuestion",
      id: `item-${entryIndex}`,
    })),
  };
  assert.equal(
    validateAssignmentEditorState(overfullAssignmentContent),
    "Keep this assignment to 1024 ordered entries or fewer.",
  );

  const pool = (entryIndex, itemCount) => ({
    kind: "questionPool",
    items: Array.from({ length: itemCount }, (_value, index) => ({
      ...item,
      questionId: `${entryIndex.toString(16).padStart(2, "0").toUpperCase()}${index
        .toString(16)
        .padStart(1, "0")}-0000`,
    })),
    availability: "available",
    selectionCount: 1,
    pointsPerItem: "1",
    selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
  });
  const tooManyQuestionPoolItems = {
    ...createMasteryAssignmentEditorState("course-1"),
    entries: [
      ...Array.from({ length: 8 }, (_value, entryIndex) => pool(entryIndex, 1024)),
      pool(8, 1),
    ],
  };
  assert.equal(
    validateAssignmentEditorState(tooManyQuestionPoolItems),
    "Keep all Question Pools to 8192 Question IDs or fewer.",
  );
});

test("shared picker caps each assignment destination before the dialog opens", () => {
  const fixed = publishedQuestionFixture.assignment.entries[0];
  assert.ok(fixed);
  const draft = {
    ...createMasteryAssignmentEditorState("course-1"),
    entries: [
      { ...fixed, kind: "fixedQuestion" },
      {
        kind: "questionPool",
        items: [{ questionId: "7K4-M9QP", title: "Item", backend: "ple" }],
        availability: "available",
        selectionCount: 1,
        pointsPerItem: "1",
        selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
      },
    ],
  };
  assert.equal(assignmentPickerMaximum(draft, { kind: "fixedQuestion" }), 1022);
  assert.equal(assignmentPickerMaximum(draft, { kind: "pool", entryIndex: 1 }), 1023);
});
