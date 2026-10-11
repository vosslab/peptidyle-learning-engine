import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentFeedback } from "../src/api/decoders.ts";

test("disclosed Student Feedback rejects private grading data and malformed blocks", () => {
  assert.throws(
    () => decodeStudentFeedback({ correctness: true, answerKey: "private" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeStudentFeedback({
        choiceFeedback: [{ kind: "text", markdown: "Try again.", providerTranscript: "private" }],
      }),
    DecodeError,
  );
});
