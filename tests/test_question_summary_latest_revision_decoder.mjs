import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionSummary } from "../src/api/decoders/question_library.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

function currentQuestionSummary() {
  return { ...publishedQuestionFixture.publishedQuestion, questionFormat: "pleQuestionJson" };
}

test("Question Summary carries one exact Question Revision in its stable lineage", () => {
  const summary = decodeQuestionSummary(currentQuestionSummary(), "summary", true);
  assert.deepEqual(summary.questionRevision, {
    questionId: summary.questionId,
    revisionNumber: 1,
  });
});

test("Question Summary rejects an absent, extraneous, or cross-lineage Question Revision", () => {
  const summary = currentQuestionSummary();
  const { questionRevision: _questionRevision, ...withoutLatest } = summary;
  assert.throws(() => decodeQuestionSummary(withoutLatest, "summary", true), DecodeError);
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          questionRevision: { questionId: "2R5X-E7YA", revisionNumber: 1 },
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
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          bloom: { ...summary.bloom, classificationEditNumber: 9_007_199_254_740_992 },
        },
        "summary",
        true,
      ),
    DecodeError,
  );
});
