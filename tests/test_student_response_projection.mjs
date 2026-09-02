import assert from "node:assert/strict";
import test from "node:test";

import { projectStudentResponse } from "../src/features/question_attempt/student_response.ts";

const envelope = {
  version: "version-a",
  seed: 2,
  title: "Question",
  prompt: [],
  response: {
    kind: "multipleChoice",
    choices: [
      { id: "a", body: [{ kind: "text", markdown: "Visible A" }] },
      { id: "b", body: [{ kind: "text", markdown: "Visible B" }] },
    ],
    selection: { min: 1, max: 2 },
  },
};

test("Student Response Inspection Feedback uses only public choice bodies and rejects mismatches", () => {
  assert.deepEqual(projectStudentResponse(envelope, { kind: "multipleChoice", selected: ["b"] }), [
    { kind: "text", markdown: "Visible B" },
  ]);
  assert.deepEqual(
    projectStudentResponse(envelope, { kind: "multipleChoice", selected: ["forged"] }),
    [],
  );
  assert.deepEqual(projectStudentResponse(envelope, { kind: "shortText", text: "wrong kind" }), []);
});
