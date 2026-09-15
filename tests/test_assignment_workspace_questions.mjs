import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";

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
  reference: { questionId: "7K3M-X9QP", revisionNumber: 1 },
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
      reference: { questionId: "2R5X-Z7YA", revisionNumber: 3 },
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
  assert.deepEqual(moved[0].items[0].reference, { questionId: "2R5X-Z7YA", revisionNumber: 3 });
  assert.equal(moved[0].selectionRule.selectedQuestionOrder, "questionPoolOrder");
  assert.equal(moved[0].questionAttemptTimeLimit.seconds, 60);
});

test("Questions picker adds an Available exact revision without flattening retained pools", () => {
  const entries = [pool];
  const added = appendAvailableFixedQuestion(
    entries,
    { reference: { questionId: "7K4M-X9QP", revisionNumber: 4 }, description: "Exact revision" },
    "00000000-0000-0000-0000-000000000004",
  );

  assert.equal(added[0], pool);
  assert.deepEqual(added[1], {
    kind: "fixedQuestion",
    id: "00000000-0000-0000-0000-000000000004",
    reference: { questionId: "7K4M-X9QP", revisionNumber: 4 },
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

async function loadQuestionsPageExport(name) {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL(
        "../src/pages/assignment_workspace/assignment_workspace_questions_page.tsx",
        import.meta.url,
      ).pathname,
    ],
    format: "cjs",
    outfile: "assignment_workspace_questions_page.js",
    platform: "node",
    external: ["*"],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Questions workspace bundle is missing JavaScript.");
  const module = { exports: {} };
  new Function("require", "module", "exports", javascript.text)(() => ({}), module, module.exports);
  if (typeof module.exports[name] !== "function") {
    throw new Error(`Questions workspace does not export ${name}.`);
  }
  return module.exports[name];
}

test("Question structural edits remain guarded until a successful Save", async () => {
  const nextQuestionEditDirty = await loadQuestionsPageExport("nextQuestionEditDirty");
  for (const event of ["title", "move", "remove", "add"]) {
    assert.equal(nextQuestionEditDirty(false, event), true, `${event} marks the editor dirty`);
  }
  assert.equal(nextQuestionEditDirty(true, "saveFailed"), true);
  assert.equal(nextQuestionEditDirty(true, "saveSucceeded"), false);
});

test("Question saves carry every editable policy without leaking workspace response fields", async () => {
  const questionSaveInput = await loadQuestionsPageExport("questionSaveInput");
  const current = {
    reference: "00000000-0000-0000-0000-000000000099",
    editNumber: 4,
    status: "unreleased",
    source: { blueprint_revision: { reference: "BP-1", revision: "1" } },
    title: "Original title",
    instructions: "Practice carefully.",
    entries: [fixed],
    dueAt: "2026-09-15T23:59:00.000",
    availableAt: "2026-09-01T08:00:00.000",
    closesAt: "2026-09-17T23:59:00.000",
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: 1800,
    attemptLimit: 2,
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
    displayTimeZone: "America/Chicago",
    questions: [],
  };
  const entries = [pool, fixed];
  const saved = questionSaveInput(current, "Reordered Questions", entries);

  assert.deepEqual(saved, {
    title: "Reordered Questions",
    instructions: current.instructions,
    entries,
    dueAt: current.dueAt,
    availableAt: current.availableAt,
    closesAt: current.closesAt,
    lateWorkRule: current.lateWorkRule,
    assignmentAttemptTimeLimitSeconds: current.assignmentAttemptTimeLimitSeconds,
    attemptLimit: current.attemptLimit,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule: current.studentFeedbackReleaseRule,
  });
  for (const responseOnlyKey of [
    "reference",
    "editNumber",
    "status",
    "source",
    "displayTimeZone",
    "questions",
  ]) {
    assert.equal(
      responseOnlyKey in saved,
      false,
      `${responseOnlyKey} stays out of the save payload`,
    );
  }
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
