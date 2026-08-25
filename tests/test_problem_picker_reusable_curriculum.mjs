import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { reusableCurriculumProblemPickerRepository } from "../src/features/problem_picker/problem_picker_model.ts";

function catalog(questionId, title) {
  return {
    summary: {
      ...publishedProblemFixture.catalogProblem,
      questionId,
      metadata: { ...publishedProblemFixture.catalogProblem.metadata, title },
    },
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

test("reusable picker sources preserve interleaved entry and candidate order", async () => {
  const source = reusableCurriculumProblemPickerRepository({
    getBlueprint: async () => ({ blueprint: { definition: definition() } }),
    getAlphaCourse: async () => ({ alpha: { modules: [{ definitions: [definition()] }] } }),
  });
  const query = {
    search: "",
    byline: null,
    backend: null,
    tag: null,
    responseFamily: null,
    taxonomy: null,
    capability: null,
    license: null,
    evidence: null,
    usedInMyCourses: null,
    authorship: "any",
    publicationScopes: [],
  };
  const personal = await source.search({
    source: { kind: "personalBlueprint", blueprint: "BP-7", label: "My blueprint" },
    query,
    cursor: null,
  });
  assert.deepEqual(
    personal.items.map((row) => row.displayId),
    ["7K3-M9QP", "2R5-X7YA", "3S8-B4DZ", "4T9-C5EW"],
  );
  const alpha = await source.search({
    source: {
      kind: "alphaCurriculum",
      alpha: "AC-3",
      modulePosition: 1,
      assignmentPosition: 1,
      label: "Shared curriculum",
    },
    query,
    cursor: null,
  });
  assert.deepEqual(
    alpha.items.map((row) => row.displayId),
    ["7K3-M9QP", "2R5-X7YA", "3S8-B4DZ", "4T9-C5EW"],
  );
});

test("reusable picker rejects malformed human-facing Alpha positions before access", async () => {
  let alphaReads = 0;
  const source = reusableCurriculumProblemPickerRepository({
    getBlueprint: async () => ({ blueprint: { definition: definition() } }),
    getAlphaCourse: async () => {
      alphaReads += 1;
      return { alpha: { modules: [{ definitions: [definition()] }] } };
    },
  });
  const query = {
    search: "",
    byline: null,
    backend: null,
    tag: null,
    responseFamily: null,
    taxonomy: null,
    capability: null,
    license: null,
    evidence: null,
    usedInMyCourses: null,
    authorship: "any",
    publicationScopes: [],
  };
  await assert.rejects(
    source.search({
      source: {
        kind: "alphaCurriculum",
        alpha: "AC-3",
        modulePosition: 0,
        assignmentPosition: 1,
        label: "Broken",
      },
      query,
      cursor: null,
    }),
  );
  assert.equal(alphaReads, 0);
});
