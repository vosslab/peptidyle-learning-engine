import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { blueprintCourseQuestionPickerRepository } from "../src/features/question_picker/question_picker_model.ts";

const { scope: _retiredPublicationScope, ...publishedQuestion } =
  publishedQuestionFixture.publishedQuestion;

function questionLibraryEntry(questionId, questionTitle) {
  return {
    summary: {
      ...publishedQuestion,
      questionId,
      metadata: { ...publishedQuestion.metadata, questionTitle },
    },
    evidence: { state: "unavailable" },
  };
}

function content() {
  return {
    title: "Blueprint Assessment",
    instructions: "Select the fixed questions.",
    entries: [
      {
        kind: "fixed",
        question: {
          question_library: questionLibraryEntry("7K3M-X9QP", "First fixed"),
          selection_availability: "available",
        },
      },
      {
        kind: "pool",
        question_pool_revision: { questionPoolId: "2R5X-Z7YA", revisionNumber: 1 },
        selection_count: 1,
        points_per_item: "1",
        scoring_rule: "normal",
        selection_rule: { selectedQuestionOrder: "questionPoolOrder" },
        question_attempt_limit: { maxAttempts: null },
        question_attempt_time_limit: { kind: "unlimited" },
      },
      {
        kind: "fixed",
        question: {
          question_library: questionLibraryEntry("4T9C-Z5EW", "Final fixed"),
          selection_availability: "available",
        },
      },
    ],
    defaults: {},
  };
}

const query = {
  search: "",
  authorName: null,
  backend: null,
  tag: null,
  questionType: null,
  capability: null,
  questionLicense: null,
  usedInMyCourses: null,
  authorship: "any",
};

function revision(revisionNumber = "2") {
  return {
    blueprintRevision: { reference: "BP7K3MX9", revision: revisionNumber },
    modules: [
      {
        blueprint_module_reference: "module-7",
        label: "Module 1",
        assessments: [{ blueprint_assessment_reference: "assessment-7", content: content() }],
      },
    ],
  };
}

test("Blueprint Assessment picker presents fixed Questions in authored order", async () => {
  const resolved = [];
  const source = blueprintCourseQuestionPickerRepository({
    getBlueprintRevision: async (reference, revisionNumber) => {
      resolved.push({ reference, revisionNumber });
      return revision();
    },
  });
  const result = await source.search({
    source: {
      kind: "blueprintCourseAssessment",
      source: {
        blueprint_revision: { reference: "BP7K3MX9", revision: "2" },
        blueprint_assessment_reference: "assessment-7",
      },
      label: "Blueprint Assessment",
    },
    query,
    cursor: null,
  });

  assert.deepEqual(
    result.items.map((row) => row.displayId),
    ["7K3M-X9QP", "4T9C-Z5EW"],
  );
  assert.deepEqual(resolved, [{ reference: "BP7K3MX9", revisionNumber: "2" }]);
});

test("Blueprint Assessment picker refuses a Blueprint Course revision that changed before access", async () => {
  const source = blueprintCourseQuestionPickerRepository({
    getBlueprintRevision: async () => revision("3"),
  });
  await assert.rejects(
    source.search({
      source: {
        kind: "blueprintCourseAssessment",
        source: {
          blueprint_revision: { reference: "BP7K3MX9", revision: "2" },
          blueprint_assessment_reference: "assessment-7",
        },
        label: "Stale Blueprint Assessment",
      },
      query,
      cursor: null,
    }),
    /did not resolve/u,
  );
});
