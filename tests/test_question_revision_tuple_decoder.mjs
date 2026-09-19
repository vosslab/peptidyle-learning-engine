import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionRevisionTuple } from "../src/api/decoders/shared.ts";

const TUPLE = { questionId: "ABCD-XEFG", revisionNumber: 1 };

test("Question Revision Tuple decodes questionId plus revisionNumber", () => {
  assert.deepEqual(decodeQuestionRevisionTuple(TUPLE, "questionRevision"), TUPLE);
});

test("Question Revision Tuple rejects leftover reference JSON", () => {
  assert.throws(
    () =>
      decodeQuestionRevisionTuple(
        { reference: TUPLE },
        "questionRevision",
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionRevisionTuple(
        { ...TUPLE, reference: "ABCD-XEFG" },
        "questionRevision",
      ),
    DecodeError,
  );
});
