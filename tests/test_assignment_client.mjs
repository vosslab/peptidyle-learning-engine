import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeAddAssignmentItemInput,
  decodeReplaceAssignmentItemQuestionInput,
} from "../src/api/decoders/catalog_course.ts";

test("assignment command inputs carry only public Question IDs and positions", () => {
  assert.deepEqual(decodeAddAssignmentItemInput({ questionId: "7K3-M9QP", position: 1 }), {
    questionId: "7K3-M9QP",
    position: 1,
  });
  assert.throws(() =>
    decodeAddAssignmentItemInput({ questionId: "7K3-M9QP", position: 1, browserClock: 1 }),
  );
  assert.deepEqual(decodeReplaceAssignmentItemQuestionInput({ questionId: "7K3-M9QP" }), {
    questionId: "7K3-M9QP",
  });
});
