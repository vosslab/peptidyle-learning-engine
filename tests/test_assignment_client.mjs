import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeAddAssignmentItemInput,
  decodeReplaceAssignmentItemQuestionInput,
} from "../src/api/decoders/catalog_course.ts";
import { decodeInstructorStudentView } from "../src/api/decoders/assignment_teaching_delivery.ts";

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
      perItemCorrectness: "never",
      feedbackText: "never",
      solution: "never",
      classStatistics: "never",
    },
  });
  assert.equal(view.questionsPerRun, 0);
  assert.equal(view.variation, "fullRegeneration");
  assert.equal("lateStatus" in view.delivery, false);
  assert.throws(() =>
    decodeInstructorStudentView({ ...view, assignmentId: "00000000-0000-0000-0000-000000000001" }),
  );
});
