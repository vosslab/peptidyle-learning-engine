import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { createMasteryAssignmentDraft } from "./support/assignment_editor_test_support.ts";
import {
  assignmentContentInput,
  moveAssignmentEntry,
  parseExactProblemDisplayReferences,
  validateAssignmentEditorDraft,
  validateSelectionGroupEntry,
} from "../src/pages/assignment_editor_model.ts";
import { assignmentPickerMaximum } from "../src/pages/assignment_editor_picker_model.ts";

test("assignment Questions payload uses Question IDs as its only question identity", () => {
  const draft = createMasteryAssignmentDraft("course-1");
  const item = publishedProblemFixture.assignment.items[0];
  assert.ok(item);
  const configured = { ...draft, title: "Practice", entries: [{ ...item, kind: "fixed" }] };
  assert.deepEqual(assignmentContentInput(configured).entries[0]?.questionId, item.questionId);
  assert.equal(JSON.stringify(assignmentContentInput(configured)).includes("problem"), false);
  assert.equal(JSON.stringify(assignmentContentInput(configured)).includes("version"), false);
});

test("Question ID paste supports instructor punctuation and rejects duplicate choices", () => {
  assert.deepEqual(parseExactProblemDisplayReferences("7k3-m9qp"), ["7K3-M9QP"]);
  assert.throws(() => parseExactProblemDisplayReferences("7K3-M9QP, 7K3-M9QP"), /once/u);
});

test("ordinary editing preserves a fixed question while changing shared entry order", () => {
  const first = publishedProblemFixture.assignment.items[0];
  assert.ok(first);
  const second = { ...first, id: "item-2", questionId: "7K4-M9QP", position: 1 };
  const moved = moveAssignmentEntry(
    {
      ...createMasteryAssignmentDraft("course-1"),
      entries: [
        { ...first, kind: "fixed" },
        { ...second, kind: "fixed" },
      ],
    },
    1,
    -1,
  );
  assert.deepEqual(
    moved.entries.map((entry) => [entry.kind, entry.position]),
    [
      ["fixed", 0],
      ["fixed", 1],
    ],
  );
  assert.deepEqual(
    assignmentContentInput(moved).entries.map((entry) => entry.questionId),
    [second.questionId, first.questionId],
  );
});

test("pool editor encodes public candidate Question IDs in the shared position namespace", () => {
  const draft = {
    ...createMasteryAssignmentDraft("course-1"),
    entries: [
      { ...publishedProblemFixture.assignment.items[0], kind: "fixed" },
      {
        kind: "selectionGroup",
        position: 1,
        candidates: [
          { questionId: "7K4-M9QP", title: "Candidate one", backend: "native" },
          { questionId: "7K5-M9QP", title: "Candidate two", backend: "native" },
        ],
        drawCount: 1,
        pointsPerItem: "2",
        ordering: "randomized",
        algorithmVersion: 1,
      },
    ],
  };
  const body = assignmentContentInput(draft);
  assert.deepEqual(
    body.entries.map((entry) => entry.kind),
    ["fixed", "selectionGroup"],
  );
  assert.deepEqual(body.entries[1], {
    kind: "selectionGroup",
    candidateQuestionIds: ["7K4-M9QP", "7K5-M9QP"],
    position: 1,
    drawCount: 1,
    pointsPerItem: "2",
    ordering: "randomized",
  });
  assert.equal(JSON.stringify(body).includes("algorithm"), false);
  assert.equal(JSON.stringify(body).includes("version"), false);
});

test("pool validation keeps an actionable correction path", () => {
  const invalid = {
    kind: "selectionGroup",
    position: 0,
    candidates: [{ questionId: "7K4-M9QP", title: "Candidate", backend: "native" }],
    drawCount: 2,
    pointsPerItem: "1",
    ordering: "candidateOrder",
    algorithmVersion: 1,
  };
  assert.equal(
    validateSelectionGroupEntry(invalid),
    "Draw count cannot exceed the number of candidate Question IDs.",
  );
});

test("pool authoring reports shared cardinality recovery paths before save", () => {
  const candidate = { questionId: "7K4-M9QP", title: "Candidate", backend: "native" };
  const overfullPool = {
    kind: "selectionGroup",
    position: 0,
    candidates: Array.from({ length: 1025 }, (_value, index) => ({
      ...candidate,
      questionId: `${index.toString(16).padStart(3, "0").toUpperCase()}-0000`,
    })),
    drawCount: 1,
    pointsPerItem: "1",
    ordering: "candidateOrder",
    algorithmVersion: 1,
  };
  assert.equal(
    validateSelectionGroupEntry(overfullPool),
    "Keep this pool to 1024 candidate Question IDs or fewer.",
  );

  const overfullDefinition = {
    ...createMasteryAssignmentDraft("course-1"),
    entries: Array.from({ length: 1025 }, (_value, position) => ({
      ...publishedProblemFixture.assignment.items[0],
      kind: "fixed",
      id: `item-${position}`,
      position,
    })),
  };
  assert.equal(
    validateAssignmentEditorDraft(overfullDefinition),
    "Keep this assignment to 1024 ordered entries or fewer.",
  );

  const pool = (position, candidateCount) => ({
    kind: "selectionGroup",
    position,
    candidates: Array.from({ length: candidateCount }, (_value, index) => ({
      ...candidate,
      questionId: `${position.toString(16).padStart(2, "0").toUpperCase()}${index
        .toString(16)
        .padStart(1, "0")}-0000`,
    })),
    drawCount: 1,
    pointsPerItem: "1",
    ordering: "candidateOrder",
    algorithmVersion: 1,
  });
  const tooManyCandidates = {
    ...createMasteryAssignmentDraft("course-1"),
    entries: [...Array.from({ length: 8 }, (_value, position) => pool(position, 1024)), pool(8, 1)],
  };
  assert.equal(
    validateAssignmentEditorDraft(tooManyCandidates),
    "Keep all pools to 8192 candidate Question IDs or fewer.",
  );
});

test("shared picker caps each assignment destination before the dialog opens", () => {
  const fixed = publishedProblemFixture.assignment.items[0];
  assert.ok(fixed);
  const draft = {
    ...createMasteryAssignmentDraft("course-1"),
    entries: [
      { ...fixed, kind: "fixed" },
      {
        kind: "selectionGroup",
        position: 1,
        candidates: [{ questionId: "7K4-M9QP", title: "Candidate", backend: "native" }],
        drawCount: 1,
        pointsPerItem: "1",
        ordering: "candidateOrder",
        algorithmVersion: 1,
      },
    ],
  };
  assert.equal(assignmentPickerMaximum(draft, { kind: "fixed" }), 1022);
  assert.equal(assignmentPickerMaximum(draft, { kind: "pool", entryIndex: 1 }), 1023);
  assert.equal(assignmentPickerMaximum(draft, { kind: "replacement", itemId: fixed.id }), 1);
});
