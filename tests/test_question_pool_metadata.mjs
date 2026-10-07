import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionPoolView } from "../src/api/decoders/question_pool_detail.ts";
import { decodeQuestionPoolMetadata } from "../src/api/decoders/question_pool_summary.ts";

const publishedQuestionRevisionTuple = {
  publishedQuestionId: "7K3M-79QP",
  revisionNumber: 1,
};

function questionSummary() {
  return {
    questionId: publishedQuestionRevisionTuple.publishedQuestionId,
    publishedQuestionRevisionTuple,
    parentPublishedQuestionRevisionTuple: null,
    backend: "ple",
    questionFormat: "pleQuestionJson",
    questionType: "multipleChoice",
    capabilities: ["clientRendering", "serverGrading"],
    metadata: {
      questionTitle: "Peptide bond resonance",
      questionDescription: "Identify the bond with partial double-bond character.",
      tags: [],
      questionLicense: null,
      questionCitation: null,
      language: null,
    },
    authorship: { authors: [{ displayName: "Test Author", accountId: null }] },
    availability: { availability: "available" },
    publishedAt: 1_786_000_000_000,
    bloom: null,
  };
}

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
  bloomCognitiveProcess: null,
  bloomKnowledgeDimension: null,
};

const bloom = {
  cognitiveProcess: "Analyze",
  knowledgeDimension: "Conceptual Knowledge",
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
    bloomCognitiveProcess: "Analyze",
    bloomKnowledgeDimension: null,
  };
  const decoded = decodeQuestionPoolMetadata(shared, "metadata");
  assert.equal(decoded.disciplineUuid, shared.disciplineUuid);
  assert.equal(decoded.subjectUuid, shared.subjectUuid);
  assert.equal(decoded.topicUuid, shared.topicUuid);
  assert.equal(decoded.subtopicUuid, shared.subtopicUuid);
  assert.equal(decoded.bloomCognitiveProcess, "Analyze");
  assert.equal(decoded.bloomKnowledgeDimension, null);
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
  const summary = questionSummary();
  const detail = {
    questionPoolId: "3S8B-24DZ",
    ownerAccountId: "UABCDEFGM",
    questionType: "multipleChoice",
    backend: "ple",
    license: "CC0-1.0",
    questionPoolEditNumber: 4,
    canEditMetadata: true,
    metadata,
    bloom,
    members: [
      {
        publishedQuestionRevisionTuple,
        question: {
          published_question_revision_tuple: publishedQuestionRevisionTuple,
          question_library: {
            summary,
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
  const secondTuple = { publishedQuestionId: "2R5X-E7YA", revisionNumber: 1 };
  const secondQuestion = {
    ...summary,
    questionId: secondTuple.publishedQuestionId,
    publishedQuestionRevisionTuple: secondTuple,
  };
  const secondMember = {
    publishedQuestionRevisionTuple: secondTuple,
    question: {
      published_question_revision_tuple: secondTuple,
      question_library: {
        summary: secondQuestion,
        disciplineName: "Biology",
        disciplineIsRetired: false,
        evidence: { state: "unavailable" },
      },
      selection_availability: "available",
    },
  };
  assert.deepEqual(
    decodeQuestionPoolView({ ...detail, members: [secondMember, detail.members[0]] }).members.map(
      (member) => member.publishedQuestionRevisionTuple.publishedQuestionId,
    ),
    [secondTuple.publishedQuestionId, publishedQuestionRevisionTuple.publishedQuestionId],
    "the API decoder accepts an unordered exact Pool member set",
  );
  assert.throws(() => decodeQuestionPoolView({ ...detail, canEditMetadata: "true" }), DecodeError);
  assert.equal(decodeQuestionPoolView({ ...detail, bloom: null }).bloom, null);
  const { bloom: _bloom, ...withoutBloom } = detail;
  assert.throws(() => decodeQuestionPoolView(withoutBloom), DecodeError);
});
