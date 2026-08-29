import assert from "node:assert/strict";
import test from "node:test";

import { decodeInstructorStudentView } from "../src/api/decoders/assignment_teaching_delivery.ts";

test("Instructor Student view accepts an empty draft and full regeneration without identities", () => {
  const view = decodeInstructorStudentView({
    title: "Peptide bonds",
    instructions: "Add questions before publishing.",
    timeZone: "America/Chicago",
    delivery: {
      availableAt: null,
      dueAt: null,
      closesAt: null,
      timeLimitSeconds: null,
      attemptLimit: null,
      lateSubmission: "accept",
      deadlineBehavior: "autoSubmit",
    },
    questionsPerRun: 0,
    variation: "fullRegeneration",
    disclosurePolicy: {
      score: "never",
      per_item_correctness: "never",
      feedback_text: "never",
      solution: "never",
      class_statistics: "never",
    },
  });
  assert.equal(view.questionsPerRun, 0);
  assert.equal(view.variation, "fullRegeneration");
  assert.equal("lateStatus" in view.delivery, false);
  assert.throws(() =>
    decodeInstructorStudentView({ ...view, assignmentId: "00000000-0000-0000-0000-000000000001" }),
  );
});
