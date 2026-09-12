// Stable Question Response Format decoder and educational Question Type boundary.

import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeQuestionResponseFormat,
  questionResponseFormatSupportsType,
} from "../src/api/decoders/question_response_format.ts";

test("Backend-Owned response formats are independent of author-declared Question Type", () => {
  const response = decodeQuestionResponseFormat({ kind: "backendOwned" }, "response", true);

  assert.deepEqual(response, { kind: "backendOwned" });
  assert.equal(questionResponseFormatSupportsType(response, "ordering"), true);
});
