import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import {
  assignmentCreateInput,
  assignmentInput,
  createMasteryAssignmentDraft,
  moveAssignmentItem,
  parseExactProblemDisplayReferences,
} from "../src/pages/assignment_editor_model.ts";

test("assignment editor uses Question IDs as its only question identity", () => {
  const draft = createMasteryAssignmentDraft("course-1");
  const item = publishedProblemFixture.assignment.items[0];
  assert.ok(item);
  const configured = { ...draft, title: "Practice", items: [item] };
  assert.deepEqual(assignmentCreateInput(configured).questionIds, [item.questionId]);
  assert.deepEqual(assignmentInput(configured).items[0]?.questionId, item.questionId);
  assert.equal(JSON.stringify(assignmentInput(configured)).includes("problem"), false);
  assert.equal(JSON.stringify(assignmentInput(configured)).includes("version"), false);
});

test("Question ID paste supports instructor punctuation and rejects duplicate choices", () => {
  assert.deepEqual(parseExactProblemDisplayReferences("7k3-m9qp"), ["7K3-M9QP"]);
  assert.throws(() => parseExactProblemDisplayReferences("7K3-M9QP, 7K3-M9QP"), /once/u);
});

test("ordinary editing preserves assigned item identity while changing only order", () => {
  const first = publishedProblemFixture.assignment.items[0];
  assert.ok(first);
  const second = { ...first, id: "item-2", questionId: "7K4-M9QP", position: 1 };
  const moved = moveAssignmentItem(
    { ...createMasteryAssignmentDraft("course-1"), items: [first, second] },
    second.id,
    -1,
  );
  assert.deepEqual(
    moved.items.map((item) => [item.id, item.questionId, item.position]),
    [
      [second.id, second.questionId, 0],
      [first.id, first.questionId, 1],
    ],
  );
  assert.deepEqual(
    assignmentInput(moved).items.map((item) => item.id),
    [second.id, first.id],
  );
});
