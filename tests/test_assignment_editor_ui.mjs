// Assignment editor model and narrow repository behavior.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import {
  addCatalogReference,
  assignmentInput,
  createMasteryAssignmentDraft,
  moveCatalogReference,
  removeCatalogReference,
} from "../src/pages/assignment_editor_model.ts";
import { createAssignmentEditorRepository } from "../src/pages/assignment_editor_repository.ts";

const reference = {
  problem: publishedProblemFixture.catalogProblem.problem,
  version: publishedProblemFixture.catalogProblem.version,
};

function draft() {
  return {
    id: publishedProblemFixture.assignment.id,
    courseId: publishedProblemFixture.course.id,
    title: "Editable peptide practice",
    problems: [],
    policies: publishedProblemFixture.assignment.policies,
    revision: '"7"',
  };
}

test("assignment editor retains ordered immutable tuples and emits only editable input", () => {
  const row = {
    reference,
    title: "Peptide bond resonance and planarity",
  };
  const once = addCatalogReference(draft(), row);
  const twice = addCatalogReference(once, row);
  assert.equal(twice.problems.length, 1, "selecting the same immutable version is idempotent");

  const other = {
    problem: "0198e000-0000-7000-8000-000000000099",
    version: "0198e000-0000-7000-8000-000000000098",
  };
  const moved = moveCatalogReference({ ...twice, problems: [reference, other] }, other, -1);
  assert.deepEqual(moved.problems, [other, reference]);
  assert.deepEqual(removeCatalogReference(moved, other).problems, [reference]);

  assert.deepEqual(assignmentInput(twice), {
    title: "Editable peptide practice",
    problems: [reference],
    policies: publishedProblemFixture.assignment.policies,
  });
  assert.equal("workspace" in assignmentInput(twice), false);
  assert.equal("source" in assignmentInput(twice), false);
  assert.equal("capabilities" in assignmentInput(twice), false);
});

test("new assignments start with the Fall-pilot Mastery policy and no private state", () => {
  const created = createMasteryAssignmentDraft(publishedProblemFixture.course.id);
  assert.deepEqual(created.policies, {
    completion: { kind: "allCorrect" },
    grade: "highest",
    continuedPractice: { kind: "unlimited" },
    variation: "newSeeds",
  });
  assert.deepEqual(assignmentInput(created), {
    title: "",
    problems: [],
    policies: created.policies,
  });
  assert.equal("source" in created, false);
  assert.equal("answerKey" in created, false);
});

test("assignment editor repository creates a bounded catalog query and passes the exact CAS input", async () => {
  const calls = [];
  const client = {
    getAssignmentEditor: async (id) => ({ ...draft(), id }),
    createAssignment: async (course, input) => {
      calls.push({ course, input, create: true });
      return { ...draft(), id: "created-assignment", courseId: course, ...input, revision: '"1"' };
    },
    saveAssignment: async (course, assignment, input, revision) => {
      calls.push({ course, assignment, input, revision });
      return { ...draft(), id: assignment, courseId: course, ...input, revision: '"8"' };
    },
    searchCatalog: async (query) => {
      calls.push({ query });
      return { items: [publishedProblemFixture.catalogProblem] };
    },
  };
  const repository = createAssignmentEditorRepository(client);
  const rows = await repository.searchPublished(" peptide bond ");
  assert.deepEqual(rows[0].reference, reference);
  assert.deepEqual(calls[0].query, {
    text: "peptide bond",
    taxonomy: [],
    capabilities: [],
    licenses: [],
    statistics: "any",
    cursor: null,
    pageSize: 20,
  });

  const input = assignmentInput({ ...draft(), problems: [reference] });
  await repository.create(publishedProblemFixture.course.id, input);
  assert.deepEqual(calls[1], {
    course: publishedProblemFixture.course.id,
    input,
    create: true,
  });
  await repository.save(
    publishedProblemFixture.course.id,
    publishedProblemFixture.assignment.id,
    input,
    '"7"',
  );
  assert.deepEqual(calls[2], {
    course: publishedProblemFixture.course.id,
    assignment: publishedProblemFixture.assignment.id,
    input,
    revision: '"7"',
  });
});
