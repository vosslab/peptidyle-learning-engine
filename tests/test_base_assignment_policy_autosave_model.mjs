import assert from "node:assert/strict";
import test from "node:test";

import {
  allBaseAssignmentPolicyEditsPersisted,
  baseAssignmentPolicyDraftChanged,
  baseAssignmentPolicyReloaded,
  baseAssignmentPolicyRequest,
  baseAssignmentPolicyRetry,
  baseAssignmentPolicySaveError,
  baseAssignmentPolicySaveSucceeded,
  createBaseAssignmentPolicyAutosaveState,
} from "../src/pages/assignment_workspace/base_assignment_policy_autosave_model.ts";

const draft = {
  instructions: "Read carefully.",
  dueAt: null,
  availableAt: null,
  closesAt: null,
  lateWorkRule: "reject",
  assignmentAttemptTimeLimitSeconds: 60,
  attemptLimit: null,
  activityRules: {
    assignmentCompletionRule: { kind: "answerAll" },
    assignmentAttemptGradeRule: "highest",
    assignmentAttemptContinuationRule: { kind: "unlimited" },
    questionPoolReuseRule: "reuseSelection",
    questionVariationRule: "newVariation",
    assignmentAttemptResumeRule: "resumable",
    assignmentQuestionDisplayRule: "oneQuestionAtATime",
    assignmentNavigationRule: "freeNavigation",
    assignmentQuestionOrderRule: "authoredOrder",
  },
  studentFeedbackReleaseRule: {
    score: "afterSubmit",
    per_item_correctness: "afterSubmit",
    submitted_response: "afterSubmit",
    question_feedback: "afterSubmit",
    question_answer: "afterSubmit",
    question_answer_explanation: "afterSubmit",
    class_statistics: "never",
  },
};

test("Base Assignment Policy invalid drafts remain visible and block Release", () => {
  const invalid = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    { ...draft, attemptLimit: 0 },
    false,
  );
  assert.equal(invalid.persistence, "invalid");
  assert.equal(allBaseAssignmentPolicyEditsPersisted(invalid), false);
  assert.equal(invalid.inFlightSeq, undefined);
});

test("Base Assignment Policy coalesces a later valid draft behind one request", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const laterDraft = { ...draft, instructions: "Updated." };
  const later = baseAssignmentPolicyDraftChanged(saving, laterDraft, true);
  assert.equal(later.persistence, "saving");
  assert.equal(later.inFlightSeq, saving.inFlightSeq);
  assert.equal(later.pending.instructions, "Updated.");
  assert.equal(allBaseAssignmentPolicyEditsPersisted(later), false);
  const next = baseAssignmentPolicySaveSucceeded(later, saving.inFlightSeq, draft, true);
  assert.equal(next.persistence, "saving");
  assert.equal(baseAssignmentPolicyRequest(next)?.input.instructions, "Updated.");
  const saved = baseAssignmentPolicySaveSucceeded(next, next.inFlightSeq, laterDraft, true);
  assert.equal(saved.persistence, "saved");
});

test("stale success retains a newer visible draft and stays unsaved", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const later = baseAssignmentPolicyDraftChanged(
    saving,
    { ...draft, instructions: "Newer." },
    true,
  );
  const stale = baseAssignmentPolicySaveSucceeded(later, saving.inFlightSeq + 1, draft, true);
  assert.equal(stale, later);
  assert.equal(stale.draft.instructions, "Newer.");
  assert.equal(stale.persistence, "saving");
});

test("success while the visible draft is invalid advances accepted state but remains invalid", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const invalid = baseAssignmentPolicyDraftChanged(saving, { ...draft, attemptLimit: 0 }, false);
  const result = baseAssignmentPolicySaveSucceeded(invalid, saving.inFlightSeq, draft, false);
  assert.equal(result.persistence, "invalid");
  assert.equal(result.lastAccepted.instructions, draft.instructions);
  assert.equal(allBaseAssignmentPolicyEditsPersisted(result), false);
});

test("a rejected save retains the visible draft and a later edit sends normally", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const rejected = baseAssignmentPolicySaveError(saving, saving.inFlightSeq, "rejected");
  assert.equal(rejected.persistence, "rejected");
  const edited = baseAssignmentPolicyDraftChanged(
    rejected,
    { ...draft, instructions: "Revised." },
    true,
  );
  assert.equal(baseAssignmentPolicyRequest(edited)?.input.instructions, "Revised.");
});

test("a failed save retries the current visible draft", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const edited = baseAssignmentPolicyDraftChanged(
    saving,
    { ...draft, instructions: "Current." },
    true,
  );
  const failed = baseAssignmentPolicySaveError(edited, saving.inFlightSeq, "failed");
  const retried = baseAssignmentPolicyRetry(failed, true);
  assert.equal(baseAssignmentPolicyRequest(retried)?.input.instructions, "Current.");
});

test("a conflict retains the draft until reload replaces it with server state", () => {
  const saving = baseAssignmentPolicyDraftChanged(
    createBaseAssignmentPolicyAutosaveState(draft),
    { ...draft, instructions: "Local." },
    true,
  );
  const conflict = baseAssignmentPolicySaveError(saving, saving.inFlightSeq, "conflict");
  const reloaded = baseAssignmentPolicyReloaded(conflict, { ...draft, instructions: "Server." });
  assert.equal(conflict.persistence, "conflict");
  assert.equal(conflict.draft.instructions, "Local.");
  assert.equal(reloaded.persistence, "saved");
  assert.equal(reloaded.draft.instructions, "Server.");
});
