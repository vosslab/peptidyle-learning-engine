import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { blueprintCourseQuestionPickerRepository } from "../src/features/question_picker/question_picker_model.ts";

const { scope: _retiredPublicationScope, ...publishedQuestion } =
  publishedQuestionFixture.publishedQuestion;

function questionLibraryEntry(questionId, questionTitle, revisionNumber) {
  return {
    summary: {
      ...publishedQuestion,
      questionId,
      questionRevision: { questionId, revisionNumber },
      metadata: { ...publishedQuestion.metadata, questionTitle },
    },
    disciplineName: "Biology",
    disciplineIsRetired: false,
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
          reference: { questionId: "7K3M-79QP", revisionNumber: 3 },
          question_library: questionLibraryEntry("7K3M-79QP", "First fixed", 3),
          selection_availability: "available",
        },
      },
      {
        kind: "pool",
        question_pool_id: "2R5X-E7YA",
        question_pool_edit_number: 1,
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
          reference: { questionId: "4T9C-C5EW", revisionNumber: 5 },
          question_library: questionLibraryEntry("4T9C-C5EW", "Final fixed", 5),
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
    blueprintRevision: { blueprint_course_id: "BP7K3MX9AA", revision: revisionNumber },
    modules: [
      {
        blueprint_module_reference: "module-7",
        label: "Module 1",
        assessments: [
          { blueprint_assessment_id: "00000000-0000-0000-0000-000000000007", content: content() },
        ],
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
        blueprint_revision: { blueprint_course_id: "BP7K3MX9AA", revision: "2" },
        blueprint_assessment_id: "00000000-0000-0000-0000-000000000007",
      },
      label: "Blueprint Assessment",
    },
    query,
    cursor: null,
  });

  assert.deepEqual(
    result.items.map((row) => row.displayId),
    ["7K3M-79QP", "4T9C-C5EW"],
  );
  assert.deepEqual(
    result.items.map((row) => row.questionRevision),
    [
      { questionId: "7K3M-79QP", revisionNumber: 3 },
      { questionId: "4T9C-C5EW", revisionNumber: 5 },
    ],
  );
  assert.deepEqual(resolved, [{ reference: "BP7K3MX9AA", revisionNumber: "2" }]);
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
          blueprint_revision: { blueprint_course_id: "BP7K3MX9AA", revision: "2" },
          blueprint_assessment_id: "00000000-0000-0000-0000-000000000007",
        },
        label: "Stale Blueprint Assessment",
      },
      query,
      cursor: null,
    }),
    /did not resolve/u,
  );
});
