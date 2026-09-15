import assert from "node:assert/strict";
import test from "node:test";

import {
  assessmentTemplateDraft,
  assessmentTemplateSettings,
  assessmentTypeHasOneAttempt,
} from "../src/pages/assessment_template_settings_model.ts";

function template(assessmentType, attemptLimit) {
  return {
    id: "00000000-0000-0000-0000-000000000123",
    name: "Genetics settings",
    assessmentType,
    editNumber: "1",
    settings: {
      instructions: "Show your reasoning.",
      assessmentAttemptTimeLimitSeconds: null,
      attemptLimit,
      lateWorkRule: "reject",
      activityRules: {
        assessmentAttemptGradeRule: "highest",
        questionPoolReuseRule: "reuseSelection",
        questionVariationRule: "newVariation",
        assessmentAttemptResumeRule: "resumable",
        assessmentQuestionDisplayRule: "oneQuestionAtATime",
        assessmentNavigationRule: "freeNavigation",
        assessmentQuestionOrderRule: "shuffled",
      },
      studentFeedbackReleaseRule: {
        score: "after_submit",
        per_item_correctness: "after_submit",
        submitted_response: "after_submit",
        question_feedback: "never",
        question_answer: "never",
        question_answer_explanation: "never",
        class_statistics: "never",
      },
    },
  };
}

test("Quiz and Exam Template drafts and payloads impose exactly one Attempt", () => {
  for (const assessmentType of ["quiz", "exam"]) {
    const draft = assessmentTemplateDraft(template(assessmentType, 7));
    assert.equal(assessmentTypeHasOneAttempt(assessmentType), true);
    assert.equal(draft.attemptLimit, "1");
    assert.equal(
      assessmentTemplateSettings({ ...draft, attemptLimit: "99" }).settings?.attemptLimit,
      1,
    );
  }
});

test("regular Template unlimited Attempts remain unlimited without completion gating", () => {
  const draft = assessmentTemplateDraft(template("regular_assignment", null));
  const settings = assessmentTemplateSettings(draft).settings;

  assert.equal(assessmentTypeHasOneAttempt("regular_assignment"), false);
  assert.equal(settings?.attemptLimit, null);
});
