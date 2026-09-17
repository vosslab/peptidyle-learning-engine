import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeAssessmentSummary } from "../src/api/decoders/question_library.ts";

const policies = {
  questionVariationRule: "newVariation",
  assessmentQuestionOrderRule: "shuffled",
};

const feedback = {
  score: "after_submit",
  per_item_correctness: "after_submit",
  submitted_response: "after_submit",
  question_answer: "never",
  question_answer_explanation: "never",
  class_statistics: "never",
};

function summary() {
  return {
    id: "00000000-0000-0000-0000-000000000001",
    reference: "A8H4N6PA6",
    courseId: "00000000-0000-0000-0000-000000000002",
    title: "Protein structure",
    entries: [],
    studentFeedbackReleaseRule: feedback,
    policies,
  };
}

test("Question Library Assessment decoder propagates strictness into nested policies", () => {
  assert.deepEqual(decodeAssessmentSummary(summary(), "summary", true).policies, policies);
  assert.throws(
    () =>
      decodeAssessmentSummary(
        {
          ...summary(),
          policies: {
            ...policies,
            unexpectedPolicyField: true,
          },
        },
        "summary",
        true,
      ),
    DecodeError,
  );
});
