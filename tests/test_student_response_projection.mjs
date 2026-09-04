import assert from "node:assert/strict";
import test from "node:test";

import { projectStudentResponse } from "../src/features/question_attempt/student_response.ts";

const presentation = {
  questionRevision: {
    questionId: "ABC-1234",
    revisionNumber: 1,
  },
  question_seed: 2,
  presentationNonce: "0123456789abcdef0123456789abcdef",
  questionTitle: "Question",
  prompt: [],
  response: {
    kind: "singleChoice",
    choices: [
      { id: "000a", body: [{ kind: "text", markdown: "Visible A" }] },
      { id: "000b", body: [{ kind: "text", markdown: "Visible B" }] },
    ],
  },
};

test("Student Response Inspection uses only public choice bodies and rejects mismatches", () => {
  assert.deepEqual(
    projectStudentResponse(presentation, { kind: "multipleChoice", selected: ["000b"] }),
    [{ kind: "text", markdown: "Visible B" }],
  );
  assert.deepEqual(
    projectStudentResponse(presentation, { kind: "multipleChoice", selected: ["forged"] }),
    [],
  );
  assert.deepEqual(
    projectStudentResponse(presentation, { kind: "shortText", text: "wrong kind" }),
    [],
  );
});
