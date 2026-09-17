import assert from "node:assert/strict";
import test from "node:test";

import { decodeQuestionResponsePreview } from "../src/api/decoders/question_response_preview.ts";

test("Library response previews reject answer, author identity, and grading fields", () => {
  const preview = {
    kind: "multipleChoice",
    choices: [[{ kind: "text", markdown: "Visible choice" }]],
    selection: { kind: "exactlyOne" },
  };
  assert.deepEqual(decodeQuestionResponsePreview(preview, "preview"), preview);
  for (const field of ["answerKey", "grading", "hints", "feedback", "source"]) {
    assert.throws(() =>
      decodeQuestionResponsePreview({ ...preview, [field]: "private" }, "preview"),
    );
  }
  assert.throws(() =>
    decodeQuestionResponsePreview(
      { ...preview, choices: [{ id: "correct", body: preview.choices[0] }] },
      "preview",
    ),
  );
  assert.throws(() =>
    decodeQuestionResponsePreview({ kind: "numeric", unit: "mL", tolerance: 0.1 }, "preview"),
  );
  assert.throws(() =>
    decodeQuestionResponsePreview({ kind: "shortText", matchMode: "exact" }, "preview"),
  );
});
