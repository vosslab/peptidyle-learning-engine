import assert from "node:assert/strict";
import test from "node:test";

import { EMPTY_QUESTION_LIBRARY_BROWSE_QUERY } from "../src/pages/library_page_model.ts";
import {
  questionPoolMemberReferences,
  questionPoolSourcePickerRepository,
} from "../src/components/question_pool_create_model.ts";

const startingQuestion = {
  questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
  questionTitle: "Starting Question",
  disciplineName: "Biology",
  subjectName: "Genetics",
};

function row(displayId, disciplineName, questionTitle = "Question") {
  return {
    displayId,
    questionRevision: { questionId: displayId, revisionNumber: 1 },
    questionTitle,
    summary: "Answer-free summary.",
    bloom: {
      cognitiveProcess: "Understand",
      knowledgeDimension: "Conceptual Knowledge",
      classificationEditNumber: "1",
    },
    disciplineName,
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authorNames: ["Published author"],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  };
}

function page(items, nextCursor) {
  return {
    items,
    aggregates: [],
    nextCursor,
    facetTruncation: { authorNames: false, tags: false, subjects: false, topics: false },
  };
}

test("source-bound Pool members keep the exact starting Revision first", async () => {
  const lookedUp = [];
  const members = await questionPoolMemberReferences(
    {
      questionIds: ["2R5X-E7YA"],
      questions: [
        {
          questionId: "2R5X-E7YA",
          row: row("2R5X-E7YA", "Biology", "Additional Question"),
        },
      ],
    },
    async (questionId) => {
      lookedUp.push(questionId);
      return {
        summary: { latestQuestionRevision: { questionId, revisionNumber: 5 } },
        disciplineName: "Biology",
        subjectName: "Genetics",
      };
    },
    startingQuestion,
  );

  assert.deepEqual(members, [
    { questionId: "7K3M-79QP", revisionNumber: 2 },
    { questionId: "2R5X-E7YA", revisionNumber: 5 },
  ]);
  assert.deepEqual(lookedUp, ["2R5X-E7YA"]);
});

test("source-bound Pool picker starts with the starting Subject and Discipline", async () => {
  const requests = [];
  const repository = questionPoolSourcePickerRepository(
    {
      search: async (query, cursor) => {
        requests.push({ query, cursor });
        if (cursor === null) {
          return page(
            [
              row("7K3M-79QP", "Biology", "Starting Question"),
              row("3S8B-24DZ", "Chemistry", "Wrong Discipline"),
            ],
            "next",
          );
        }
        return page([row("2R5X-E7YA", "Biology", "Matching Question")], null);
      },
    },
    startingQuestion,
  );

  const result = await repository.search({
    source: { kind: "library", label: "Question Library" },
    query: EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    cursor: null,
  });

  assert.deepEqual(
    requests.map((request) => ({ subjects: request.query.subjects, cursor: request.cursor })),
    [
      { subjects: ["Genetics"], cursor: null },
      { subjects: ["Genetics"], cursor: "next" },
    ],
  );
  assert.deepEqual(
    result.items.map((item) => item.displayId),
    ["2R5X-E7YA"],
  );
});
