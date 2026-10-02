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
    assessmentTemplateEditNumber: "1",
    settings: {
      instructions: "Show your reasoning.",
      assessmentAttemptTimeLimitSeconds: null,
      attemptLimit,
      lateWorkRule: "reject",
      activityRules: {
        questionVariationRule: "newVariation",
        assessmentQuestionOrderRule: "shuffled",
      },
      studentFeedbackReleaseRule: {
        score: "after_submit",
        per_item_correctness: "after_submit",
        submitted_response: "after_submit",
        question_answer: "never",
        question_answer_explanation: "never",
        class_statistics: "never",
        hints: "never",
        worked_solutions: "never",
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

test("Template duration saves exact whole minutes and rejects fractional-minute drafts", () => {
  const exact = template("regular_assignment", null);
  exact.settings.assessmentAttemptTimeLimitSeconds = 5_400;
  const exactDraft = assessmentTemplateDraft(exact);
  assert.equal(exactDraft.timeLimit, "90");
  assert.equal(
    assessmentTemplateSettings(exactDraft).settings?.assessmentAttemptTimeLimitSeconds,
    5_400,
  );

  assert.equal(
    assessmentTemplateSettings({ ...exactDraft, timeLimit: "2" }).settings
      ?.assessmentAttemptTimeLimitSeconds,
    120,
  );
  assert.match(
    assessmentTemplateSettings({ ...exactDraft, timeLimit: "1.5" }).error ?? "",
    /whole number/u,
  );
});
