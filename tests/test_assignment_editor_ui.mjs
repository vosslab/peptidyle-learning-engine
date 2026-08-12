// Assignment editor model and narrow repository behavior.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import {
  addCatalogReference,
  assignmentProblemLabel,
  assignmentInput,
  createMasteryAssignmentDraft,
  minutesToRunTimeLimit,
  moveCatalogReference,
  parseExactProblemDisplayReferences,
  questionBackendLabel,
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
    assignmentTiming: { timeLimitSeconds: null },
    revision: '"7"',
  };
}

test("assignment editor retains ordered immutable tuples and emits only editable input", () => {
  const row = {
    reference,
    publicId: publishedProblemFixture.catalogProblem.publicId,
    versionNumber: publishedProblemFixture.catalogProblem.versionNumber,
    title: "Peptide bond resonance and planarity",
    backend: publishedProblemFixture.catalogProblem.backend,
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
    assignmentTiming: { timeLimitSeconds: null },
  });
  assert.equal("workspace" in assignmentInput(twice), false);
  assert.equal("source" in assignmentInput(twice), false);
  assert.equal("capabilities" in assignmentInput(twice), false);
  assert.equal(assignmentProblemLabel(row), "P-1-v1");
  assert.equal(questionBackendLabel(row.backend), "PLE native");
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
    assignmentTiming: { timeLimitSeconds: 900 },
  });
  assert.equal("source" in created, false);
  assert.equal("answerKey" in created, false);
});

test("run timing minutes parse exactly and reject values outside the storage domain", () => {
  assert.deepEqual(minutesToRunTimeLimit("1.5", true), { seconds: 90, error: null });
  assert.equal(
    minutesToRunTimeLimit("0.016", true).error,
    "Enter minutes that convert to a whole number of seconds.",
  );
  assert.deepEqual(minutesToRunTimeLimit("35791394", true), {
    seconds: 2_147_483_640,
    error: null,
  });
  assert.equal(
    minutesToRunTimeLimit("35791395", true).error,
    "Enter a duration no longer than 2147483647 seconds.",
  );
});

test("direct assignment import accepts only bounded exact human-readable versions", () => {
  assert.deepEqual(parseExactProblemDisplayReferences(" P-12-v3,\nP-41-v1 "), [
    "P-12-v3",
    "P-41-v1",
  ]);
  assert.deepEqual(parseExactProblemDisplayReferences("P-2147483647-v2147483647"), [
    "P-2147483647-v2147483647",
  ]);
  for (const value of ["", "P-12", "12-v3", "P-0-v1", "P-12-v0", "P-12-v3, P-12-v3"]) {
    assert.throws(() => parseExactProblemDisplayReferences(value));
  }
  for (const value of ["P-2147483648-v1", "P-1-v2147483648"]) {
    assert.throws(
      () => parseExactProblemDisplayReferences(value),
      new Error(`${value} is not an exact question ID. Use the form P-12-v3.`),
    );
  }
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
    resolveCatalogProblem: async (displayReference) => {
      calls.push({ displayReference });
      return publishedProblemFixture.catalogProblem;
    },
    getCatalogProblemDetail: async (problem, version) => ({
      summary: { ...publishedProblemFixture.catalogProblem, problem, version },
      prompt: [],
      statistics: "unavailable",
    }),
  };
  const repository = createAssignmentEditorRepository(client);
  const rows = await repository.searchPublished(" peptide bond ");
  assert.deepEqual(rows[0].reference, reference);
  assert.equal(rows[0].publicId, publishedProblemFixture.catalogProblem.publicId);
  assert.equal(rows[0].versionNumber, publishedProblemFixture.catalogProblem.versionNumber);
  assert.equal(rows[0].backend, publishedProblemFixture.catalogProblem.backend);
  assert.deepEqual(calls[0].query, {
    text: "peptide bond",
    taxonomy: [],
    capabilities: [],
    licenses: [],
    statistics: "any",
    cursor: null,
    pageSize: 20,
  });

  const resolved = await repository.resolvePublished("P-1-v1");
  assert.deepEqual(resolved, rows[0]);
  assert.deepEqual(calls[1], { displayReference: "P-1-v1" });

  const described = await repository.describePublished([reference]);
  assert.deepEqual(described, rows);

  const input = assignmentInput({ ...draft(), problems: [reference] });
  await repository.create(publishedProblemFixture.course.id, input);
  assert.deepEqual(calls[2], {
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
  assert.deepEqual(calls[3], {
    course: publishedProblemFixture.course.id,
    assignment: publishedProblemFixture.assignment.id,
    input,
    revision: '"7"',
  });
});
