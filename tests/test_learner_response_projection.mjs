import assert from "node:assert/strict";
import test from "node:test";

import { projectLearnerResponse } from "../src/features/attempt/learner_response.ts";

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

test("learner response projection uses only public choice bodies and rejects mismatches", () => {
  assert.deepEqual(projectLearnerResponse(envelope, { kind: "multipleChoice", selected: ["b"] }), [
    { kind: "text", markdown: "Visible B" },
  ]);
  assert.deepEqual(
    projectLearnerResponse(envelope, { kind: "multipleChoice", selected: ["forged"] }),
    [],
  );
  assert.deepEqual(projectLearnerResponse(envelope, { kind: "shortText", text: "wrong kind" }), []);
});

test("file and external learner projections never expose private object identifiers", () => {
  const file = {
    ...envelope,
    response: { kind: "fileUpload", maxBytes: 1, acceptedExtensions: [] },
  };
  const blocks = projectLearnerResponse(file, {
    kind: "fileUpload",
    objectKey: "student-records/private-key",
  });
  assert.deepEqual(blocks, [{ kind: "text", markdown: "A file was submitted." }]);
  assert.equal(JSON.stringify(blocks).includes("private-key"), false);
});
