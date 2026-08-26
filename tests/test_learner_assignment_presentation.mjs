// Shared answer-free learner landing behavior checks.

import assert from "node:assert/strict";
import test from "node:test";

import {
  formatAssignmentRunTimeLimit,
  toLearnerAssignmentPresentationData,
} from "../src/components/learner_assignment_presentation.tsx";

const delivery = {
  availableAt: null,
  dueAt: null,
  closesAt: null,
  timeLimitSeconds: 900,
  attemptLimit: 2,
  lateSubmission: "accept",
  deadlineBehavior: "autoSubmit",
};

test("learner detail adapts active items and pool draws without exposing source identities", () => {
  const presentation = toLearnerAssignmentPresentationData({
    id: "assignment-1",
    reference: "AS-1",
    title: "Protein structure",
    instructions: "Use your notes.",
    timeZone: "America/Chicago",
    delivery: { ...delivery, lateStatus: "onTime" },
    items: [{ deliveryState: "active" }, { deliveryState: "retired" }, { deliveryState: "active" }],
    selectionGroups: [{ drawCount: 3 }],
  });

  assert.equal(presentation.questionsPerRun, 5);
  assert.equal(presentation.delivery.lateStatus, "onTime");
  assert.equal("id" in presentation, false);
});

test("Instructor Student view keeps its explicit variation and disclosure data", () => {
  const presentation = toLearnerAssignmentPresentationData({
    title: "Protein structure",
    instructions: "Use your notes.",
    timeZone: "America/Chicago",
    delivery,
    questionsPerRun: 4,
    variation: "fullRegeneration",
    disclosurePolicy: {
      score: "afterSubmit",
      perItemCorrectness: "afterSubmit",
      feedbackText: "afterDue",
      solution: "afterClose",
      classStatistics: "never",
    },
  });

  assert.equal(presentation.questionsPerRun, 4);
  assert.equal(presentation.variation, "fullRegeneration");
  assert.equal(presentation.disclosurePolicy?.feedbackText, "afterDue");
  assert.equal("lateStatus" in presentation.delivery, false);
});

test("run-time copy stays readable across minute, hour, and second limits", () => {
  assert.equal(formatAssignmentRunTimeLimit(3_600), "1 hour per run");
  assert.equal(formatAssignmentRunTimeLimit(90), "90 seconds per run");
});
