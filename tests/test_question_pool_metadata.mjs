import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionPoolView } from "../src/api/decoders/question_pool_detail.ts";
import { decodeQuestionPoolMetadata } from "../src/api/decoders/question_pool_summary.ts";
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

test("Pool metadata retains its independent current identity", () => {
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

test("Question Pools use the shared Question Library metadata required for publication", () => {
  const shared = {
    ...metadata,
    topicUuid: "00000000-0000-0000-0000-000000000003",
    subtopicUuid: "00000000-0000-0000-0000-000000000004",
    tags: ["review"],
  };
  const decoded = decodeQuestionPoolMetadata(shared, "metadata");
  assert.equal(decoded.disciplineUuid, shared.disciplineUuid);
  assert.equal(decoded.subjectUuid, shared.subjectUuid);
  assert.equal(decoded.topicUuid, shared.topicUuid);
  assert.equal(decoded.subtopicUuid, shared.subtopicUuid);
  assert.deepEqual(decoded.tags, shared.tags);
  for (const field of ["disciplineUuid", "subjectUuid"]) {
    const incomplete = { ...shared };
    delete incomplete[field];
    assert.throws(() => decodeQuestionPoolMetadata(incomplete, "metadata"), DecodeError);
  }
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

test("Pool exact detail keeps its own assigned Bloom pair or a blank pair", () => {
  const publishedQuestionRevisionTuple =
    publishedQuestionFixture.publishedQuestion.publishedQuestionRevisionTuple;
  const detail = {
    questionPoolId: "3S8B-24DZ",
    ownerAccountId: "UABCDEFGM",
    questionType: "multipleChoice",
    backend: "ple",
    license: "CC0-1.0",
    questionPoolEditNumber: 4,
    metadata,
    bloom,
    members: [
      {
        memberPosition: 0,
        publishedQuestionRevisionTuple,
        question: {
          published_question_revision_tuple: publishedQuestionRevisionTuple,
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
  const publishedQuestionRevisionTuple =
    publishedQuestionFixture.publishedQuestion.publishedQuestionRevisionTuple;
  const fork = {
    assessmentEntryId: "00000000-0000-0000-0000-000000000011",
    questionPoolId: "3S8B-24DZ",
    ownerAccountId: "UABCDEFGM",
    questionType: "multipleChoice",
    backend: "ple",
    license: "CC0-1.0",
    questionPoolEditNumber: 4,
    selectionCount: 1,
    metadata,
    bloom,
    members: [
      {
        memberPosition: 0,
        publishedQuestionRevisionTuple,
        question: {
          published_question_revision_tuple: publishedQuestionRevisionTuple,
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
