import assert from "node:assert/strict";
import test from "node:test";

import {
  questionPoolMemberTuples,
  questionPoolSourcePickerRepository,
  questionPoolStartingEligibility,
} from "../src/components/question_pool_create_model.ts";
import { questionPickerSearchDefinition } from "../src/features/question_picker/question_picker_model.ts";

const startingQuestion = {
  publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
  questionTitle: "Starting Question",
  disciplineName: "Biology",
  subjectName: "Genetics",
  questionType: "multipleChoice",
  backend: "ple",
};
const eligibilityMetadata = {
  questionId: "7K3M-79QP",
  metadataEditNumber: 1,
  tags: [],
  disciplineUuid: "00000000-0000-0000-0000-000000000001",
  subjectUuid: "00000000-0000-0000-0000-000000000002",
  topicUuid: null,
  subtopicUuid: null,
};

function row(displayId, questionTitle = "Question", license = "CC-BY-4.0") {
  return {
    kind: "question",
    displayId,
    publishedQuestionRevisionTuple: { publishedQuestionId: displayId, revisionNumber: 1 },
    questionTitle,
    summary: "Answer-free summary.",
    bloom: null,
    disciplineName: "Biology",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authors: [{ displayName: "Published Author", accountId: null }],
    capabilities: [],
    questionLicense: license,
    evidence: { state: "unavailable" },
  };
}

function page(items) {
  return {
    items,
    aggregates: [],
    nextCursor: null,
    facetTruncation: { authorNames: false, tags: false, subjects: false, topics: false },
  };
}

async function currentMetadata(questionIds) {
  return questionIds.map((questionId) => ({ ...eligibilityMetadata, questionId }));
}

function details(questionId, overrides = {}) {
  return {
    summary: {
      publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber: 5 },
      questionType: "multipleChoice",
      backend: "ple",
      metadata: { questionLicense: "CC-BY-4.0" },
      ...overrides,
    },
  };
}

test("source-bound Pool members keep the exact starting Revision first and recheck UUID Type and Backend", async () => {
  const eligibility = await questionPoolStartingEligibility(startingQuestion, currentMetadata);
  const members = await questionPoolMemberTuples(
    {
      questionIds: ["2R5X-E7YA"],
      questions: [{ questionId: "2R5X-E7YA", row: row("2R5X-E7YA", "Eligible Question") }],
    },
    async (questionId) => details(questionId),
    currentMetadata,
    eligibility,
    startingQuestion,
  );
  assert.deepEqual(members, [
    { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
    { publishedQuestionId: "2R5X-E7YA", revisionNumber: 5 },
  ]);
  await assert.rejects(
    questionPoolMemberTuples(
      {
        questionIds: ["2R5X-E7YA"],
        questions: [{ questionId: "2R5X-E7YA", row: row("2R5X-E7YA") }],
      },
      async (questionId) => details(questionId, { backend: "webwork" }),
      currentMetadata,
      eligibility,
      startingQuestion,
    ),
    /share the Pool's Discipline, Subject, Type, and Backend/u,
  );
});

test("source-bound picker sends UUID eligibility and leaves an eligible Question already in another Pool available", async () => {
  const requests = [];
  const eligibility = await questionPoolStartingEligibility(startingQuestion, currentMetadata);
  const source = questionPoolSourcePickerRepository(
    {
      search: async (query, cursor) => {
        requests.push({ query, cursor });
        return page([
          row("7K3M-79QP", "Starting Question"),
          row("2R5X-E7YA", "Eligible already in another Pool"),
          row("3S8B-24DZ", "Unlicensed Question", null),
        ]);
      },
    },
    eligibility,
  );
  const definition = questionPickerSearchDefinition(
    source,
    { kind: "library", label: "Question Library" },
    eligibility,
  );
  const result = await definition.fetchPage(definition.initialQuery, null, 50);
  assert.deepEqual(
    requests.map(({ query, cursor }) => ({
      kind: query.kind,
      questions: query.questions,
      disciplineUuid: query.discipline_uuid,
      subjectUuid: query.subject_uuid,
      questionType: query.questionType,
      backend: query.backend,
      cursor,
    })),
    [
      {
        kind: "questions",
        questions: "all",
        disciplineUuid: eligibility.disciplineUuid,
        subjectUuid: eligibility.subjectUuid,
        questionType: "multipleChoice",
        backend: "ple",
        cursor: null,
      },
    ],
  );
  assert.deepEqual(
    result.items.map((item) => item.displayId),
    ["2R5X-E7YA"],
  );
});
