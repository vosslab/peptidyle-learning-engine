import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { reusableCurriculumQuestionPickerRepository } from "../src/features/question_picker/question_picker_model.ts";

const { scope: _retiredPublicationScope, ...catalogProblem } =
  publishedProblemFixture.catalogProblem;

function catalog(questionId, title) {
  return {
    summary: { ...catalogProblem, questionId, metadata: { ...catalogProblem.metadata, title } },
    evidence: { state: "insufficientEvidence" },
  };
}

function definition() {
  return {
    entries: [
      { kind: "fixed", question: { catalog: catalog("7K3-M9QP", "First fixed") } },
      {
        kind: "pool",
        candidates: [
          { catalog: catalog("2R5-X7YA", "Pool first") },
          { catalog: catalog("3S8-B4DZ", "Pool second") },
        ],
      },
      { kind: "fixed", question: { catalog: catalog("4T9-C5EW", "Final fixed") } },
    ],
  };
}

const query = {
  search: "",
  byline: null,
  backend: null,
  tag: null,
  questionType: null,
  taxonomy: null,
  capability: null,
  license: null,
  evidence: null,
  usedInMyCourses: null,
  authorship: "any",
};

function course(revision = "2") {
  return {
    reference: "BP-7",
    revision,
    modules: [
      {
        module_id: "module-7",
        definitions: [{ assignment_id: "assignment-7", definition: definition() }],
      },
    ],
  };
}

test("reusable picker preserves fixed-entry and Question Pool candidate order", async () => {
  const source = reusableCurriculumQuestionPickerRepository({
    getBlueprintCourse: async () => ({ blueprintCourse: course() }),
  });
  const result = await source.search({
    source: {
      kind: "blueprintCourseAssignment",
      source: { reference: "BP-7", revision: "2", assignment_id: "assignment-7" },
      label: "Blueprint Course assignment",
    },
    query,
    cursor: null,
  });

  assert.deepEqual(
    result.items.map((row) => row.displayId),
    ["7K3-M9QP", "2R5-X7YA", "3S8-B4DZ", "4T9-C5EW"],
  );
});

test("reusable picker refuses a Blueprint Course revision that changed before access", async () => {
  const source = reusableCurriculumQuestionPickerRepository({
    getBlueprintCourse: async () => ({ blueprintCourse: course("3") }),
  });
  await assert.rejects(
    source.search({
      source: {
        kind: "blueprintCourseAssignment",
        source: { reference: "BP-7", revision: "2", assignment_id: "assignment-7" },
        label: "Stale Blueprint Course assignment",
      },
      query,
      cursor: null,
    }),
    /changed/u,
  );
});
