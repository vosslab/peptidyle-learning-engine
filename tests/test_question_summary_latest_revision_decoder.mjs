import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionSummary } from "../src/api/decoders/question_library.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

function currentQuestionSummary() {
  return { ...publishedQuestionFixture.publishedQuestion, questionFormat: "pleQuestionJson" };
}

test("Question Summary carries one exact Latest Question Revision in its stable lineage", () => {
  const summary = decodeQuestionSummary(currentQuestionSummary(), "summary", true);
  assert.deepEqual(summary.latestQuestionRevision, {
    questionId: summary.questionId,
    revisionNumber: 1,
  });
});

test("Question Summary rejects an absent, extraneous, or cross-lineage Latest Question Revision", () => {
  const summary = currentQuestionSummary();
  const { latestQuestionRevision: _latestQuestionRevision, ...withoutLatest } = summary;
  assert.throws(() => decodeQuestionSummary(withoutLatest, "summary", true), DecodeError);
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          latestQuestionRevision: { questionId: "2R5X-E7YA", revisionNumber: 1 },
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
