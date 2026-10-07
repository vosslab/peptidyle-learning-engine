import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  appendAvailableFixedQuestion,
  assessmentPointValueDraft,
  deliveredAssessmentQuestionCount,
  moveAssessmentEntry,
  removeAssessmentEntry,
  sortAssessmentEntriesByBloom,
  withFixedQuestionPointValues,
} from "../src/pages/assessment_workspace/assessment_workspace_questions_model.ts";
import { selectedAssessmentEntryContent } from "../src/pages/assessment_workspace/assessment_workspace_selected_entry.ts";
import { assessmentPolicySaveInput } from "../src/pages/assessment_workspace/assessment_workspace_policy_model.ts";
import { resolveQuestionIdBatch } from "../src/pages/assessment_workspace/assessment_question_id_batch.ts";

const fixed = {
  kind: "fixedQuestion",
  id: "00000000-0000-0000-0000-000000000001",
  publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
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
  questionAttemptLimit: { maxAttempts: 2 },
  questionAttemptTimeLimit: { kind: "limited", seconds: 60, graceSeconds: 5 },
  items: [
    {
      id: "00000000-0000-0000-0000-000000000003",
      publishedQuestionRevisionTuple: { publishedQuestionId: "2R5X-E7YA", revisionNumber: 3 },
      availability: "available",
    },
  ],
};

test("Questions editing retains pool identity, item pin, availability, and policy when ordering Entries", () => {
  const entries = [fixed, pool];
  const moved = moveAssessmentEntry(entries, 1, -1);

  assert.deepEqual(
    moved.map((entry) => entry.id),
    [pool.id, fixed.id],
  );
  assert.equal(moved[0], pool);
  assert.equal(moved[0].items[0], pool.items[0]);
  assert.deepEqual(moved[0].items[0].publishedQuestionRevisionTuple, {
    publishedQuestionId: "2R5X-E7YA",
    revisionNumber: 3,
  });
  assert.equal(moved[0].questionAttemptTimeLimit.seconds, 60);
});

test("Questions picker adds an Available exact revision without flattening retained pools", () => {
  const entries = [pool];
  const added = appendAvailableFixedQuestion(
    entries,
    { publishedQuestionId: "7K4M-69QP", revisionNumber: 4 },
    "00000000-0000-0000-0000-000000000004",
  );

  assert.equal(added[0], pool);
  assert.deepEqual(added[1], {
    kind: "fixedQuestion",
    id: "00000000-0000-0000-0000-000000000004",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K4M-69QP", revisionNumber: 4 },
    pointsPossible: "1",
    availability: "available",
    scoringRule: "normal",
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
  });
  assert.equal(JSON.stringify(added).includes("questionIds"), false);
});

test("Bloom sort retains unavailable exact fixed and Pool Entries without changing payloads", () => {
  const laterFixed = {
    ...fixed,
    id: "00000000-0000-0000-0000-000000000010",
    availability: "retired",
  };
  const tiedFixed = { ...fixed, id: "00000000-0000-0000-0000-000000000011" };
  const tiedPool = {
    ...pool,
    id: "00000000-0000-0000-0000-000000000012",
    availability: "retired",
  };
  const firstFixed = { ...fixed, id: "00000000-0000-0000-0000-000000000013" };
  const entries = [laterFixed, tiedPool, firstFixed, tiedFixed];
  const bloomByEntryId = new Map([
    [
      laterFixed.id,
      {
        cognitiveProcess: "Evaluate",
        knowledgeDimension: "Factual Knowledge",
      },
    ],
    [
      tiedPool.id,
      {
        cognitiveProcess: "Apply",
        knowledgeDimension: "Conceptual Knowledge",
      },
    ],
    [
      firstFixed.id,
      {
        cognitiveProcess: "Remember",
        knowledgeDimension: "Metacognitive Knowledge",
      },
    ],
    [
      tiedFixed.id,
      {
        cognitiveProcess: "Apply",
        knowledgeDimension: "Conceptual Knowledge",
      },
    ],
  ]);

  const sorted = sortAssessmentEntriesByBloom(entries, bloomByEntryId);
  assert.deepEqual(
    sorted.map((entry) => entry.id),
    [firstFixed.id, tiedPool.id, tiedFixed.id, laterFixed.id],
  );
  assert.equal(sorted[1], tiedPool);
  assert.equal(sorted[1].items[0], pool.items[0]);
  assert.equal(sorted[3], laterFixed);
  assert.deepEqual(
    sorted[3].publishedQuestionRevisionTuple,
    laterFixed.publishedQuestionRevisionTuple,
  );
  assert.equal(sortAssessmentEntriesByBloom(sorted, bloomByEntryId), sorted);
  const partiallyClassified = new Map(bloomByEntryId);
  partiallyClassified.delete(tiedPool.id);
  assert.deepEqual(
    sortAssessmentEntriesByBloom(entries, partiallyClassified).map((entry) => entry.id),
    [firstFixed.id, tiedFixed.id, laterFixed.id, tiedPool.id],
  );
  const incomplete = new Map(bloomByEntryId);
  incomplete.set(tiedPool.id, {
    cognitiveProcess: "Apply",
    knowledgeDimension: null,
  });
  assert.deepEqual(
    sortAssessmentEntriesByBloom(entries, incomplete).map((entry) => entry.id),
    [firstFixed.id, tiedFixed.id, laterFixed.id, tiedPool.id],
  );
});

