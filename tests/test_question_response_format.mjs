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

test("FIB and MULTI-FIB response formats decode the regex matching rule", () => {
  assert.deepEqual(
    decodeQuestionResponseFormat(
      { kind: "shortText", matchMode: "regex", maxLength: 16 },
      "response",
      true,
    ),
    { kind: "shortText", matchMode: "regex", maxLength: 16 },
  );
  assert.deepEqual(
    decodeQuestionResponseFormat(
      {
        kind: "multiBlank",
        blanks: [{ id: "first", label: [], matchMode: "regex", maxLength: 16 }],
      },
      "response",
      true,
    ),
    {
      kind: "multiBlank",
      blanks: [{ id: "first", label: [], matchMode: "regex", maxLength: 16 }],
    },
  );
});
