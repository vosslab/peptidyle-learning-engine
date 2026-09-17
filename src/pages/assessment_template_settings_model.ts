// assessment_template_settings_model.ts - local controls for one Assessment Template.

import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateSettings } from "../../generated/api/AssessmentTemplateSettings";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import {
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
} from "../assessment_duration";
import { optionalPositiveIntegerDraft } from "./assessment_workspace/assessment_workspace_policy_model";

export function assessmentTypeHasOneAttempt(assessmentType: AssessmentType): boolean {
  return assessmentType === "quiz" || assessmentType === "exam";
}

export interface AssessmentTemplateDraft {
  readonly name: string;
  readonly assessmentType: AssessmentType;
  readonly instructions: string;
  readonly timeLimit: string;
  /** Blocks an unrelated save from silently clearing a legacy non-minute duration. */
  readonly legacyTimeLimitSeconds: number | null;
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
    timeLimit: assessmentDurationOverrideMinutesDraft(
      template.settings.assessmentAttemptTimeLimitSeconds,
    ),
    legacyTimeLimitSeconds:
      template.settings.assessmentAttemptTimeLimitSeconds !== null &&
      template.settings.assessmentAttemptTimeLimitSeconds % 60 !== 0
        ? template.settings.assessmentAttemptTimeLimitSeconds
        : null,
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
  const timeLimitSeconds = assessmentDurationOverrideSecondsFromMinutesDraft(draft.timeLimit);
  const timeLimitError = assessmentDurationOverrideMinutesError(
    draft.timeLimit,
    draft.legacyTimeLimitSeconds,
  );
  const attemptLimit = optionalPositiveIntegerDraft(
    assessmentTypeHasOneAttempt(draft.assessmentType) ? "1" : draft.attemptLimit,
  );
  if (timeLimitSeconds === undefined || timeLimitError !== undefined || !attemptLimit.valid) {
    return {
      error: timeLimitError ?? "Attempt limits must be positive whole numbers or blank.",
    };
  }

  const settings: AssessmentTemplateSettings = {
    instructions: draft.instructions,
    assessmentAttemptTimeLimitSeconds: timeLimitSeconds ?? null,
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
