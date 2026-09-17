// assessment_template_settings_model.ts - local controls for one Assessment Template.

import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateSettings } from "../../generated/api/AssessmentTemplateSettings";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import { optionalPositiveIntegerDraft } from "./assessment_workspace/assessment_workspace_policy_model";

export function assessmentTypeHasOneAttempt(assessmentType: AssessmentType): boolean {
  return assessmentType === "quiz" || assessmentType === "exam";
}

export interface AssessmentTemplateDraft {
  readonly name: string;
  readonly assessmentType: AssessmentType;
  readonly instructions: string;
  readonly timeLimit: string;
  readonly attemptLimit: string;
  readonly lateWorkRule: LateWorkRule;
  readonly variationRule: AssessmentTemplateSettings["activityRules"]["questionVariationRule"];
  readonly orderRule: AssessmentTemplateSettings["activityRules"]["assessmentQuestionOrderRule"];
  readonly feedback: StudentFeedbackReleaseRule;
}

export type AssessmentTemplateDraftPatch = Partial<AssessmentTemplateDraft>;

export interface AssessmentTemplateDraftResult {
  readonly settings?: AssessmentTemplateSettings;
  readonly error?: string;
}

/** Retains every server-projected value in editable control form. */
export function assessmentTemplateDraft(template: AssessmentTemplate): AssessmentTemplateDraft {
  const rules = template.settings.activityRules;
  return {
    name: template.name,
    assessmentType: template.assessmentType,
    instructions: template.settings.instructions,
    timeLimit:
      template.settings.assessmentAttemptTimeLimitSeconds === null
        ? ""
        : (template.settings.assessmentAttemptTimeLimitSeconds / 60).toString(),
    attemptLimit: assessmentTypeHasOneAttempt(template.assessmentType)
      ? "1"
      : (template.settings.attemptLimit?.toString() ?? ""),
    lateWorkRule: template.settings.lateWorkRule,
    variationRule: rules.questionVariationRule,
    orderRule: rules.assessmentQuestionOrderRule,
    feedback: { ...template.settings.studentFeedbackReleaseRule },
  };
}

/** Builds the exact six-field settings payload only when conditional controls are valid. */
export function assessmentTemplateSettings(
  draft: AssessmentTemplateDraft,
): AssessmentTemplateDraftResult {
  const timeLimitSeconds = draft.timeLimit === "" ? null : Math.round(Number(draft.timeLimit) * 60);
  const validTimeLimit =
    draft.timeLimit === "" ||
    (/^(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)$/u.test(draft.timeLimit) &&
      Number(draft.timeLimit) >= 1 / 60 &&
      Number(draft.timeLimit) <= 720);
  const attemptLimit = optionalPositiveIntegerDraft(
    assessmentTypeHasOneAttempt(draft.assessmentType) ? "1" : draft.attemptLimit,
  );
  if (!validTimeLimit || !attemptLimit.valid) {
    return {
      error:
        "Enter a duration override in minutes, from 1/60 minute (1 second) to 720 minutes (12 hours), or leave it blank for the calculated default. Fractional minutes are rounded to the nearest second. Attempt limits must be positive whole numbers or blank.",
    };
  }

  const settings: AssessmentTemplateSettings = {
    instructions: draft.instructions,
    assessmentAttemptTimeLimitSeconds: timeLimitSeconds,
    attemptLimit: attemptLimit.value,
    lateWorkRule: draft.lateWorkRule,
    activityRules: {
      questionVariationRule: draft.variationRule,
      assessmentQuestionOrderRule: draft.orderRule,
    },
    studentFeedbackReleaseRule: { ...draft.feedback },
  };
  return { settings };
}

export function assessmentTemplateNameError(name: string): string | undefined {
  if (name.trim() === "") return "Enter a Template name.";
  if (name !== name.trim()) return "Remove spaces before or after the Template name.";
  if ([...name].length > 200) return "Use 200 characters or fewer for the Template name.";
  return undefined;
}