test("Assessment Entry display labels each absent Bloom dimension", () => {
  const content = selectedAssessmentEntryContent({
    entry: fixed,
    entryNumber: 1,
    questionTitle: () => "Amino acid charge",
    poolTitle: () => "Protein questions",
    description: () => "Use the supplied pH.",
    bloom: { cognitiveProcess: "Apply", knowledgeDimension: null },
    removeDisabled: false,
    remove: () => undefined,
  });
  assert.ok(
    content.details.some(
      (fact) =>
        fact.kind === "text" && fact.label === "Bloom Cognitive Process" && fact.value === "Apply",
    ),
  );
  assert.ok(
    content.details.some(
      (fact) =>
        fact.kind === "text" &&
        fact.label === "Bloom Knowledge Dimension" &&
        fact.value === "Not assigned",
    ),
  );
});

test("Questions removal changes only the chosen stable Entry", () => {
  const entries = [fixed, pool];
  const remaining = removeAssessmentEntry(entries, 0);
  assert.deepEqual(
    remaining.map((entry) => entry.id),
    [pool.id],
  );
  assert.equal(remaining[0], pool);
});

test("Unavailable Entries do not consume delivery capacity and remain read-only evidence", () => {
  const retiredFixed = { ...fixed, availability: "retired" };
  const retiredPool = { ...pool, availability: "retired", selectionCount: 249 };
  const entries = [retiredFixed, fixed, retiredPool, pool];

  assert.equal(deliveredAssessmentQuestionCount(entries), 2);
  assert.equal(removeAssessmentEntry(entries, 0), entries);
  assert.equal(removeAssessmentEntry(entries, 2), entries);
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
    assessmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      partialCreditEnabled: true,
      questionVariationRule: "newVariation",
      assessmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      per_item_correctness: "never",
      submitted_response: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
      hints: "never",
      worked_solutions: "never",
    },
  };
  const studentFeedbackReleaseRule = {
    ...current.studentFeedbackReleaseRule,
    question_answer: "after_due",
  };
  const saved = assessmentPolicySaveInput(current, {
    instructions: "New instructions",
    dueAt: "2026-09-16T23:59:00.000",
    lateWorkRule: "accept",
    assessmentAttemptTimeLimitSeconds: 3600,
    attemptLimit: 2,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule,
  });

  assert.equal(saved.entries, current.entries);
  assert.equal(saved.availableAt, current.availableAt);
  assert.equal(saved.closesAt, current.closesAt);
  assert.equal(saved.dueAt, "2026-09-16T23:59:00.000");
  assert.equal(saved.attemptLimit, 2);
  assert.equal(saved.lateWorkRule, "accept");
  assert.equal(saved.studentFeedbackReleaseRule.question_answer, "after_due");
});

test("Assessment Properties scoring changes only the chosen fixed Question points", () => {
  const points = assessmentPointValueDraft("4.5");
  assert.equal(points, "4.5");
  const updated = withFixedQuestionPointValues([fixed, pool], { [fixed.id]: points });
  assert.equal(updated[1], pool);
  assert.notEqual(updated[0], fixed);
  assert.equal(updated[0].pointsPossible, "4.5");
  assert.equal(updated[0].publishedQuestionRevisionTuple, fixed.publishedQuestionRevisionTuple);
});

test("Question ID paste adds the canonical batch and withholds invalid or missing IDs", async () => {
  const resolved = [];
  const pin = {
    questionId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 4 },
    questionTitle: "Peptide bond",
    description: "Formation",
    bloom: null,
  };
  const batch = await resolveQuestionIdBatch(
    "7K3M-79QP, 7k3m79qp\n2R5X-E7YA",
    2,
    async (questionId) => {
      resolved.push(questionId);
      if (questionId === "2R5X-E7YA") return "missing";
      return pin;
    },
  );
  assert.deepEqual(resolved, ["7K3M-79QP", "2R5X-E7YA"]);
  assert.equal(batch.kind, "resolved");
  if (batch.kind !== "resolved") return;
  assert.deepEqual(batch.pins, [pin]);
  assert.deepEqual(batch.missing, ["2R5X-E7YA"]);
  assert.deepEqual(batch.unavailable, []);

  let calls = 0;
  const invalid = await resolveQuestionIdBatch("7K3M-79QP not-an-id", 2, async () => {
    calls += 1;
    return pin;
  });
  assert.equal(calls, 0);
  assert.equal(invalid.kind, "invalid");
  if (invalid.kind === "invalid") assert.deepEqual(invalid.invalidTokens, ["not-an-id"]);

  const empty = await resolveQuestionIdBatch(" \n,", 2, async () => pin);
  assert.equal(empty.kind, "empty");
  const tooMany = await resolveQuestionIdBatch("7K3M-79QP 2R5X-E7YA", 1, async () => pin);
  assert.equal(tooMany.kind, "overCapacity");

  const page = readFileSync(
    new URL(
      "../src/pages/assessment_workspace/assessment_workspace_questions_page.tsx",
      import.meta.url,
    ),
    "utf8",
  );
  const view = readFileSync(
    new URL(
      "../src/pages/assessment_workspace/assessment_workspace_questions_view.tsx",
      import.meta.url,
    ),
    "utf8",
  );
  assert.equal(page.includes("addQuestionsById"), true);
  assert.equal(page.includes("resolveQuestionIdBatch"), true);
  assert.equal(view.includes("Add by Question ID"), true);
  assert.equal(view.includes("Add Question IDs"), true);
});
