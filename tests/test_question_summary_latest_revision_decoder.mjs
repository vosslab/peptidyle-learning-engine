import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionSummary } from "../src/api/decoders/question_library.ts";

function currentQuestionSummary() {
  return {
    questionId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
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
      language: "en-US",
    },
    authorship: { authors: [{ displayName: "Test Author", accountId: null }] },
    availability: { availability: "available" },
    publishedAt: 1_786_000_000_000,
    bloom: null,
  };
}

test("Question Summary carries one exact Question Revision in its stable lineage", () => {
  const summary = decodeQuestionSummary(currentQuestionSummary(), "summary", true);
  assert.deepEqual(summary.publishedQuestionRevisionTuple, {
    publishedQuestionId: summary.questionId,
    revisionNumber: 1,
  });
  assert.equal(summary.parentPublishedQuestionRevisionTuple, null);
});

test("Question Summary preserves one nullable exact immediate parent Revision", () => {
  const summary = currentQuestionSummary();
  const parent = {
    publishedQuestionId: "BPFX-Y001",
    revisionNumber: 3,
  };
  assert.deepEqual(
    decodeQuestionSummary(
      { ...summary, parentPublishedQuestionRevisionTuple: parent },
      "summary",
      true,
    ).parentPublishedQuestionRevisionTuple,
    parent,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          parentPublishedQuestionRevisionTuple: summary.publishedQuestionRevisionTuple,
        },
        "summary",
        true,
      ),
    DecodeError,
  );
  const { parentPublishedQuestionRevisionTuple: _parent, ...withoutParent } = summary;
  assert.throws(() => decodeQuestionSummary(withoutParent, "summary", true), DecodeError);
});

test("Question Summary preserves nullable supplied language and rejects invalid language types", () => {
  const summary = currentQuestionSummary();
  const withoutLanguage = {
    ...summary,
    metadata: { ...summary.metadata, language: null },
  };
  assert.equal(decodeQuestionSummary(withoutLanguage, "summary", true).metadata.language, null);
  assert.equal(
    decodeQuestionSummary(summary, "summary", true).metadata.language,
    summary.metadata.language,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        { ...summary, metadata: { ...summary.metadata, language: 7 } },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        { ...summary, metadata: { ...summary.metadata, unrecognized: true } },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        { ...summary, metadata: { ...summary.metadata, questionTitle: "   " } },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        { ...summary, metadata: { ...summary.metadata, questionDescription: "   " } },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          metadata: {
            ...summary.metadata,
            questionCitation: { citationUrl: null, citationText: null },
          },
        },
        "summary",
        true,
      ),
    DecodeError,
  );
});

test("Question Summary rejects an absent, extraneous, or cross-lineage Question Revision", () => {
  const summary = currentQuestionSummary();
  const { publishedQuestionRevisionTuple: _publishedQuestionRevisionTuple, ...withoutLatest } =
    summary;
  assert.throws(() => decodeQuestionSummary(withoutLatest, "summary", true), DecodeError);
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          publishedQuestionRevisionTuple: { publishedQuestionId: "2R5X-E7YA", revisionNumber: 1 },
        },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.throws(
    () => decodeQuestionSummary({ ...summary, currentQuestionRevision: null }, "summary", true),
    DecodeError,
  );
});

test("Question Summary requires the complete exact Bloom pair and precision-safe Edit Number", () => {
  const summary = currentQuestionSummary();
  const { bloom: _bloom, ...withoutBloom } = summary;
  assert.throws(() => decodeQuestionSummary(withoutBloom, "summary", true), DecodeError);
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          bloom: { ...summary.bloom, cognitiveProcess: "Synthesize" },
        },
        "summary",
        true,
      ),
    DecodeError,
  );
  assert.deepEqual(
    decodeQuestionSummary(
      {
        ...summary,
        bloom: { cognitiveProcess: null, knowledgeDimension: "Conceptual Knowledge" },
      },
      "summary",
      true,
    ).bloom,
    { cognitiveProcess: null, knowledgeDimension: "Conceptual Knowledge" },
  );
});
