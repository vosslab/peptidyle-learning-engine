import assert from "node:assert/strict";
import test from "node:test";

import {
  allBaseAssessmentPolicyEditsPersisted,
  baseAssessmentPolicyDraftChanged,
  baseAssessmentPolicyReloaded,
  baseAssessmentPolicyRequest,
  baseAssessmentPolicyRetry,
  baseAssessmentPolicySaveError,
  baseAssessmentPolicySaveSucceeded,
  createBaseAssessmentPolicyAutosaveState,
} from "../src/pages/assessment_workspace/base_assessment_policy_autosave_model.ts";

const draft = {
  instructions: "Read carefully.",
  dueAt: null,
  availableAt: null,
  closesAt: null,
  lateWorkRule: "reject",
  assessmentAttemptTimeLimitSeconds: 60,
  attemptLimit: null,
  activityRules: {
    assessmentCompletionRule: { kind: "answerAll" },
    assessmentAttemptGradeRule: "highest",
    assessmentAttemptContinuationRule: { kind: "unlimited" },
    questionPoolReuseRule: "reuseSelection",
    questionVariationRule: "newVariation",
    assessmentAttemptResumeRule: "resumable",
    assessmentQuestionDisplayRule: "oneQuestionAtATime",
    assessmentNavigationRule: "freeNavigation",
    assessmentQuestionOrderRule: "authoredOrder",
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

test("Base Assessment Policy invalid drafts remain visible and block Release", () => {
  const invalid = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    { ...draft, attemptLimit: 0 },
    false,
  );
  assert.equal(invalid.persistence, "invalid");
  assert.equal(allBaseAssessmentPolicyEditsPersisted(invalid), false);
  assert.equal(invalid.inFlightSeq, undefined);
});

test("Base Assessment Policy coalesces a later valid draft behind one request", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const laterDraft = { ...draft, instructions: "Updated." };
  const later = baseAssessmentPolicyDraftChanged(saving, laterDraft, true);
  assert.equal(later.persistence, "saving");
  assert.equal(later.inFlightSeq, saving.inFlightSeq);
  assert.equal(later.pending.instructions, "Updated.");
  assert.equal(allBaseAssessmentPolicyEditsPersisted(later), false);
  const next = baseAssessmentPolicySaveSucceeded(later, saving.inFlightSeq, draft, true);
  assert.equal(next.persistence, "saving");
  assert.equal(baseAssessmentPolicyRequest(next)?.input.instructions, "Updated.");
  const saved = baseAssessmentPolicySaveSucceeded(next, next.inFlightSeq, laterDraft, true);
  assert.equal(saved.persistence, "saved");
});

test("stale success retains a newer visible draft and stays unsaved", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const later = baseAssessmentPolicyDraftChanged(
    saving,
    { ...draft, instructions: "Newer." },
    true,
  );
  const stale = baseAssessmentPolicySaveSucceeded(later, saving.inFlightSeq + 1, draft, true);
  assert.equal(stale, later);
  assert.equal(stale.draft.instructions, "Newer.");
  assert.equal(stale.persistence, "saving");
});

test("success while the visible draft is invalid advances accepted state but remains invalid", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const invalid = baseAssessmentPolicyDraftChanged(saving, { ...draft, attemptLimit: 0 }, false);
  const result = baseAssessmentPolicySaveSucceeded(invalid, saving.inFlightSeq, draft, false);
  assert.equal(result.persistence, "invalid");
  assert.equal(result.lastAccepted.instructions, draft.instructions);
  assert.equal(allBaseAssessmentPolicyEditsPersisted(result), false);
});

test("a rejected save retains the visible draft and a later edit sends normally", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const rejected = baseAssessmentPolicySaveError(saving, saving.inFlightSeq, "rejected");
  assert.equal(rejected.persistence, "rejected");
  const edited = baseAssessmentPolicyDraftChanged(
    rejected,
    { ...draft, instructions: "Revised." },
    true,
  );
  assert.equal(baseAssessmentPolicyRequest(edited)?.input.instructions, "Revised.");
});

test("a failed save retries the current visible draft", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    draft,
    true,
  );
  const edited = baseAssessmentPolicyDraftChanged(
    saving,
    { ...draft, instructions: "Current." },
    true,
  );
  const failed = baseAssessmentPolicySaveError(edited, saving.inFlightSeq, "failed");
  const retried = baseAssessmentPolicyRetry(failed, true);
  assert.equal(baseAssessmentPolicyRequest(retried)?.input.instructions, "Current.");
});

test("a conflict retains the draft until reload replaces it with server state", () => {
  const saving = baseAssessmentPolicyDraftChanged(
    createBaseAssessmentPolicyAutosaveState(draft),
    { ...draft, instructions: "Local." },
    true,
  );
  const conflict = baseAssessmentPolicySaveError(saving, saving.inFlightSeq, "conflict");
  const reloaded = baseAssessmentPolicyReloaded(conflict, { ...draft, instructions: "Server." });
  assert.equal(conflict.persistence, "conflict");
  assert.equal(conflict.draft.instructions, "Local.");
  assert.equal(reloaded.persistence, "saved");
  assert.equal(reloaded.draft.instructions, "Server.");
});
