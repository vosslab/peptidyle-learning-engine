import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeIssuedQuestionPresentation,
  decodeStudentQuestionPresentation,
} from "../src/api/decoders/presentation_delivery.ts";

function presentation(response) {
  return {
    questionRevisionTuple: { questionId: "7K3M-79QP", revisionNumber: 1 },
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

// Permanent boundary test: native reproduction evidence must never become browser data.
// A failure means restore the closed public decoder, not a compatibility path for these fields.
test("Question Presentation rejects legacy native reproduction fields", () => {
  const issued = presentation({ kind: "fillIn", maxCharacters: 10 });
  for (const field of ["question_seed", "generated_parameter_sha256"]) {
    assert.throws(
      () => decodeIssuedQuestionPresentation({ ...issued, [field]: "server-only" }),
      DecodeError,
      field,
    );
  }
});

test("selected Student Question Presentation retains its exact Question Revision", () => {
  const selected = {
    questionRevisionTuple: { questionId: "7K3M-79QP", revisionNumber: 2 },
    prompt: [],
    response: { kind: "fillIn", maxCharacters: 10 },
  };
  assert.deepEqual(decodeStudentQuestionPresentation(selected), selected);
  assert.throws(
    () =>
      decodeStudentQuestionPresentation({
        prompt: [],
        response: { kind: "fillIn", maxCharacters: 10 },
      }),
    DecodeError,
  );
});
