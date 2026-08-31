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

test("Student response projection uses only public choice bodies and rejects mismatches", () => {
  assert.deepEqual(projectStudentResponse(envelope, { kind: "multipleChoice", selected: ["b"] }), [
    { kind: "text", markdown: "Visible B" },
  ]);
  assert.deepEqual(
    projectStudentResponse(envelope, { kind: "multipleChoice", selected: ["forged"] }),
    [],
  );
  assert.deepEqual(projectStudentResponse(envelope, { kind: "shortText", text: "wrong kind" }), []);
});

test("file and external Student projections never expose private object identifiers", () => {
  const file = {
    ...envelope,
    response: { kind: "fileUpload", maxBytes: 1, acceptedExtensions: [] },
  };
  const blocks = projectStudentResponse(file, {
    kind: "fileUpload",
    objectKey: "student-records/private-key",
  });
  assert.deepEqual(blocks, [{ kind: "text", markdown: "A file was submitted." }]);
  assert.equal(JSON.stringify(blocks).includes("private-key"), false);
});
