import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionSummary } from "../src/api/decoders/question_library.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

test("Question Summary carries one exact Latest Question Revision in its stable lineage", () => {
  const summary = decodeQuestionSummary(
    publishedQuestionFixture.publishedQuestion,
    "summary",
    true,
  );
  assert.deepEqual(summary.latestQuestionRevision, {
    questionId: summary.questionId,
    revisionNumber: 1,
  });
});

test("Question Summary rejects an absent, extraneous, or cross-lineage Latest Question Revision", () => {
  const summary = publishedQuestionFixture.publishedQuestion;
  const { latestQuestionRevision: _latestQuestionRevision, ...withoutLatest } = summary;
  assert.throws(() => decodeQuestionSummary(withoutLatest, "summary", true), DecodeError);
  assert.throws(
    () =>
      decodeQuestionSummary(
        {
          ...summary,
          latestQuestionRevision: { questionId: "2R5-X7YA", revisionNumber: 1 },
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
