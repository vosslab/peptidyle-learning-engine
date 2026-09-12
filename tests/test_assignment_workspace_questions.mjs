import assert from "node:assert/strict";
import test from "node:test";

import {
  appendAvailableFixedQuestion,
  moveAssignmentEntry,
  removeAssignmentEntry,
} from "../src/pages/assignment_workspace/assignment_workspace_questions_model.ts";
import { selectedAssignmentSource } from "../src/pages/assignment_workspace/assignment_workspace_create_model.ts";
import { assignmentPolicySaveInput } from "../src/pages/assignment_workspace/assignment_workspace_policy_model.ts";

const fixed = {
  kind: "fixedQuestion",
  id: "00000000-0000-0000-0000-000000000001",
  reference: { questionId: "7K3-M9QP", revisionNumber: 1 },
  pointsPossible: "1",
  availability: "available",
  scoringRule: "normal",
  questionAttemptLimit: { maxAttempts: null },
  questionAttemptTimeLimit: { kind: "unlimited" },
};

const pool = {
  kind: "questionPool",
  id: "00000000-0000-0000-0000-000000000002",
  availability: "available",
  scoringRule: "normal",
  selectionCount: 1,
  pointsPerItem: "2",
  selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
  questionAttemptLimit: { maxAttempts: 2 },
  questionAttemptTimeLimit: { kind: "limited", seconds: 60, graceSeconds: 5 },
  items: [
    {
      id: "00000000-0000-0000-0000-000000000003",
      reference: { questionId: "2R5-X7YA", revisionNumber: 3 },
      availability: "available",
    },
  ],
};

test("Questions editing retains pool identity, item pin, availability, and policy when ordering Entries", () => {
  const entries = [fixed, pool];
  const moved = moveAssignmentEntry(entries, 1, -1);

  assert.deepEqual(
    moved.map((entry) => entry.id),
    [pool.id, fixed.id],
  );
  assert.equal(moved[0], pool);
  assert.equal(moved[0].items[0], pool.items[0]);
  assert.deepEqual(moved[0].items[0].reference, { questionId: "2R5-X7YA", revisionNumber: 3 });
  assert.equal(moved[0].selectionRule.selectedQuestionOrder, "questionPoolOrder");
  assert.equal(moved[0].questionAttemptTimeLimit.seconds, 60);
});

test("Questions picker adds an Available exact revision without flattening retained pools", () => {
  const entries = [pool];
  const added = appendAvailableFixedQuestion(
    entries,
    { reference: { questionId: "7K4-M9QP", revisionNumber: 4 }, description: "Exact revision" },
    "00000000-0000-0000-0000-000000000004",
  );

  assert.equal(added[0], pool);
  assert.deepEqual(added[1], {
    kind: "fixedQuestion",
    id: "00000000-0000-0000-0000-000000000004",
    reference: { questionId: "7K4-M9QP", revisionNumber: 4 },
    pointsPossible: "1",
    availability: "available",
    scoringRule: "normal",
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
  });
  assert.equal(JSON.stringify(added).includes("questionIds"), false);
});

test("Questions removal changes only the chosen stable Entry", () => {
  const entries = [fixed, pool];
  const remaining = removeAssignmentEntry(entries, 0);
  assert.deepEqual(
    remaining.map((entry) => entry.id),
    [pool.id],
  );
  assert.equal(remaining[0], pool);
});

test("Assignment creation uses only the deliberately selected stable Blueprint Assignment source", () => {
  const choices = [
    {
      source: {
        blueprint_revision: { reference: "BP-7", revision: "3" },
        blueprint_assignment_reference: "00000000-0000-0000-0000-000000000007",
      },
      label: "Protein structure practice",
    },
  ];

  assert.equal(selectedAssignmentSource(choices, ""), undefined);
  assert.equal(
    selectedAssignmentSource(choices, "00000000-0000-0000-0000-000000000007"),
    choices[0],
  );
});

test("Policy save retains normalized Entries and the current availability and close bounds", () => {
  const current = {
    title: "Protein structure",
    instructions: "Old instructions",
    entries: [fixed, pool],
    dueAt: "2026-09-15T23:59:00.000",
    availableAt: "2026-09-01T08:00:00.000",
    closesAt: "2026-09-17T23:59:00.000",
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      assignmentCompletionRule: { kind: "answerAll" },
      assignmentAttemptGradeRule: "latest",
      assignmentAttemptContinuationRule: { kind: "closed" },
      questionPoolReuseRule: "selectAgain",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "allQuestions",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "never",
      per_item_correctness: "never",
      submitted_response: "never",
      question_feedback: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
    },
  };
  const saved = assignmentPolicySaveInput(current, {
    instructions: "New instructions",
    dueAt: "2026-09-16T23:59:00.000",
    lateWorkRule: "accept",
    assignmentAttemptTimeLimitSeconds: 3600,
    attemptLimit: 2,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule: current.studentFeedbackReleaseRule,
  });

  assert.equal(saved.entries, current.entries);
  assert.equal(saved.availableAt, current.availableAt);
  assert.equal(saved.closesAt, current.closesAt);
  assert.equal(saved.dueAt, "2026-09-16T23:59:00.000");
});
