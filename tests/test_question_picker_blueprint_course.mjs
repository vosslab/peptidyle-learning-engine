import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { blueprintCourseQuestionPickerRepository } from "../src/features/question_picker/question_picker_model.ts";

const { scope: _retiredPublicationScope, ...publishedQuestion } =
  publishedQuestionFixture.publishedQuestion;

function questionLibraryEntry(questionId, title) {
  return {
    summary: {
      ...publishedQuestion,
      questionId,
      metadata: { ...publishedQuestion.metadata, title },
    },
    evidence: { state: "insufficientEvidence" },
  };
}

function content() {
  return {
    entries: [
      {
        kind: "fixed",
        question: { question_library: questionLibraryEntry("7K3-M9QP", "First fixed") },
      },
      {
        kind: "pool",
        items: [
          { question_library: questionLibraryEntry("2R5-X7YA", "Pool first") },
          { question_library: questionLibraryEntry("3S8-B4DZ", "Pool second") },
        ],
      },
      {
        kind: "fixed",
        question: { question_library: questionLibraryEntry("4T9-C5EW", "Final fixed") },
      },
    ],
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
        assignments: [{ assignment_id: "assignment-7", content: content() }],
      },
    ],
  };
}

test("reusable picker preserves Fixed Question and Question Pool Assignment Entry order", async () => {
  const source = blueprintCourseQuestionPickerRepository({
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
  const source = blueprintCourseQuestionPickerRepository({
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
