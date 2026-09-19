import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPoolLibraryPage,
  decodeQuestionPoolMetadata,
  decodeQuestionPoolView,
} from "../src/api/decoders/question_pool_library.ts";
import { decodeAssessmentQuestionPoolForkView } from "../src/api/decoders/assessment_pool_fork.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const metadata = {
  title: "Inheritance reasoning",
  description: "Interpret interchangeable pedigrees.",
  disciplineUuid: "00000000-0000-0000-0000-000000000001",
  disciplineName: "Biology",
  disciplineIsRetired: false,
  subjectUuid: "00000000-0000-0000-0000-000000000002",
  topicUuid: null,
  subtopicUuid: null,
  tags: ["pedigrees"],
};

const bloom = {
  cognitiveProcess: "Analyze",
  knowledgeDimension: "Conceptual Knowledge",
  classificationEditNumber: "7",
};

const bloomFacets = {
  cognitiveProcesses: ["Remember", "Understand", "Apply", "Analyze", "Evaluate", "Create"].map(
    (cognitiveProcess, count) => ({ cognitiveProcess, count }),
  ),
  knowledgeDimensions: [
    "Factual Knowledge",
    "Conceptual Knowledge",
    "Procedural Knowledge",
    "Metacognitive Knowledge",
  ].map((knowledgeDimension, count) => ({ knowledgeDimension, count })),
};

test("Pool list retains independent metadata and current Pool identity", () => {
  const page = {
    items: [
      {
        questionPoolId: "3S8B-24DZ",
        questionPoolEditNumber: 4,
        metadata,
        memberCount: 2,
        bloom,
      },
    ],
    nextCursor: null,
    bloomFacets,
  };
  assert.deepEqual(decodeQuestionPoolLibraryPage(page), page);
  const retiredPage = {
    ...page,
    items: [{ ...page.items[0], metadata: { ...metadata, disciplineIsRetired: true } }],
  };
  assert.equal(
    decodeQuestionPoolLibraryPage(retiredPage).items[0]?.metadata.disciplineIsRetired,
    true,
  );
  const unicodeMetadata = {
    ...metadata,
    title: "\u00a0Inheritance\u00a0",
    description: "\u2003Interpret pedigrees\u2003",
    topicUuid: "00000000-0000-0000-0000-000000000003",
    subtopicUuid: "00000000-0000-0000-0000-000000000004",
    tags: ["\u00a0pedigrees\u00a0"],
  };
  const unicodePage = { ...page, items: [{ ...page.items[0], metadata: unicodeMetadata }] };
  assert.deepEqual(decodeQuestionPoolLibraryPage(unicodePage), unicodePage);
  const manyTags = Array.from({ length: 100 }, (_value, index) => `tag-${index}`);
  assert.deepEqual(
    decodeQuestionPoolMetadata({ ...metadata, tags: manyTags }, "metadata").tags,
    manyTags,
  );
  assert.deepEqual(
    decodeQuestionPoolMetadata({ ...metadata, title: "\u{1f9ec}".repeat(512) }, "metadata").title,
    "\u{1f9ec}".repeat(512),
  );
});

test("Pool metadata rejects missing required fields, unknown fields and malformed classifications", () => {
  for (const patch of [
    { title: undefined },
    { description: " " },
    { title: " trailing " },
    { disciplineUuid: null },
    { disciplineName: "" },
    { disciplineIsRetired: "false" },
    { subjectUuid: "Biology" },
    { topicUuid: "Genetics" },
    { subtopicUuid: "00000000-0000-0000-0000-000000000004" },
    { tags: ["pedigrees", "pedigrees"] },
    { tags: ["a".repeat(121)] },
    { ownerAccountId: "forbidden" },
  ]) {
    assert.throws(
      () => decodeQuestionPoolMetadata({ ...metadata, ...patch }, "metadata"),
      DecodeError,
    );
  }
});

