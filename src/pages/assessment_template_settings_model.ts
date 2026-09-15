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
  readonly gradeRule: AssessmentTemplateSettings["activityRules"]["assessmentAttemptGradeRule"];
  readonly poolReuseRule: AssessmentTemplateSettings["activityRules"]["questionPoolReuseRule"];
  readonly variationRule: AssessmentTemplateSettings["activityRules"]["questionVariationRule"];
  readonly resumeRule: AssessmentTemplateSettings["activityRules"]["assessmentAttemptResumeRule"];
  readonly displayRule: AssessmentTemplateSettings["activityRules"]["assessmentQuestionDisplayRule"];
  readonly navigationRule: AssessmentTemplateSettings["activityRules"]["assessmentNavigationRule"];
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
    timeLimit: template.settings.assessmentAttemptTimeLimitSeconds?.toString() ?? "",
    attemptLimit: assessmentTypeHasOneAttempt(template.assessmentType)
      ? "1"
      : (template.settings.attemptLimit?.toString() ?? ""),
    lateWorkRule: template.settings.lateWorkRule,
    gradeRule: rules.assessmentAttemptGradeRule,
    poolReuseRule: rules.questionPoolReuseRule,
    variationRule: rules.questionVariationRule,
    resumeRule: rules.assessmentAttemptResumeRule,
    displayRule: rules.assessmentQuestionDisplayRule,
    navigationRule: rules.assessmentNavigationRule,
    orderRule: rules.assessmentQuestionOrderRule,
    feedback: { ...template.settings.studentFeedbackReleaseRule },
  };
}

/** Builds the exact six-field settings payload only when conditional controls are valid. */
export function assessmentTemplateSettings(
  draft: AssessmentTemplateDraft,
): AssessmentTemplateDraftResult {
  const timeLimit = optionalPositiveIntegerDraft(draft.timeLimit);
  const attemptLimit = optionalPositiveIntegerDraft(
    assessmentTypeHasOneAttempt(draft.assessmentType) ? "1" : draft.attemptLimit,
  );
  if (!timeLimit.valid || !attemptLimit.valid) {
    return { error: "Enter positive whole-number limits, or leave them blank." };
  }

  const settings: AssessmentTemplateSettings = {
    instructions: draft.instructions,
    assessmentAttemptTimeLimitSeconds: timeLimit.value,
    attemptLimit: attemptLimit.value,
    lateWorkRule: draft.lateWorkRule,
    activityRules: {
      assessmentAttemptGradeRule: draft.gradeRule,
      questionPoolReuseRule: draft.poolReuseRule,
      questionVariationRule: draft.variationRule,
      assessmentAttemptResumeRule: draft.resumeRule,
      assessmentQuestionDisplayRule: draft.displayRule,
      assessmentNavigationRule: draft.navigationRule,
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
