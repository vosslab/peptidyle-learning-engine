import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeIssuedQuestionPresentation } from "../src/api/decoders/presentation_delivery.ts";

function presentation(response) {
  return {
    questionRevision: { questionId: "7K3-M9QP", revisionNumber: 1 },
    question_seed: 2,
    presentationNonce: "0123456789abcdef0123456789abcdef",
    questionTitle: "Question",
    prompt: [],
    response,
  };
}

test("Question Presentation accepts a multiple-answer response with no choices", () => {
  assert.deepEqual(
    decodeIssuedQuestionPresentation(
      presentation({ kind: "multipleAnswer", choices: [], minimum: 0, maximum: 0 }),
    ).response,
    { kind: "multipleAnswer", choices: [], minimum: 0, maximum: 0 },
  );
});

test("Question Presentation accepts a matching response with one prompt and one choice", () => {
  assert.equal(
    decodeIssuedQuestionPresentation(
      presentation({
        kind: "matching",
        prompts: [{ id: "0001", body: [] }],
        choices: [{ id: "0002", body: [] }],
        reuseChoices: false,
      }),
    ).response.kind,
    "matching",
  );
});

test("Question Presentation rejects an extra multiple-answer response field", () => {
  assert.throws(
    () =>
      decodeIssuedQuestionPresentation(
        presentation({
          kind: "multipleAnswer",
          choices: [],
          minimum: 0,
          maximum: 0,
          answerKey: "private",
        }),
      ),
    DecodeError,
  );
});