test("Pool list permits a blank Bloom pair and rejects partial or malformed Bloom", () => {
  const item = {
    questionPoolId: "3S8B-24DZ",
    questionPoolEditNumber: 4,
    metadata,
    memberCount: 2,
    bloom,
  };
  assert.deepEqual(
    decodeQuestionPoolLibraryPage({
      items: [{ ...item, bloom: null }],
      nextCursor: null,
      bloomFacets,
    }).items[0]?.bloom,
    null,
  );
  for (const malformedBloom of [
    undefined,
    { ...bloom, cognitiveProcess: "Synthesize" },
    { ...bloom, knowledgeDimension: "Strategic Knowledge" },
    { ...bloom, classificationEditNumber: 7 },
    { ...bloom, classificationEditNumber: "0" },
    { ...bloom, classificationEditNumber: "9223372036854775808" },
    { ...bloom, difficulty: "Hard" },
  ]) {
    assert.throws(
      () =>
        decodeQuestionPoolLibraryPage({
          items: [{ ...item, bloom: malformedBloom }],
          nextCursor: null,
          bloomFacets,
        }),
      DecodeError,
    );
  }
});

test("Pool list requires complete ordered whole-result Bloom counts", () => {
  const page = { items: [], nextCursor: null, bloomFacets };
  assert.deepEqual(decodeQuestionPoolLibraryPage(page), page);
  for (const malformed of [
    { ...bloomFacets, cognitiveProcesses: bloomFacets.cognitiveProcesses.slice(1) },
    {
      ...bloomFacets,
      cognitiveProcesses: bloomFacets.cognitiveProcesses.toReversed(),
    },
    {
      ...bloomFacets,
      knowledgeDimensions: bloomFacets.knowledgeDimensions.map((facet, index) =>
        index === 0 ? { ...facet, count: -1 } : facet,
      ),
    },
    { ...bloomFacets, memberCounts: [] },
  ]) {
    assert.throws(
      () => decodeQuestionPoolLibraryPage({ ...page, bloomFacets: malformed }),
      DecodeError,
    );
  }
});

test("Pool exact detail keeps its own assigned Bloom pair or a blank pair", () => {
  const questionRevisionTuple = publishedQuestionFixture.publishedQuestion.questionRevisionTuple;
  const detail = {
    questionPoolId: "3S8B-24DZ",
    questionPoolEditNumber: 4,
    metadata,
    bloom,
    members: [
      {
        memberPosition: 0,
        questionRevisionTuple,
        question: {
          question_revision_tuple: questionRevisionTuple,
          question_library: {
            summary: publishedQuestionFixture.publishedQuestion,
            disciplineName: "Biology",
            disciplineIsRetired: false,
            evidence: { state: "unavailable" },
          },
          selection_availability: "available",
        },
      },
    ],
    evidence: { state: "unavailable" },
  };
  assert.deepEqual(decodeQuestionPoolView(detail), detail);
  assert.equal(decodeQuestionPoolView({ ...detail, bloom: null }).bloom, null);
  const { bloom: _bloom, ...withoutBloom } = detail;
  assert.throws(() => decodeQuestionPoolView(withoutBloom), DecodeError);
});

test("Assessment-owned Pool fork keeps its own assigned Bloom pair or a blank pair", () => {
  const questionRevisionTuple = publishedQuestionFixture.publishedQuestion.questionRevisionTuple;
  const fork = {
    assessmentEntryId: "00000000-0000-0000-0000-000000000011",
    questionPoolId: "3S8B-24DZ",
    questionPoolEditNumber: 4,
    selectionCount: 1,
    metadata,
    bloom,
    members: [
      {
        memberPosition: 0,
        questionRevisionTuple,
        question: {
          question_revision_tuple: questionRevisionTuple,
          question_library: {
            summary: publishedQuestionFixture.publishedQuestion,
            disciplineName: "Biology",
            disciplineIsRetired: false,
            evidence: { state: "unavailable" },
          },
          selection_availability: "available",
        },
      },
    ],
  };
  assert.deepEqual(decodeAssessmentQuestionPoolForkView(fork), fork);
  assert.equal(decodeAssessmentQuestionPoolForkView({ ...fork, bloom: null }).bloom, null);
  const { bloom: _bloom, ...withoutBloom } = fork;
  assert.throws(() => decodeAssessmentQuestionPoolForkView(withoutBloom), DecodeError);
});
